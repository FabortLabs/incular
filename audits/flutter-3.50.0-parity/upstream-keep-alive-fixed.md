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

## A child changes its request without rebuilding its viewport

The viewport previously kept reading the builder's original immutable
request. An update to a mounted child now changes the cache's parent
data and requests layout on the owning viewport. This works for
variable and fixed list rows and for a grid tile inside its row.
The child can revoke the request while parked; neither new input
nor a viewport rebuild is needed. Three authored pinned reference
scenes and their Rust counterpart pass.

The exact 300px touch outward/return sequence from
`widgets/keep_alive_test.dart:50, :147` also pairs:
the fixed list and two-lane grid retain Item 60 but not 59 or 61
after returning to the top. Item 60 keeps the same identity, does
not appear in accessibility, and is disposed when it locally
withdraws its request. The original variable-list fixture (:98)
is still red: on the first 600px frame with 12.3px natural rows,
Incular does not even mount Item 30. That divergence occurs before
any keep-alive request and is tracked separately from the passing
local request/revocation control in a smaller variable viewport.

## Natural rows fill the initial cache before painting

The original variable-list registration :98 now pairs as well. Its
12.3px rows were shorter than the existing initial extent estimates.
Three reconciliation sweeps stopped before the shrinking measurements
could expose enough subsequent children. A fixed-size viewport now
continues the same frame while the measured cache discovery is still
growing, with a separate finite cap. It applies the final published
child set before painting, and the established three-sweep
shrink-wrap bounds are unchanged.

A one-million-item authored control checks the full set of first-frame
rectangles, the edge at y=600, and the exact number of builder calls
with no cache extension. Both Flutter and Rust build exactly the 49
intersecting rows and none beyond them. Sixty-four focused Rust tests
passed, including the three original keep-alive input sequences and
the existing deep-jump and structural-work controls.
