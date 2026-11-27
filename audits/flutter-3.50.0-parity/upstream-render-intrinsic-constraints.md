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
