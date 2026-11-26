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

## Pins and physical transforms

Paired :673 (local origins transform to (0, 0) and (150, 0)),
:691 (60 px pin pushed back by 20 px at scroll 560), and :749
(40 px collapsed pin pushed back 10 px at 570, but not at 550).
The original registrations passed on pinned Flutter. Native
uses real persistent-header delegates and retained child geometry.

An authored pinned reference establishes that the reverse-axis
parent offset is physical, not a mirrored logical offset. At scroll
570 the 40 px header starts at y=560, at scroll 550 it starts at
y=550. Two side-by-side pinned headers share y=0 at scroll 200
rather than stacking. At scroll 580 their top coordinates are
−40 and −20, so both end exactly at the group's last 20 px.

The viewport keeps using the geometry of the longest lane, while
the group independently offsets each lane and its pin before painting.
29 focused native tests and the two authored Flutter controls passed.
Other floating-header gesture and paint-visibility cases remain separate.

## Hit testing and pointer coordinates

Paired :459 and :1059. Rebuilding switches a tap handler from lane 0
row 0 to lane 1 row 2. Real down/up dispatch reaches each new target
and does not call the removed handler. A separate 20 × 40 fixture
taps both lanes at four positions on each axis. All eight callbacks
receive the same local and global coordinates as Flutter.

The coordinate fixture originally received no callbacks. When a
scrollable's drag recognizer was also present, a quick tap reached
release before the press deadline; Incular then rejected a detector
that used only onTapDown. The release now checks the same set of
tap callbacks as enrollment. Tests ran against the original Flutter
registrations and the native retained tree and recognizer.
