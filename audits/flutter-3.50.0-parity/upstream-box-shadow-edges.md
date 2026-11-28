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
