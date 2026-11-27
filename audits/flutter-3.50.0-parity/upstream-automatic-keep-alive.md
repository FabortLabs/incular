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
