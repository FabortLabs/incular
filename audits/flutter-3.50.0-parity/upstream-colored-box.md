# ColoredBox paint, zero size, and edge coverage

Pinned source: `widgets/basic.dart`, `_RenderColoredBox`, and
`widgets/basic_test.dart` (1105, 1124, 1150, 1163, 1179, 1187, 1196, 1944).
These eight registrations verify painting and render configuration:

- A childless ColoredBox takes the incoming minimum without creating a
  placeholder child. When either extent is zero it emits no filled
  rectangle. The child's painter still runs when a child exists.
- Positive area emits one exact solid rectangle before the child, with
  no border and the same child paint origin. A transparent color still
  emits a paint operation and claims its half-open hit-test bounds.
- Reports its own color and type. Antialiasing defaults to true; changing
  it on a retained box repaints without changing size or re-laying out.
- Wgpu's unaliased mode uses a direct rectangle for translation and an
  affine quad with full coverage and no fringe for nontranslation.
  Ordinary antialiased rects keep the existing rendering paths.

The old ColoredBox lowered to a DecoratedBox with a synthetic child when
empty, emitted a rounded-rectangle primitive even at zero size, and had
no antialiasing setting. Native integration:
`incular-runtime --test flutter350_colored_box`. The separate Flutter
golden registration at 1209 remains open; command, hit-test, and mesh
checks are not a cross-renderer framebuffer comparison.

Nine native integration tests, seven preview tests, the targeted basic
builder contract, four FittedBox and four Hero integration tests, and
the single pure wgpu hard-edge mesh test passed. The driver-independent
wgpu library check also passed. No workspace test was run.
