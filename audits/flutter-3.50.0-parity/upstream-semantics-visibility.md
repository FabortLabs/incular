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
forest tests, and 24 mounted parity controls passed. An adjacent
widget descriptor test reported a focus flag expectation that is
tracked separately; it does not exercise the new validation path.
