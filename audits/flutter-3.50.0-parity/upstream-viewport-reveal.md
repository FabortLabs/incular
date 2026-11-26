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
