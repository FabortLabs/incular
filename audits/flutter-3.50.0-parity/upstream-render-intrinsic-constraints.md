# Mounted intrinsic width and height constraints

Six exact registrations in Flutter 3.50.0
`rendering/intrinsic_width_test.dart` passed against the pinned source.
Each lays out both the parent and its child, whose natural layout size
(55×110) differs from its maximum intrinsic dimensions (100×200).

| Render parent | Incoming constraint on measured axis | Parent and child size |
| --- | --- | --- |
| IntrinsicWidth | width 50–70 | 70×110 |
| IntrinsicWidth | width 500 tight | 500×110 |
| IntrinsicWidth | width 50 tight | 50×110 |
| IntrinsicHeight | max height 80 | 55×80 |
| IntrinsicHeight | height 400 tight | 55×400 |
| IntrinsicHeight | height 80 tight | 55×80 |

The new native max-intrinsic override supplies the measured axis
independently of ordinary layout. The two loose cases previously
conflated the natural size and maximum intrinsic size: both
`IntrinsicWidth` and `IntrinsicHeight` used the ordinary layout
as their fallback probe. Tight constraints continue to skip the
measurement. The two additional native controls check ordinary
layout without an intrinsic parent and an override update under
unchanged parent constraints without replacing the child render.

All 21 focused native intrinsic and proxy tests passed. These six
pairs cover mounted layout; direct minimum/maximum intrinsic queries
and their cross-extent matrices are not counted by this batch.

## Padding around a measured child

Two authored Flutter cases verify the child and parent sizes when
asymmetric padding surrounds a box whose ordinary layout differs from
its maximum intrinsic size. The native test started at 75×140 on both
axes. Flutter and corrected native layout use 120×140 for the width
proxy (100×110 child) and 75×230 for the height proxy (55×200 child).
Padding now subtracts the insets for the child's cross-axis measurement,
then adds its insets to the returned extent on the measured axis.
All 28 focused native regressions passed. These authored cases do not
add paired credit to the original manifest.

## Direct Padding measurements before layout

Four pinned registrations passed, including the duplicate pairs in the
original file. The native pair runs both min/max queries on both axes at
cross extents 0, 10, 80 and infinity, then lays out under tight 10×10
constraints. A padded fixed 20×20 box reports 50 at all cross extents.
A padded 1:1 aspect box reports 30, 30, 80 and 30 respectively.

The queries read supported retained render kinds without performing
layout or calling builders. Unsupported and invalid inputs return no
measurement, rather than substituting a committed layout size. The
native fixture covers this with a callback counter. All 18 focused
native tests passed. This batch does not claim direct sizing support
for adaptive text, slivers or arbitrary user layout closures.
