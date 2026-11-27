# Sequential focus scope and route edges

Flutter 3.50.0, `ab2912035e160d9cad6306df307576da6128ebdb`:
`packages/flutter/test/widgets/focus_traversal_test.dart:3817` passed.
It changes an existing FocusScopeNode from its default closed loop to leave
the view, then stop and wrap in reverse. The mounted Incular runtime runs the
same transitions. It compares each leaf's primary focus and checks whether
the operation reports movement. It also checks that a Tab that leaves focus
is not consumed by the framework.

The unchanged native default route case uses a real RouteOutlet and passes
both wrap directions, matching the Linux branch of the pinned original
`Default route edge traversal behavior` (line 3749). Web behavior remains
partial: the web route opts into leaving the view, but the browser/native
handoff and its original web branch have not been run.

An authored pinned and native comparison also drives Tab across an inner
ParentScope in both directions. After crossing the boundary, returning
enters from the approached edge rather than restoring a stale child as
if the previous traversal never escaped. Focus history is retained for
scopes that were not exited via this edge policy.

All 54 focused native tests passed in `flutter350_focus_edge_policies`,
`flutter350_focus_key_routing`, `flutter350_focus_scope_nodes`, and
`flutter_focus_traversal_upstream_parity`. The authored probe passed in
the pinned reference and adds no original-registration credit.
