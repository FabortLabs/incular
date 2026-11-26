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

### Proxy follower hit gating and semantic translation

Pinned rendering/proxy_box_test.dart :631/:641/:652/:664 and the mounted
native fixture compare all four leader-present/absent and
show-when-unlinked combinations at the local origin. Before the fix,
the hidden orphan accepted a real press/release and invoked its child.
The ordinary hit path checked visibility after taking the follower's
transform branch, and the combined pointer walk explicitly fell back
to that ordinary path for a hidden follower. Both now stop at the same
visibility decision before transforming. The standalone linked-layer
control rejects any hit target, not just the follower wrapper.

Pinned :619 checks that the translation setter marks semantics after
initial layout and each real value change. The native retained probe
changes (0.5,0.5) to (0.4,0.4) and (0.3,0.3) on a 200×200 box. The
same semantic node keeps its size and moves from (100,100) to (80,80)
and (60,60), with 10⁻⁵ px tolerance for stored f32 fractions. This
compares the effect of invalidation, not a private mark-call counter.

The focused runtime proxy (2), linked transform (7), and widget
linked-layer (20) suites passed. No live window or OS input was needed.

### Follower unlinking and stationary pointer geometry

The authored mounted Flutter and Incular probes keep a follower mounted
while removing and replacing its leader. A press admitted before unlink
still completes on its captured route. New taps at either the former
leader or the layout slot miss the hidden follower. Relinking moves hits
to the new leader, not the old location; a visible orphan then responds
only at its normal layout placement.

For a stationary mouse, moving the leader (80,60) and back gives
enter→exit→enter. The follower's focus-node offset also moves from
(55,70) to (135,130) and back. Incular formerly kept the mouse hover
until a later input event. The frame now invalidates hover for
compositor-only geometry and publishes current links before hover,
focus, or semantic readers, with one shared pre-paint publication per
geometry stamp. The native probe disables semantic collection and
verifies the result both with and without a focus attachment. The move
uses no layout or repaint. These authored cases do not add upstream
registration sites.
