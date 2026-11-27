# Widget shader-mask bounds

Pinned Flutter 3.50.0 `widgets/shader_mask_test.dart:23,28` passed on Linux.
Incular mounts the default mask around the original unpainted SizedBox
without drawing a rectangle. The size-constrained callback receives
`(0, 0, 400, 400)` in an 800×600 view, not its on-screen origin or the
100×100 size requested by the child. The shader uses the original vertical
gradient and its two stops at 0.1 and 0.35.

A painted Rust control verifies the flattened compositor receives one
modulate mask with the same local gradient and bounds positioned at
`(200, 100)`. Both new tests and the 15 adjacent shader/backdrop
contracts passed. This does not match the separate screenshot golden
at line 50; no raster equivalence is claimed here.

## Live transform and cached pixels

The authored `shader_mask_live_transform_test.dart` captures actual Flutter
pixels across translation and reflection. An opaque, two-color mask follows
the child, swaps sides after reflection, and leaves the old position and
neighboring pixels untouched.

Incular's flattened stage used to keep its resolve-time world transform
while child bounds moved. The layer now combines the resolved local mask
with its live world transform at flatten time. The mounted native comparison
exercises the same four frames, checking the unchanged local callback,
the exact mask bounds, and both transformed sample locations. Thus the
callback is still invoked only once. In wgpu the same target-relative
transform controls both source placement and cache identity: moving an
integer number of device pixels can reuse the isolated result, but a
reflection or fractional phase change cannot. The focused wgpu cache
control and the neighboring compositor and widget tests passed. This is
not an Incular native surface pixel capture; a local display socket was
unavailable in this environment. This authored comparison adds no
upstream registration credit.

## Rotated coverage and child overflow

The pinned Flutter `shader_mask_rotated_overflow_test.dart` draws a
100×100 mask over a white child extending 20 px to the left. It checks
an aligned control, a 45° rotation, and a fully transparent rotated
mask. Samples include the immediately adjacent covered and uncovered
pixels at the hard edge. The overflow stays white outside the actual
quad; it is not clipped to the mask bounds.

The Incular mounted test checks that the overflow is in the group and
inside the mask's broad scissor, but outside its quad. The wgpu
offscreen readback uses production pipeline resources and geometry. An
AABB-only control reproduced the old erased overflow. The corrected
blend preserves the overflow on llvmpipe GL and leaves genuinely
covered pixels transparent when the shader is transparent. Pixel-aligned
rectangles keep their simple scissor; rotated and fractional rectangles
use independent hard stencil coverage. The mask fill itself uses
non-antialiased geometry, matching the pinned engine. The shader-blend,
clip geometry and four mounted shader-mask controls passed. This is an
authored probe and does not credit the separate positioning golden.

## No child and reattachment

The authored pinned Flutter probe mounts a mask without a child, adds
one, removes it, and adds it again. The same render object sizes to
zero with no child under loose constraints. It has no mask layer and
does not invoke the shader until the child exists. Removal drops the
layer; reattachment creates a fresh one. The Incular test compares
these transitions and adds a tight-constraint control where size does
not change, so layer creation and callback resolution cannot depend
only on a size change. These are authored checks, not extra original
tests in the coverage count.
