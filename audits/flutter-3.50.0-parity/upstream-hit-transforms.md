# Box hit-test coordinate paths

Pinned `rendering/box_test.dart:856,882,966,1037,1108` passed by exact
name. The Rust paths verify shared entry order and transforms through a
borrowed box view, point projection for null, identity and translated
inputs, inverse paint mapping versus forward raw mapping, and the
delegate's handled result. A singular paint transform does not invoke
a delegate. The out-of-band API requires exactly one transformation
by type, accepts singular raw transforms, and rejects a singular paint
transform in debug. Its transform does not leak into sibling entries.

The mounted pointer path uses the same paint-to-local inverse helper.
The targeted retained tap and rotated/scaled drag tests also passed,
including local deltas, global slop thresholds and sibling recognizer
arbitration. Rust stores a two-dimensional affine transform; the tested
matrices all operate on that plane.
