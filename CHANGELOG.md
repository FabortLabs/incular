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

### Fixed

- EditableText now honors its resolved direction for alignment, painting,
  caret and pointer geometry, selection, and native input snapshots.

### Performance

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
- Relayout of many labels no longer reshapes them all: text layout pools
  grow when their working set does not fit, and a single unwrapped,
  start-aligned left-to-right line is reused at any width it still fits.
  Resizing a window over 3,000 labels fell from about 90 ms to 7 ms a frame.
- Paragraphs that wrap keep their shaped layout, so a new width re-breaks
  lines instead of reshaping the text.
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

The 0.1 API is experimental. Patch releases preserve compatible application
APIs; breaking changes require a new 0.x minor line.
