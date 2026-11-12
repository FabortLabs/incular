# Gesture parity audit

Flutter reference: `C:/Manual Installs/flutter/packages/flutter/lib/src` (Flutter 3.41.7).

## Coverage

| Area | Method | Result |
| --- | --- | --- |
| GestureDetector tap | `gestures__tap` frame twin | Match; initial geometry and tap values agree. |
| Double tap | `gestures__double_tap`, `onDoubleTap` only | Diverges at frame 5 (80 ms); initial values and frame count agree. |
| Long press | `gestures__long_press`, `onLongPress` and `onLongPressStart` | `onLongPressStart` position agrees; `onLongPress` fires at the deadline in Flutter but only on pointer-up in Incular. |
| Drag recognizer, slop, `DragStartBehavior`, velocity tracking | `gestures__vertical_drag_down`; ten-pixel moves then a 30 px move; explicit `DragStartBehavior.down` | Match, including start position, total delta, and release velocity. Default-start scroll case is separately covered by `sample_list_fling`, which diverges by one 40 px move. |
| Gesture arena | `sample_list_fling`; single scroll recognizer | Diverges at the first slop-crossing move. Flutter's singleton arena resolves on pointer-down; Incular waits for the recognizer's explicit acceptance. |
| Scale recognizer | `gestures__scale_single_pointer_pan`, one touch pointer | Diverges before the first move: Flutter starts on the sole-member arena win; Incular waits until focal movement exceeds pan slop. Multi-pointer pinch was not covered because FrameHarness has only one touch pointer. |
| `Draggable`, `LongPressDraggable`, `DragTarget` | Source review; attempted a `LongPressDraggable` twin | No accepted runtime comparison. Flutter's `Draggable` requires an `Overlay`; FrameHarness mounts no `Overlay`. Source review found Incular's `LongPressDraggable` converts directly to `Draggable`, omitting Flutter's delay. Add an Overlay wrapper to the Flutter frame harness before adding this scenario. |
| `Dismissible` | Not exercised | No deterministic twin added in this pass; left open after prioritizing runnable recognizer cases and the confirmed scroll regression. |
| `InteractiveViewer` | Not exercised | No deterministic twin added in this pass; single-pointer pan can be covered in a follow-up. Pinch additionally needs a second pointer in FrameHarness. |
| `ReorderableList` | Not exercised | No deterministic twin added in this pass; drag-proxy/overlay behavior needs a harness setup that has not been added. |

All accepted gesture scenarios start with matching frame-0 probe geometry and values. The double-tap, long-press, and scale divergences therefore do not come from different initial layout. The double-tap twins have the same 9-frame timeline after correction.

## Findings

### gestures-001 — Scroll drags drop the first move when the scrollable is the sole arena member

**Priority:** P1. **Confidence:** high.

`sample_list_fling` has identical twins and input: six 40 px moves, one frame between each move, then a 90-frame settle (`harness/flutter/test/frames/samples_test.dart:82-104`, `harness/incular/tests/frames/samples.rs:100-126`). Frame 0 matches. The first divergence is frame 3 at 48 ms: Flutter offset `40.0` and row0 y `-40.0`; Incular offset `0.0` and row0 y `0.0`. Every later sampled offset remains 40 px apart. At 1568 ms Flutter ends at `1193.7479`, Incular at `1153.7479`.

Flutter closes the arena after pointer-down (`gestures/binding.dart:526-532`); a closed arena with one member picks it by default (`gestures/arena.dart:251-276`). The list's scroll recognizer therefore already owns the pointer when the first move arrives, and Flutter's accepted drag path reports that move through `_checkUpdate` (`gestures/monodrag.dart:712-720`). Incular's arena adds the member but has no close/default-singleton resolution (`crates/incular-gestures/src/arena.rs:81-96`). Its recognizer instead accepts on the threshold-crossing move and the `DragStartBehavior::Start` path drops that pending delta (`crates/incular-gestures/src/pointer.rs:635-665`).

**Fix direction:** implement Flutter's close-after-down and default winner for a singleton arena, preserving competition when multiple members joined. Recheck both scroll movement and scale start after the arena change.

### gestures-002 — Double-tap-only detectors never retain the first tap

**Priority:** P1. **Confidence:** high.

In `gestures__double_tap`, frame 0 matches and both twins produce 9 frames. At frame 5 / 80 ms Flutter reports `double_tap.count=1`; Incular reports `0` through the remaining frames. The scene intentionally sets `onDoubleTap` without `onTap`.

Flutter uses a dedicated `DoubleTapGestureRecognizer` (`gestures/multitap.dart:122-150`); it tracks a first tap on pointer-up and registers the second independently (`:257-263`, `:328-345`). Incular recognizes that a double-tap callback exists, but only records `last_tap` inside its ordinary tap-callback branch (`crates/incular-gestures/src/pointer.rs:763-782`). With `onDoubleTap` alone, the first up returns `Reject` and the second press has no first-tap state.

