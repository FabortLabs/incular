# StadiumBorder transition parity

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
Reference: the four StadiumBorder interpolation registrations in
`painting/stadium_border_test.dart` and the transition geometry and
diagnostic strings in `lib/src/painting/stadium_border.dart`.

Each Rust case verifies the pinned inclusions/exclusions for direct,
reversed and nested interpolations, and the exact percentages, equality
and hashing of repeated and equivalent intermediate values. The rounded
targets use zero, circular and RTL-resolved directional corner radii.
None-style defaults still submit no stroke, including in intermediate
shapes. Stadium morphs now report the target type, progress and radius
rather than exposing an implementation Debug string.

All four original Flutter cases, four native Stadium cases, and eight
adjacent shape interpolation tests passed. Verification is at path and
display-command level, not a GPU raster golden.
