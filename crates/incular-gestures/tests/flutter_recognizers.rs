//! Flutter 3.41 `ForcePressGestureRecognizer`, `MultiTapGestureRecognizer`,
//! `SerialTapGestureRecognizer`, the multi-drag recognizers and the
//! tap-and-drag recognizers. Expected sequences follow the Dart sources
//! (`gestures/force_press.dart`, `multitap.dart`, `multidrag.dart`,
//! `tap_and_drag.dart`) with the recognizer alone in each arena, where the
//! sole member wins when the arena closes after the press, unless a test
//! supplies an external arena.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::{
    NormalizedPressure, Offset, PointerPhase, PointerSampleMetadata, time::Instant,
};
use incular_gestures::{
    Drag, DragEndDetails, DragUpdateDetails, ForcePressGestureRecognizer, GestureDisposition,
    MultiDragGestureRecognizer, MultiTapGestureRecognizer, PointerDeviceKind, RawPointerEvent,
    SerialTapGestureRecognizer, TapAndDragGestureRecognizer,
};

type Log = Rc<RefCell<Vec<String>>>;

fn log() -> Log {
    Rc::new(RefCell::new(Vec::new()))
}

fn event(
    base: Instant,
    ms: u64,
    pointer: u64,
    kind: PointerDeviceKind,
    phase: PointerPhase,
    x: f32,
    y: f32,
) -> RawPointerEvent {
    let buttons = match phase {
        PointerPhase::Down | PointerPhase::Move => 1,
        _ => 0,
    };
    RawPointerEvent::new(
        pointer,
        0,
        kind,
        buttons,
        Offset::new(x, y),
        phase,
        base + Duration::from_millis(ms),
    )
}

fn touch(
    base: Instant,
    ms: u64,
    pointer: u64,
    phase: PointerPhase,
    x: f32,
    y: f32,
) -> RawPointerEvent {
    event(base, ms, pointer, PointerDeviceKind::Touch, phase, x, y)
}

fn pressed(event: RawPointerEvent, pressure: f64) -> RawPointerEvent {
    event.with_sample(PointerSampleMetadata {
        pressure: NormalizedPressure::new(pressure),
        stylus: None,
    })
}

fn force_press(log: &Log) -> ForcePressGestureRecognizer {
    let entry = |name: &'static str, log: &Log| {
        let log = log.clone();
        move |details: incular_gestures::ForcePressDetails| {
            log.borrow_mut().push(format!(
                "{name} {} ({}, {})",
                details.pressure, details.global_position.x, details.global_position.y
            ));
        }
    };
    ForcePressGestureRecognizer::new()
        .on_start(entry("start", log))
        .on_update(entry("update", log))
        .on_peak(entry("peak", log))
        .on_end(entry("end", log))
}

#[test]
fn force_press_starts_peaks_and_ends() {
    // Alone in the arena the press is accepted when the arena closes, with
    // the pressure (0.2) below startPressure 0.4: `acceptGesture` only moves
    // to `accepted`. Passing 0.4 starts it (onStart then onUpdate), passing
    // peakPressure 0.85 peaks it (onPeak then onUpdate), and the release
    // ends it with pressure 0 at the last position.
    use PointerPhase::{Down, Move, Up};
    let log = log();
    let mut recognizer = force_press(&log);
    let base = Instant::now();
    recognizer.handle_raw(pressed(touch(base, 0, 1, Down, 10., 10.), 0.2));
    assert!(log.borrow().is_empty());
    recognizer.handle_raw(pressed(touch(base, 10, 1, Move, 10., 12.), 0.5));
    recognizer.handle_raw(pressed(touch(base, 20, 1, Move, 10., 12.), 0.9));
    recognizer.handle_raw(touch(base, 30, 1, Up, 10., 12.));
    assert_eq!(
        log.borrow().as_slice(),
        [
            "start 0.5 (10, 12)",
            "update 0.5 (10, 12)",
            "peak 0.9 (10, 12)",
            "update 0.9 (10, 12)",
            "end 0 (10, 12)",
        ]
    );
}

#[test]
fn force_press_ignores_devices_without_pressure() {
    // `pressureMax <= 1.0`: the pointer never enters the arena.
    use PointerPhase::{Down, Move, Up};
    let log = log();
    let mut recognizer = force_press(&log);
    let base = Instant::now();
    recognizer.handle_raw(touch(base, 0, 1, Down, 0., 0.));
    recognizer.handle_raw(pressed(touch(base, 10, 1, Move, 0., 0.), 0.9));
    recognizer.handle_raw(touch(base, 20, 1, Up, 0., 0.));
    assert!(log.borrow().is_empty());
}

