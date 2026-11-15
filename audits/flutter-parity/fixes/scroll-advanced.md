# Fixes: scroll-advanced group

Seven findings plus one test check. Outcomes: 5 fixed, 2 deferred. The test check is also fixed.

## Shared mechanism: scroll drive delegates

I added a seam in `incular-scroll`: `ScrollDriveDelegate` (`coordinator.rs`). It stands in for Flutter's custom `ScrollPosition`s. A delegate installed on a controller takes over two things:
- user drag updates (`ScrollController::apply_user_drag`);
- the release ballistic (`delegated_ballistic`). This returns per-controller runs, each a simulation in that controller's own offset space.

Changes that support it:
- `FlingSimulation` gains a `NestedMap`, so several positions can read one shared simulation.
- `request_ballistic` lets a delegate hand a fling back to its list.
- The tree calls these from four places in `tree/interaction.rs`: the drag drive, the press chain, `maybe_begin_fling`, and the new `start_handed_over_ballistics`.

## scroll-012: NestedScrollView drag (P1), fixed

- **Confirmed before fixing:** yes. The header stayed at 0; Flutter's moved to -50 at frame 3.
- **Cause:** the body sat in a `SliverToBoxAdapter`, so it had unbounded height, and nothing coordinated the two positions.
- **Change:**
  - The body now sits in `SliverFillRemaining`, as Flutter's `_buildSlivers` does.
  - `NestedScrollViewCoordinator` ports `applyUserOffset`, `goBallistic` and `_getMetrics`. A drag up collapses the header, then scrolls the body. A drag down returns the body to its start, then expands the header; with `float_header_slivers` the header moves first. A fling runs as one combined simulation shared by both positions, so it crosses between them.
  - The body's innermost same-axis viewport is bound as the inner position on the first press over it.
  - The coordinated pair joins the gesture arena as one member. Flutter's trace shows the first move applied before slop.
- **Tests:**
  - `crates/incular-scroll/tests/nested_scroll_view.rs`: 6 tests.
  - `scroll_viewport_lists.rs::nested_scroll_view_drag_collapses_the_header_then_scrolls_the_body`.
- **Harness:** `scroll__nested_scroll_view__drag_offsets` matches.
- **Limitations:**
  - A drag that starts on the header before any press on the body moves only the header.
  - Wheel scrolling, nested `animateTo`/`jumpTo`, overlap absorbers and multiple inner positions are not coordinated.
  - The partner position gets no Start/End bracket of its own.

## scroll-013: DraggableScrollableSheet drag (P1), fixed

- **Confirmed before fixing:** yes. The size stayed at 0.5; Flutter reached 0.583 at frame 3.
- **Cause:** the tree never routed drags to the sheet, and a size change did not relayout the sheet.
- **Change:**
  - A `SheetDrive` delegate on the inner controller ports `applyUserOffset` and `goBallistic`.
  - The sheet ballistic is a `ClampingScrollSimulation` over the sheet's pixels, run on a controller the sheet owns.
  - At an edge, the sheet hands its velocity plus the tolerance to the list. The list's fling starts on the current frame's clock (`start_handed_over_ballistics`).
  - A committed size marks the sheet for layout: sheets are now tracked with the wheels in `tree.rs`, and `reconciliation.rs` checks the flag.
- **Tests:** `scroll_viewport_lists.rs::draggable_sheet_grows_under_a_drag_from_its_list`.
- **Harness:** `scroll__draggable_sheet__drag_extent` matches, including the list fling after the sheet reaches 1.0.
- **Limitations:** a snapping sheet keeps the plain list ballistic, because `_SnappingSimulation` is not ported.

## slivers-009: persistent header delegate (P2), fixed

- **Change:** `SliverPersistentHeader::with_delegate(min, max, |shrink_offset, overlaps_content| ...)` returns a `SliverResizingHeader` that builds its child on every layout.
  - `shrink_offset` is `min(scrollOffset, maxExtent)`, or the effective offset for floating headers.
  - `overlaps_content` is `pinned && constraints.overlap > 0`, per Flutter `rendering/sliver_persistent_header.dart:423`.
- **Tests:** `sliver_fixes.rs::persistent_header_delegate_builds_with_the_shrink_offset`.
- **Limitations:** there is no `shouldRebuild`, snap configuration or stretch configuration.

## slivers-010: Viewport center and anchor (P2), deferred

- **Reason:** this needs a bidirectional viewport port. Slivers before `center` lay out with reversed growth, the scroll offset becomes relative to the anchor, and the scroll range needs a negative minimum (`incular-scroll` fixes the minimum at 0). Paint and hit-testing must also handle the reversed slivers, and `ShrinkWrappingViewport` must size itself to its content. That is too large for this pass.

## slivers-011: SliverVisibility (P2), fixed

