import {createServer} from "node:http";
import {readFileSync} from "node:fs";
import {spawn} from "node:child_process";
import {fileURLToPath} from "node:url";

const [chrome, ...chromeOptions] = process.argv.slice(2);
if (!chrome) throw Error("Pass the Chrome executable");
const moduleName = process.env.INCULAR_TEST_WASM_MODULE ?? "browser_platform";
if (!/^[a-z_]+$/.test(moduleName)) throw Error("Invalid wasm module name");
const allowed = new Map([
  ["/", ["index.html", "text/html"]],
  [`/${moduleName}.js`, [`${moduleName}.js`, "text/javascript"]],
  [`/${moduleName}_bg.wasm`, [`${moduleName}_bg.wasm`, "application/wasm"]],
]);
const profile = fileURLToPath(new URL("./test-profile", import.meta.url));
let resolveResult;
const result = new Promise(resolve => { resolveResult = resolve });
const server = createServer(async (request, response) => {
  const {pathname} = new URL(request.url, "http://127.0.0.1");
  if (request.method === "POST" && pathname === "/result") {
    let content = "";
    for await (const chunk of request) {
      content += chunk.toString();
      if (content.length > 4096) {
        response.statusCode = 413;
        response.end();
        return;
      }
    }
    response.end("ok");
    resolveResult(content);
    return;
  }
  const asset = allowed.get(pathname);
  if (!asset) { response.statusCode = 404; response.end(); return; }
  response.setHeader("Content-Type", asset[1]);
  response.end(readFileSync(new URL(asset[0], import.meta.url)));
});
server.listen(0, "127.0.0.1");
await new Promise(resolve => server.once("listening", resolve));
const url = `http://127.0.0.1:${server.address().port}/`;
const browser = spawn(chrome, [
  "--headless", "--disable-background-networking", "--disable-dev-shm-usage",
  `--user-data-dir=${profile}`, ...chromeOptions, url,
], {stdio: "ignore"});
let timeout;
try {
  const observed = await Promise.race([
    result,
    new Promise(resolve => browser.once("exit", code => resolve(`Chrome exited: ${code}`))),
    new Promise(resolve => browser.once("error", error => resolve(String(error)))),
    new Promise(resolve => { timeout = setTimeout(() => resolve("No test result"), 15_000) }),
  ]);
  console.log(observed);
  if (!observed.startsWith("PASS\n")) process.exitCode = 1;
} finally {
  clearTimeout(timeout);
  server.close();
  browser.kill();
}
