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
inside extent is consumed. The overlap hit-test matrix is verified on the reference side and still needs a complete native fixture.

## Retained rectangle and rounded-rectangle clipping

The public SliverClipRect and SliverClipRRect now clip the sliver, not
each box independently. They use hardEdge and antiAlias respectively,
and default to followEdge; Clip.none removes the layer and hit restriction.
A saved-layer rounded clip over five actual lazy rows produces one
compositor isolation, and its rounded corner rejects pointer hits.

The pinned rectangular registrations at :70, :534, and :548 pair with
the native command and hit tests. A half-height clip accepts y=25 and
rejects y=75; removing it preserves the mounted child. A top inset
stays fixed until its inside span is consumed (scroll 0/100/385
gives top 30/0/−15).

Three rounded preserve-shape references (:684, :720, :751) pair for
vertical, horizontal and reversed vertical scrolling: 40 px corners
are preserved with an 80 px final main extent even under a 50 px
header overlap. The focused native clip tests, and the adjacent native
decorator, grouped focus and grouped tap tests passed.

Approximate semantic clipping, clipper-driven reclip callbacks,
elliptical radii, and the rest of the nested and overlap edge cases
remain open. No physical GPU pixel readback was required to compare
these exact retained clip commands and input targets.
