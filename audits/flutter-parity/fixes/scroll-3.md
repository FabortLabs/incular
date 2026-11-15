# Fixes: scroll-3 group

Three findings plus a regression triage. Outcomes: scroll-004 fixed, slivers-012 fixed, slivers-010 deferred. All seven triaged test failures were stale expectations that the Flutter fixes correctly changed. No code regressions were found.

## Test triage

Each failure was run in this checkout and in the clean baseline (`base-e25e45bc`). All seven pass in the baseline.

- **`draggable_sheet.rs::draggable_sheet_reset_inner_listeners_observe_committed_sheet`** and **`::draggable_sheet_reset_inner_listener_mutation_orders_events`**: stale expectation.
  - They expected 1 inner notification and got 3.
  - The sheet reset restores the inner position with a changed `jump_to`. Since scroll-005, that jump is bracketed as Start, Update, End, as Flutter's `ScrollPositionWithSingleContext.jumpTo` does (`widgets/scroll_position_with_single_context.dart:197-206`). Flutter's own sheet reset animates the inner position to 0, which also opens and closes an activity bracket.
  - Update: the tests now assert the kinds `[Start, Update, End]`. The ordering contract they protect still holds. In the first test, every inner notification sees the committed sheet (0.5). In the second, the first one sees 0.5 and the later ones see the listener's reentrant drag (0.9).
- **`resizing_headers.rs::shrink_wrap_fallback_keeps_reverse_offset_coherent`**: stale expectation.
  - After `jump_to(10000)` it expected an immediate clamp to 200.
  - Since scroll-001, an attached jump keeps the raw pixels and springs back over the following frames, as Flutter's `jumpTo` then `goBallistic(0)` does.
  - Update: the test asserts that the offset is still 10000 right after the jump, pumps `pump_scroll_flings` until the spring settles (the same pattern as `tree_scrolling_slivers.rs`), and then keeps its paint, hit-test and semantics checks at 200.
- **`resizing_headers.rs::controller_driven_text_change_revalidates_same_delegate`**: stale fixture after a text-metric change.
  - It reported "growth must exceed 135, got 133".
  - The text worker removed the old 16px inset from the retained `EditableText` in `tree/layout/behavior/text.rs`. Flutter's editor has no inset of its own: `_preferredHeight` is the number of lines times the line height, and a wrapper's content padding adds to that.
  - Each header is now 16px shorter. The baseline was 35, 111 and 151; this checkout gives 19, 95 and 133. The second text only added two 19px lines, which is less than the 40px stretch.
  - Update: the second text is longer, so it adds more lines than the stretch covers. The test's intent, that growth during overscroll beyond the stale stretched total lands authoritatively, is unchanged.
- **`scroll_scrollbar_2d.rs::raw_track_click_stays_programmatic`**: stale expectation.
  - It expected no Start or End around a track click, and got `[Start, Update, End]`.
  - The page move is a changed `jump_to` (`raw_scrollbar.rs`), which scroll-005 brackets as Flutter's `jumpTo` does. Flutter's own `handleTrackTapDown` pages with `position.moveTo(..., duration: 100ms)` (`widgets/scrollbar.dart:1846-1881`), which runs a driven activity that is bracketed in the same way.
  - Update: the test asserts exactly `[Start, Update, End]`, and that the release adds nothing.
  - Residual, not fixed here: Flutter animates the page over 100ms with `Curves.easeInOut`, but Incular jumps.
- **`scroll_scrollbar_2d.rs::retained_replacement_mid_drag_ends_old_bracket`**: stale expectation.
  - The replacement controller now sees one End, where the test expected 0.
  - That End belongs to the move's own jump bracket. After the controller swap, the thumb move lands on the replacement as a changed `jump_to`, outside any open bracket there, so it brackets itself (scroll-005). It is not caused by the release.
  - Update: the test asserts that the End is present before the release, and that the release then closes only the held controller (old End count 1; the replacement stays at 1).
- **`wheel_scrolling.rs::wheel_reports_selection_on_update_but_defers_on_end`**: stale expectation.
  - It expected item 0 to be reported at the first layout.
  - Flutter seeds `_lastReportedItemIndex` with `controller.initialItem` and reports only changes from it (`widgets/list_wheel_scroll_view.dart:748-773`). This is scroll-009.
  - Update: nothing is reported at mount, and the jump reports `[5]`.

## scroll-004: Scrollable.ensureVisible (P2), fixed

