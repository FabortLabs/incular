# Semantic visibility and merge membership

Pinned `semantics/semantics_test.dart:175` and `:229` passed.
A pre-merge graph now exposes the independently published ancestor
that absorbs each node. Turning on a root's merge absorbs its children
and grandchildren. Turning on a middle node's merge and removing
the root's merge leaves the middle node independent and still absorbs
its children. A node does not become its own merge ancestor.

The visibility check follows the same cases: an empty childless root
is allowed, an empty child may be absorbed by a visible merge owner,
a visible ordinary tree is accepted, and an empty root with children
or an independently empty child is rejected. It reports the bad node,
rectangle, and parent in a typed error rather than reproducing
Flutter's Diagnosticable wording. It does not report descendants
of a rejected node twice, and hidden viewport caches with nonempty
rectangles remain valid.

The checked adapter publishes nothing on rejection, and a successful
snapshot only includes attached nodes. The normal incremental widget
bridge is unchanged: widgets already merge their semantic descendants
during collection. Sixteen direct semantics tests, eight accessibility
forest tests, and 24 mounted parity controls passed.

A nearby focus descriptor test failed on the prior committed baseline
as well. It was checking a published focus flag before mounting the
description. The corrected test checks the declaration first, then
mounts a real checkbox and verifies its published focus and checked
state. The declaration does not create a keyboard target or runnable
focus action. All ten focused descriptor and focus annotation tests
passed; no coverage credit was added for this control.

## Semantic data and node JSON

Pinned `semantics/semantics_test.dart:1357` passed. The data map
exports label, value, hint, flags, actions, and the actual rectangle
from the retained semantic model. The node map adds the retained
node ID and both child orders. Retained rectangles are already in
window coordinates and do not need a separate transform field.

The extra native control changes traversal order and paint order
independently on a mounted stack and checks that the JSON arrays
match the orders used by actual accessibility dispatch. It also
verifies that an obscured input never exposes its retained text in
JSON, and that actions serialize in protocol order, not callback
registration order. Eight focused native tests passed.
