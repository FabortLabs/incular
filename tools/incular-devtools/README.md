# incular-devtools-ui

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Standalone desktop DevTools application over the public facade and diagnostics protocol. |
| API class | application; this package is a tool, not a framework re-export surface. |
| Support | Available as an opt-in desktop tool; transport/model limits are enforced independently of application UI state. |

The tool opts into the exact facade features it needs (desktop, controls,
material, and devtools) instead of inheriting the facade default set.

## Workspace

The native workbench uses compact top navigation and independently scrolling
widget-tree and detail panes. The inspector keeps widget identity, dimensions,
and detail tabs visible while inspecting long property lists. It opens directly
on the widget inspector. Overview shows CPU
frame statistics, display-budget misses, memory inventory, and recent diagnostics.

- **Widget inspector:** virtualized rows with indent guides, typed labels and
  child counts; a draggable tree/details divider; selection history and clickable
  ancestor breadcrumbs; subtree isolation; and search across collapsed branches.
  Combine `type:Text`, `text:hello`, `key:save`, `id:…`, and `has:children` filters.
  Enter selects the first match; Up/Down move through matches and wrap around.
  Reveal selection clears the filter and restores the widget's tree context.
  Properties are filterable and grouped into collapsible sections. Properties the
  target exposes as editable edit in place with typed editors: numbers (field,
  -/+ steppers, ArrowUp/Down with Shift for x10, drag the label to scrub, a slider
  for small bounded ranges, clamped to the advertised min/max, and an infinity
  toggle for unbounded maxima), colours (swatch with a 12-colour palette plus a
  `#RRGGBB`/`#RRGGBBAA` field), enums (segmented control, menu, or a 3x3 grid for
  alignments), insets (four fields with Link), booleans (switch) and text. Edits
  are sent at up to 30 Hz. Overridden rows show an accent dot and an Undo button
  that restores the value first seen in this session; Reset all overrides clears
  every override, and a rejected edit shows the target's message inline. Layout includes a retained
  box model, constraints, clipping, and layout decisions. Signals, invalidation
  causes, semantics, and overlays have their own persistent tabs.
- **Target picker:** click Pick widget or press Ctrl+Shift+C, hover the target to
  preview geometry, and click to inspect. The chosen widget is revealed in the
  tree. While picking, the target swallows every click, so the state is
  impossible to miss: the button reads "Picking… (Esc)" in red, a matching pill
  appears in the header of every view and the status bar explains it. Picking
  ends on a pick, Escape (in either window), clicking the button again,
  switching views or disconnecting.
- **Hover highlight:** like a browser's element inspector, the target outlines a
  widget only while the pointer is over its row (tree, breadcrumbs, Top rebuilds,
  Semantics and Layers rows, expensive-work rows) and clears it on leaving the
  row, switching views, scrolling the tree or disconnecting. Selecting never
  leaves an overlay behind. The **Highlight on hover** toggle (inspector toolbar
  and Overlays panel, on by default) turns it off entirely.
- **Live FPS:** the header pill shows frames presented in the last second of
  wall-clock time with a sparkline of the last 60 frame CPU times; it decays to
  0 and reads "idle" when the app stops redrawing (a static UI presents no
  frames). Click it to open Performance, which labels its chart LIVE or
  RECORDING; frames stream into the chart without pressing Record.
- **Overlays:** the inspector's Overlays tab and the header **Overlays** menu
  (reachable from any view, Esc closes) group the toggles into Hover highlight,
  Layout, Hit test & semantics, Repaint & rebuild, Performance (protocol 4
  targets) and Animation speed (Paused, 0.1x, 0.25x, 0.5x, 1x; the target clamps
  the scale to 0..1, so there are no faster presets).
- **Performance:** nearest-rank CPU p95, budget-aware jank rate, a frame chart
  (janky frames get a red cap and outline on their own bar), recording,
  selected ranges, responsive flamegraphs, and ranked trace work. Profiler
  modes are **Basic** (counts), **Timings** (phase timings) and **Deep**
  (per-widget traces).
- **Rendering** (protocol 3): text layout cache (hit rate, misses, reshapes,
  fallback runs, missing clusters), widget pipeline, semantics and compositor
  counters with per-second deltas, polled once a second while the view is open.
  **Top rebuilds** ranks elements by build count; selecting a row opens that
  widget in the inspector. **Reset deltas** restarts the rate window.
- **Semantics** and **Layers** (protocol 3): searchable, indented trees with a
  details pane (role, label, value, description, state and action chips,
  bounds; or layer kind, summary and bounds). Selecting a node outlines its
  bounds in the target window (`HighlightRect`); the outline clears on deselect
  and when you leave the view. Targets older than protocol 3 show a
  "requires a newer target" note instead.
- **Layers** hides identity transforms by default (toggle **Hide identity
  transforms**); their children move up for display and the rows that remain
  show as `Transform · identity` when revealed. The text-cache hit rate stays
  neutral ("cold") until 200 lookups can judge it.
- **Memory:** resource inventory cards and an A/B comparison table.
- **Console:** live text and severity filters; pause the display while target
  capture continues. Clearing the console also resets retained byte accounting.