#[test]
fn force_press_rejects_on_the_squared_delta() {
    // Flutter compares `event.delta.distanceSquared` with the hit slop
    // (18): a 5 px move (25 > 18) rejects while still possible.
    use PointerPhase::{Down, Move};
    let log = log();
    let resolutions = Rc::new(RefCell::new(Vec::new()));
    let mut recognizer = force_press(&log).with_external_arena({
        let resolutions = resolutions.clone();
        move |pointer, disposition| {
            resolutions.borrow_mut().push((pointer, disposition));
            None
        }
    });
    let base = Instant::now();
    let down = pressed(touch(base, 0, 1, Down, 0., 0.), 0.1);
    recognizer.add_pointer(down);
    recognizer.handle_event(down);
    recognizer.handle_event(pressed(touch(base, 10, 1, Move, 4., 0.), 0.1));
    assert!(resolutions.borrow().is_empty(), "16 is within the slop");
    recognizer.handle_event(pressed(touch(base, 20, 1, Move, 9., 0.), 0.1));
    assert_eq!(
        resolutions.borrow().as_slice(),
        [(1, GestureDisposition::Rejected)]
    );
}

#[test]
fn force_press_in_a_closed_arena_starts_before_updating() {
    // A closed arena resolves synchronously in Flutter: `resolve(accepted)`
    // calls `acceptGesture`, which reports onStart before the same event's
    // onUpdate.
    use PointerPhase::{Down, Move};
    let log = log();
    let mut recognizer = force_press(&log).with_external_arena(|_, disposition| Some(disposition));
    let base = Instant::now();
    let down = pressed(touch(base, 0, 1, Down, 0., 0.), 0.1);
    recognizer.add_pointer(down);
    recognizer.handle_event(down);
    recognizer.handle_event(pressed(touch(base, 10, 1, Move, 0., 0.), 0.6));
    assert_eq!(
        log.borrow().as_slice(),
        ["start 0.6 (0, 0)", "update 0.6 (0, 0)"]
    );
}

#[test]
fn force_press_inverse_lerp_clamps() {
    assert_eq!(ForcePressGestureRecognizer::inverse_lerp(0., 1., 1.5), 1.);
    assert_eq!(ForcePressGestureRecognizer::inverse_lerp(1., 3., 2.), 0.5);
    assert!(ForcePressGestureRecognizer::inverse_lerp(1., 1., 1.).is_nan());
}

fn multi_tap(log: &Log) -> MultiTapGestureRecognizer {
    MultiTapGestureRecognizer::new()
        .on_tap_down({
            let log = log.clone();
            move |pointer, details| {
                log.borrow_mut().push(format!(
                    "down {pointer} ({}, {})",
                    details.global_position.x, details.global_position.y
                ));
            }
        })
        .on_tap_up({
            let log = log.clone();
            move |pointer, details| {
                log.borrow_mut().push(format!(
                    "up {pointer} ({}, {})",
                    details.global_position.x, details.global_position.y
                ));
            }
        })
        .on_tap({
            let log = log.clone();
            move |pointer| log.borrow_mut().push(format!("tap {pointer}"))
        })
        .on_tap_cancel({
            let log = log.clone();
            move |pointer| log.borrow_mut().push(format!("cancel {pointer}"))
        })
        .on_long_tap_down({
            let log = log.clone();
            move |pointer, details| {
                log.borrow_mut().push(format!(
                    "long {pointer} ({}, {})",
                    details.global_position.x, details.global_position.y
                ));
            }
        })
}

#[test]
fn multi_tap_tracks_each_pointer() {
    // Each pointer taps on its own: pointer 1 lifts within the touch slop
    // and taps; pointer 2 moves 19 px (> kTouchSlop 18) after winning its
    // arena and is cancelled.
    use PointerPhase::{Down, Move, Up};
    let log = log();
    let mut recognizer = multi_tap(&log);
    let base = Instant::now();
    recognizer.handle_raw(touch(base, 0, 1, Down, 0., 0.));
    recognizer.handle_raw(touch(base, 10, 2, Down, 100., 0.));
    recognizer.handle_raw(touch(base, 20, 1, Move, 5., 0.));
    recognizer.handle_raw(touch(base, 30, 1, Up, 5., 0.));
    recognizer.handle_raw(touch(base, 40, 2, Move, 119., 0.));
    recognizer.handle_raw(touch(base, 50, 2, Up, 119., 0.));
    assert_eq!(
        log.borrow().as_slice(),
        [
            "down 1 (0, 0)",
            "down 2 (100, 0)",
            "up 1 (5, 0)",
            "tap 1",
            "cancel 2",
        ]
    );
}

