# Mounted reading-order focus directions

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
Reference: the six selected ReadingOrderTraversalPolicy registrations
in `widgets/focus_traversal_test.dart`, and the band/scope sort in
`lib/src/widgets/focus_traversal.dart`.

A horizontal band uses its nearest shared Directionality ancestor to
establish geometric order, then consecutive runs read in their local
direction. By default ties remain stable. Group edges come from the
run's union, and each pick recomputes the group of the remaining band.
Monodirectional rows still use the existing fast row sort, mirrored by
the right edge for RTL. This uses the real mounted widget ancestry,
not the node's potentially different explicit focus parent.

Before the patch the mixed RTL/LTR fixture placed node 3 before node 4.
The full expected sequence now matches in both root directions. The
overlapping RTL fixture checks equal-origin and concentric rectangles
and then rewraps the same nodes into a nonoverlapping vertical stack.
That also found a focus loss: the old element was gone, even though
the same eligible FocusNode was attached in the new tree. The host now
preserves that exact node's primary focus unless a newer explicit
request wins. The authored editable text fixture verifies that
remounting a focused editor with the same node does not select all
or discard its existing range.

Six pinned originals and the authored editor probe passed. Four native
reading-order cases and ten sequential cases passed, along with focused
scope, observer, editor, and focus-policy regressions. The audited path
is the mounted runtime; standalone FocusManager still has no widget
Directionality ancestry.
