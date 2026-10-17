# Changelog

Maintainers: record user-visible changes under `Unreleased`, then move them to
a version and date when publishing.

## Unreleased

The first 0.1 release is still being prepared; no release is claimed here.

### Added

- Declarative native UI with reactive state, retained layout/painting, input,
  scrolling, text, navigation, and optional Controls/Material presentation.
- Shared Winit/WGPU desktop host and native Windows/Linux/macOS adapters.
- Android/iOS semantic adapters; complete mobile host integration remains pending.
- Opt-in local DevTools and a separate unpublished desktop inspector.
- `devtools_playground` example: one app that exercises every DevTools feature
  (continuous motion, jank and rebuild-storm toggles, keyed editable widgets,
  named editable signals, scroll/layout/clip/transform layers, text and
  semantics samples). Run with `--features devtools`.
- `incular::devtools::log(LogLevel, target, message)`: application log messages
  appear in the DevTools Console. Records go to a bounded, thread-safe queue
  (1024 entries, oldest dropped and reported, 4 KiB per message) that the
  desktop host forwards as `Log` events and wakes for while idle. It is a no-op
  without the `devtools` feature.
- `devtools_playground` Console page with buttons that log info, warning, error
  and a 100-message burst; `INCULAR_PLAYGROUND_LOG_DEMO=1` adds a background
  heartbeat log.
- Opt-in `hot-reload` with `dx serve --hot-patch`; `examples/hot_reload` is a
  ready-to-serve playground.
- Self-contained crate packaging, a published hello example, user guides,
  docs.rs metadata, and build-only desktop CI.
- `DocumentEditor` in Controls: a `TextArea` for long documents such as source
  files and logs. It lays its text out line by line through the new opt-in
  `EditableText::paragraph_layout`, so an edit lays out only the lines it
  changes. Text gains `TextDocumentLayout` (`TextEngine::layout_paragraphs`)
  and the `TextGeometry` trait that it and `TextLayout` implement; `TextLayout`
  keeps its Flutter-shaped API.
- DevTools protocol 3 (older protocol 1 and 2 peers still connect): read-only
  `GetRuntimeDiagnostics` (widget pipeline, full text-cache, semantics and
  compositor counters), bounded `GetSemanticsTree` and `GetLayerTree`,
  `GetWorkCounts` (per-element rebuild ranking) and `HighlightRect`. `Hello`
  gains `take_over`, and `ErrorCode` gains `SessionBusy`.
- DevTools protocol 4 (protocols 1 to 3 still connect): `DebugProperty` gains
  `options`, `min`, `max` and `step` editor metadata, and `DebugOption` gains
  `PerformanceOverlay`; every addition decodes with a default on older peers.
- DevTools property editing: Text (text, fontSize, color), DecoratedBox and
  Container (background, borderWidth, borderColor, radius), Padding (padding),
  SizedBox and ConstrainedBox (width, height, min/max extents), Row and Column
  (mainAxisAlignment, crossAxisAlignment, mainAxisSize, spacing), Wrap spacing,
  Align (alignment), Opacity, Visibility and Box (color). Each edit reaches the
  retained element through the normal update path, so it invalidates exactly
  what the property affects. An override is re-applied when the application's
  own rebuild replaces the widget, until it is reset. Editable properties carry
  `options`, `min`, `max` and `step`.
- DevTools target overlays: phase flashes now fade out over about 500 ms
  (the host keeps redrawing the window until they finish) and draw a 2 px
  outline per changed element with a light fill on the innermost ones; Repaint
  Rainbow draws a persistent outline whose colour rotates on every repaint;
  layout bounds and the other geometry overlays are outlines instead of stacked
  fills; the selected or hovered node is highlighted Chrome-style (blue content
  box, green padding band, edge outline and a `Type  W x H` label); and a new
  Performance overlay draws a CPU and GPU frame graph in the window's top-right.
