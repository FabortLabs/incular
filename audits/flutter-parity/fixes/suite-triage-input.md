# suite-triage-input

These tests passed on the `e25e45bc` baseline and failed on `fix/flutter-parity` after the fix pass. The Flutter reference is 3.41.7 (`C:/Manual Installs/flutter/packages/flutter/lib/src`). All nine tests encoded old, non-Flutter behavior, or depended on tree structure. None is a framework regression, so I changed no framework code. I updated test files only, each with a Flutter citation.

Shared cause for the gesture tests (gestures-001): when a drag recognizer is the only member of its arena, Flutter accepts it right after the press (`gestures/arena.dart:254-255`, `_tryToResolveArena` -> `_resolveByDefault`). `acceptGesture` then calls `_checkDrag` (`gestures/monodrag.dart:738-745, 796-812`). Under `DragStartBehavior.start` that starts the drag at the press with no pending delta, so every later move is an update, the first one included. The slop only matters when another recognizer contests the arena.

## incular-runtime pointer_input::touch_fling_interruption_stops_demand
- Cause: gestures-001.
- Verdict: the old expectation was not Flutter behavior. The test is updated.
- Justification: a press on a flinging list holds the fling, which ends it (End 1). The list's drag recognizer is the sole member, so it is accepted and `onStart` turns the hold into a drag. The release then calls `_checkEnd` (`monodrag.dart:763-764`) with zero velocity, which goes idle and emits End 2. Flutter therefore emits 2 Ends, not 1.
- Change: the test now expects `ends == 2`, with the citation in a comment.

## incular-widgets scrollable_options::start_behavior_scrolls_only_after_the_slop
- Cause: gestures-001. The list had no other arena member, so the drag won at the press and all 35 px scrolled.
- Verdict: the old expectation was not Flutter behavior for a sole member. The test is updated, and its intent is kept.
- Change: `drag_list(behavior, contested)`.
  - The contested case makes every row a tappable `GestureDetector`, so the drag wins only at the slop. `start_behavior_scrolls_only_after_the_slop` still expects 10 px, and `down_behavior_applies_the_slop_movement` still expects 35 px.
  - New test `sole_start_behavior_drag_reports_every_move` expects 35 px, with the arena and monodrag citations.

## incular-widgets stretch_overscroll::pulling_past_the_start_stretches_then_springs_back
- Cause: gestures-001. The first 30 px move is now overscroll too.
- Verdict: test updated.
- Justification: Flutter adds each overscroll into `_totalOverscroll` and pulls by `_totalOverscroll / viewportDimension` (`widgets/overscroll_indicator.dart:738, 752-754`). That gives (30 + 50) / 100 = 0.8.
- Change: the test now expects `distance = 0.8`.

## incular-widgets utility_coverage::split_view_pan_reports_main_axis_delta, split_view_callback_replacement_applies_to_new_gestures
- Cause: gestures-001. The divider's `GestureDetector` pan is the only arena member.
- Verdict: test updated.
- Justification: the sole pan is accepted at the press, before kPanSlop. With the default `start` behavior the 40 px move is an update and so is the 20 px move.
- Change: the tests now expect deltas `[40, 20]`, and the first move is asserted to be an update.

## incular-material w7_slider_behavior::range_drag_uses_press_origin_not_accumulated_total_delta, second_range_drag_uses_the_new_start_value
- Cause: gestures-001. The thumb's horizontal drag is the only member at the thumb.
- Verdict: the old expectation was not Flutter behavior. I checked this one skeptically.
  - Flutter's `_RenderRangeSlider` puts its drag and tap recognizers in one `GestureArenaTeam` (`material/range_slider.dart:931-943`). The team is therefore a single arena member and wins by default at the press.
  - The team gives the win to the drag, which is its first member (`gestures/team.dart:41-45`). The drag starts at the press.
  - Flutter then maps every `onUpdate` from the pointer's absolute position (`_handleDragUpdate`, `range_slider.dart:1384-1436`), so the thumb stays under the pointer.
  - In the old 0.4 result the thumb lagged the pointer by the 20 px slop. The new 0.5 result has the thumb exactly under the pointer at x = 100 on the 200 px track.
  - The test still separates press-origin travel (0.5) from a running sum of totals (which would clamp near 0.8).
- Change: the first drag now expects 0.5. The second drag presses the moved thumb at x = 100 and travels 40 px, and expects 0.5 -> 0.7.
- Limitation, not fixed here: Incular's range slider uses per-thumb delta detectors on a track with no padding. Flutter selects the thumb at the press and uses a track inset by the overlay radius. That is a separate geometry and structure parity gap.

## incular-material sliver_app_bar::floating_material_header_reappears_with_measured_bottom_content
- Cause: slivers-002. A floating header now reveals itself only during a user scroll.
- Verdict: test updated.
- Justification: `RenderSliverFloatingPersistentHeader.performLayout` allows expansion only when `userScrollDirection` or the last-started direction is `forward` (`rendering/sliver_persistent_header.dart:661-673`). A bare `jumpTo` is idle and never reveals the header.
- Change: the test calls `controller.begin_activity()` before the jumps, the same pattern `floating_headers.rs` uses.

## incular-material sliver_app_bar::text_style_change_reflows_bottom_through_render_updates
- Cause: text-004. `DefaultTextStyle` now publishes two environment scopes, its paragraph policy and its style. The test's fixed `children(children(content)[0])[0]` walk landed on the viewport (200x200) instead of the header (145).
- Verdict: this was a structural assumption in the test, not a behavior change and not a regression.
- Change: the test now walks the single-child chain down to the viewport, the first element with several children, and takes its first sliver.

## Not in scope
- utility_coverage `safe_area_*`: 6 tests also fail. They belong to the layout and widget triage group, and I did not touch them.

## Validation
- `cargo test` passes for every target named above.
- `cargo clippy -- -D warnings` is clean for those test targets.
- `rustfmt --check` is clean for the six files edited.
- No harness rerun was needed, because no framework code changed.