#[test]
fn multi_tap_reports_a_long_tap_at_the_last_position() {
    use PointerPhase::{Down, Move, Up};
    let log = log();
    let mut recognizer = multi_tap(&log).long_tap_delay(Duration::from_millis(500));
    let base = Instant::now();
    recognizer.handle_raw(touch(base, 0, 1, Down, 0., 0.));
    assert_eq!(
        recognizer.next_deadline(),
        Some(base + Duration::from_millis(500))
    );
    recognizer.handle_raw(touch(base, 100, 1, Move, 3., 4.));
    recognizer.poll(base + Duration::from_millis(499));
    assert_eq!(log.borrow().len(), 1);
    recognizer.poll(base + Duration::from_millis(500));
    recognizer.handle_raw(touch(base, 600, 1, Up, 3., 4.));
    assert_eq!(
        log.borrow().as_slice(),
        ["down 1 (0, 0)", "long 1 (3, 4)", "up 1 (3, 4)", "tap 1"]
    );
}

#[test]
fn multi_tap_uses_the_precise_slop_for_a_mouse() {
    // kPrecisePointerHitSlop is 1: a 1.5 px mouse move cancels.
    use PointerPhase::{Down, Move};
    let log = log();
    let mut recognizer = multi_tap(&log);
    let base = Instant::now();
    let mouse = PointerDeviceKind::Mouse;
    recognizer.handle_raw(event(base, 0, 1, mouse, Down, 0., 0.));
    recognizer.handle_raw(event(base, 10, 1, mouse, Move, 1.5, 0.));
    assert_eq!(log.borrow().as_slice(), ["down 1 (0, 0)", "cancel 1"]);
}

fn serial_tap(log: &Log) -> SerialTapGestureRecognizer {
    SerialTapGestureRecognizer::new()
        .on_serial_tap_down({
            let log = log.clone();
            move |details| log.borrow_mut().push(format!("down {}", details.count))
        })
        .on_serial_tap_up({
            let log = log.clone();
            move |details| log.borrow_mut().push(format!("up {}", details.count))
        })
        .on_serial_tap_cancel({
            let log = log.clone();
            move |details| log.borrow_mut().push(format!("cancel {}", details.count))
        })
}

#[test]
fn serial_tap_counts_a_series() {
    // The second press is 100 ms after the first (>= kDoubleTapMinTime
    // 40 ms), 10 px away (<= kDoubleTapSlop 100) with the same button: tap
    // 2. The series timer (kDoubleTapTimeout 300 ms) restarts at the second
    // release (150 ms), so a press at 500 ms finds it expired: tap 1 again.
    use PointerPhase::{Down, Up};
    let log = log();
    let mut recognizer = serial_tap(&log);
    let base = Instant::now();
    recognizer.handle_raw(touch(base, 0, 1, Down, 0., 0.));
    recognizer.handle_raw(touch(base, 50, 1, Up, 0., 0.));
    assert_eq!(
        recognizer.next_deadline(),
        Some(base + Duration::from_millis(350))
    );
    recognizer.handle_raw(touch(base, 100, 2, Down, 10., 0.));
    assert_eq!(recognizer.next_deadline(), None);
    recognizer.handle_raw(touch(base, 150, 2, Up, 10., 0.));
    assert_eq!(
        recognizer.next_deadline(),
        Some(base + Duration::from_millis(450))
    );
    recognizer.handle_raw(touch(base, 500, 3, Down, 10., 0.));
    recognizer.handle_raw(touch(base, 550, 3, Up, 10., 0.));
    assert_eq!(
        log.borrow().as_slice(),
        ["down 1", "up 1", "down 2", "up 2", "down 1", "up 1"]
    );
}

