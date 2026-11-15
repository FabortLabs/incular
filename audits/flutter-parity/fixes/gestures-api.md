# gestures-api fixes

Records: `gestures-api.jsonl` (one line per finding). Harness: `FLUTTER="C:/Manual Installs/flutter/bin/flutter" harness/run-frames.sh gestures__` gives 11/11 match (`harness/out/frames/report-gestures__.md`).

## gestures-007 (P1): secondary and tertiary taps and long presses — fixed
- Cause: no secondary or tertiary callbacks, and button masks did not reach gesture dispatch.
- Change: `PointerButtonClass` and `ButtonGestureCallbacks` (`incular-gestures/src/details.rs`); `GestureCallbacks::for_button_class` keeps only the press's button callbacks. `dispatch_pointer_gesture_with_hit_buttons` (`tree/interaction.rs`). `frame.rs` (outside ownership) admits secondary and tertiary Down and Up. 22 builders in `incular-widgets/src/gestures.rs`.
- Tests: `incular-gestures/tests/button_taps.rs` (10 tests); `incular-widgets/tests/gesture_buttons.rs` (3 tests).
- Limitation: no harness scenario (no right-click input). The runtime gate is covered only by compile and existing runtime tests.

## gestures-008 (P2): tap move, long-press down and cancel, force press — deferred (partial)
- Fixed: `onTapMove`, `onLongPressDown`, `onLongPressCancel` (`pointer.rs`). Movement beyond slop before the deadline now rejects the long press, as Flutter does.
- Deferred: force press. `ForcePressGestureRecognizer` still has no arena member or tree wiring. It needs a member kind like scale, plus pressure from `RawPointerEvent.sample.pressure`.
- Tests: `button_taps.rs` (4 tests).

## gestures-009 (P1): framework-painted draggable feedback — fixed
- Cause: the feedback snapshot was stored but never painted.
- Change: `drag_drop.rs` `From<Draggable>` wraps the child in a stateful `OverlayPortal` (root overlay) that places `IgnorePointer(feedback)` at the feedback's top-left. The shared revision bumps on movement.
- Test: `incular-widgets/tests/drag_feedback.rs` (surface present during the drag, removed after release).
- Limitation: the root overlay loses inherited ancestors such as `Directionality`, so a `Text` feedback fails layout under a tree that only has an ancestor direction. The tests use box feedback. Feedback offset is not painted, matching Flutter.

## gestures-010 (P2): rejected targets and nested fall-through — fixed
- Change: `tree/interaction.rs` `drag_target_ancestors`, `update_drag_target` (Flutter `updateDrag`), `finish_drag`. `tree.rs` (outside ownership): `ActiveDrag.entered`.
- Tests: `incular-widgets/tests/drag_target_paths.rs` (2 tests).

## gestures-011 (P3): DragTarget builder with candidate and rejected data — fixed
- Change: `DragTarget::builder(context, Fn(Vec<T>, Vec<T>) -> Widget)`, rebuilt through a revision.
- Test: `drag_target_paths.rs::builder_receives_candidate_and_rejected_payloads`.
- Limitation: payloads are typed `T`, not dynamic.

## gestures-012 (P2): dismissible fling — fixed
- Change: `dismissible.rs` flings with `fling_on_next_tick(velocity / 300)`, as Flutter `_moveController.fling`.
- Test: `incular-widgets/tests/dismissible.rs::a_faster_fling_dismisses_sooner_than_a_slower_one` (24 px and 12 px per 10 ms). The first draft used speeds below the fling threshold, so the drag reversed; the test uses speeds well above it.

## gestures-013 (P3): trackpadScrollCausesScale — deferred (partial)
- Fixed: `InteractiveViewer::trackpad_scroll_causes_scale` (default false). Trackpad scroll signals no longer zoom by default, as in Flutter.
- Deferred: `DeviceGestureSettings` slop overrides (no Incular counterpart).
- No regression test: `scroll_at` cannot carry a device kind.

## gestures-014 (P3): DiagonalDragBehavior weighted modes — deferred
- Flutter's modes are per-gesture axis locks with state reset at drag end. Incular's `Weighted` is stateless and matches neither. Needs lock state in the two-dimensional controller.

## Validation
- `cargo test -p incular-widgets --no-fail-fast` on detector_hover, drag_drop_options, dismissible, interactive_viewer, tap_cancel_arena, gesture_local_positions, reorderable, tree_editing_semantics_gestures, drag_feedback, drag_target_paths, gesture_buttons: all pass.
- `cargo test -p incular-gestures`: all pass (button_taps, press_deadlines, tap_cancel, flutter_timing, gestures and the rest).
- `cargo check -p incular-parity-harness --tests`: passes.
- Harness `gestures__`: 11/11 match.
- rustfmt run on owned files only. Not run on the shared `tree/interaction.rs`, `tree.rs` or `frame.rs`, which other workers edit.

## Out-of-ownership edits
- `crates/incular-runtime/src/frame.rs`: buttons passed to dispatch; secondary and tertiary Down and Up admitted.
- `crates/incular-widgets/src/tree.rs`: `ActiveDrag` gains `entered`.
- `crates/incular-widgets/src/tree/interaction.rs`: shared file, changes confined to pointer dispatch and drag-target functions.

## Notes
- An empty file I created by mistake under `.tmp/` was removed.
- Clippy results: see the final report.
