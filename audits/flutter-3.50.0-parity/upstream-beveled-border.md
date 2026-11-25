# Painting: beveled rectangle border

Flutter 3.50.0 strokes the outer (outset) and inner (inset) beveled
contours in one path. It draws nothing for an empty rectangle. Incular
previously stroked a single center contour, and a zero-width rectangle
could panic while constructing a negative size. The new path keeps
the signed edge coordinates until it is built, so very thick strokes
on small nonempty rectangles also work.

Three authored Flutter tests and the corresponding native paint tests
pass: a centered 4 px border has two closed contours and exact bounds
`(-2, -2)..(102, 82)`; a zero-width target emits no draw; a sufficiently
wide stroke on a small nonempty target still draws. The original
`painting/beveled_rectangle_border_test.dart:56` pairs the exact inside
and outside path samples for zero radius. Dialed edge geometry is
kept separate from eventual platform raster differences.
