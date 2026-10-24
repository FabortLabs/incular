# incular-web

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Browser entry and page services over the shared desktop host. |
| API class | backend; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Experimental wasm32 canvas host; WebGPU validated in Chromium, WebGL2 is the fallback backend. |

Browser adapter for Incular's shared Winit host.

A browser page is one more Winit platform. `incular-desktop` owns the event
loop, input, WGPU renderer, accessibility and teardown paths on `wasm32`
exactly as it does natively; this crate only binds windows to page canvases
and reports page-level environment and lifecycle state. It does not duplicate
or source-include the shared host.

## Building

Enable the facade's `web` feature with default features disabled and build
for `wasm32-unknown-unknown`, then generate the JavaScript bindings with a
`wasm-bindgen-cli` matching the locked `wasm-bindgen` version:

```text
cargo build -p incular --example counter --profile dist \
  --target wasm32-unknown-unknown --no-default-features --features web,controls,material
wasm-bindgen --target web --out-dir dist \
  target/wasm32-unknown-unknown/dist/examples/counter.wasm
```

Ship the workspace's `dist` profile (size-optimized, fat LTO, stripped), not
`release`: the `counter` module is 6.0 MB (2.3 MB gzip) instead of 10.7 MB
(3.2 MB gzip). Running `wasm-opt -Oz` over the generated module and serving
it compressed shrink the download further. About 0.8 MB of the module is the
bundled Inter font (see `default-font` below).

Load the generated module from a page (`import init from "./counter.js";
await init();`) served over HTTP. `incular::run` returns as soon as the host is
installed: the page owns the event loop and the application keeps running in
browser callbacks.

Every repository example builds this way. Examples need only the `runner`
feature, which both `desktop-host` and `web-host` enable, so one source runs
natively and in a page.

## Portable application code

- Use `incular::time::{Instant, SystemTime}`; they are `std::time` natively.
  `std::time::Instant::now` panics on `wasm32-unknown-unknown`.
- Use `incular::runtime::sleep` in spawned tasks instead of a Tokio timer.
- Do not block or spawn threads: the page has one thread, and a blocked UI
  thread never sees its reply.

## Page integration

- `run_application` (and `incular::run`) draws the first window into a canvas
  that fills the viewport and follows browser resizes.
  `run_application_in(app, CanvasTarget::Element(id))` draws it into an
  existing `<canvas id=...>` instead, sized by the page's CSS. Either way the
  first window ignores its `initial_logical_size` and size limits.
- Later windows keep their logical size and float over the page in a
  cascade from the bottom-right corner, one canvas each. Users cannot move or
  close them; the application does, and a closed window's canvas is removed.
- The first window's title is the document title, and it takes keyboard
  focus at launch. Canvases set `touch-action: none` so touches reach gesture
  recognizers instead of panning or zooming the page, and file drops on them
  are refused instead of navigating away.
- Keyboard: browser shortcuts (F5, F11, F12, Ctrl/Cmd+R, Ctrl/Cmd+L, page zoom
  and developer tools) stay with the browser. Text fields use the macOS
  shortcut table when the browser runs on a Mac.
- Text input: while a text field is focused, a transparent `<textarea>` sits
  at its caret and holds the page focus, so input methods compose (preedit
  and commit reach the field as IME events) and touch devices show their
  on-screen keyboard, configured from the field's input type and action.
  Ordinary key presses still reach the application as key events.
- Screen readers: semantics are off until a screen reader activates the
  visually hidden "Enable accessibility" button at the start of the page, as
  in Flutter. From then on each window's semantic tree is mirrored as
  transparent elements with ARIA roles and states over its canvas. A screen
  reader's click activates a node (or focuses a text field), focusing a node
  focuses it in the application, Enter and Space activate, and other keys
  reach the application. The application's focused node takes the page
  focus.
- Browser history: give a `Router` a `BrowserRouteInformationProvider`. The
  router starts at the route in the URL, its navigations add history entries,
  and the back and forward buttons deliver the entry they reach. Routes live in
  the URL fragment (`UrlStrategy::Hash`, any static host) or the path
  (`UrlStrategy::Path`, which needs the server to answer every route with the
  page). Off the browser the provider is an in-memory one starting at `/`.
- Clipboard: plain text, HTML and images. Copy and cut write the system
  clipboard (images as PNG); Ctrl/Cmd+V pastes its text, HTML and image.
- Browsers expose no system fonts. The on-by-default `default-font` feature
  embeds Inter Regular and Bold (SIL OFL 1.1, about 830 KB) and registers them
  at launch; Inter covers Latin, Greek and Cyrillic. When text needs a
  character no registered font has, the Noto font for its script (Arabic,
  Hebrew, Indic and Southeast Asian scripts, Chinese, Japanese, Korean,
  symbols, monochrome emoji, ...) is fetched once from the jsDelivr CDN and
  the text is laid out again, as in Flutter. Shared ideographs follow the
  page language. `set_font_fallback(FontFallback::Mirror(url))` fetches the
  same files (`FALLBACK_FONT_FILES`) from your own host instead, and
  `FontFallback::Disabled` turns fetching off. Fonts registered with
  `incular::text::register_application_font(bytes)` fill the generic families
  (`sans-serif`, `monospace`, ...) and act as fallback for each other. Disable
  `default-font` to ship only your own fonts.
- Word selection and word-wise caret movement use the browser's
  `Intl.Segmenter`, which covers Chinese, Japanese and Thai; the text engine's
  own dictionaries are left out of browser builds.
- `RestorationConfig::file_backed` stores snapshots in the origin's
  `localStorage` (`BrowserRestorationStore`).
- `navigator.languages`, `prefers-reduced-motion` and `prefers-contrast` /
  `forced-colors` feed the runtime environment and are observed for changes.
  Winit reports the color scheme per window. A hidden page is `Suspended`
  (restoration is flushed, since the browser may discard it) and `Resumed`
  when shown again.
- Panics are routed to the browser console.

## Unsupported in browsers

These capabilities are reported `Unsupported` rather than emulated:
URI-list, file and custom clipboard formats, external drag and drop, native file dialogs, global shortcuts, trays,
notifications and taskbar state, single-instance ownership (launch fails with
a typed error when a policy is set), native transient popups (they stay in the
in-window overlay), window position, level, icon, decorations and display
geometry. Recursion crash logs have no filesystem to persist to, frame
capture is unavailable, and the blocking `Simulation` driver needs a second
thread. Direct Tokio access (`TokioHandle`) does not exist on
`wasm32`; application tasks run as page microtasks and `spawn_blocking` work
runs on the page thread after the current UI turn.

## Testing

`python scripts/web-smoke.py` builds every example for the browser and loads
each one in headless Chrome or Edge, with WebGPU and again with WebGL2,
failing on console errors, a missing context, no presented frame or a blank
screenshot. `--examples`, `--backends`, `--profile` and `--software`
(SwiftShader, for machines without a GPU) narrow or adapt the run. It runs
locally before a release, not in CI. Firefox and Safari are not covered by it.
