# Scroll fixes

Outcomes are recorded per finding. Scenario reruns used `harness/run-frames.sh scroll__` and `scroll__drag_overscroll__bouncing` (Flutter 3.41.7 reference).

## scroll-005: jump notifications (P0), fixed

- **Outcome:** fixed, with one deliberate narrowing (see limitations).
- **Cause:** `jump_to` dispatched only an Update, so a changed jump had no Start or End.
- **Change:** `crates/incular-scroll/src/controller.rs` `jump_to`. A changed jump with no open activity bracket dispatches Start, Update, End. Inside an open bracket it dispatches a bare Update.
- **Tests:** `crates/incular-scroll/tests/jump_and_pages.rs` (in-range order, silent no-op, in-bracket Update only); `crates/incular-scroll/tests/scroll.rs::programmatic_movement_needs_no_bracket`; `crates/incular-scroll/tests/physics_audit.rs` (updated for the Start/Update/End first jump).
- **Harness:** `scroll__controller__jump_activity_notifications` now matches (was 0 vs 1 at frame 2).
- **Limitations:** Flutter's `goIdle` also ends an open bracket on every jump. Ending the bracket broke about 30 pinned gesture and fling contracts in `scroll_viewport_lists.rs`, so I did not do it. Jump-during-gesture notification parity is not matched.

## scroll-001: out-of-range jump springs back (P2), fixed

- **Cause:** `jump_to` clamped the target before writing it.
- **Change:** an attached controller keeps the raw offset and records a pending settle. `take_pending_settle` builds the physics ballistic simulation. `physics.rs` `commit_extent_state` keeps a springing position across unchanged republications. The tree starts the spring (`tree/interaction.rs`, see out-of-ownership edits). Unattached controllers still clamp. The layout-time corrections in `sliver.rs` moved to a new `correct_to`, which clamps without springing.
- **Tests:** `jump_and_pages.rs` (unattached clamp, attached spring, springing preserved across republication); `tree_scrolling_slivers.rs::scroll_positions_are_logical_clamped_and_clip_the_viewport` (spring frames, updated from an immediate clamp).
- **Harness:** `scroll__controller__jump_clamp`: offset 97856.2031 vs Flutter 97856.2010 at 64 ms (0.0021 diff). The value tolerance is 0.001, so this is a float-drift residual.

## scroll-003: initial scroll offset (P2), fixed

- **Change:** `ScrollController::with_initial_scroll_offset`, applied on the first publication with a non-zero viewport, clamped, not persisted. Restoration then deferred-jump take precedence over it.
- **Tests:** `jump_and_pages.rs::initial_scroll_offset_*`.
- **Harness:** none (source finding).

## scroll-011: bouncing overscroll (P2), fixed

- **Cause:** bouncing paths emitted `Overscroll` in `physics.rs` `apply_physics` (drags) and in `activity.rs` `drive_unclamped` (ballistic steps).
- **Change:** both now report Overscroll only for clamping physics. Clamping springs still report it.
- **Tests:** `jump_and_pages.rs::bouncing_drag_past_edge_emits_no_overscroll`.
- **Harness:** `scroll__drag_overscroll__bouncing`: overscroll now 0 vs 0. The remaining divergence is `user_scroll` (Flutter 1, Incular 0), which affects every drag scenario and is not part of this change.
- **Limitations:** material `RefreshIndicator` and `StretchingOverscrollIndicator` consumed Overscroll under bouncing. They must now use the Update path. Not checked by an incular-material test.

## scroll-002: PageController API (P2), fixed

- **Change:** `PageController` in `scroll_views.rs` is a newtype that Derefs to `ScrollController`. It adds `with_initial_page`, `with_viewport_fraction`, `initial_page`, `viewport_fraction`, `page()` (None before layout), `jump_to_page`, `animate_to_page`, `next_page`, `previous_page`. Page mapping follows Flutter's formulas (`pixels_for_page`, `page_for_pixels` in `controller.rs`, exported). Pre-layout requests are deferred. `From<ScrollController>` keeps callers working; `PageView::controller` takes `impl Into<PageController>`.
- **Tests:** `crates/incular-widgets/tests/scroll_parity_page_wheel.rs` (initial page, fraction jump, pre-layout jump, `page()` None); `jump_and_pages.rs` (mapping, deferred page).
- **Harness:** `scroll__page_view__fraction_initial_page` still matches (it uses `deferred_jump_to`).
- **Limitations:** page math uses the controller's fraction, not `PageView::viewport_fraction`. `animate_to_page` and `next_page` are not covered by a frame twin.

