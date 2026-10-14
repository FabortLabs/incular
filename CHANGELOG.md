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

The 0.1 API is experimental. Patch releases preserve compatible application
APIs; breaking changes require a new 0.x minor line.
