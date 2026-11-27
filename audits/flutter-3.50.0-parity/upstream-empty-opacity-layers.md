# Childless retained opacity groups

The pinned Flutter 3.50.0 originals at `rendering/layers_test.dart:664`
and :711 passed. Flutter submits no opacity/offset pass without children
and clears its old engine layer when its last child is detached.

The native compositor previously emitted a `PushOpacity`/`PopOpacity`
pair even when the group had no source. Both focused native tests failed
before the fix. Empty opacity now emits no pass at alpha 0, 128/255 or 1.
Detaching the only picture stops the pass immediately while retaining the
opacity layer identity. Reattaching a different picture starts a new
source generation; an earlier published buffer remains immutable.

All 36 native cases passed across the five focused layer, flatten and
box/sliver opacity targets. These originals remain partial: the
renderer-neutral fixture does not verify Flutter's engine offset-layer
type changes or immediate destruction of a device-side texture. wgpu
already bypasses isolation for full alpha during lowering; it is not
replaced or changed here.
