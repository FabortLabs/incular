# Flutter 3.50.0 viewport reveal

## Leading, trailing, and subrectangle geometry

Pinned rendering/viewport_test.dart :91/:145/:201/:257 all passed.
The native fixtures mount the public ListView with twenty explicit
100 px rows, a 200 px viewport on its main axis, and a 300 px initial
offset. Without an explicit item extent, the old native extent index
still used a 48 px estimate for rows before the initial cache window.
It reported 448 instead of 500 for the leading edge of tile five.

The explicit ListView now recognizes a unanimous positive tight extent
among its already-described children. That uses the existing fixed
viewport path without mounting or measuring offscreen rows. The
variable fallback and caller-supplied item extents, prototypes and
builder extents are unchanged. Correctness here is for actual known
uniform children, not arbitrary unmeasured variable children.

In each direction, the public read-only reveal query returns both the
offset and the rectangle projected into the viewport. Whole-child
leading and trailing offsets are 500 and 400. The (40,40,10,10)
subrect returns 540/350 on down and right, or 550/360 on up and left.
The four projected rectangles per direction match exactly, including
the reversed physical edge. The query leaves the scroll position at
300. It currently selects the nearest standard viewport and defaults
to layout bounds; it does not claim arbitrary visual overflow or
two-dimensional viewport parity.

An additional native control uses different 60 and 80 px rows. It
gets an unclamped reveal offset of 60 while the scrolling operation
clamps to the available 40 and leaves the row visible at y=20.
The focused native viewport, EnsureVisible and grid-cache suites and
the minimal facade check passed.

## Padded box sliver geometry

Pinned :314/:363/:413/:567 and the native CustomScrollView fixtures
use 100 px box adapters with 22 px physical leading-side padding and
23 px on the opposite side. The mounted box for the adapter at index five has the
same bounds as the one-box Flutter sliver. The query returns 747/647
down and right, and 748/648 up and left, for the whole box. A local
(1,2,2,2) subrectangle matches all eight pinned offsets, including
the reversed edges. None of the queries moves the controller.
The separate reverse-growth center case is still under comparison.

## Reverse growth uses the combined padding direction

Pinned :464/:515 place a padded box before the viewport center, where
growth opposes the viewport. Before the fix the native leading offset
was −123 rather than −122: padding selected the physical edge using
the viewport direction alone and ignored the sliver's growth. The
padding now uses their combined direction, matching all whole-box and
subrectangle offsets for up and left, including negative results
beyond the scroll range. The authored Flutter/native mirror also
checks down and right viewports with a reverse-growing box.

## Pinned headers in forward growth

Pinned :619/:679/:740/:801 and the native fixed-height header fixtures
return offset 300 before the header sticks. After jumping to 500,
the reveal query returns the current offset of 500, not its original
scroll origin. The 50 px header's projected rectangle remains against
the viewport's logical leading edge, physically y/x=0 or 350 according
to direction. The test uses a lazy list of twenty 50 px trailing rows
and checks the actual viewport position and rect.

## Pinned headers before the viewport center

Pinned :863/:923 exercise a pinned header in the reverse-growth side
of a centered viewport. Trailing-aligned reveal returns −300 before the
header pins and the current offset, −500, after it pins. Incular had
treated it as a moving box and returned −700 for the first query.
The layout now records which sliver actually pins. The reveal solver
uses the pinned edge only for rectangles that do not protrude from its
growth-leading edge, and shares the result with EnsureVisible. The
forward-growth header and ordinary negative-offset tests stay green.

## Automatic reveal of an oversized tile and implicit-scroll veto

Pinned :1419 and the mounted semantic ShowOnScreen route act on
300 px tiles inside a 200 px viewport. The native node carries the
full tile bounds. Before the fix its first reveal jumped from 300
to 400 even though the requested tile was already aligned. The
automatic solver now sorts the two aligned offsets and leaves the
current position alone while between them. Exact successive results
are 300, 100, 300, 100 after a jump to 250, and 300 after a jump to 150.
The native control checks both axes and both viewport directions.

Pinned :1319/:1367 and the native mounted nested controls keep the
horizontal viewport at zero when page physics disallows implicit
scrolling. The outer viewport moves 50 for the full-height box but
only 25 for the padded sliver's actual descendant. The semantic
route preserves the requested bounds rather than substituting the
entire inner viewport. Both controls already passed before the
oversized-tile correction.

## Automatic reveal across multiple viewports

Pinned :985 and the native public ShowOnScreen test mount a 10×10
grid of 100 px boxes from nested vertical and horizontal lists in a
200×200 view. Both controllers start at 400. The native test checks
all nine center/side/diagonal requests against the pinned offsets
300, 400, and 500 on each axis. Its two-second diagonal request
starts both controllers, moves both partway by one second and
settles both at 500.