- **Transport:** actual local DevTools request status, duration, and sent/received
  bytes, with failure filtering and a 256-entry bound. This view does not capture
  application HTTP traffic.
- **Application:** target identity, protocol, native windows, and connection retry.

## Themes and shortcuts

The accent is the Incular brand violet in both themes (success states stay
green). Hover is a subtle translucent overlay, pressed a darker one, and
keyboard focus draws a ring; all come from the palette, never literals. The
header mark is the native Incular logo (also the window icon). The header
button switches between the dark and light themes (labelled with
the theme it switches to). Start in a theme with
`INCULAR_DEVTOOLS_THEME=light|dark`; dark is the default. Both palettes are
contrast-checked (text at least 4.5:1, accents at least 3:1).

| Shortcut | Action |
| --- | --- |
| Ctrl+1 … Ctrl+9, Ctrl+0 | Switch to the 1st … 10th view in tab order (Overview is Ctrl+1, Application Ctrl+0) |
| Ctrl+PageDown / Ctrl+PageUp | Next / previous view |
| Ctrl+R | Refresh (or reconnect) |
| Ctrl+E | Export report |
| Ctrl+Shift+C | Pick a widget; Escape cancels |

Every shortcut needs Ctrl, so typing in a search field never triggers one. The
status bar shows the active view's shortcut.

**Export report** saves versioned JSON to `Downloads/Incular DevTools` (falling
back to the OS temporary directory) and shows the saved path in the status bar.
Reports contain retained frames, traces, snapshots, logs, transport metadata,
and the selected widget; discovery credentials and request bodies are omitted.

The header badge and status text always show one connection state: searching
for targets, connecting, connected, session busy, taken over, or disconnected
with a retry countdown. Plain disconnects reconnect automatically (1 to 8 s
backoff). A target accepts one DevTools window at a time: a second window sees
**Session busy**, and the first sees **Taken over** when another window uses
**Take over**. Neither state reconnects by itself. While not connected, every
view except Console, Transport and Application shows the connection workspace
with each live target (app name, pid, start time, **Connect**) and **Refresh
list**. Start a DevTools-enabled app with `INCULAR_DEVTOOLS=1` (agent only) or
`--devtools` (which also opens a DevTools window). Use
`--target-pid <pid>` to select an exact process. `INCULAR_DEVTOOLS_VIEW` can
select the initial view (`overview`, `widgets`, `performance`, `rendering`,
`semantics`, `layers`, `memory`, `console`, `transport`, or `application`).

## Native visual review

Run with a live target:

```text
cargo run -p incular-devtools-ui --example devtools_visual_review -- --target-pid <pid>
```

This opt-in review uses Incular's normal input pipeline and GPU frame capture
to exercise navigation, layout inspection, overlays, memory snapshots,
console controls, transport filters, and recording. PNG captures are saved to
`target/devtools-review`. Set `INCULAR_DEVTOOLS_REVIEW_DIR` to choose a capture
directory, and `INCULAR_DEVTOOLS_REVIEW_WIDTH` / `INCULAR_DEVTOOLS_REVIEW_HEIGHT`
to inspect another window size. Set `INCULAR_DEVTOOLS_REVIEW_OFFLINE=1` with a
nonexistent `--target-pid` to capture the connection workspace.
Set `INCULAR_DEVTOOLS_REVIEW_NEW=1` to visit Rendering (and its top-rebuild
selection), Semantics and Layers. With `INCULAR_DEVTOOLS_REVIEW_OFFLINE=1`,
`INCULAR_DEVTOOLS_REVIEW_HOLD=<ms>` delays the capture and
`INCULAR_DEVTOOLS_REVIEW_TAKEOVER=1` clicks **Take over** and captures again;
run two reviews against one target to capture the busy and taken-over states.
Set `INCULAR_DEVTOOLS_REVIEW_THEME=1` to capture the main views (inspector,
Performance with a recording, Rendering, Semantics, Layers, Memory, Console,
Transport, Application) in both themes as `dark-*.png` and `light-*.png`;
`INCULAR_DEVTOOLS_THEME` picks the starting theme and, with
`INCULAR_DEVTOOLS_REVIEW_OFFLINE=1`, the connection workspace is captured in
both. Performance needs a target that redraws continuously (for example
`perf_probe`); an idle target streams no frames.
Set `INCULAR_DEVTOOLS_REVIEW_INSPECTOR=1` for the shorter inspector review,
including large trees, keyboard navigation, picking, subtree isolation, overlays,
property groups, search, and reveal.

Round-2 review: set `INCULAR_DEVTOOLS_REVIEW_ROUND2=1` to capture, in both
themes, the header with live FPS, tree hover (DevTools window and, with
`.tmp/overlay-check/capture.ps1`, the target window while hovering, after
leaving and after selecting), the picker, picker cancelled by view switch and
Escape, the Overlays panel, animation speed and the header Overlays menu.
`INCULAR_DEVTOOLS_REVIEW_ROW=x,y` picks the hovered tree row. Set
`INCULAR_DEVTOOLS_REVIEW_STRESS=1` to click rapidly through every view, divider
drags, search, picking, overlays and theme switches and then prove the UI still
answers.