- DevTools UI connection life cycle: a single state (searching, connecting,
  connected, session busy, taken over, disconnected with a retry countdown)
  drives the header badge, status text and console. Plain disconnects reconnect
  with 1 to 8 s backoff; a busy or taken-over session never reconnects on its
  own and offers an explicit take over. Failures are logged once per change
  instead of on every dropped request. The UI also stores the v3 diagnostics,
  semantics, layer and work-count responses and polls diagnostics while the
  Rendering view is open.
- DevTools UI views for protocol 3: Rendering (text cache, widget pipeline,
  semantics and compositor counters with per-second deltas, plus a Top rebuilds
  table that opens the widget in the inspector) and searchable Semantics and
  Layers trees with details panes whose selection outlines the node in the
  target window. The offline workspace lists every live target with Connect,
  Take over and Refresh list.

- DevTools UI light theme (header toggle, `INCULAR_DEVTOOLS_THEME`), view
  shortcuts (Ctrl+1 to Ctrl+0, Ctrl+PageUp/PageDown, Ctrl+R, Ctrl+E), a Layers
  toggle that hides identity transforms, and a theme pass in the visual review.

### Changed
- DevTools Performance view redesigned: profiler mode as a segmented control
  with a prominent Record/Stop toggle, a fixed-width per-frame chart stacked by
  phase (build, layout, paint, semantics, composite, other) with a budget line,
  jank flags, legend and click-to-select frames, a selected-frame breakdown
  table, and a proportional flamegraph and ranked work list that show widget
  type names.

- Simplified framework defaults while preserving all existing default values.
- Upgraded Parley from 0.7 to 0.12 and passed resolved paragraph direction into
  text shaping, including mixed-script bidirectional text.
- Trailing non-breaking spaces retain width; trailing ideographic spaces no
  longer affect alignment width, matching Flutter.
- DevTools discovery files use private exclusive creation; idle HTTP upgrades
  time out and cancel on shutdown, and browser-origin upgrades are rejected.
- Minimum supported Rust version is 1.89 to include the Linux notification stack.
- `TextClusterSpan::start` and `end` are `u32` byte offsets, like
  `GlyphPosition::cluster`; `TextClusterSpan::range` returns them as a
  `usize` range.
- DevTools UI: header shows live FPS (trailing-second frame count that decays
  to 0 and reads "idle" when the app stops redrawing) with a frame-time
  sparkline; Performance labels its chart LIVE or RECORDING.
- DevTools UI: Overlays tab regrouped (Hover highlight, Layout, Hit test &
  semantics, Repaint & rebuild, Performance, Animation speed) with short
  descriptions, and also available from a header Overlays menu in every view.
- DevTools UI: Incular brand mark in the header, connection and empty states
  and as the window icon; accent shifted to the brand violet in both themes
  (success states stay green); hover, pressed and focus colours centralised in
  the palette with a visible keyboard-focus ring.

### Fixed
- Dispatching UI work from another thread no longer panics in debug builds
  when the UI thread drains the message before the sender counts it; the
  queue counter is incremented before sending.
- Reading-order focus traversal no longer panics with "comparison function
  does not correctly implement a total order" on staggered layouts; members
  are grouped into rows before being ordered left to right.
- DevTools: `SetAnimationSpeed` and `SetProfilerMode` replies now serialize
  (as `{"scale": …}` / `{"mode": …}`) instead of failing with
  `InternalError`, and the animation time scale accepts speeds up to 4×
  (`MAX_ANIMATION_TIME_SCALE`) as well as slow motion.
- DevTools UI: the target is outlined only while the pointer is over a row
  (with a Highlight on hover toggle); selecting a widget no longer leaves a
  persistent tint over the application. The overlay clears on leaving, view
  switches, scrolling and disconnects.
- DevTools UI: the widget picker, which makes the target swallow all clicks, now
  ends on view switches, Escape, a second click and disconnects, and is shown as
  "Picking… (Esc)" in the header, toolbar and status bar. A panic while the
  inspector model is locked no longer silently disables every later UI update.
