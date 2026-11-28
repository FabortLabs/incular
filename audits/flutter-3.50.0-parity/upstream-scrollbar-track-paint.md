# RawScrollbar track ink, border, and corners

The Flutter 3.50.0 originals in `widgets/scrollbar_test.dart` at lines
2423, 2459, and 2498 were paired with mounted Incular display-list tests
in `flutter350_raw_scrollbar_paint.rs`. At an 800×600 viewport the
opted-in right track spans x=794–800 and y=0–600, with a one-logical-pixel
butt-capped line at x=794 before the thumb. Its fill and edge default to
alpha 8 and 26 respectively; independent color overrides preserve their
opacity. Track and thumb radii are separate, and neither alters the edge's
opt-in policy.

The additional pinned probe `raw_scrollbar_track_paint_test.dart` checks
that passing a color alone does not show an edge: Flutter records a
transparent line, while Incular skips that invisible draw. An explicitly
transparent fill does not suppress the opted-in border. Native tests also
check the physical inner edge on all four sides. Incular's pre-existing
nontransparent style field remains a fill-only extension when visibility
is unspecified.

Four pinned upstream tests, one authored pinned probe, and 17 focused
native paint/padding tests passed. Upstream line 570 remains partial:
that standalone painter test captures *nominal* radii bigger than the
shape before rasterization. Incular's analytic retained rounded-rectangle
command already normalizes out-of-range radii rather than exposing
equivalent pre-rasterization arguments. No whole-image comparison claimed.