- **Confirmed before fixing:** yes. No `ensure_visible` API existed.
- **Cause:** not implemented. The tree had a key lookup (`element_with_key`) and world geometry (`element_geometry`), but no operation that reveals an element through its ancestor viewports.
- **Change:** a new file, `crates/incular-widgets/src/scrolling/ensure_visible.rs`.
  - It adds `Scrollable::ensure_visible(&WidgetTree, ElementId, EnsureVisible) -> usize`.
  - `EnsureVisible` has the builders `alignment`, `duration`, `curve` and `alignment_policy`. Its defaults are Flutter's: 0.0, zero, `Curves.ease` and explicit.
  - It adds `ScrollPositionAlignmentPolicy`, with the variants `Explicit`, `KeepVisibleAtEnd` and `KeepVisibleAtStart`.
  - The walk finds each enclosing `Scroll` or `SliverViewport` render, innermost first.
    - The innermost viewport reveals the element's whole box.
    - Each outer viewport reveals the inner viewport at the place where the first target sits. It uses the target's box clipped to the object's size, as Flutter's `targetRenderObject` / `paintBounds.intersect` does.
  - The offset follows `getOffsetToReveal`: `leading - (viewport - target) * alignment`. On reversed axes, the leading edge is measured from the far side (`AxisDirection.up/left`). The result is clamped to the scroll range.
  - The keep-visible policies move only toward their edge. On reversed axes they are swapped (`_applyAxisDirectionToAlignmentPolicy`).
  - All targets are computed from the last layout's geometry before anything moves, as in Flutter. A viewport that is already at its target is skipped.
  - A zero duration calls `jump_to`; a non-zero duration calls `animate_to(duration, curve)`.
  - Everything goes through public `WidgetTree` queries, so no tree internals changed.
- **Tests:** `crates/incular-widgets/tests/ensure_visible.rs` has 7 tests, all passing:
  - explicit alignment 0, 0.5 and 1 with clamping;
  - both keep-visible policies;
  - a reversed axis;
  - `ListView` (sliver) rows;
  - nested horizontal-in-vertical viewports, where both move from the pre-scroll geometry;
  - a duration starts an animation;
  - no enclosing viewport.
- **Harness:** none. This was a source finding with no twin.
- **API change:** additive. `EnsureVisible`, `ScrollPositionAlignmentPolicy` and the associated function `Scrollable::ensure_visible` are new. The README documents them.
- **Limitations:**
  - It takes a `&WidgetTree` and an `ElementId`. There is no `BuildContext` entry point that application callbacks can call.
  - Pinned slivers do not shrink the reveal extent. Flutter subtracts `maxScrollObstructionExtentBefore`, and Flutter returns ±infinity for a pinned target itself.
  - Wheel, two-dimensional and sheet scrollables are not walked.
  - The minimum scroll extent is assumed to be 0 (see slivers-010).
  - It returns a count, not a future.

## slivers-012: viewport paint order (P3), fixed

- **Confirmed before fixing:** yes. Incular painted slivers first to last and lifted pinned or floating overlays on top. That is neither of Flutter's orders.
- **Cause:** there was no paint-order option. Layer order and hit order hard-coded "flow, then overlays".
- **Change:**
  - `scrolling/geometry.rs`:
    - `SliverPaintOrder { FirstIsTop (default), LastIsTop }`;
    - a `SliverViewportConfig.paint_order` field, included in `PartialEq` and `Debug`;
    - `sliver_paint_slots()`, which orders retained children by top-level sliver (each `SliverChildId::scope()`) and keeps each sliver's own children in their order. It returns `None` when the order is already right.
  - `CustomScrollView::paint_order` and `Viewport::paint_order` builders.
  - Layer order uses the slots in `tree/reconciliation.rs` `sync_render_children`.
  - Hit order uses the slots in `tree/painting.rs` `hit_children` (iterated in reverse, so `FirstIsTop` hit-tests first to last).
  - The old overlay lift is gone. In both orders, slivers now stack purely by their position, as Flutter's `childrenInPaintOrder` and `childrenInHitTestOrder` do (`rendering/viewport.dart:1243-1260`).
- **Tests:** `crates/incular-widgets/tests/sliver_paint_order.rs`, 2 tests, both passing:
  - the default paints last to first, and the pinned header is hit;
  - `LastIsTop` paints first to last, and the content under the header is hit.
  - Both would fail on the old order.
- **Behavior change:** under the default, a pinned or floating header placed *after* earlier slivers no longer jumps above them, which matches Flutter. A leading header, the common case, is still on top. These test files pass: `snap_headers` (23), `floating_headers` (4), `tree_scrolling_slivers` (39), `sliver_fixes` (8) and `scroll_viewport_lists` (99).
- **Harness:** `slivers__`: 15 of 16 match. The only divergence is the known slivers-004 keep-alive rect residual. No twin probes paint order.
- **Limitations:**
  - Nested `SliverMainAxisGroup` children keep first-to-last order. Flutter's group paints its children last to first.
  - The semantics child order is unchanged. Flutter visits semantics in paint order.
  - There is no pixel scene. The finding asked for one; that is an orchestrator request.
  - `ListView`, `GridView` and the other single-sliver views have nothing to order, so they gain no builder.

