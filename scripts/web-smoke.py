"""Browser smoke test: build every example for wasm32 and load each in headless Chrome.

An example fails when the page reports an error or panic, presents no frame,
or shows a blank canvas. Each runs on WebGPU and on the WebGL2 fallback.
Like the native GUI scenarios, this runs locally before release, not in CI.

Needs Python 3.10+, Chrome, Chromium or Edge, the wasm32-unknown-unknown target
and a wasm-bindgen-cli matching Cargo.lock:

    cargo binstall -y wasm-bindgen-cli@<version in Cargo.lock>
    python scripts/web-smoke.py [--examples counter,text] [--backends webgl]
"""
import argparse
import base64
import functools
import http.server
import json
import os
import re
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FEATURES = "web,controls,material"
TARGET = "wasm32-unknown-unknown"

# Records errors and presented frames before the application starts. The
# WebGL2 run hides `navigator.gpu`, as a browser without WebGPU would.
PROBE = """
window.__smoke = {errors: [], frames: 0, contexts: []};
const fail = message => window.__smoke.errors.push(String(message).slice(0, 2000));
const error = console.error.bind(console);
console.error = (...parts) => { fail(parts.join(" ")); error(...parts); };
addEventListener("error", event => fail("uncaught: " + (event.message || event.error)));
addEventListener("unhandledrejection", event => fail("rejection: " + event.reason));
if (window.GPUCanvasContext) {
  const texture = GPUCanvasContext.prototype.getCurrentTexture;
  GPUCanvasContext.prototype.getCurrentTexture = function (...args) {
    window.__smoke.frames++;
    return texture.apply(this, args);
  };
}
const getContext = HTMLCanvasElement.prototype.getContext;
HTMLCanvasElement.prototype.getContext = function (kind, ...args) {
  const context = getContext.call(this, kind, ...args);
  if (context) window.__smoke.contexts.push(kind);
  return context;
};
for (const name of ["drawArrays", "drawElements", "drawArraysInstanced", "drawElementsInstanced",
                    "drawRangeElements", "blitFramebuffer"]) {
  const draw = WebGL2RenderingContext.prototype[name];
  WebGL2RenderingContext.prototype[name] = function (...args) {
    window.__smoke.frames++;
    return draw.apply(this, args);
  };
}
if (BACKEND === "webgl") {
  Object.defineProperty(Navigator.prototype, "gpu", {get: () => undefined});
}
"""

