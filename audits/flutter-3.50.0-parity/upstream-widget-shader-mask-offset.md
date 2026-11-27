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
