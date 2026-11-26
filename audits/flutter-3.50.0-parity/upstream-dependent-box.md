# Positioning a measured box relative to a target

Pinned `painting/geometry_test.dart:9` passes. All ten native
coordinates match: an overwide child is centered even when the
target is on an edge; otherwise margin constraints clamp the
centered position only when there is room for both margins.

A pinned authored control checks above/below fallback, preference
when neither side fits, clamping the near edge, and the horizontal
NaN target selecting the upper clamp bound. Both implementations
passed those seven variants. Fourteen focused native checks passed.
This registers the geometry helper only, not tooltip placement or
a design-system tooltip.
