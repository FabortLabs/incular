# RenderFlex spacing and retained child placement

Nine original registrations in pinned `rendering/flex_test.dart` passed.
Eight have matching retained behavior; the ninth only compares public
default values because the full Dart diagnostic tree is not portable.

| Registration | Compared behavior |
| --- | --- |
| :216 | Changing the second child's flex factor from zero to one keeps the same child and reassigns its width from 0px to 100px. The first child stays zero-sized. |
| :236 | Stretch sets the full 100px cross-axis size of both children, without changing which child gets the main-axis share. The retained children still receive the correct share after flipping to vertical. |
| :265 | With no fixed spacing, three 100px children use offsets [50,200,350] in a 500px row and [25,150,275] in a 400px column. |
| :303, :338, :377 | Start, end and center preserve 14px gaps in both axes and place the remaining free space in the expected outer positions. |
| :416, :455 | Space-evenly and space-around distribute the unused space while applying the 14px fixed spacing only between children. The fractional space-around positions agree with the pinned tolerance. |
| :194 | Horizontal, Start, Max, Center, Down and zero spacing match. This does not reproduce Dart's debug tree text. |

The Rust tests drive real layout and descriptor updates through Runtime,
not just a formula. The spacing cases assert both child dimensions and
positions and verify the keyed elements retain their identity when
the axis changes.

## Overflow-dependent clipping

Pinned `:62` verifies all Flex clip modes. Incular now records the
same default/no-clip, hard-edge, antialias and isolated save-layer
coverages at the actual 100×100 bounds. Tests inspect that the
child painting is inside the clip rather than only querying the
descriptor. The pinned debug warning and striped overflow
indicator are not claimed by this test, so the registration
remains partial.

An authored paired reference resizes one retained overflowing flex
from 100px to 250px and back. The clip disappears once content fits
and reappears when main-axis overflow returns. Rows and columns
also expose the same option in Incular. The existing placement
batch stays green.

## Flexible sizing before shrink wrap

Pinned `:533` checks loose fit before and after changing the child
request from 100px to 1000px. The measured child stops at its 300px
share; its siblings move from [200,400] to [300,400].
Pinned `:589` checks a retained `MainAxisSize::Min` row with the
first child changing from inflexible to tight and then loose.
Before the fix, Incular laid out the tight child at 300px but
positioned siblings as if that child had zero width. It also
shrink-wrapped both the tight and loose states to 200px. After the
fix the line wraps to 300, 500 and 300px across the three frames,
with no overlap and the exact pinned child widths and offsets.

The identically named pinned registrations at `:650`, `:689` and
`:716` all executed. In the legal loose/min configuration, both
engines keep the same layout object and report widths
[300,300,300,500,500,505] with a constant 100px height as the
constraints and child fit change. The other two registrations
exercise invalid unbounded configurations: tight/min and loose/max.
Flutter reports a debug layout exception in each. Incular now reports
a typed debug layout error rather than silently painting an intrinsic
300×100 substitute. The test repeats the failure, then reuses the same
retained flex under a finite width and verifies recovery. Release
layout still contains the intrinsic fallback, as Flutter's normal
release sizing algorithm does. A native vertical control verifies the
same diagnostic names height when the vertical main axis is unbounded.

## Constrained overflow and rounding

Pinned `:16` fills the tight 200×200 parent in both engines even
with a contentless decorated child. Pinned `:33` shrinks to the
parent's main-axis maximum without an overflow exception. Its
two f64 extents round to the same f32 coordinate in Incular, so
it is partial: the retained control verifies no clip at that
coordinate, then reduces the maximum by one representable f32
step. The latter must clamp the Flex to the new bound and clip
the child, not grow the Flex beyond its parent.

A second pinned Flutter probe and a retained Rust test split
500px among three tight children. Both place the children at
0, 500/3, and 1000/3 within coordinate precision and emit no
overflow clip. Before the fix, Incular emitted a hard clip
because the independently rounded shares summed to slightly
more than 500px in f64. Placement now uses the measured
children without repeating their allocation, while the
overflow check compares the space that is free to exceed the
budget (nonflex children and spacing) against the constraint.

## Baseline of a nested Flex

The pinned vertical baseline test (`:937`) and two authored
actual-layout probes passed. The Rust and Flutter probes put the
nested Flex inside a baseline-aligned horizontal parent and
measure how far the parent moves it in real layout. A nested
Column uses the first real child baseline in logical order,
even when the vertical direction is reversed. The observed
baselines are 110, 210, 310 and 410px across the pinned alignment
sequence. The original registration remains partial because it
also calls the Dart dry-baseline API directly.

A nested Row uses the highest real baseline after child
placement, not simply the first raw child metric. The second
child has a 5px baseline and a 20px alignment offset, so the
parent places the Row 55px below an 80px anchor. A retained
update removes both real baselines and restores top alignment.
This also exercises baseline forwarding through Flexible and
ConstrainedBox, which had previously hidden the child baseline.

## Direction changes

The pinned `Flex RTL` sequence runs through sixteen states of the same
retained Flex. The Rust fixture matches each child's exact origin and
size when alignment, writing direction and vertical direction flip,
when the Flex changes axis, and when stretch takes the full 800px
cross-axis allocation. The original three element identities persist
throughout. The pinned no-baselines case also matches the retained
geometry: baseline alignment leaves both boxes at the top, regardless
of vertical direction. Its existing partial API disposition is unchanged.

## Negative spacing

Pinned `rendering/flex_test.dart:1256` and the Rust constructor both
reject a negative gap in debug. The Rust controls cover fluent, typed
builder and raw-widget conversion paths for Flex, Row and Column.
Positive spacing still preserves placement. Incular leaves its previous
zero-gap sanitization unchanged in release; the pinned case describes
Flutter's debug assertion.

## Multiline alphabetic baseline in Row

Pinned widget registrations `widgets/basic_test.dart:832,896` both
passed. The native fixture uses the original FlutterTest face,
glyphs, 52/12 px text sizes, line-height multiplier 1, and seven
lines. The rendered paragraph heights are exactly 52 and 84; their
top offsets are 0 and 30. The Row is 114 px high because the tallest
ascent and descent come from different children.

Adding a 250 px child without an alphabetic baseline grows the Row to
250 px without moving either aligned text; the additional child is
top-aligned. A fixed square represents only the FlutterLogo's
geometry, since the original never asserts logo painting. Removing
that child restores 114 px in the retained runtime. All three tests
in the focused Flex baseline target passed with
`INCULAR_FLUTTER_TEST_FONT_PATH` set to the extracted pinned face.
