# LimitedBox axis limits and BoxDecoration blend contracts

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

LimitedBox geometry matches the exact width-only and height-only
unbounded-axis cases (:59, :75). Both unbounded axes and no-child
layout (:13, :92) also match coordinates, with the Dart debug string
and optional child API unpaired. See `flutter_limited_box_geometry_parity`.

Two original BoxDecoration registrations passed. Self-interpolation
(:63) was rechecked on the same decoration instance with an explicit
filled Multiply blend in both frameworks. Flutter preserves it,
Incular loses it at the midpoint (the distinct-endpoint and distinct
blend controls pass). Original equality (:187) can only be partially
compared with the Rust blend enum; the scoped distinct modes remain
distinguishable in the interpolation controls. A separate pinned
fixture also verifies an image-only decoration with a background blend
fails Flutter debug validation while Rust accepts it. Color plus image
plus blend, image alone, and empty-fill controls agree. This is public
value/validation behavior, not a raster claim. See
`flutter350_box_decoration_contracts` and
`box_decoration_contracts_test.dart`.

## BoxDecoration clip and hit geometry

Pinned original registrations at `painting/box_decoration_test.dart:154,166,177`
and all their sample points match. On a 100×20 box the circular clip is
centered at (50,10) with radius 10, not an ellipse covering the full box.
The hit test includes the top and left tangencies inside the box; the
rounded clip excludes the sampled corners. A separate Rust control
compares translated, scaled-down elliptical corner geometry for clipping
and hits. All 11 focused Rust tests and all three named Flutter originals
passed on Linux. These are geometry checks, not painted pixel comparisons.
