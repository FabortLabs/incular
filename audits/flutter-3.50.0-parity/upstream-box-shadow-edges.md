# Offset box shadows and fill edges

Pinned Flutter 3.50.0 reference: `box_painter_test.dart:343,424`. The
authored `offset_box_shadow_edges_test.dart` reads Flutter RGBA instead
of relying on a test reporter's success line. At the exact 64 px black
box edge, a spread or offset does not add a black stroke. At the
shadow's moved edge, outer blur leaves the inside untouched and extends
outside, while inner blur does the reverse. Solid fills its mask;
normal softens both sides. The original Gold image assertions are
skipped on this host, so the registrations remain partial.

The native retained paint comparison uses a 128 px backdrop, a 64 px
box at (32,32), spread 2, offset (4,4), and blur radius 16. The box
keeps its original fill bounds; the shadow masks paint a 68 px box at
(34,34) and use sigma 9.7376. The outer/inner blend operations are
isolated from the backdrop, the solid/normal cases use their expected
layer structure, and none emits a border or stroke command. Four
focused native tests passed. This does not establish final wgpu pixel
equality with Flutter's images.

## A fractional rounded stroke must fade to zero

The pinned Flutter Canvas probe `rounded_border_subpixel_test.dart`
paints an inside stroke around a 32×32 box at (16.25,16.25), with
8 px corners. Its 0.03125 px stroke covers the left edge's neighboring
pixels with alpha `[2,5]`; an omitted stroke paints neither. The previous
wgpu rounded shape shader read back `[0,54]` with the same geometry.
Multiplying the outer and inverted inner AA ramps left a visible line
even as the two contours approached one another.

Incular now measures the device-space band between the two contours and
scales subpixel coverage by its width. A ring with no remaining interior
uses the ordinary outer fill coverage. The offscreen GPU test exercises
the production shader and instance packing through wgpu GL, without a
window. Straight-edge samples match within one alpha level and the
small curved coverage within three. The 2 px stroke stays hollow, a
16 px stroke fills the center, and reducing the stroke to 0.001 px
leaves no pixel above alpha 1. These authored cases add no upstream
registrations and do not claim whole-image equality.
