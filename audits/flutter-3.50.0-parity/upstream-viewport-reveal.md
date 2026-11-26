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
