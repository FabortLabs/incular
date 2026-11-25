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

## Container decoration and child clips

The typed builder previously dropped an explicit plain fill. The same
40×40 declaration via the fluent constructor painted it. The Rust regression
failed for the builder before the fix; afterward both constructors emit the
same full-size fill with no child. A pinned, authored Flutter sample checks
the center pixel with that decoration and no child.

The clip regression additionally showed a 100×60 circular Container clipping
its child to a 100×60 rectangle, and a child clip wrapping the 110×110 outer
box including its margin and blurred decoration shadow. With the repair, an
explicit BoxDecoration clips only its padded child. The 100×60 case has a
centered 60×60 circle; the retained shadow command precedes the 50×50 child
clip even when there is a 30-pixel outer margin. Without opt-in clipping,
the child still paints outside the circular outline. The simpler Incular shorthand without a
BoxDecoration retains its outer clipping behavior.

The pinned authored probe passes at both selected circle pixels with and
without clipping. Exact original Flutter registrations at
`widgets/container_test.dart:608,621,767` passed. These original cases are
only partially paired here: Incular uses its analytic specializations
instead of Flutter's literal ClipPath widget and compares the shadow paint
order rather than reproducing Flutter's golden. All 51 focused Rust checks passed. No Incular GPU pixel
comparison in this batch.
