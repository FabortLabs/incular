# Linear text scaler clamping

Five pinned originals at `painting/text_scaler_test.dart` passed on
Flutter 3.50.0. The two linear cases pair. Four independent operations
produce equal 3× policies; neutral scaling equals linear(1). Capping a
5× policy at 3, flooring neutral scaling at 3, and setting a 3–3 range
all produce exactly 3×.

## Platform scaling snapshots and clamp composition

The three system registrations at `painting/text_scaler_test.dart:48,61,86`
passed on Flutter and in Incular's view-backed tests. The snapshot follows
platform preferences (123×, then 345×, then 123×). Reclamping intersects
overlapping ranges, returns the common point for touching ranges, and
takes the nearest new edge when ranges do not overlap. Inverted ranges,
NaN maxima, negative minima, and infinite minima reject with typed errors.

The additional matched probe verifies that two platform clamps which
happen to produce 2× do not collapse into the same linear object: their
bounds must remain distinct. That probe failed before the fix. Three
focused and 12 adjacent tests passed, including mounted text relayout
and explicit local override controls. Incular hosts currently expose a
proportional platform preference; native size-dependent platform curves
are not claimed by these checks.

The authored `linear_scaler_layout_test.dart` uses the resulting policy
in a real mounted paragraph. A clamp from 5× to 3× measures the same
as directly selecting 3×. Reclamping with minimum 4 gives exactly the
direct 4× size and increases both dimensions. The native control also
retains the render object. This is a size comparison within each
engine, not a cross-engine glyph pixel claim.
