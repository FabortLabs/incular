# Rotated clips and sibling isolation

Flutter `ab2912035e160d9cad6306df307576da6128ebdb` rendered all five
selected `widgets/clip_test.dart` originals in an isolated copy of the
test package using `flutter test --update-goldens`. The four standalone
images were 300×300; the overlap image was 800×600. These are fresh
reference renders, not a claim that the local pixels matched a downloaded
golden. The target names were ClipRect painting, ClipRect save, overlay,
and antialiasing, ClipRRect painting, ClipOval painting, and ClipPath
painting.

## Rect clip does not leak to the overlay

The counterpart of `widgets/clip_test.dart:409` builds the original
0–100 blue box and the later red 100×100 box at (50,50), rotated
one radian around (100,100). The flattened retained scene draws only
those two boxes, closes the blue clip before the red draw, and keeps
the red pivot fixed. The reference pixels at (50,50) and (100,100)
are solid blue and red, respectively. This verifies paint order and
clip isolation, not native raster edges.

## Rounded corners retain independent axes

The original `widgets/clip_test.dart:439` rotates four different
elliptical corner radii one radian. Before the fix, Incular dropped
every vertical radius. Its retained path included local (3,3) when
the 10×20 top-left ellipse should exclude it. Changing only the
vertical radius on a mounted clip also produced no new geometry.

Incular retains both dimensions through physical and directional corner
resolution. Circular radii still use the existing analytic rounded
clip. Elliptical radii use the normalized quarter-conic path, switch
representations without replacing the layer or its child, and keep
the path identity when only coverage changes. The direction control
checks matching horizontal radii with different vertical radii and
verifies that both axes move to the correct side. The authored pinned
`elliptical_rotated_clip_test.dart` checks actual blue/white raster
samples at the same transform and geometry away from the edge; the
native test checks the retained path against those points.

All 44 focused native clip, hit-test, and layer-reuse tests passed.
These registrations remain partial. This Dot host could not start a
local display to capture an Incular presentation. The wgpu path-clip
lowering still uses stencil coverage for transformed paths, so default
antialiasing and save-layer edges need separate verification and work.
