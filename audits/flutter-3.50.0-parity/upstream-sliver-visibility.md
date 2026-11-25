# SliverVisibility retained focus and replacement

Pinned original `widgets/sliver_visibility_test.dart:35` passes. The
native comparison covers its hidden replacement, restored visibility,
retained child and ticker-mode branches with fixed-size layout probes.
It does not compare the complete original matrix, Ahem text metrics,
or Flutter's offstage widget finder.

The three authored reference and Incular runtime comparisons pass:

- Without maintained size, a hidden retained sliver takes no scroll
  space and mutes its subtree. With maintained size and animation it
  keeps a 120px extent and permits ticking. In either mode the retained
  child can keep focus, explicitly unfocus and request focus again.
- SliverVisibility does not introduce a box Visibility.of ancestor.
  Without an enclosing box Visibility, the child reads true regardless
  of the sliver's own paint state.
- An initially hidden, nonretaining lazy list builds no children.
  Showing it invokes the lazy builder; hiding it again does not build
  anything. Before the fix Incular materialized nine hidden items in
  each direction of the hide transition even though none entered the
  retained tree.
