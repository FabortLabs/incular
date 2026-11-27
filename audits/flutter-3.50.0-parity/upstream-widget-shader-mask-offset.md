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