- DevTools Select Widget mode no longer lets an unpaired pointer release reach
  the application after a pick, swallows every pointer event while active,
  keeps selecting after a press over empty space, ends when the window loses
  focus, and draws a border around the captured window.

- DevTools UI polish: janky frames are marked on their own bar instead of a
  full-height band, profiler modes are Basic/Timings/Deep so "Performance" is
  unambiguous, a cold text cache is no longer shown as a red 0.0% hit rate,
  and the compositor card reads "Live layers".
- DevTools UI prunes stale discovery records on Windows with a real process
  liveness check (`OpenProcess` + exit code) instead of probing the recorded
  loopback port; macOS keeps the port probe.
- DevTools UI no longer sits on "connecting" next to an OFFLINE badge with a
  stream of `request admission failed` errors when the target is busy or gone;
  discovery records of exited processes are also pruned on Windows and macOS by
  probing the recorded loopback port.
- DevTools targets no longer leave a second inspector hanging: a concurrent
  authenticated client is told immediately that the session is busy (or takes
  it over with `take_over`). Stale discovery records for exited processes are
  skipped and removed, and the memory view now reports compositor layers.
- EditableText now honors its resolved direction for alignment, painting,
  caret and pointer geometry, selection, and native input snapshots.
- Hover follows content that moves under a still pointer. After a frame whose
  layout or scroll offsets changed, the runtime hit-tests the last pointer
  position again, as Flutter's `MouseTracker` does: `MouseRegion` enter and
  exit callbacks fire and button hover moves to the new row after a
  programmatic or wheel scroll, with no hover event sent.
- Editing long multi-line text no longer retains every edited version. The
  text engine kept each composed document in its paragraph cache, which grows
  with the longest file, so every keystroke in a 5,000-line `TextArea` kept a
  full copy of the text and its layout alive. Only the four most recent
  compositions are now kept.

### Performance

- The semantics pass is incremental: invalidation marks render objects and
  the path to them, and the pass enters only marked subtrees, keeping every
  clean subtree's retained nodes. An idle frame of 2,000 buttons with
  semantics on allocates 13 times instead of 16,040. The AccessKit and mobile
  projections read only the nodes changed since their last sync instead of
  copying and comparing the whole tree.
- A resize frame no longer copies each render object's child list, allocates
  an offset vector per node, formats a text-call label per text layout, or
  re-records unchanged pictures: paint records into one reused buffer and a
  layer keeps its picture when the content is equal. 2,000 buttons resize
  with 4,020 allocations instead of 60,024 (semantics off).
- Text layouts of files longer than 2,048 lines no longer reshape every
  paragraph on each layout: the cache is now LRU, grows to fit the longest
  document, and returns unchanged documents whole. `TextEngine::with_shared`
  gives application measurement code a cache that persists across frames.
- Editing unwrapped multi-line text with `Clip`/`Fade` overflow and a fixed
  width reshapes only the changed paragraph instead of the whole string.
- The WGPU renderer skips glyph runs wholly outside the active clip before
  atlas lookup, instead of emitting an instance per off-screen glyph. A long
  scrolled document no longer costs GPU preparation for every line per frame.
- Caret placement in a long text layout derives caret stops for the one line
  it needs instead of every line of the document after each edit.
- A pointer event hit-tests the tree once and shares the result between
  mouse regions, listeners, tap regions, gestures, text fields, selectable
  text and buttons; it used to walk the tree up to six times. The cursor
  lookup after input walks it once instead of twice. A pointer move that
  changes nothing over 2,000 buttons in a scroll view fell from about
  0.47 ms to 0.08 ms, and the cursor lookup from 0.16 ms to 0.08 ms.
- Relayout of many labels no longer reshapes them all: text layout pools
  grow when their working set does not fit, and a single unwrapped,
  start-aligned left-to-right line is reused at any width it still fits.
  Resizing a window over 3,000 labels fell from about 90 ms to 7 ms a frame.
