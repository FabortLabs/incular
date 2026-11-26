# Custom BoxBorder dispatch and BoxDecoration identity

The pinned Flutter 3.50.0 originals in
`painting/box_decoration_test.dart:63,69,79` now pair. Nullable
interpolation returns no decoration for two absent endpoints and keeps
the exact same borrowed decoration for a self-interpolation. A real
custom border interpolates widths 2 and 6 to 3 at 0.25 via its hook
and scales width 8 to 2. The native check observes the concrete custom
value, not a replacement with a uniform built-in border.

Custom borders provide dimensions, painting, scaling and endpoint
interpolation. Retained identity is preserved; a custom implementation
may also opt into value equality. Tests exercise receiver-first lerp,
fallback to a sender hook, absent endpoints and a zero-width custom
border that does not accidentally draw a hairline. Converting to a
square ShapeDecoration keeps its custom geometry, while circular and
rounded conversion take the top side of a uniform border, as Flutter
does.

An authored Flutter/native comparison mounts a child with a 3 px
leading border and changes LTR → RTL → LTR. A 10 × 8 child takes a
13 × 8 container. Padding follows the leading edge and exactly one
3 × 8 strip is painted there, with no opposite border. Native checks
use retained layout and generated renderer-neutral paint commands.
Eleven native tests, three pinned originals, and the authored
Flutter paint probe passed. This does not certify native raster
readback or all custom-border use on specialized sliver paths.
