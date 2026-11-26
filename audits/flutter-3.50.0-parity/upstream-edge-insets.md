# Physical and directional painting insets

Seven exact upstream cases passed on the pinned Flutter 3.50.0 reference
(`painting/edge_insets_test.dart:12,80,115,228,358,370,423`). Incular now
explicitly resolves physical edges unchanged in either direction, preserves
unspecified edges on a copy, interpolates an absent endpoint from zero,
and performs modulo and integer division independently on all four edges.
The negative controls distinguish Euclidean remainder from signed Rust
remainder and truncation from flooring.

The numeric control also checks all four signed corners, opposite-side
flipping, both totals, and exact rectangle and size inflation and deflation.
It is partial against Flutter's broader diagnostic/hash test: Incular
exposes valid render sizes only, returns None for a negative or non-finite
extent, and does not emulate Dart diagnostic/hash output. No existing
layout normalization or rendering behavior was changed. Six focused
Rust inset cases and two interpolation controls passed.

```sh
cargo test --locked --offline -j1 -p incular-config --test flutter350_edge_insets_parity --test lerp_extrapolation -- --test-threads=1
```

## Independent axes in rounded borders

Six pinned rounded-rectangle registrations and all 18 focused native
rounded-geometry, box-border, and table-border tests passed. The public
`EllipticalRoundedRect` preserves the authored corner radii separately
from their paint-time normalization. Inflation and deflation apply the
two adjacent physical sides to each corner, clamp each axis at zero,
and preserve all edges on zero adjustments and a reversible round trip.

An authored raster probe showed a less obvious border case. A 3px
inside border on a 5×4 rounded box paints the edge but retains a white
center. Flutter orders the inverted inner bounds at raster conversion.
A truly flat inner rectangle (6×6 or 6×7 outside) has no hole. At 7×7
there is again a one-pixel hole. Incular's retained path now distinguishes
inverted and exactly empty interiors rather than panicking on a
negative size or filling the center incorrectly. Rust checks point
coverage of the actual EvenOdd border paint path; it does not claim
an Incular GPU readback.

## Mixed values resolve at layout

Physical left/right and logical start/end now coexist in the public
`EdgeInsetsGeometry`. Comparing, interpolating and doing arithmetic
on them does not resolve the directions early. Equality compares all
six components, so equal top/bottom-only values can cross concrete
kinds but physical left is not logical start. Display preserves the
pinned decimal diagnostics. Physical geometry works without a
direction; other values return a typed missing-direction error.

Ten exact pinned cases pair across equality, interpolation, mixed
operators, diagnostics, factories, and missing direction. The three
additional Dart identity cases are partial: Rust preserves the value
when both endpoints match, but a Copy value has no Dart reference
identity. The authored retained SliverPadding probe verifies that
LTR→RTL→LTR moves the same child across x=16,22,16 with y=22 and
unchanged 58×50 size. The Rust checks also cover existing sliver
corrections and implicit animations. All 15 focused configuration and
39 widget checks passed, and the public Incular facade compiles.

### Running padding and margin animations survive a direction change

The pinned `animated_insets_direction_test.dart` checks AnimatedPadding,
AnimatedContainer padding and AnimatedContainer margin, changing direction
at the same timestamp a quarter of the way through each run. Incular
previously kept the left inset at 15px rather than immediately resolving
it to the pinned 9px. The tween now retains physical and directional
components, and the current frame resolves them without changing the
animation target or clock. Exact subsequent insets and measured child
rectangles match, the element stays mounted, and completion fires once
at the original deadline. An additional overshoot probe confirms that
AnimatedPadding clamps each unresolved component before combining it
(16.25px left and 3.25px right, not zero).

All four authored Flutter cases and 42 focused native implicit-animation
tests passed. These authored cases do not create new upstream rows.

## Rounded outline stroke, independently normalized contours

Four pinned `rounded_rectangle_border_test.dart` registrations pair.
RoundedRectangleBorder now keeps each ellipse axis and resolves logical
corners before generating the path. Its value methods retain physical
or directional geometry, including the endpoints of a mixed-radius
interpolation. The 10 px inside, center and outside side consumes
10 / 5 / 0 px in both rectangle and circle layouts, and the exact
10→20 width with inside→outside lerps to 15 px centered.

A positive-width outline now fills the region between its normalized
outer and inner contours, rather than stroking one centerline. Flat
inner bounds contribute no hole; inverted bounds are ordered by the
same policy as BoxBorder. Pinned Flutter raster samples for 5×4, 6×6,
6×7 and 7×7 boxes with a 3 px stroke agree with the native retained
path's coverage. An explicit zero-width solid side is still a hairline;
an absent side draws nothing. No Incular GPU readback is claimed.
Four originals, one authored raster test, and 24 focused Rust cases passed.

