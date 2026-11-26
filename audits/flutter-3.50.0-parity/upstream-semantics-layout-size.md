# Semantic bounds after layout changes

Five pinned `rendering/semantics_layout_sized_test.dart` registrations
(13, 38, 67, 87, 127) passed. Native fixtures exercise a real retained
tree and update its semantic output after layout, not a mock node.
A 100-by-100 boundary becomes 200 by 200 while retaining its render
object and semantic node. Resizing to zero removes the old hit bounds.

A BlockSemantics sibling excludes the prior semantic branch while that
branch continues relaying size changes. Native IndexedStack reproduces
the custom reference parent's selected-child visitation behavior: only
the selected branch is exposed, hidden branches can be dirtied without
leaking labels, and the same retained child returns with current bounds.
Hiding the branch containing a zero-size child does not accidentally
put it back into the semantic tree. These comparisons use the framework's
semantic tree, not a physical screen reader.

The remaining reference at line 162 passed. A retained leaf becomes
dirty while its whole branch leaves the semantics tree. Removing the
replacement branch and changing an ancestor's transform returns the
same mounted leaf at (20, 30), never at its stale (0, 0) world position.
The native comparison checks public world bounds; Flutter stores the
corresponding local transform in the node.
