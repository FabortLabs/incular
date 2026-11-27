# RenderSliver maximum paint rectangle

All thirteen pinned originals from `rendering/sliver_max_paint_rect_test.dart`
passed. The Dart test sliver sets the supplied geometry directly. The
native counterparts feed that same geometry to the production
`SliverPaintBounds` used by sliver decoration, not a second test
implementation of the calculation.

The assertions cover all four physical directions before and after
scrolling by 50, growth reversal, pinned obstruction (leading offset
90 at scroll 95), an infinite maximum bounded by the 150-pixel cache
reach, a 50-pixel reported cross-axis override, and a 150-pixel
maximum even with only 100 pixels remaining to paint. This is the
uncropped local paint rectangle, not the current viewport's clip.
All four focused native sliver paint geometry tests passed.
