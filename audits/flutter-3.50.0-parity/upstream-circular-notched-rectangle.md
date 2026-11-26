# Circular notched rectangle geometry

Six pinned `painting/notched_shapes_test.dart` originals passed:
`:12, :26, :36, :46, :56, :66`. The new renderer-independent
NotchedShape API uses the exact two quadratic shoulder positions
and a small circular arc, inverted when the notch is on the bottom.
The path uses the existing neutral conic builder and does not change
the rendering backend or add a design-system component.

The native path matches the host rectangle across the sampled area
with no guest or an off-host guest. For each of the four guest
placements, 4,100 radial probes land outside the filled outline.
Positive controls keep the host body filled. Twenty-nine selected
native shape checks passed. The separate AutomaticNotchedShape
originals remain open because they require arbitrary path difference,
not just the circular notch.
