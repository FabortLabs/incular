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