Pinned :1236/:1252/:1268/:1284/:1300 compare two nested vertical
viewports. The outer viewport receives the inner viewport's projected
target rectangle rather than the previous world bounds or the entire
inner viewport. Exact controller pairs are (inner, outer):
(0,100), (300,200), (200,200), (400,200), and (0,200).
The render-facing API does not require a semantic node; the previous
section independently tests the accessibility action adapter.

## Runtime axis change and an offstage negative item

Pinned :51 and the mounted runtime both change a live 100×100
SingleChildScrollView from vertical to horizontal at offset zero.
The same admitted touch stream moves 10 px down and back without
scrolling, then 10 px left and back with offsets 10 and 0. No
synthetic controller movement stands in for the real drag.

Pinned :1188 and the native centered reverse list begin with the
first negative sliver item outside the 400×400 viewport but still
cached. A one-second ShowOnScreen request moves the controller in
the negative direction and the item's retained bounds enter the
viewport. This exercises the offstage target rather than the
already-visible center.

## Empty viewport at zero area

Pinned :2860 and the native mounted Viewport both lay out an empty
viewport at a zero-sized root, with a nonzero initial-offset request,
without failing. The laid-out viewport size is exactly zero. The
comparison does not assert that the native scroll controller preserves
an out-of-range offset for an empty viewport.

## Centered paint and hit order

Flutter's `rendering/viewport_test.dart:2274,2291,2315,2337` uses five slivers
around a viewport center and checks both paint orders and their inverse
hit-test orders. All four pinned tests ran and passed. Incular's
`flutter350_viewport_order` tests both orders with five mounted slivers:

- Default paint invokes each painter and flattens its display list in
  `[4,3,2,1,0]` order. Explicit `LastIsTop` produces `[0,1,2,3,4]`.
- A raw pointer down at the center of five overlapping translucent slivers
  reaches all listeners from front to back in the inverse order, not just
  the winning child.

Before the fix, the retained layer order was correct but custom painters
ran in logical child order in the default case. Paint traversal now uses the
same sliver slot order as layer installation and hit testing. The existing
`sliver_paint_order` regressions for the published scene and topmost hit
also passed. This did not change the wgpu renderer.

## Nested box-to-sliver coordinate conversion

The eight direction/growth registrations at `rendering/viewport_test.dart:1962–2039`
passed in the pinned SDK. The Incular
`nested_box_padding_converts_to_scroll_space_for_all_growth_directions`
fixture mounts the far target in a box with asymmetric inner padding inside
an equally padded sliver. Leading-edge reveal is exactly
`±5 × 356 + 44` (down/right) or `±5 × 356 + 68` (up/left). The target's
projected edge lands at 0 or 400 in viewport coordinates, and the query
leaves the controller at 0. The target is kept mounted by increasing the
cache extent, not by scrolling to it.

The separate mismatched-axis registration at line 2050 passed in Flutter.
Flutter ignores the supplied axis for one-dimensional viewports;
Incular's one-dimensional query takes no axis argument and always uses the
viewport's axis. The mismatched argument itself has no paired exercise.

## Visible pinned header and an inner viewport with no outer cache

Both `rendering/viewport_test.dart:1479,2061` passed in pinned Flutter.
The matching mounted Incular regressions are in
`flutter350_viewport_show_on_screen`:

- A variable-size pinned header collapses to 100 px and stays at y=0.
  Revealing the already-visible header leaves the controller at 4500.
  Revealing the box after it from offsets 3600 (upstream) and 3400
  (partly visible) moves to 3200 and places the box at y=100, clear of
  the header. The checked-in Dart probe `viewport_pinned_geometry_test.dart`
  verifies these exact positions and offsets in Flutter 3.50.0.
- With a zero-cache outer viewport, a full-height tile begins at y=300
  in the inner viewport. After reveal it occupies y=0–300; the inner
  controller moves to 300 and the outer stays at 0.

These results do not establish floating-header expansion on reveal.

## Unbounded viewport constraints

All four nested-axis tests at `rendering/viewport_test.dart:2142–2190`
and the shrink-wrapping test at line 2751 passed in the pinned SDK.
The mounted Incular tests in `flutter350_viewport_constraints` verify the
matching worded diagnosis for both axes, including the empty shrink-wrapping
case, and ensure the invalid inner viewport does not attach its controller.
An expanding viewport requires both axes to be bounded in debug builds;
a shrink-wrapping viewport still accepts an unbounded main axis and measured
50 px there in each orientation. Previously the invalid fixtures laid out
silently, commonly with a zero extent. The release path is unchanged.

