# Sliver paint bounds and clipping

Flutter's `RenderSliver.getMaxPaintRect` computes the full paint rectangle
relative to the current physical paint origin. Reverse axes reflect it
across the *current paint extent*, not the viewport or total scroll extent.
Pinning caps the logical leading shift at the scroll extent minus the
obstruction, and an infinite maximum ends at the live cache boundary.

Incular now shares this coordinate calculation with DecoratedSliver.
Two focused unit tests cover reverse orientation, reverse growth, a
reported cross-axis cap, a pinned obstruction, and an infinite paint
extent. All 12 decorated-sliver and painter-lifecycle controls still pass.

Five original SliverClipRect registrations ran on the pinned Flutter
checkout: custom bounds must not expand at small overlaps, followEdge
rejects overlapped hits in vertical and reversed horizontal viewports,
Clip.none allows overlap hits, and a leading clip stays pinned until its
inside extent is consumed. This is reference and geometry evidence only:
the retained SliverClipRect/RRect paint and hit adapters are still open.