PAGE = """<!doctype html>
<html><head><meta charset="utf-8"><title>{name}</title></head>
<body style="margin:0;background:#000">
<script type="module">import init from "./app.js"; await init();</script>
</body></html>
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--examples", help="comma-separated example names (default: all that build for the web)")
    parser.add_argument("--backends", default="webgpu,webgl", help="comma-separated: webgpu, webgl")
    parser.add_argument("--browser", help="path to Chrome, Chromium or Edge")
    parser.add_argument("--profile", default="release", help="Cargo profile (release, dist, ...)")
    parser.add_argument("--settle", type=float, default=4.0, help="seconds to let each page run")
    parser.add_argument("--software", action="store_true", help="render with SwiftShader (machines without a GPU)")
    parser.add_argument("--headed", action="store_true", help="show the browser window")
    parser.add_argument("--out", default=str(ROOT / "target" / "web-smoke"), help="pages and screenshots")
    args = parser.parse_args()

    out = Path(args.out)
    names = build(args.profile, args.examples, out)
    backends = [backend.strip() for backend in args.backends.split(",") if backend.strip()]
    browser = args.browser or find_browser()
    server, port = serve(out)
    failures = []
    try:
        with Chrome(browser, args.headed, args.software) as chrome:
            for backend in backends:
                for name in names:
                    result = chrome.check(f"http://127.0.0.1:{port}/{name}/", backend, args.settle)
                    (out / name / f"{backend}.png").write_bytes(result["screenshot"])
                    problems = result["problems"]
                    status = "ok" if not problems else "FAIL: " + "; ".join(problems)
                    print(f"{backend:7} {name:32} frames={result['frames']:<5} {status}", flush=True)
                    if problems:
                        failures.append(f"{backend}/{name}")
    finally:
        server.shutdown()
    print(f"\nScreenshots: {out}")
    if failures:
        print(f"{len(failures)} failed: " + ", ".join(failures))
        sys.exit(1)
    print(f"All {len(names) * len(backends)} page loads passed.")


def build(profile, only, out):
    lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
    wanted = re.search(r'name = "wasm-bindgen"\nversion = "([^"]+)"', lock).group(1)
    try:
        installed = subprocess.run(["wasm-bindgen", "--version"], capture_output=True, text=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError):
        sys.exit(f"wasm-bindgen not found; install it with: cargo binstall -y wasm-bindgen-cli@{wanted}")
    if wanted not in installed:
        sys.exit(f"wasm-bindgen {installed.strip()} does not match Cargo.lock {wanted}; "
                 f"install it with: cargo binstall -y wasm-bindgen-cli@{wanted}")

    command = ["cargo", "build", "-p", "incular", "--locked", "--target", TARGET, "--profile", profile,
               "--no-default-features", "--features", FEATURES]
    command += [f"--example={name}" for name in only.split(",")] if only else ["--examples"]
    print("Building: " + " ".join(command), flush=True)
    subprocess.run(command, cwd=ROOT, check=True)

    built = ROOT / "target" / TARGET / profile / "examples"
    names = sorted(path.stem for path in built.glob("*.wasm") if "-" not in path.stem)
    if only:
        names = [name for name in names if name in only.split(",")]
    if not names:
        sys.exit("no example was built for the web")
    for name in names:
        page = out / name
        page.mkdir(parents=True, exist_ok=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(page),
                        "--out-name", "app", str(built / f"{name}.wasm")], check=True)
        (page / "index.html").write_text(PAGE.format(name=name), encoding="utf-8")
    return names


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map,
                      ".wasm": "application/wasm", ".js": "text/javascript"}

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def log_message(self, *args):
        pass


def serve(directory):
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=str(directory)))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, server.server_address[1]


def find_browser():
    candidates = [shutil.which(name) for name in ("google-chrome", "chromium", "chromium-browser", "chrome", "msedge")]
    for variable in ("PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"):
        base = os.environ.get(variable)
        if base:
            candidates += [str(Path(base) / "Google/Chrome/Application/chrome.exe"),
                           str(Path(base) / "Microsoft/Edge/Application/msedge.exe")]
    candidates.append("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
    for candidate in candidates:
        if candidate and Path(candidate).is_file():
            return candidate
    sys.exit("no Chrome, Chromium or Edge found; pass --browser")


class Chrome:
    """A browser driven over the Chrome DevTools Protocol."""

    def __init__(self, path, headed, software):
        self.profile = tempfile.mkdtemp(prefix="incular-web-smoke-")
        flags = [f"--user-data-dir={self.profile}", "--remote-debugging-port=0", "--no-first-run",
                 "--no-default-browser-check", "--window-size=1024,768", "--enable-unsafe-webgpu"]
        if not headed:
            flags.append("--headless=new")
        if software:
            flags += ["--enable-unsafe-swiftshader", "--use-angle=swiftshader", "--use-webgpu-adapter=swiftshader"]
        self.process = subprocess.Popen([path, *flags, "about:blank"],
                                        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        port_file = Path(self.profile) / "DevToolsActivePort"
        deadline = time.time() + 30
        while not port_file.is_file() or len(port_file.read_text().splitlines()) < 2:
            if time.time() > deadline or self.process.poll() is not None:
                raise RuntimeError("the browser did not open a DevTools port")
            time.sleep(0.1)
        port, path = port_file.read_text().splitlines()[:2]
        self.socket = WebSocket("127.0.0.1", int(port), path)
        self.next_id = 0
        self.events = []

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.socket.close()
        self.process.kill()
        self.process.wait()
        shutil.rmtree(self.profile, ignore_errors=True)

    def call(self, method, params=None, session=None):
        self.next_id += 1
        message = {"id": self.next_id, "method": method, "params": params or {}}
        if session:
            message["sessionId"] = session
        self.socket.send(json.dumps(message))
        while True:
            reply = json.loads(self.socket.receive())
            if reply.get("id") == self.next_id:
                if "error" in reply:
                    raise RuntimeError(f"{method}: {reply['error']}")
                return reply.get("result", {})
            self.events.append(reply)

    def check(self, url, backend, settle):
        target = self.call("Target.createTarget", {"url": "about:blank"})["targetId"]
        session = self.call("Target.attachToTarget", {"targetId": target, "flatten": True})["sessionId"]
        try:
            self.call("Runtime.enable", session=session)
            self.call("Page.enable", session=session)
            self.call("Page.addScriptToEvaluateOnNewDocument",
                      {"source": f"const BACKEND = {json.dumps(backend)};\n{PROBE}"}, session=session)
            self.events.clear()
            self.call("Page.navigate", {"url": url}, session=session)
            time.sleep(settle)
            state = self.call("Runtime.evaluate", {"expression": "JSON.stringify(window.__smoke)",
                                                   "returnByValue": True}, session=session)
            smoke = json.loads(state.get("result", {}).get("value")
                               or '{"errors": ["page did not load"], "frames": 0, "contexts": []}')
            screenshot = base64.b64decode(
                self.call("Page.captureScreenshot", {"format": "png"}, session=session)["data"])
            viewport = self.call("Page.getLayoutMetrics", session=session)["cssLayoutViewport"]
            # A half-scale copy keeps the pure-Python blank check fast.
            thumbnail = base64.b64decode(self.call("Page.captureScreenshot", {
                "format": "png",
                "clip": {"x": 0, "y": 0, "width": viewport["clientWidth"],
                         "height": viewport["clientHeight"], "scale": 0.5},
            }, session=session)["data"])
        finally:
            self.call("Target.closeTarget", {"targetId": target})
        problems = list(dict.fromkeys(smoke["errors"]))
        problems += [f"exception: {event['params']['exceptionDetails'].get('text', '')} "
                     f"{event['params']['exceptionDetails'].get('exception', {}).get('description', '')}".strip()
                     for event in self.events
                     if event.get("method") == "Runtime.exceptionThrown" and event.get("sessionId") == session]
        expected = "webgpu" if backend == "webgpu" else "webgl2"
        if expected not in smoke["contexts"]:
            problems.append(f"no {expected} context (got {', '.join(smoke['contexts']) or 'none'})")
        if smoke["frames"] == 0:
            problems.append("no frame presented")
        elif is_blank(thumbnail):
            problems.append("blank screenshot")
        return {"problems": problems, "frames": smoke["frames"], "screenshot": screenshot}


def is_blank(png):
    """Whether fewer than 50 pixels differ from the dominant color; a single
    short line of text is enough to pass."""
    width, height, pixels, stride = decode_png(png)
    counts = {}
    for row in pixels:
        for x in range(0, width * stride, stride):
            color = bytes(row[x:x + 3])
            counts[color] = counts.get(color, 0) + 1
    return sum(counts.values()) - max(counts.values()) < 50


def decode_png(data):
    """Decodes the 8-bit, non-interlaced RGB(A) PNGs Chrome produces."""
    position, chunks = 8, []
    width = height = color_type = None
    while position < len(data):
        length, kind = struct.unpack(">I4s", data[position:position + 8])
        body = data[position + 8:position + 8 + length]
        if kind == b"IHDR":
            width, height, depth, color_type, _, _, interlace = struct.unpack(">IIBBBBB", body)
            if depth != 8 or interlace or color_type not in (2, 6):
                raise ValueError("unsupported PNG layout")
        elif kind == b"IDAT":
            chunks.append(body)
        position += 12 + length
    stride = 4 if color_type == 6 else 3
    raw = zlib.decompress(b"".join(chunks))
    rows, previous, offset = [], bytearray(width * stride), 0
    for _ in range(height):
        kind, row = raw[offset], bytearray(raw[offset + 1:offset + 1 + width * stride])
        offset += 1 + width * stride
        for i in range(len(row)):
            left = row[i - stride] if i >= stride else 0
            up = previous[i]
            corner = previous[i - stride] if i >= stride else 0
            if kind == 1:
                row[i] = (row[i] + left) & 0xFF
            elif kind == 2:
                row[i] = (row[i] + up) & 0xFF
            elif kind == 3:
                row[i] = (row[i] + (left + up) // 2) & 0xFF
            elif kind == 4:
                estimate = left + up - corner
                distances = abs(estimate - left), abs(estimate - up), abs(estimate - corner)
                predictor = left if distances[0] <= distances[1] and distances[0] <= distances[2] \
                    else up if distances[1] <= distances[2] else corner
                row[i] = (row[i] + predictor) & 0xFF
        rows.append(row)
        previous = row
    return width, height, rows, stride


class WebSocket:
    """Just enough RFC 6455 for one local DevTools connection."""

    def __init__(self, host, port, path):
        self.connection = socket.create_connection((host, port))
        key = base64.b64encode(os.urandom(16)).decode()
        self.connection.sendall((f"GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\n"
                                 f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n"
                                 "Sec-WebSocket-Version: 13\r\n\r\n").encode())
        response = b""
        while b"\r\n\r\n" not in response:
            response += self.connection.recv(4096)
        if b" 101 " not in response.split(b"\r\n", 1)[0]:
            raise RuntimeError("DevTools refused the websocket upgrade")
        self.buffer = response.split(b"\r\n\r\n", 1)[1]

    def send(self, text):
        payload = text.encode()
        header = bytearray([0x81])
        if len(payload) < 126:
            header.append(0x80 | len(payload))
        elif len(payload) < 1 << 16:
            header += bytes([0x80 | 126]) + struct.pack(">H", len(payload))
        else:
            header += bytes([0x80 | 127]) + struct.pack(">Q", len(payload))
        mask = os.urandom(4)
        self.connection.sendall(bytes(header) + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(payload)))

    def receive(self):
        message = b""
        while True:
            first, second = self.read(2)
            length = second & 0x7F
            if length == 126:
                length = struct.unpack(">H", self.read(2))[0]
            elif length == 127:
                length = struct.unpack(">Q", self.read(8))[0]
            payload = self.read(length)
            opcode = first & 0x0F
            if opcode == 0x8:
                raise RuntimeError("DevTools closed the connection")
            if opcode in (0x0, 0x1, 0x2):
                message += payload
                if first & 0x80:
                    return message.decode()

    def read(self, count):
        while len(self.buffer) < count:
            chunk = self.connection.recv(1 << 16)
            if not chunk:
                raise RuntimeError("DevTools connection ended")
            self.buffer += chunk
        data, self.buffer = self.buffer[:count], self.buffer[count:]
        return data

    def close(self):
        self.connection.close()


if __name__ == "__main__":
    main()
