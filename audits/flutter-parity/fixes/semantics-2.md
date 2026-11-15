# semantics-2 fixes

Harness: `harness/run-frames.sh semantics__` gives 18 of 19 matching. The one divergence is `enabled_default` (raw bool vs tristate; known residual, outside this group). `opacity_zero` now matches.

## semantics-010 (fixed): sort key
- Cause: no ordinal sort key; sibling order was widget order.
- Change: `OrdinalSortKey`, `Semantics::sort_key`; `sort_sibling_groups` in `tree/semantics.rs` sorts compatible-key runs (Flutter `_childrenInTraversalOrder`).
- Test: `semantics_flutter_parity.rs::sort_keys_reorder_keyed_siblings_and_keep_unkeyed_order`.
- Limit: live key-change re-sort untested (relies on the existing Subtree mark).

## semantics-011 (fixed): showOnScreen
- Change: `ShowOnScreen` kind and action; advertised inside viewports; `semantic_show_on_screen` jumps the nearest viewport; runtime dispatch arm; AccessKit `ScrollIntoView` mapping.
- Tests: `nodes_inside_a_scroll_view_advertise_show_on_screen`; runtime `show_on_screen_scrolls_the_nearest_viewport_to_the_node` (offset 380).
- Limit: AccessKit mapping untested; immediate jump, not animated.

## semantics-012 (fixed, boundary partial): container and explicitChildNodes
- Change: `container(true)` forces a node; `explicit_child_nodes(true)` on a merging element keeps children separate.
- Tests: `container_forces_a_node_for_a_bare_annotation`, `explicit_child_nodes_keep_children_out_of_the_merged_label`.
- Limit: container is not a merge boundary under a MergeSemantics ancestor.

## semantics-009 (blocked for desktop): increased and decreased values
- Mobile projection carries them (test `increased_and_decreased_values_reach_the_mobile_projection`).
- AccessKit 0.24 has no property for them; desktop blocked on the library.

## implicit-004 (fixed): opacity always-include
- Change: `always_include_semantics` on SemanticProperties and Widget; a zero-alpha (0..255 rounded) Opacity drops child semantics unless flagged; builders on Opacity, FadeTransition, AnimatedOpacity, SliverOpacity; a controlled alpha change marks Subtree.
- Tests: `zero_opacity_hides_child_semantics_unless_always_included`; updated `clip_opacity.rs::opacity_zero_stays_hittable_and_hides_semantics_unless_included` and `tree_layout_painting.rs::transparent_opacity_keeps_hit_testing_and_semantics` (old assertions encoded the paint-only design).
- Harness `semantics__opacity_zero`: match.
- Limit: SliverAnimatedOpacity and SliverFadeTransition have no flag.

## semantics-013 (deferred): CustomPaint regions
- Reason: the pass keys nodes by element; regions need synthetic nodes, a public painter API and a harness scene.
