# Directional focus scope edge policies

Flutter 3.50.0, `ab2912035e160d9cad6306df307576da6128ebdb`:
`packages/flutter/test/widgets/focus_traversal_test.dart:3977`, original
`Edge cases for inDirection`, passed. It changes the mounted child focus
scope's policy without rebuilding and presses Up/Down at both edges.

The equivalent native test uses six focusable nodes, the same geometry and
scope tiers, and compares every leaf's focus and focus-change callback.
It checks the default Stop policy, scope-local ClosedLoop, ParentScope
escape to [5]/[0] from the two edges, and LeaveFlutterView clearing the
focused control. The mounted runtime passed all eight transitions.

Directional focus previously searched the entire window and wrapped
unconditionally. It now looks within the nearest mounted focus scope first,
then applies that scope's explicit edge policy. It uses the same candidate
ranking for forward and wrapped search and does not record a wrapped move
or scope escape as an ordinary directional-history stop.

All 21 focused native tests passed across `flutter350_focus_edge_policies`,
`flutter350_directional_focus_paths`, `flutter350_focus_key_routing`, and
`focus_directional`. This batch does not claim behavior for sequential
Tab/Shift+Tab edge policies or actual browser/host focus after leaving.

## First directional target in an empty scope

Exact pinned `widgets/focus_traversal_test.dart:4115` passed. Starting
downward in a scope with no focused child can put primary focus on a
nested scope's leading edge, rather than on its padded descendant. The
native pre-fix test incorrectly focused that leaf. The initial scan now
includes eligible scope anchors without changing ordinary Tab stops or
directional moves from an active child.

Two pinned authored checks and native controls verify equal-edge order,
`skipTraversal` (the scope is skipped, its eligible child is not), and
falling back to the initiating scope when there are no candidates. A
native editor without an explicit focus node remains eligible. All four
direct checks and 21 adjacent focus checks passed.
