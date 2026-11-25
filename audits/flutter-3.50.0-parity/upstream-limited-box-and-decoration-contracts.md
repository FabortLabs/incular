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

## BoxDecoration blend equality and validation repair

The exact equality registration at `painting/box_decoration_test.dart:187`
now pairs transparent-color decorations using the actual Color and
Difference blend modes. The image-only validity and self-interpolation
regressions were reproduced in Rust, then fixed. An image does not make a
background blend valid without a fill or gradient. Interpolating a
decoration with itself preserves the blend; distinct instances with the
same value still interpolate normally. The blend is absent at the
midpoint for those distinct instances.

The original identity registration (:63) remains partial: the Rust
value-returning API preserves the full value on self-interpolation but
does not provide Flutter's nullable inputs or return the identical object.
All 18 focused Rust tests, two exact pinned originals, and four authored
Flutter value/validation cases passed. No raster claim.

## Container decoration hit testing

All four tap results from pinned `widgets/container_test.dart:666` now
match: a background decoration, a foreground decoration, and a color
each activate an enclosing default gesture detector; a padded, centered,
rotated layout-only Container does not. Prior Rust failed to activate a
foreground decoration and a shape-only circle at its center.

Decoration hit testing now checks local geometry after children get the
first chance to accept a hit. The separate circle probes hit the center
but not the side of the box for an empty decoration. A painted child can
still take a side tap unless child clipping is explicitly enabled. The
rounded corner control rejects the top-left point. The exact Flutter
registration, three authored Flutter probes, and 33 focused Rust tests
passed. The new behavior is not a raster comparison.

## Childless and zero-area Container layout

Flutter drops the Align when a childless Container has no tight explicit
constraint. This was verified structurally against the mounted Incular tree;
an extra control verifies that tight explicit sizing still keeps the Align.
The childless width-only probe now reports 40×60 in loose 100×60 bounds,
where Incular previously reported 40×0. Bounded axes fill available space;
an unbounded axis keeps its parent's finite minimum. The exact vertical
ListView case completes successfully with a 100×0 child, and the two
zero-area decoration cases also complete at 0×0.

All four exact original Flutter cases at `widgets/container_test.dart:558,
730,792,806`, all four authored Flutter samples, and 24 focused Rust tests
passed. The original zero-area assertions only check size and successful
layout, not rendered text in an empty area.

## Container transform alignment

The original `widgets/container_test.dart:567` pairs all four unchanged
layout corners of the 100×100 Container placed at (100,100). Authored
comparisons also measure the transformed child at scale 0.5: (150,125)
for center-right, (100,125) for RTL center-end, and (100,100) with no pivot.
The child stays laid out at 100×100 and paints in a 50×50 box. The retained
Rust control changes Directionality while mounted. All 18 focused Rust
tests, the exact Flutter original, and five authored Flutter probes passed.
