import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const root = ".output/public";
const base = "/incular/";
const data = JSON.parse(readFileSync(join(root, "api/docs.json"), "utf8"));

assert(data.pages.length > 0, "Documentation index is empty");
assert(!existsSync(".output/server"), "Build must contain only static output");
for (const page of data.pages) {
  const path = page.slugs.join("/");
  assert(
    existsSync(join(root, "docs", path, "index.html")),
    `Missing HTML page: ${path}`,
  );
  assert(
    existsSync(join(root, "docs", `${path || "index"}.md`)),
    `Missing Markdown page: ${path}`,
  );
}
for (const path of ["index.html", "404.html", "llms.txt", "llms-full.txt"]) {
  assert(existsSync(join(root, path)), `Missing static export: ${path}`);
}
JSON.parse(readFileSync(join(root, "api/search.json"), "utf8"));

let links = 0;
for (const file of readdirSync(root, { recursive: true })) {
  if (!file.endsWith(".html")) continue;
  const html = readFileSync(join(root, file), "utf8");
  for (const [, href] of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
    if (!href.startsWith("/") || href.startsWith("//")) continue;
    assert(href.startsWith(base), `URL outside Pages base in ${file}: ${href}`);
    const path = decodeURIComponent(href.slice(base.length).split(/[?#]/)[0]);
    assert(
      existsSync(join(root, path)) ||
        existsSync(join(root, path, "index.html")),
      `Missing link target in ${file}: ${href}`,
    );
    links++;
  }
}
console.log(
  `Verified ${data.pages.length} documentation pages and ${links} local URLs.`,
);