#[test]
fn serial_tap_restarts_on_a_quick_press_and_cancels_on_movement() {
    // A press 30 ms after the previous one (< kDoubleTapMinTime) starts a
    // new series. Moving 19 px (> kDoubleTapTouchSlop 18) resets it,
    // cancelling tap 1.
    use PointerPhase::{Down, Move, Up};
    let log = log();
    let mut recognizer = serial_tap(&log);
    let base = Instant::now();
    recognizer.handle_raw(touch(base, 0, 1, Down, 0., 0.));
    recognizer.handle_raw(touch(base, 20, 1, Up, 0., 0.));
    recognizer.handle_raw(touch(base, 30, 2, Down, 0., 0.));
    recognizer.handle_raw(touch(base, 40, 2, Move, 19., 0.));
    recognizer.handle_raw(touch(base, 50, 2, Up, 19., 0.));
    assert_eq!(
        log.borrow().as_slice(),
        ["down 1", "up 1", "down 1", "cancel 1"]
    );
    assert!(!recognizer.is_tracking_pointer());
}

struct RecordingDrag(Log);

impl Drag for RecordingDrag {
    fn update(&mut self, details: DragUpdateDetails) {
        self.0.borrow_mut().push(format!(
            "update ({}, {}) at ({}, {})",
            details.delta.x, details.delta.y, details.global_position.x, details.global_position.y
        ));
    }

    fn end(&mut self, _details: DragEndDetails) {
        self.0.borrow_mut().push("end".to_owned());
    }

    fn cancel(&mut self) {
        self.0.borrow_mut().push("cancel".to_owned());
    }
}

fn starting(recognizer: MultiDragGestureRecognizer, log: &Log) -> MultiDragGestureRecognizer {
    let log = log.clone();
    recognizer.on_start(move |position| {
        log.borrow_mut()
            .push(format!("start ({}, {})", position.x, position.y));
        Some(Box::new(RecordingDrag(log.clone())) as Box<dyn Drag>)
    })
}

#[test]
fn delayed_multi_drag_starts_after_the_delay() {
    // Accepted when the arena closes, the drag waits for the delay
    // (kLongPressTimeout, 500 ms); its first update carries the movement
    // pending since the press, at the initial position.
    use PointerPhase::{Down, Move, Up};
    let log = log();
    let mut recognizer = starting(MultiDragGestureRecognizer::delayed(None), &log);
    let base = Instant::now();
    recognizer.handle_raw(touch(base, 0, 1, Down, 0., 0.));
    recognizer.handle_raw(touch(base, 100, 1, Move, 5., 0.));
    assert!(log.borrow().is_empty());
    assert_eq!(
        recognizer.next_deadline(),
        Some(base + Duration::from_millis(500))
    );
    recognizer.handle_raw(touch(base, 600, 1, Move, 8., 0.));
    recognizer.handle_raw(touch(base, 700, 1, Up, 8., 0.));
    assert_eq!(
        log.borrow().as_slice(),
        [
            "start (0, 0)",
            "update (5, 0) at (0, 0)",
            "update (3, 0) at (8, 0)",
            "end",
        ]
    );
}

#[test]
fn delayed_multi_drag_never_starts_after_moving_too_far() {
    // Moving 20 px (> kTouchSlop) before the delay stops the timer; the
    // accepted drag then waits forever.
    use PointerPhase::{Down, Move, Up};
    let log = log();
    let mut recognizer = starting(MultiDragGestureRecognizer::delayed(None), &log);
    let base = Instant::now();
    recognizer.handle_raw(touch(base, 0, 1, Down, 0., 0.));
    recognizer.handle_raw(touch(base, 100, 1, Move, 20., 0.));
    assert_eq!(recognizer.next_deadline(), None);
    recognizer.handle_raw(touch(base, 600, 1, Move, 25., 0.));
    recognizer.handle_raw(touch(base, 700, 1, Up, 25., 0.));
    assert!(log.borrow().is_empty());
}

#[test]
fn immediate_multi_drag_waits_for_the_slop_in_a_shared_arena() {
    // With other members the drag claims the arena once the pending
    // movement passes the hit slop (20 > 18), then starts at the press with
    // that movement.
    use PointerPhase::{Down, Move, Up};
    let log = log();
    let mut recognizer = starting(MultiDragGestureRecognizer::immediate(), &log)
        .with_external_arena(|_, disposition| Some(disposition));
    let base = Instant::now();
    let down = touch(base, 0, 1, Down, 0., 0.);
    recognizer.add_pointer(down);
    recognizer.handle_event(down);
    recognizer.handle_event(touch(base, 10, 1, Move, 10., 0.));
    assert!(log.borrow().is_empty());
    recognizer.handle_event(touch(base, 20, 1, Move, 20., 0.));
    recognizer.handle_event(touch(base, 30, 1, Move, 22., 1.));
    recognizer.handle_event(touch(base, 40, 1, Up, 22., 1.));
    assert_eq!(
        log.borrow().as_slice(),
        [
            "start (0, 0)",
            "update (20, 0) at (0, 0)",
            "update (2, 1) at (22, 1)",
            "end",
        ]
    );
}

