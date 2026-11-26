# Sliver semantics annotations

The ten pinned Flutter originals at `widgets/sliver_semantics_widget_test.dart:
18,73,114,210,232,251,298,342,386,437` passed.

- **Reading direction (210, 232, 251, 298).** The real retained node
  inherits the ambient RTL/LTR direction. An explicit direction overrides
  it in either direction, as the original assertions show despite the
  test names. The two inherited cases are paired; the tests for explicit
  direction also assert Flutter's complete two-pane tree.
- **Reparenting (114).** Two annotations on one sliver previously collided
  on the same generated child ID, rejecting layout. The native test inserts
  and removes a nested container and verifies exact edges, stale node
  removal, and a stable outer node. The native fixture does not model
  Flutter's GlobalKey-based render-object move or two-pane graph.
- **Annotation content (342, 386, 437).** The native graph includes label,
  hint, and current value on one node. Independent hints merge in order
  with one newline for both sliver and box annotations. The field and
  merging checks pass; the renderer-specific outer topology is partial.
- **Disable/restart (18).** The host projection removes the label when
  disabled, schedules a frame when re-enabled, and publishes the fresh
  node with the same direction beneath an implicit-scroll ancestor.
  The native host stays idle once the update is presented.

Incular currently projects one scroll semantic node per viewport; it
does not split Flutter's scrolling and non-scrolling panes or install
the internal `RenderViewport.twoPane` tag. The direct viewport tag
constraint at line 73 remains reference-only. The use of the renderer's
reserved anchor IDs changes neither leaf child IDs nor paint/hit routing.
