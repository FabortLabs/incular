# Custom sequential focus policies

The pinned Flutter 3.50.0 originals at
`widgets/focus_traversal_test.dart:394,3418` passed. With seven
real mounted nodes, a custom first/current/last sorter preserves
direct focus on the third node, then selects the last on Next
and the first on Previous. It does not remove any node from the
eligible direct or spatial focus set.

Looking up the custom policy follows an explicitly reparented focus
node into its new focus group, even though it remains visually
inside the old widget-order group. The retained test compares the
actual attached parent and policy identity. An authored control
also rejects duplicate/out-of-range selection indices without
producing invalid traversal stops. Twenty-one focused tests passed,
including the adjacent built-in and retained-group controls.
