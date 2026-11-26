# Wrap axis distribution and vertical intrinsic width

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

Five exact Flutter originals passed. The four retaining Wrap
alignment tests match all positions for center, space between,
space around, and space evenly. This includes LTR/RTL
(:256,:342) and up/down run alignment (:428,:534), with
start edges inside each run mirrored appropriately.

Vertical intrinsic width (:1001) differs. Flutter measures 188
for two narrow items before/after a full-height 60-wide Column.
To avoid comparing typography, a separately pinned Flutter
fixture replaces its Ahem text by equal-width 80 and 48 boxes
and independently still measures 188. The same Rust scene
measures only 80. This is partial against the original's font
fixture, but the authored geometry comparison isolates the
layout error without mocking font or widening tolerance.
See `flutter350_wrap_constraints_parity` and
`vertical_wrap_intrinsic_width_test.dart`.


## Current intrinsic sizing

The former vertical cross-axis discrepancy no longer reproduces on
current Incular. Both the pinned original and the font-independent
probe pass: the wrapper is 188 × 600, and the children start at x
coordinates 0, 80 and 140. The original remains a partial match
because Rust uses fixed measured boxes instead of the Ahem font.

A separate source review found an adjacent gap: Flutter's maximum
intrinsic **parallel** to a Wrap's direction is the sum of the
unconstrained child maxima without inter-item spacing. Measuring
normal layout instead included the spacing and produced one run
where Flutter made two. Incular now computes that intrinsic sum,
including through a decoration-only proxy, and lays out final runs
with ordinary spacing. The new pinned test checks both axes, with
and without a decoration: the main extent is 50, the cross extent
is 18, and the second run begins at cross coordinate 9.

Both exact upstream tests and five authored Flutter cases passed.
All 68 focused Rust Wrap, sizing, OverflowBar, and ListBody tests
passed. The new authored probe does not add an upstream row.
