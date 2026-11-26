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
