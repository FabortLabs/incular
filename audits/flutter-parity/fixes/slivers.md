# Fixes: slivers group

Nine findings assigned (eight slivers, one implicit). Outcomes: 6 fixed, 3 blocked.

## slivers-001 - fixed
- Cause: `SliverVariedExtentList` rows were laid out with unbounded main-axis constraints.
- Change: `VariableExtentRenderSliver.tight_main_extent`; the varied list sets it, so rows take their indexed extent.
- Test: `sliver_fixes::varied_extent_rows_take_their_indexed_main_extent` (pass).
- Harness: `slivers__varied_extent_list__indices` now matches.

## slivers-002 - blocked
- Cause: `SliverFloatingHeader` snap defaulted to false.
- Change: default true; `snap(false)` is the opt-out (API default change, migration noted in the JSONL).
- Tests: `floating_header_default_snap_starts_a_run_after_scroll_end` and `floating_header_snap_can_be_disabled_explicitly` (pass).
- Harness: `slivers__floating_header__default_snap_after_wheel_scroll` still diverges at frame 5. The wheel path emits Start and End, but the snap run does not start in the harness. Not resolved.

## slivers-003 - fixed
- Cause: `FillRemainingRenderSliver` carried the largest past extent forward.
- Change: extent recomputed from the current remaining space and child hint.
- Test: `fill_remaining_without_scroll_body_follows_preceding_growth_back` (pass). Harness now matches.

## slivers-004 - fixed
- Cause: `KeepAlive` dropped its request; no channel reached the sliver.
- Change: the request travels as a flag on the child descriptor (`SemanticProperties.keep_alive`); `VariableExtentRenderSliver` keeps requesting rows outside the cache window.
- Test: `keep_alive_child_stays_mounted_after_scrolling_out_of_the_cache_window` (pass). Harness presence now matches; the kept row is laid out at y=-900 where Flutter reports no geometry (residual).
- Limitation: fixed-extent lists and grids do not honour the request yet.

## slivers-005 - fixed
- Cause: resizing header used the pinned persistent-header cache formula.
- Change: `calculate_cache_offset(0..current)` for Pinned and FloatingPinned.
- Harness: `slivers__resizing_header__collapse_scroll` now matches. No unit test; the scenario is the regression.

## slivers-006 - fixed
- Cause: the trailing cache edge was inclusive; Flutter excludes a row starting exactly at it.
- Change: `MeasuredExtentIndex::flutter_child_range` (new) used by the variable-extent sliver. `materialized_range` is unchanged.
- Test: `crates/incular-scroll/tests/sliver.rs::flutter_child_range_includes_leading_edge_row_and_excludes_trailing_edge_row` (pass).
- Harness: `slivers__list_builder__default_cache_window` now matches.
- Limitation: `crates/incular-scroll/tests/scroll.rs` encodes the old edge for `materialized_range`; changing that globally would break it, and that file belongs to the other worker.

## slivers-007 - fixed (source-only)
- Change: `SliverFillRemaining::fill_overscroll(bool)` and `FillRemainingRenderSliver::overscroll_layout`.
- No runtime test: the attempted bouncing-overscroll test did not produce overscroll and was removed.

## slivers-008 - fixed (source-only)
- Change: `FloatingHeaderSnapMode` (Overlay default, Scroll), `snap_duration`, `snap_curve` builders; re-exported from `lib.rs`.
- Limitation: the Scroll-mode layout effect is unverified at runtime; no reverse duration or curve.

## implicit-001 - blocked
- Experiment: start the adopted run from the current frame time. Scenario output was identical, so reverted (`sliver_fade.rs` matches HEAD).
- Still diverges at frame 4 (Flutter 0.99632, Incular 1.0). Cause not confirmed; the update may reach the render sliver one frame after Flutter's `didUpdateWidget`.

## Validation
- `cargo test -p incular-widgets --test sliver_fixes` 5 pass; `snap_headers` 23, `floating_headers` 4, `advanced_slivers` 4, `tree_scrolling_slivers` 39 pass.
- `cargo test -p incular-scroll --test sliver` 3 pass; `--test scroll` 25 pass.
- Harness (Incular side, against existing Flutter recordings): `slivers__` 12 of 15 match; diverging: floating header (002), keep-alive residual rect (004). `implicit__sliver_opacity` diverges (implicit-001).
- `rustfmt --check` clean on owned files. Clippy blocked by a lint in `crates/incular-widgets/src/scrolling/scroll_views.rs` (other worker).

## Out-of-ownership edits
- `crates/incular-widgets/src/tree/specs.rs`: `SemanticProperties.keep_alive` field.
- `crates/incular-widgets/src/tree/widget/constructors.rs`: `with_keep_alive_request` and `keep_alive_request`.
- `crates/incular-widgets/src/lib.rs`: re-export `FloatingHeaderSnapMode`.
- New test file `crates/incular-widgets/tests/sliver_fixes.rs`.

## Harness note
The Flutter side of `run-frames.sh` was queued behind other workers' Flutter processes; the Incular side was run against the existing recordings from 20:52 and 23:57, which share the same twins.

## Residual pass (residual-input worker)

- **slivers-002: fixed.** `slivers__floating_header__default_snap_after_wheel_scroll` matches. There were three causes:
  1. Incular revealed the header on any backward delta. Flutter `_RenderSliverFloatingHeader` reveals only while `userScrollDirection` is forward. A wheel (`pointerScroll` -> `goIdle`) or a `jumpTo` lays out with an idle direction, so the effective offset only clamps to the scroll offset (10 of 70 visible).
  2. The snap used a half-visible threshold. Flutter snaps in the user direction at scroll end, starting from the previous layout's effective offset (100).
  3. Sliver ticks ran in the compositor pass after layout, so each snap value was laid out one frame late.
  - Change: `FloatingHeaderRenderSliver` is now a port of the Flutter render object (`layout.rs`: `FloatingHeaderState`, plus a `FloatingScrollSignal` listener that derives the direction from UserScroll samples or drag updates and ignores `jump_to` brackets). `WidgetTree::tick_animations` now ticks animating sliver viewports before layout (`interaction.rs`).
  - Tests: `sliver_fixes.rs::wheel_back_reveals_the_floating_header_through_the_snap_only`. `floating_headers.rs` (3 tests) now drives a user-scroll activity, because `jump_to` alone never reveals the header in Flutter. Two `snap_headers.rs` replacement tests moved to the direction rule.
  - Limitations:
    - Drags emit no UserScroll, so their direction comes from Update deltas; `animate_to` also takes its delta direction, where Flutter stays idle.
    - The persistent and resizing floating headers keep `HeaderScrollState`.
- **implicit-001: fixed** as a side effect of the pre-layout sliver tick. `implicit__sliver_opacity__opacity` matches.
- Harness: `slivers__` scenarios all match except the keep-alive residual rect (slivers-004).
- Not caused by this pass: 6 `snap_headers.rs` tests, `floating_headers.rs::returning_header_paints_and_hits_above_flowing_content` and 10 `tree_scrolling_slivers.rs` tests fail because `tree.paint()` currently emits no `PaintCommand::Rect` for scroll-viewport content. Plain `SingleChildScrollView` tests fail too, and those tests do not touch code changed here. The two replacement tests above passed before the concurrent paint changes landed.
