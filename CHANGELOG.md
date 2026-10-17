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

### Changed

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

### Fixed

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

The 0.1 API is experimental. Patch releases preserve compatible application
APIs; breaking changes require a new 0.x minor line.
