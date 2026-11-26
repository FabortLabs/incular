# Flutter 3.50.0: SliverCrossAxisGroup

## Plain, weighted, and capped lanes

Compared the original registrations in
`widgets/sliver_cross_axis_group_test.dart`: :19, :84, :182, :240,
:302, :367, and :537. All seven ran on the pinned Flutter checkout.
Native tests use the public variable-extent SliverList with actual
20-child delegates. The vertical lanes use widths of 30/180/90
and the horizontal lanes use 30/380/190. Reverse scrolling reverses
the main axis, not lane order. Other retained scenes verify 3:2
weights, equal weights, and a 200 px cap reduced to 100 px because
the preceding fixed lane used the first 200 px.

In all these cases the longest lane gives a 6000 px scroll extent.
The unweighted case starts with two visible rows in the first lane
and three in the second. Jumping to 5400 shows the final two rows in
the first lane and none in the shorter lane. The native viewport checks
actual clipped bounds, not the presence of rows in its offscreen cache.

Before the fix, plain SliverList reported 5496 px in the mixed scene,
using a 48 px default for the unmeasured tail. An early measurement
pass also clipped the 5400 jump to 4896. Plain SliverList now projects
the tail from the current measured child sample, carrying that estimate
provisionally across a compatible rebuild. The same chunked index
continues to provide positioned lookups; explicit Incular ListView item
extent estimates retain their separate behavior. Arbitrary distant seeks
with an unmeasured, nonuniform prefix are not established by this probe.

29 focused native tests passed across the group, sliver delegates, padding
corrections, neighboring sliver behavior, and centered viewports.