#[test]
fn vertical_multi_drag_checks_its_axis_and_buttons() {
    // A 30 px horizontal move does not claim a vertical drag, a 19 px
    // vertical one does; a secondary-button press is not allowed.
    use PointerPhase::{Down, Move};
    let log = log();
    let claims = Rc::new(RefCell::new(Vec::new()));
    let mut recognizer = starting(MultiDragGestureRecognizer::vertical(), &log)
        .with_external_arena({
            let claims = claims.clone();
            move |pointer, disposition| {
                claims.borrow_mut().push((pointer, disposition));
                None
            }
        });
    let base = Instant::now();
    let down = touch(base, 0, 1, Down, 0., 0.);
    recognizer.add_pointer(down);
    recognizer.handle_event(down);
    recognizer.handle_event(touch(base, 10, 1, Move, 30., 0.));
    assert!(claims.borrow().is_empty());
    recognizer.handle_event(touch(base, 20, 1, Move, 30., 19.));
    assert_eq!(
        claims.borrow().as_slice(),
        [(1, GestureDisposition::Accepted)]
    );

    let mut secondary = touch(base, 30, 2, Down, 0., 0.);
    secondary.buttons = 2;
    recognizer.add_pointer(secondary);
    recognizer.accept_gesture(2);
    assert!(log.borrow().is_empty());
}

fn tap_and_drag(recognizer: TapAndDragGestureRecognizer, log: &Log) -> TapAndDragGestureRecognizer {
    recognizer
        .on_tap_down({
            let log = log.clone();
            move |details| {
                log.borrow_mut().push(format!(
                    "down {} ({}, {})",
                    details.consecutive_tap_count,
                    details.global_position.x,
                    details.global_position.y
                ));
            }
        })
        .on_tap_up({
            let log = log.clone();
            move |details| {
                log.borrow_mut()
                    .push(format!("up {}", details.consecutive_tap_count));
            }
        })
        .on_drag_start({
            let log = log.clone();
            move |details| {
                log.borrow_mut().push(format!(
                    "start ({}, {})",
                    details.global_position.x, details.global_position.y
                ));
            }
        })
        .on_drag_update({
            let log = log.clone();
            move |details| {
                log.borrow_mut().push(format!(
                    "update ({}, {}) delta ({}, {}) from ({}, {})",
                    details.global_position.x,
                    details.global_position.y,
                    details.delta.x,
                    details.delta.y,
                    details.offset_from_origin.x,
                    details.offset_from_origin.y
                ));
            }
        })
        .on_drag_end({
            let log = log.clone();
            move |details| {
                log.borrow_mut().push(format!(
                    "end ({}, {}) {:?}",
                    details.global_position.x, details.global_position.y, details.primary_velocity
                ));
            }
        })
        .on_cancel({
            let log = log.clone();
            move || log.borrow_mut().push("cancel".to_owned())
        })
}

#[test]
fn tap_and_pan_counts_consecutive_taps() {
    // The second press comes 50 ms after the first release (within
    // kDoubleTapTimeout) and 2 px away (within kDoubleTapSlop): tap 2.
    use PointerPhase::{Down, Up};
    let log = log();
    let mut recognizer = tap_and_drag(TapAndDragGestureRecognizer::pan(), &log);
    let base = Instant::now();
    recognizer.handle_raw(touch(base, 0, 1, Down, 10., 10.));
    recognizer.handle_raw(touch(base, 50, 1, Up, 10., 10.));
    recognizer.handle_raw(touch(base, 100, 2, Down, 12., 10.));
    recognizer.handle_raw(touch(base, 150, 2, Up, 12., 10.));
    assert_eq!(recognizer.consecutive_tap_count(), 2);
    // 500 ms is past the series timer (150 + 300): a new series.
    recognizer.handle_raw(touch(base, 500, 3, Down, 12., 10.));
    recognizer.handle_raw(touch(base, 550, 3, Up, 12., 10.));
    assert_eq!(
        log.borrow().as_slice(),
        [
            "down 1 (10, 10)",
            "up 1",
            "down 2 (12, 10)",
            "up 2",
            "down 1 (12, 10)",
            "up 1",
        ]
    );
}