- Paragraphs that wrap keep their shaped layout, so a new width re-breaks
  lines instead of reshaping the text.
- Text layout pools are bounded by approximate bytes as well as entries
  (32 MiB by default, `TextEngine::set_cache_byte_budget`), and pools grown
  by a burst shrink back once a smaller working set hits steadily. After a
  burst growing the pools to 20,000 labels, the engine's retained memory with
  a 100-label working set fell from 32.8 MiB, never released, to 3.3 MiB,
  with no extra misses for that working set. A document being composed keeps
  its paragraphs regardless of the budget.
- Editing a long document carries the lines before the edit from the previous
  layout instead of rebuilding them; typing at the end of a 5,000-line field
  fell from about 9 ms to about 1 ms a keystroke.
- A multi-line `EditableText` paints only the lines inside its viewport. In a
  5,000-line field, arrow keys, drag selection and caret blinking now cost a
  quarter or less of what they did.
- Typing in a 5,000-line `DocumentEditor` costs about 0.4 ms a keystroke
  wherever the caret is, against about 10 ms near the top of a `TextArea`.
- Paint enters only the render subtrees that hold a pending repaint, and
  flattening culls a whole off-screen layer subtree by its memoized picture
  bounds instead of testing every picture inside it. A frame over 2,000
  buttons in a scroll view spent about 3.8 ms in paint whether idle or after
  a hover; it now spends about 0.08 ms idle and 0.11 ms after a hover.
- Controls and Material widgets read their theme through the new `Builder`
  (Flutter's context-only builder) and `Widget::stateful_builder` instead of
  a `LayoutBuilder`, so they rebuild when the theme or their state changes,
  not when their constraints do. Resizing a column of 308 controls rebuilt
  2,156 elements a frame and now rebuilds none; layout fell from about
  2.3 ms to 0.3 ms a frame.
- Shaped cluster spans take 20 bytes instead of 32, so a laid-out line no
  longer spends more on cluster spans than on glyphs. A 59-glyph label
  retains about 3.3 KB in the text engine instead of 4.0 KB.
- Text layout cache hits no longer allocate: lookups hash and compare a key
  that borrows the text, and only an insert copies it. A layout reused at a
  new width it still fits is no longer inserted again under that width. Over
  2,000 labels, a pass of cache hits fell from 2,000 allocations to none,
  and three resize passes from 18,005 to none.
- Undo history stores each step as the edited range instead of a full copy
  of the text. A hundred keystrokes in a 5,000-line field retained 28.8 MB of
  history and now retain 0.3 MB. A text field without a restoration binding
  no longer serializes its whole text on every commit.
- Moving, resizing or scrolling content updates only the bounds of semantic
  nodes whose role, text, state and actions did not change, instead of
  rebuilding every node below the change. A resize or scroll frame over 500
  Controls buttons fell from 3,023 allocations to 23.
- Input events no longer format a diagnostic label each time; the label is
  rendered only when a recursion report needs it. A pointer move that
  changes nothing now allocates nothing.
- Reconciliation no longer copies child lists while invalidating sliver
  measurement, validating new subtrees or materializing layout builders, and
  checks a few keyed children without a hash set. Scrolling a 10,000-row
  builder list fell from 763 to 588 allocations a frame.
- An unchanged scene no longer rebuilds its flattened display list every
  frame: display lists share their commands, and flattening reuses a retained
  buffer and returns the previous list when nothing changed. Scrollbar fade
  pumping and flex layout also stopped allocating temporary vectors. An idle
  frame over 300 buttons fell from 12 allocations to 1, a 1 px resize from 22
  to 16.
- Text fields no longer copy their whole editing value in every layout and
  paint, and `TextStyle::default` no longer allocates. A 1 px resize over 50
  text fields fell from about 712 to 512 allocations a frame. The new
  `TextEditingController::with_value` lends the value without copying it.

The 0.1 API is experimental. Patch releases preserve compatible application
APIs; breaking changes require a new 0.x minor line.