The direct `RenderViewport` and `RenderShrinkWrappingViewport` intrinsic
diagnostic (line 2212) passed in Flutter only. Incular has no corresponding
direct render intrinsic query, and its intrinsic wrapper measurements remain
a separate comparison.

## Viewport layout correction budget

The pinned `rendering/viewport_test.dart:2822` and Incular's
`flutter350_sliver_viewport_correction` accept five offset corrections
from each of 30 top-level slivers in one frame, and reject a single sliver
that still requests corrections after the tenth layout. Previously the
viewport limited the whole sequence to ten corrections and allowed an
eleventh attempt even for a single sliver. The new limit scales with the
number of top-level slivers and counts the attempt that must accept the
offset. The extra one-sliver checks verify success with nine corrections,
failure with ten, and no publication of an unvalidated controller offset.
The separate shrink-wrap measurement guard was not changed.

## Shrink-wrap bounce and Apple platform defaults

Pinned Flutter and the mounted Incular
`flutter350_viewport_shrink_overscroll` suite exercise
`rendering/viewport_test.dart:2481,2529,2579,2622,2670`. A touch drag by
25 px produces exactly 25 px overscroll and the corresponding painted
item displacement at both edges. On release the virtual-clock frame loop
settles back to the exact original edge. This holds vertically and
horizontally, with explicit bouncing physics and with the default iOS/macOS
platform policies. The platform variants here select the policies in a
Linux test host; they are not hardware tests.

A three-column shrink-wrapping grid with aspect-ratio sizing in a Column
with unbounded main-axis constraints also lays out and handles a real
drag in both Apple policies. Its finite viewport extent is about 88.22 px
and the short content stays at offset zero.

## Reverse-growing floating pinned header

The two pinned Flutter variants of `rendering/viewport_test.dart:1871`
and Incular's `reverse_growing_floating_pinned_header_reveals_without_unpinning`
check the centered 20-sliver sequence in both orientations. At offset
−4500 the header's trailing edge is at the viewport edge (400 px) and
revealing the visible header does not move the controller. Revealing
tile 9 at −3100 moves to −3000 to clear the obstruction. These cases only
cover a visible header and normal viewport reveal of content, not the
floating header's separate reveal-to-expand lifecycle.

## Shrink-wrap overscroll clips the translated content

Both pinned Flutter cases at `rendering/viewport_test.dart:2423,2452`
passed, including their visual checks. Incular's mounted
`overscrolled_shrink_wrapping_viewport_never_clips_its_siblings` verifies
the same input and geometry. A 100 px drag holds offset −100 through
layout, the first tile moves from y=100 to y=200, and the retained scene
applies a hard clip to the translated tiles, not to the siblings. That
clip is exactly y=100–250 for the constrained viewport or y=100–240 when
naturally sized. On release both instances land exactly at offset zero.

Before this fix the initial shrink-wrap measurement published its
provisional zero viewport extent, clamping away a live bounce. The
measurement now informs sizing without being published as the actual
viewport dimensions. The native test compares paint commands and exact
geometry, not Flutter's raster golden; these two cases are partial.
The existing shrink-wrap convergence and grid regressions passed.

## Floating persistent header expansion on reveal

All ten pinned Flutter executions for the five registrations at
`rendering/viewport_test.dart:1589,1640,1690,1735,1803` passed (vertical
and horizontal). Incular's `flutter350_floating_header_show_on_screen`
verifies native descendant requests for the four paired registrations:

- Opting into `PersistentHeaderShowOnScreenConfiguration` allows a
  collapsed 100 px header to expand to 300 px without changing the 4500 px
  scroll offset. A requested rectangle starting at −1 is trimmed at the
  growth-leading edge, so it does not move the viewport.
- A 3-second 100-to-150 px animation reaches its measured one-third
  frame and then 150 px. A second request for 300 px cannot retarget it
  while it is running. An unconfigured header does not expand.
- A 200 px cap or floor limits expansion, but never contracts a 250 px
  child. An idle programmatic jump to 3050 gives the natural 250 px
  extent; a held user drag can float the header before its layout slot.
- A request from a descendant with 10 px of padding inside a
  sliver-opacity wrapper expands the 100 px header to 150 px for a
  140 px descendant rectangle. It schedules a frame with no scroll move.

Flutter line 1690 addresses the sliver render object itself, without a
descendant. Incular only exposes the mounted box child as a reveal
target, so the corresponding presentation is verified but that direct
render-sliver entry point is partial.
