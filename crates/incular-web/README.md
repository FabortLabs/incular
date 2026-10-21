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
cargo build -p incular --example counter --release \
  --target wasm32-unknown-unknown --no-default-features --features web,controls,material
wasm-bindgen --target web --out-dir dist \
  target/wasm32-unknown-unknown/release/examples/counter.wasm
```

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
- Clipboard: plain text. Copy and cut write the system clipboard; Ctrl/Cmd+V
  pastes its text. Other formats are unsupported.
- Browsers expose no system fonts. The on-by-default `default-font` feature
  embeds Inter Regular and Bold (SIL OFL 1.1, about 830 KB) and registers them
  at launch. Inter covers Latin, Greek and Cyrillic; register fonts for other
  scripts and emoji with `incular::text::register_application_font(bytes)`.
  Registered families also fill the empty generic families (`sans-serif`,
  `monospace`, ...). Disable `default-font` to ship only your own fonts.
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
non-text clipboard formats, external drag and drop, native file dialogs, global shortcuts, trays,
notifications and taskbar state, single-instance ownership (launch fails with
a typed error when a policy is set), native transient popups (they stay in the
in-window overlay), window position, level, icon, decorations and display
geometry. Recursion crash logs have no filesystem to persist to, frame
capture is unavailable, and the blocking `Simulation` driver needs a second
thread. Direct Tokio access (`TokioHandle`) does not exist on
`wasm32`; application tasks run as page microtasks and `spawn_blocking` work
runs on the page thread after the current UI turn.

Not implemented yet: input method composition and on-screen keyboards
(Winit's browser backend has no IME), screen reader access (AccessKit has no
browser adapter), and browser history integration for routes.