#[test]
fn tap_and_pan_starts_a_drag_past_the_pan_slop() {
    // 20 px is past the touch slop but within kPanSlop (36); 40 px starts
    // the drag. With DragStartBehavior.start the start moves by the
    // accepting event's delta (20, 0) to (20, 0), and that delta is
    // reported as the first update.
    use PointerPhase::{Down, Move, Up};
    let log = log();
    let mut recognizer = tap_and_drag(TapAndDragGestureRecognizer::pan(), &log);
    let base = Instant::now();
    recognizer.handle_raw(touch(base, 0, 1, Down, 0., 0.));
    recognizer.handle_raw(touch(base, 10, 1, Move, 20., 0.));
    recognizer.handle_raw(touch(base, 20, 1, Move, 40., 0.));
    recognizer.handle_raw(touch(base, 30, 1, Move, 50., 5.));
    recognizer.handle_raw(touch(base, 40, 1, Up, 50., 5.));
    assert_eq!(
        log.borrow().as_slice(),
        [
            "down 1 (0, 0)",
            "start (20, 0)",
            "update (40, 0) delta (20, 0) from (20, 0)",
            "update (50, 5) delta (10, 5) from (30, 5)",
            "end (50, 5) Some(0.0)",
        ]
    );
}

#[test]
fn tap_and_horizontal_drag_accepts_any_axis_once_it_has_won() {
    // Vertical movement adds nothing along the horizontal axis, but having
    // won the arena the drag starts once the movement on all axes passes
    // kPanSlop: 40 > 36 at the second move.
    use PointerPhase::{Down, Move};
    let log = log();
    let mut recognizer = tap_and_drag(TapAndDragGestureRecognizer::horizontal_drag(), &log);
    let base = Instant::now();
    recognizer.handle_raw(touch(base, 0, 1, Down, 0., 0.));
    recognizer.handle_raw(touch(base, 10, 1, Move, 0., 30.));
    assert_eq!(log.borrow().len(), 1);
    recognizer.handle_raw(touch(base, 20, 1, Move, 0., 40.));
    assert_eq!(
        log.borrow().as_slice(),
        [
            "down 1 (0, 0)",
            "start (0, 10)",
            "update (0, 20) delta (0, 10) from (0, 10)",
        ]
    );
}

#[test]
fn tap_and_pan_reports_tap_down_at_the_press_deadline() {
    // In a shared arena onTapDown waits for kPressTimeout (100 ms). At the
    // deadline of a second consecutive tap the recognizer claims the arena.
    use PointerPhase::{Down, Up};
    let log = log();
    let claims = Rc::new(RefCell::new(Vec::new()));
    let mut recognizer = tap_and_drag(TapAndDragGestureRecognizer::pan(), &log)
        .with_external_arena({
            let claims = claims.clone();
            move |pointer, disposition| {
                claims.borrow_mut().push((pointer, disposition));
                None
            }
        });
    let base = Instant::now();
    let down = touch(base, 0, 1, Down, 0., 0.);
    recognizer.add_pointer(down);
    recognizer.handle_event(down);
    assert_eq!(
        recognizer.next_deadline(),
        Some(base + Duration::from_millis(100))
    );
    recognizer.poll(base + Duration::from_millis(100));
    assert_eq!(log.borrow().as_slice(), ["down 1 (0, 0)"]);
    assert!(claims.borrow().is_empty(), "a first tap claims nothing");
    recognizer.accept_gesture(1);
    recognizer.handle_event(touch(base, 150, 1, Up, 0., 0.));
    let second = touch(base, 200, 2, Down, 0., 0.);
    recognizer.add_pointer(second);
    recognizer.handle_event(second);
    recognizer.poll(base + Duration::from_millis(300));
    assert_eq!(
        log.borrow().as_slice(),
        ["down 1 (0, 0)", "up 1", "down 2 (0, 0)"]
    );
    // `OneSequenceGestureRecognizer.resolve` also resolves the first tap's
    // defunct entry (flutter/flutter#117356), which its finished arena
    // ignores.
    assert_eq!(
        claims.borrow().as_slice(),
        [
            (1, GestureDisposition::Accepted),
            (2, GestureDisposition::Accepted)
        ]
    );
}