**Fix direction:** keep double-tap first-tap tracking independent of `onTap` callbacks, matching Flutter's separate recognizer lifecycle.

### gestures-003 — `onLongPress` is delivered on release instead of at the 500 ms deadline

**Priority:** P1. **Confidence:** high.

In `gestures__long_press`, frame 0 matches. At frame 33 / 528 ms Flutter has `long_press.count=1` and `start_y=300`; Incular has `count=0` but the same `start_y=300`. The Incular `onLongPressStart` therefore fired on time, while the plain `onLongPress` callback did not. It fires on the later pointer-up, yielding a one-frame count divergence.

Flutter's deadline handler accepts the recognizer and invokes `_checkLongPressStart` (`gestures/long_press.dart:641-646`); that function calls both `onLongPressStart` and `onLongPress` (`:737-748`). Incular's deadline `poll` returns only `LongPressStart` (`crates/incular-gestures/src/pointer.rs:291-312`), while the `LongPress` action is emitted in the pointer-up branch after the elapsed-time check (`:700-738`) and dispatches `on_long_press` separately (`:818-827`).

**Fix direction:** dispatch the plain `onLongPress` callback at deadline alongside `onLongPressStart`, and keep `onLongPressEnd` at release.

### gestures-004 — Scale starts on pointer-down in Flutter's singleton arena, but waits for slop in Incular

**Priority:** P1. **Confidence:** high.

In `gestures__scale_single_pointer_pan`, frame 0 values and probe geometry match. At frame 2 / 32 ms, before the first move, Flutter reports `scale.starts=1`; Incular reports `0`.

The independently rerun `gestures__scale__two_pointer_pinch` extends the same issue to two contacts: the scene and values match at frame 0; at frame 3 / 48 ms Flutter reports scale 1.5 with two pointers, while Incular has not yet reported an update (scale remains 1). At frame 4 Flutter reports 1.8333 and Incular reports 1.0. The new second-pointer recorder path therefore delivers both contacts, but the late arena acceptance also resets Incular's initial span after the first movement (`crates/incular-gestures/src/scale.rs:368`).

Flutter starts a possible scale recognizer as soon as it accepts the pointer (`gestures/scale.dart:806-815`). The only recognizer in this scene is the scale recognizer, so Flutter's down-event arena close and singleton default winner are sufficient. Incular adds its scale recognizer to the arena (`crates/incular-widgets/src/tree/interaction.rs:716-753`) but waits for acceptance from focal/span movement in `ScaleGestureDetector::advance_state_machine` (`crates/incular-gestures/src/scale.rs:525-543`). The same missing singleton default-winner behavior described in G1 explains the result.

**Fix direction:** resolve the singleton arena on pointer-down and confirm that scale start, scale slop, and competing drag gestures still match after arbitration changes.

### gestures-005 — `LongPressDraggable` loses Flutter's long-press delay

**Priority:** P1. **Confidence:** high from source; runtime unverified.

Flutter's `LongPressDraggable` sets `delay` to `kLongPressTimeout` and creates a `DelayedMultiDragGestureRecognizer` (`widgets/drag_target.dart:404-438`, `:440-445`). Incular's `LongPressDraggable` conversion constructs a plain `Draggable` directly (`crates/incular-widgets/src/drag_drop.rs:578-599`), with no delay field or long-press recognizer. A quick swipe can therefore start a drag in Incular before Flutter permits it.

I attempted a Flutter/Incular drop scenario, but Flutter rejected the scene because `LongPressDraggable` requires an Overlay and the frame harness mounts none. No runtime first-divergence timestamp is claimed. **Fix direction:** carry a delayed-start mode through the Incular drag source and expose an Overlay in the Flutter frame harness before adding the twin.

## Harness request and pixels

Add an Overlay ancestor to the Flutter FrameHarness mount surface so `Draggable`, `LongPressDraggable`, `DragTarget`, and reorder drag proxies can be compared. Add a second touch pointer helper to cover pinch/rotation paths. These are recognizer/state divergences; the frame recorder captures the relevant callbacks and values, so no pixel-harness proposal is needed.

## Validation

- `harness/run-frames.sh gestures__`: Flutter tests passed; `gestures__tap` and `gestures__vertical_drag_down` matched. G2–G4 diverged as above.
- The first harness attempt exposed fixture issues (missing `flutter/gestures.dart`, an unexpired double-tap timer, and missing Overlay for the removed drag/drop twin); those fixture problems were corrected or the unsupported twin removed before the final run.
- `cargo fmt`, `cargo clippy`, and `flutter analyze` results are reported separately after their final runs.
- No pixel harness was run, per the audit scope.