## Rounded rectangle to circle morph

The pinned `rounded_rectangle_border_test.dart:84` case pairs. A
100×20 rounded rectangle includes the centerline sample at x=30;
a fitted circle does not. The 10% and 90% transitions produce the
expected coverage and both keep x=50. The nested and reversed blends
pass the full set of point probes. The 10.0%, 20.0%, and 82.0%
diagnostic strings match for the default outline, and directly and
indirectly interpolated halfway values compare and hash identically.
The native shape also retains eccentricity and paints the interpolated
side width. One exact Flutter and 24 targeted Rust cases passed.

## Circle, oval and stadium value and centerline controls

Six pinned originals pair. Each outline can retain or replace its side
without changing equality or hashing for equal values. Circle also
retains or changes eccentricity in its copy operation. The 10/15/20 px
scales and interpolation land at the exact expected values. Inner and
outer unit circles agree on their bounds and interior probes. On the
original rectangles, inside, center and outside stadium strokes retain
their exact centerlines and layout insets; the circle's inside stroke
has center (25, 40), radius 10, and width 10. Six Flutter and 22 focused
Rust cases passed.

## Oversized oval, circle and stadium outlines

Three authored Flutter 3.50.0 probes distinguish the queried inner path
from the actual paint. With a 4×4 shape and a 3 px inside side, the inner
path has ordered 2×2 bounds and still encloses the center. At width 4
the inner path's bounds coincide with the original edges, but the
centerline has flattened and emits no paint. At width 8 a zero-
eccentricity CircleBorder stays blank, while OvalBorder and StadiumBorder
paint their ordered inverted centerlines. This removes the native
negative-size panic without turning a negative-radius circle into an
unexpected reflected stroke.

The pinned Flutter checks assert actual raster samples. The 23 focused
Rust cases check retained bounds, center containment, and the
presence or absence of a genuine stroke. No GPU readback or additional
upstream registration credit is claimed for these authored probes.

## Independent superellipse axes and clamping

Two original `rounded_superellipse_border_test.dart` value and stroke
cases pair, including physical and directional radius copies, scaling,
mixed-direction endpoints, unit-circle probes, and the 10 px stroke
on the expected inset contour.

The native path now keeps both corner axes. It removes the unused axis
of a flat corner before looking for the shared clamp factor, then fits
each nonflat corner from an affine circular basis. The authored Flutter
control compares four sets of radii on a 120×300 shape, including
20×50 elliptical, 20 circular, 600 circular, and non-uniform oversize
corners with a flat top-right. All 56 containment decisions and the
bounds match. The golden image registrations remain separate. Two
pinned originals, one authored control, and 32 focused Rust cases passed.

## Superellipse clips use the final child size

The pinned `widgets/clip_test.dart:947` zero-area case pairs. A second
pinned control places a 60×30 child under loose constraints and checks
the retained clip's paint geometry and actual taps. Logical elliptical
corners mirror on direction change, but a default ClipRSuperellipse
does not exclude taps in the clipped corners: its input boundary
remains the layout box. Clip.none emits no clip. Native compares both
paint contours and live hit testing, including a miss outside the box.
The exact original, authored control and 34 focused Rust cases passed.

## Superellipse to circle morph keeps three distinct edges

The pinned `rounded_superellipse_border_test.dart:89` case pairs. Its
full nested and reverse interpolation exercises superellipse path
coverage and the default diagnostic, equality, and hash expectations.
The independently authored Flutter control uses a 4 px inside stroke
on a 100×20 rectangle halfway to a circle. The outer query spans
x=20…80 and y=0…20. The public inner query is inset 8 px, while the
painted centerline is inset 2 px and the layout dimension only 4 px.
Those are distinct Flutter behaviors, not three ways to report the
same border inset. One pinned original, one authored reference and
37 targeted Rust cases passed.

## Corner-radius factories and signed arithmetic

Four pinned `border_radius_test.dart` registrations pair. Physical
and logical factories retain the original x/y values in raw rounded
rectangles, even when the geometry will clamp at paint time. Logical
corners resolve consistently in both directions. Horizontal factories
choose one coordinate system and reject ambiguous mixed arguments.
Copying with omitted overrides preserves equality and hashing.

Negation, ordinary division, truncating division, and Euclidean
remainder operate on each stored corner. They do not normalize values
to zero, substitute a circular radius, or resolve the logical part
of a mixed radius before applying arithmetic. Four exact Flutter
and 30 focused Rust cases passed.