## scroll-007: PageView onPageChanged (P2), fixed

- **Change:** `PageView::on_page_changed`. `From<PageView>` wraps the view in a `NotificationListener` that calls the callback on depth-zero Update notifications when the rounded page changes. The last reported page starts at the initial page.
- **Tests:** `scroll_parity_page_wheel.rs::page_change_is_reported_once_per_rounded_page` (decision logic only).
- **Harness:** `scroll__page_view__fling_page_changed`: page and page_changes match. The remaining divergence is `probe.page0.present` (cache retention), not the callback. The twin still uses its workaround and was not edited.
- **Limitations:** the notification wiring itself is not covered by an automated test.

## scroll-004: ensureVisible, deferred

- **Reason:** it needs the keyed element's rectangle in every ancestor viewport's coordinates, plus a Flutter-aligned ancestor walk. The tree has `GlobalKey` but no keyed rect query. This is larger than this pass.

## scroll-009: ListWheel initial selection (P2), fixed

- **Change:** `advanced_scrolling/wheel.rs` `layout_with_offset` seeds `last_selected_index` at the first layout, so the starting item is not reported (Flutter `_lastReportedItemIndex`).
- **Tests:** `scroll_parity_page_wheel.rs::list_wheel_does_not_report_its_initial_item_at_mount`.
- **Harness:** `scroll__list_wheel__selected_fling` frame 0 now matches. The divergence starts later (frame 3, selected_changes 0 vs 1), because the drag does not scroll the wheel (offset 0 vs 66.7). That is a gesture or hit-test issue, not this report.

## scroll-010: ListWheel cross-axis centring (P2), fixed

- **Cause:** the retained tree laid children out with `measure = biggest()`, so each child was full-width and placed at x = 0.
- **Change:** `tree/layout/behavior/scrolling.rs` wheel branch sets each child's x to (viewport width - rendered width) / 2 using `Tree::render_size`. The headless path already centred.
- **Tests:** `scroll_parity_page_wheel.rs::list_wheel_centres_narrow_children_on_the_cross_axis` (headless path).
- **Harness:** item0 x now 250.0-550.0 (Flutter 250.5-549.5). The y and transform differ because of the drag issue above.
- **Limitations:** `WheelLayout.untransformed_rect` (used by `WheelLayout::hit_test`) still uses x = 0 for retained children, so hit testing may disagree with paint. Not verified.

## Out-of-ownership edits

- `crates/incular-widgets/src/tree/interaction.rs`: a small additive hunk in `has_pending_scroll_animations` and `start_scroll_animations`, which starts the spring back. The gestures worker owns this file.
- `crates/incular-widgets/src/tree/layout/behavior/sliver.rs`: two call sites changed from `jump_to` to `correct_to`.
- `crates/incular-widgets/src/tree/layout/behavior/scrolling.rs`: wheel child centring after layout.
- `crates/incular-widgets/tests/tree_scrolling_slivers.rs`: spring-back assertions replacing immediate clamps.

## Verification status

- Passing: `cargo test -p incular-scroll` for `activity`, `scroll`, `attachment`, `jump_and_pages`, `physics_audit`, `flutter_ballistics`, `sliver`. `incular-widgets` for `scroll_parity_page_wheel`, `wheel_scrolling`, `tree_scrolling_slivers`, `advanced_scrolling`. `cargo check -p incular --tests` and `cargo check -p incular-parity-harness --tests` pass. Clippy `-D warnings` is clean for `incular-scroll` and for my `incular-widgets` lib code.
- Failing, not attributed to this change: `incular-widgets/tests/scroll_viewport_lists.rs` has 39 failures in drag, touch, fling, nested-drag and cache-window tests. I inspected several. They assert gesture-arena accept semantics and layout cache windows, which are being changed in the gestures and layout workers' in-flight edits. They do not use the paths I changed. I could not confirm this with a clean baseline, because HEAD itself does not compile in this checkout.
- Scenario `scroll__` (8 of 17 match). The scroll scenarios above are the only ones I changed.
