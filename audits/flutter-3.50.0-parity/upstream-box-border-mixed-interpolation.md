# Mixed box-border interpolation

Pinned Flutter 3.50.0 `painting/border_rtl_test.dart:75` passed on Linux.
The Rust test exercises the physical, directional, and custom border cases
directly through `BoxBorderOutline::lerp`: wide and narrow horizontal sides,
shared top and bottom, absent endpoints in both positions, and samples before
and after the animation. Concrete kinds, side widths, stroke style and
alignment are checked; color comparisons allow one 8-bit channel step.

Both populated horizontal axes fade through an empty midpoint, rather than
drawing both sides at once. A separate native control checks that this midpoint
emits no physical or logical horizontal stroke in either writing direction.
No numeric raster or GPU golden is claimed. BoxDecoration shares these
interpolation rules and preserves decoration identity at endpoints. The Rust
`BoxBorder` alias remains the existing physical-border spelling.

Two mixed-border, four custom-border, and five adjacent decoration contracts
passed in the focused Rust run.
