# Flutter render coordinate transforms

Pinned originals `rendering/object_test.dart:188,197,217` and
`rendering/box_test.dart:1281` passed by exact name. Incular now exposes
relative paint transforms from the retained render tree. Addresses carry
their owning tree identity; a different window is rejected even if its
arena slot and generation match. The same-node, sibling, ancestor and
descendant identity cases match. The 50×50 inner box reports (25,25)
relative to the 100×100 ancestor instead of including the outer window's
centering offset.

The scaled sibling fixture matches the planar offset (-216,-192) and a
singular destination returns the zero affine projection or a NaN point.
The algorithm stops at the actual common ancestor rather than multiplying
full window transforms. :217 remains partial because Incular's
renderer-neutral ordinary geometry is affine in two dimensions; the
original also asserts the z component of a 4×4 matrix.