## slivers-010: public Viewport center/anchor (P2), deferred

- **Reason:** this needs a bidirectional viewport, which touches the scroll position contract in `incular-scroll` and every render sliver. That is larger than this pass. `Viewport` already exists as a public forward-only wrapper over `CustomScrollView` (`sliver_descriptors.rs`). It now also carries `paint_order`.
- **Design:**
  1. **`incular-scroll`:** add a real `min_offset` to the controller state, which is fixed at 0 today. Thread it through these places:
     - `commit_extent_state` and range maintaining;
     - `apply_physics` boundary conditions and the clamping and bouncing ballistic bounds;
     - `jump_to` and `correct_to` clamping, and spring-back targets;
     - page mapping, restoration, and scrollbar thumb geometry.
     Range-maintaining `adjustPositionForNewDimensions` already works on a min/max pair in Flutter.
  2. **`Viewport::center(Key)` and `anchor(f32)`:** the sequence delegate (`scrolling/layout.rs` `SequenceViewportDelegate`) splits the slivers at the center index. Port `RenderViewport._attemptLayout`:
     - `centerOffset = mainAxisExtent * anchor - correctedOffset`;
     - the slivers before the center lay out in reverse growth from the center, in reverse order;
     - the slivers from the center onward lay out forward;
     - each side gets its own `remainingPaintExtent` and cache budget;
     - apply `minScrollExtent = -(reverse scroll extents)` and `applyContentDimensions(min(0, min + extent*anchor), max(0, max - extent*(1 - anchor)))`.
  3. **`SliverConstraints`:** add a growth direction separate from the axis-direction `reverse` flag. Every `RenderSliver` (lists, grids, headers, fill, padding, groups) must mirror child placement for reverse growth. This is the bulk of the work.
  4. **Paint and hit testing:** use `computeAbsolutePaintOffset` with the growth direction. The paint order follows Flutter's center rule, so `firstIsTop` with a center stacks around the center sliver.
  5. **`SliverChildId` scoping:** the reverse slivers keep their top-level index, so `sliver_paint_slots` and keep-alive still work.
  6. **`ShrinkWrappingViewport`:** it should take axis, controller and reverse options. Today it is vertical only with a private controller.
- **Tests that would be needed:** `center` and `anchor` layout offsets against a Flutter twin (an orchestrator request: `slivers__viewport_center__anchor`), negative jumps and ballistics in `incular-scroll`, and reverse-growth placement for each sliver kind.

## Out-of-ownership edits

- `crates/incular-widgets/src/tree/reconciliation.rs` (`sync_render_children`): the layer order now comes from `sliver_paint_slots` instead of the overlay lift.
- `crates/incular-widgets/src/tree/widget/constructors.rs`: one line, `paint_order: Default::default()`, in the `SliverViewportConfig` literal.
- `crates/incular-widgets/src/lib.rs`: re-exports `EnsureVisible`, `ScrollPositionAlignmentPolicy` and `SliverPaintOrder`. I did not rustfmt it, because it holds other workers' lines.
- `crates/incular-widgets/README.md`: the paint-order and `ensure_visible` paragraphs.
- Test edits, all expectations: `draggable_sheet.rs`, `resizing_headers.rs`, `wheel_scrolling.rs` and `scroll_scrollbar_2d.rs`.

## Validation

- These test files pass: `ensure_visible` (7), `sliver_paint_order` (2), `draggable_sheet` (43), `resizing_headers` (29), `wheel_scrolling` (17), `snap_headers` (23), `floating_headers` (4), `tree_scrolling_slivers` (39), `sliver_fixes` (8), `scroll_viewport_lists` (99), `advanced_slivers` (4), `scroll_parity_page_wheel` (7) and `scroll_pages_reorder` (19) and `scroll_scrollbar_2d` (49).
- `cargo check -p incular-widgets --tests` passes.
- `cargo clippy -p incular-widgets --all-targets -- -D warnings` is clean.
- Harness:
  - `scroll__`: 7 of 14 match. The divergences are all earlier residuals: `user_scroll` counts in four drag and fling scenarios, the 0.0021 float drift in `jump_clamp`, page-view cache retention, and the wheel drag.
  - `slivers__`: 15 of 16 match (the keep-alive residual).
  - `fling`: `anim_core` (2) and `sample_list_fling` match. The three scroll fling scenarios diverge with the residuals above.
  - None of the divergences involves code changed here.
