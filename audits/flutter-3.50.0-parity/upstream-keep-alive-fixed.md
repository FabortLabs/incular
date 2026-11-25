# Kept-alive children of fixed-extent lists and pages

The authored pinned Flutter control keeps a stateful first child in
both a 50px fixed-extent list and a 100px PageView. It jumps 300px
away, verifies the same state is retained without an accessibility
label, rebuilds with the request still active, and verifies that
withdrawing the request disposes the parked child before returning.
The replacement is mounted afresh and the label returns. Both Flutter
controls and the retained Rust comparison pass.

Prior to this repair the fixed list removed the child on the first
jump. Fixed-extent slivers now park eligible children outside their
active range; placement, painting and accessibility skip those
parked children. On a descriptor replacement, only previously
materialized keep-alive entries are considered for transfer, and the
new keyed descriptor can withdraw the request without exposing a
frame or rebuilding unrelated off-cache rows.

Thirteen targeted Rust tests passed, including the ordinary page
eviction, page cache and variable-list paint controls. This does not
yet pair the original dynamic `KeepAlive` registration in
`widgets/keep_alive_test.dart:50`; its child changes the request
locally without replacing the list descriptor. Grid tile retention
is also a separate case.

## Grid retains the requested tile, not its neighbors

The authored grid control jumps 300px away from a two-tile row with
only the second tile marked KeepAlive. Flutter and Rust retain that
same tile, remove its unrequested sibling, and omit the parked label.
An unchanged descriptor retains the tile while it is parked. Returning
rebuilds only the unrequested sibling; withdrawing the keep-alive
request on a later parked frame disposes the old tile.

Reconciliation uses a separate internal cell identity so parking a
single tile does not allow the removed neighbor's slot to claim its
retained state. Ordinary grid cells still follow the existing cache
window; an earlier Flutter fling/rebuild comparison and the grid
performance checks remain green. The authored case and 37 targeted
Rust tests passed. This does not yet cover a request changed locally
by a mounted grid tile.
