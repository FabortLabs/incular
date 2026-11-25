# Mouse routing, ListWheel lifecycle, and gradient sampling

Pinned Flutter 3.50.0 `ab2912035e`; Incular production `c647cb85`.

- Four named `widgets/mouse_region_test.dart` originals passed.
  Cursor routing and no extra frame on inert move match. The initial cursor-only
  comparisons matched effective cursor, fallback and enter counts, while
  Flutter invokes the paint delegate once, and Incular reuses existing
  paint (zero invocations). These are observable paint-lifecycle
  differences, not a broken OS cursor, native pixel mismatch, or a
  recommendation to remove a retained-paint optimization blindly.
  Rust target: `flutter350_mouse_region_parity`.
- Three named `widgets/list_wheel_scroll_view_test.dart` originals
  passed. First build emits no selection callback on both. Actual
  drag midpoint/reversal reproduces the prior touch gap. After
  reattaching the same controller without PageStorage, Flutter starts
  at configured item 3 (offset 300); Incular retains item 10 (1000)
  from the prior attachment. The Rust fixture specifies item extent
  to seed the same original state. Rust target:
  `flutter_list_wheel_upstream_parity`.
- Three named `painting/gradient_test.dart` originals passed
  (linear interpolation with unequal stops, radial scaling with
  transform, sweep interpolation with stops). These remain reference-only:
  Incular has shader descriptors but no corresponding public APIs for
  those descriptor operations. BoxDecoration's midpoint switch is
  not an equivalent interpolation API.

## Independent Windows wgpu gradient reproduction

Linear, centered radial and full sweep use explicit transparent blue
to opaque red stops over black. At t=.25/.5/.75, pinned Flutter shows
[16,0,48,255], [64,0,64,255], [143,0,48,255]; native Incular shows
[63,0,0,255], [128,0,0,255], [191,0,0,255]. Allow one byte of
readback tolerance. All opaque blue-to-red controls match. This is
one shared stop-sampling gap across three shapes:
`incular-rendering/src/gradients.rs:36–60` premultiplies endpoints
before interpolation, used for wgpu LUT generation. The pinned
`gradient_stop_sampling_test.dart` passes and the Windows live-test
`flutter350_gradient_stop_sampling` fails at the expected pixels.
No claim about descriptor interpolation or every GPU/platform.

## Retained cursor-only paint

Pinned `:1665` and `:1726` now match: the cursor updates or defers to its
parent under a stationary mouse, with one paint and no extra enter.
The previously cached CustomPainter had skipped that observable paint
even though the cursor was correct. An authored paired control confirms
that a changed region and its same-layer sibling both repaint once,
that an explicit nested RepaintBoundary stays cached, and that setting
the same cursor produces no replay. The tests are headless.
