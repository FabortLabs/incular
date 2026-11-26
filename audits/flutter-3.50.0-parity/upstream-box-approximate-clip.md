# Ordinary box clip accessibility bounds

Pinned `rendering/proxy_box_test.dart:917` passed. The retained Rust
clip exposes no approximate paint clip under Clip.none, and the same
200×200 local rectangle for HardEdge, AntiAlias, and AntiAliasWithSaveLayer.
Changing modes retains the same render object.

Box clips now contribute a projected rectangle to descendant accessibility
bounds. A custom clipper can supply a different approximate rectangle from
its raster shape. Its live reclip updates semantic membership and bounds
without resizing children or narrowing their hit tests. The authored
pinned Flutter and native tests move the semantic right edge through
100, 140, and 225px while a separate painted point remains hittable.

A second authored Flutter control proves that a 40px node shifted 30px
under its parent clip exposes a 10×40 rectangle. The old native fixture
expected the full 40px semantic width. The retained child's own bounds
remain 40×40 and its paint and hit behavior are unchanged.

Both authored Flutter controls and 52 focused native box, Flex, and
sliver clip checks passed. Only the exact original increments the
upstream registration count.
