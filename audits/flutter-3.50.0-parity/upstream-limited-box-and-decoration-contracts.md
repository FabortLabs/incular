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
