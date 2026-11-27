# Automatic keep-alive notifications

Pinned `widgets/automatic_keep_alive_test.dart:219` is paired. A
600-pixel viewport contains three 400-pixel rows. With the same outward
and return touch drags, no requests initially retain the first row.
Then either descendant's handle is enough to keep both descendants
mounted while the row is offscreen. The second request can arrive from
the already parked subtree. Releasing only one request keeps both
descendants; releasing the last removes them without another drag.

An authored Rust regression also covers fixed and natural lists and a
grid. It dispatches and scrolls before the same frame, checks the exact
retained identity and absence from accessibility, then requests again
while parked. An old notification dispatcher cannot revive a removed
row. Existing explicit keep-alive and notification stop/continue tests
remain green. Eighteen focused Rust tests passed.

Previously `AutomaticKeepAlive` returned its child unchanged and no
mounted descendant could reach the registry. The mounted scope now
receives notifications through the ordinary listener bubble chain.
Each mount owns its default registry; an explicitly injected registry
can be shared. Only changes between no handles and some handles enqueue
sliver parent data, so inactive rows do not need a per-frame scan.
Automatic, active-editor and explicit requests are independent.
Compatible sliver replacement preserves automatic requests only for a
matching child with an automatic scope. The retained scope detaches
its listener when it goes away.

The default list/grid wrappers, keyed child remapping, and the grouped
explicit-and-implicit reference registrations are not claimed by this
batch. Five originally registered Flutter tests were run as controls;
only the exact double-request case is added to the paired ledger here.

## Default ListView and GridView delegates

The explicit and implied registrations at lines 63, 111 and 161 now
both pair for fixed lists, naturally sized lists and the two-lane grid.
They use the original 100 items, 600-pixel viewport, zero cache,
12.3-pixel list rows or 24.6-pixel grid rows, and 300-pixel touch drags
out and back. Row or tile 60 keeps the original keyed element, is not
exposed by accessibility while parked, and is removed on release.
Neither immediate neighbor is retained. Opting out of automatic
wrapping still allows an explicit ancestor to receive the notification.

The default builder installs a listener without additional widget
or render nodes and without copying a child's GlobalKey. A registry
and viewport subscription are allocated only on the first actual
request in that row. Single explicit keep-alive requests, active editor
requests, and implicit notifications stay independent.

The listener detach test at line 570 now passes through the actual
default builder chain. Lines 478 and 515 pair for requests made from
the first build. The latter checks the seven visible even rows before
the jump, the original first row's identity, exactly 23 mounted even
rows after jumping to 3025 pixels, and the presence of row 73 rather
than rows 1 and 3 in the viewport. Visible geometry is distinguished
from mounted cache children; this is not an accessibility assertion
for every cache edge. Keyed moves between different parked rows were
checked separately below. The standalone sliver mixin fixture remains open.

Forty-two focused Rust regressions passed for this delegate batch.

## Reparenting an active client

Pinned line 295, `AutomaticKeepAlive double 2`, is paired. A GlobalKey
moves an active client out of a parked 400-pixel row into the visible
last row while the scroll position stays put. The old row is released,
the moved element retains its identity, and the destination remains
parked after scrolling away until its request is released. Two subsequent
descriptor moves and the presence of the original six leaves match.
A second native variant retains a notification listener inside the
moving subtree, so the nearest listener's identity can stay unchanged
while its outer keep-alive ancestor changes.

`AutomaticKeepAliveClient` binds a keep-alive controller to mounted
lifetime. Controller changes apply even while parked. The client
remembers the full notification route for its lease, releases the old
route after reparenting, and unregisters on unmount.

Fifteen focused runtime and notification regressions passed.