- **Change:**
  - Added `replacement_sliver`, the five maintain flags, `SliverVisibility::maintain`, and Flutter's flag assertions.
  - With `maintain_size`, a hidden sliver keeps its geometry while the box `Visibility` wrap hides its children.
  - Without maintained state, a replacement sliver takes its place.
- **Tests:** `sliver_fixes.rs::sliver_visibility_maintain_size_keeps_the_scroll_extent`.

## slivers-012: paint order (P3), deferred

- **Reason:** the order lives in the tree's sliver-viewport paint and hit-test passes (`tree/painting`), which are outside this group's files.
- **Remaining work:**
  - a `SliverPaintOrder` option in the sliver viewport config;
  - reverse (`firstIsTop`) or forward iteration in paint and hit-test;
  - a pixel scene to fix the default.

## switchers-006: SliverAnimatedList item animation (P2), fixed

- **Change:**
  - `SliverAnimatedList::animated(count, |index, AnimatedItem| ...)` gives the builder the item's id, index, value and phase. An item is rebuilt when its value or phase changes.
  - `SliverAnimatedListController::animation_for_id` and `animation_for_index` are added. The audit asked for these for a future insert twin.
- **Tests:** `scroll_pages_reorder.rs::sliver_animated_list_builder_receives_the_item_animation`.
- **Harness:** both `switchers__sliver` scenarios match.

## viewport_resize_during_drag_re_ranges_coherently: fixed

- **Flutter behavior:** `RangeMaintainingScrollPhysics.adjustPositionForNewDimensions` (`scroll_physics.dart:577-645`) keeps the pixels and clamps an in-range position. It does not follow the trailing edge, so Incular was wrong.
- **Change:**
  - `commit_extent_state` now applies Flutter's rule.
  - A `PageViewport` snap keeps its fractional page across a viewport resize (`_PagePosition.applyViewportDimension`). PageView had been relying on the trailing-edge anchor for this.
- **Tests:**
  - The original start offset of 60 is restored. The position stays at 100 after the shrink and reaches 122 after the next move.
  - Two tests that encoded the anchor were updated to Flutter's values and renamed: `range_maintaining_growth_at_edge_keeps_pixels_during_drag` and `incular-scroll/tests/scroll.rs::range_maintaining_keeps_pixels_and_clamps_when_extents_change`.

## Out-of-ownership edits

- `crates/incular-widgets/src/tree/interaction.rs`:
  - the drag drive calls `apply_user_drag`;
  - `bind_drag_chain` runs after the chain is built, and the coordinated outer link is removed after the hold;
  - a delegated-ballistic branch in `maybe_begin_fling`;
  - `start_handed_over_ballistics`, called at the end of `pump_scroll_flings`.
- `crates/incular-widgets/src/tree.rs`: `WidgetKind::DraggableScrollableSheet` joins the `wheels` tracked set.
- `crates/incular-widgets/src/tree/reconciliation.rs`: a sheet loop in `refresh_wheel_ranges`.
- `crates/incular-widgets/src/lib.rs`: re-export `PersistentHeaderBuilder`.

## Validation

- **Harness:**
  - `scroll__`: 10 of 17 match. Both P1 scenarios now match. The remaining divergences are earlier residuals: `user_scroll` counts, `jump_clamp` float drift, the page-view cache and the wheel drag.
  - `slivers__`: 15 of 16 match. The remaining one is the keep-alive rect residual from slivers-004.
  - `switchers__sliver`: 2 of 2 match.
  - `fling` scenarios: unchanged.
- **Tests that pass:**
  - `incular-scroll`: all test files, including the new `nested_scroll_view`.
  - `incular-widgets`: `scroll_viewport_lists` (99), `tree_scrolling_slivers` (39), `snap_headers` (23), `floating_headers` (4), `advanced_scrolling`, `advanced_slivers`, `sliver_fixes` (8), `scroll_parity_page_wheel`, `advanced_scrolling_retained_integration`, `scroll_pages_reorder` (19).
- **Failures I did not cause** (they do not use the paths I changed):
  - `draggable_sheet.rs`: two reset tests expect 1 inner event and get 3. This comes from scroll-005's Start/Update/End on `jump_to`.
  - `resizing_headers.rs::shrink_wrap_fallback_keeps_reverse_offset_coherent`: an attached `jump_to(10000)` now springs back instead of clamping (scroll-001).
  - `resizing_headers.rs::controller_driven_text_change_revalidates_same_delegate`: text metrics, 133 vs 135.
  - `wheel_scrolling.rs::wheel_reports_selection_on_update_but_defers_on_end`: scroll-009's initial-selection seeding.
- **Clippy:** `-D warnings` is clean for `incular-scroll` and `incular-widgets --all-targets`.
