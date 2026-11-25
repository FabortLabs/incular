//! Press-time arena and deadline behavior of the pointer recognizer as the
//! retained widget tree drives it: a sole drag accepts at the press (Flutter
//! closes the arena after the hit test), a delayed drag starts at its deadline
//! (Flutter `DelayedMultiDragGestureRecognizer`), `onLongPress` fires at the
//! deadline (Flutter `_checkLongPressStart`), and a double tap completes across
//! two presses that share a first-tap record.

use incular_core::{Offset, PointerPhase};
use incular_gestures::constants::*;
use incular_gestures::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

fn event(position: Offset, phase: PointerPhase, base: Instant, millis: u64) -> PointerEvent {
    PointerEvent {
        pointer: 1,
        position,
        phase,
        time: base + Duration::from_millis(millis),
    }
}

fn long_press_recorder() -> (PointerGestureRecognizer, Rc<RefCell<Vec<&'static str>>>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_long_press_down: Some(Rc::new({
            let log = log.clone();
            move |_| log.borrow_mut().push("down")
        })),
        on_long_press_cancel: Some(Rc::new({
            let log = log.clone();
            move || log.borrow_mut().push("cancel")
        })),
        on_long_press: Some(Rc::new({
            let log = log.clone();
            move || log.borrow_mut().push("start")
        })),
        ..GestureCallbacks::default()
    });
    (recognizer, log)
}

/// gestures/long_press_test.dart:75, after registration.
#[test]
fn long_press_uses_the_requested_short_deadline() {
    let (recognizer, log) = long_press_recorder();
    let mut recognizer = recognizer.with_long_press_duration(Duration::from_millis(100));
    let now = Instant::now();
    assert!(log.borrow().is_empty());
    recognizer.observe(event(Offset::ZERO, PointerPhase::Down, now, 0));
    assert_eq!(*log.borrow(), ["down"]);
    for pending in [50, 99] {
        assert_eq!(
            recognizer.poll(now + Duration::from_millis(pending)),
            GestureDecision::Pending
        );
        assert_eq!(*log.borrow(), ["down"]);
    }
    let GestureDecision::Accept(action) = recognizer.poll(now + Duration::from_millis(100)) else {
        panic!("the custom deadline should accept");
    };
    recognizer.dispatch(action);
    drop(recognizer);
    assert_eq!(*log.borrow(), ["down", "start"]);
}

/// gestures/long_press_test.dart:91, after registration.
#[test]
fn a_release_before_the_deadline_cancels_and_cannot_restart_the_hold() {
    let (mut recognizer, log) = long_press_recorder();
    let now = Instant::now();
    recognizer.observe(event(Offset::ZERO, PointerPhase::Down, now, 0));
    assert_eq!(
        recognizer.poll(now + Duration::from_millis(300)),
        GestureDecision::Pending
    );
    assert_eq!(*log.borrow(), ["down"]);
    recognizer.observe(event(Offset::ZERO, PointerPhase::Up, now, 300));
    assert_eq!(*log.borrow(), ["down", "cancel"]);
    assert_eq!(
        recognizer.poll(now + Duration::from_millis(1300)),
        GestureDecision::Pending
    );
    drop(recognizer);
    assert_eq!(*log.borrow(), ["down", "cancel"]);
}

/// gestures-001: a sole drag member wins at the press, so the first movement
/// reports in full instead of being consumed by the slop test.
#[test]
fn sole_drag_accepts_at_press_and_reports_first_move() {
    let updates = Rc::new(RefCell::new(Vec::new()));
    let starts = Rc::new(Cell::new(0));
    let callbacks = GestureCallbacks {
        on_vertical_drag_start: Some(Rc::new({
            let starts = starts.clone();
            move |_| starts.set(starts.get() + 1)
        })),
        on_vertical_drag_update: Some(Rc::new({
            let updates = updates.clone();
            move |details| updates.borrow_mut().push(details.delta.y)
        })),
        ..GestureCallbacks::default()
    };
    let mut recognizer = PointerGestureRecognizer::new(callbacks);
    let base = Instant::now();
    let origin = Offset::new(10., 100.);
    recognizer.observe(event(origin, PointerPhase::Down, base, 0));
    assert!(recognizer.can_accept_on_press());
    let press = recognizer.accept_on_press();
    recognizer.dispatch(press.expect("sole drag starts at the press"));
    assert_eq!(starts.get(), 1);

    // A 40 px move is well past the 18 px touch slop; under the old
    // slop-crossing start its 40 px would have been consumed.
    let moved = recognizer.observe(event(
        origin + Offset::new(0., 40.),
        PointerPhase::Move,
        base,
        16,
    ));
    let GestureDecision::Accept(action) = moved else {
        panic!("first move reports an update, got {moved:?}");
    };
    recognizer.dispatch(action);
    assert_eq!(*updates.borrow(), vec![40.]);
}

/// gestures-006: a delayed drag stays pending before its delay, starts at the
/// deadline poll, and is rejected by movement beyond slop before the delay.
#[test]
fn delayed_drag_starts_at_deadline_and_rejects_early_motion() {
    let starts = Rc::new(Cell::new(0));
    let callbacks = GestureCallbacks {
        on_vertical_drag_start: Some(Rc::new({
            let starts = starts.clone();
            move |_| starts.set(starts.get() + 1)
        })),
        on_vertical_drag_update: Some(Rc::new(|_| {})),
        ..GestureCallbacks::default()
    };
    let base = Instant::now();
    let origin = Offset::new(10., 20.);

    let mut recognizer = PointerGestureRecognizer::new(callbacks.clone());
    recognizer.set_delayed_drag(Some(LONG_PRESS_TIMEOUT));
    assert!(
        !recognizer.can_accept_on_press(),
        "delayed drags wait for the deadline"
    );
    recognizer.observe(event(origin, PointerPhase::Down, base, 0));
    assert!(recognizer.has_pending_deadline());
    assert!(matches!(
        recognizer.poll(base + Duration::from_millis(499)),
        GestureDecision::Pending
    ));
    let GestureDecision::Accept(action) = recognizer.poll(base + LONG_PRESS_TIMEOUT) else {
        panic!("deadline starts the delayed drag");
    };
    recognizer.dispatch(action);
    assert_eq!(starts.get(), 1);

    let mut early = PointerGestureRecognizer::new(callbacks);
    early.set_delayed_drag(Some(LONG_PRESS_TIMEOUT));
    early.observe(event(origin, PointerPhase::Down, base, 0));
    let rejected = early.observe(event(
        origin + Offset::new(0., 60.),
        PointerPhase::Move,
        base,
        100,
    ));
    assert!(matches!(rejected, GestureDecision::Reject));
    assert!(!early.has_pending_deadline());
    assert!(matches!(
        early.poll(base + LONG_PRESS_TIMEOUT),
        GestureDecision::Pending
    ));
}

/// gestures-003: the deadline reports `onLongPressStart` and `onLongPress`
/// while the pointer is still held; release reports only the up and end.
#[test]
fn long_press_fires_at_deadline_not_release() {
    let fired = Rc::new(RefCell::new(Vec::new()));
    let callbacks = GestureCallbacks {
        on_long_press_start: Some(Rc::new({
            let fired = fired.clone();
            move |_| fired.borrow_mut().push("start")
        })),
        on_long_press: Some(Rc::new({
            let fired = fired.clone();
            move || fired.borrow_mut().push("long_press")
        })),
        on_long_press_end: Some(Rc::new({
            let fired = fired.clone();
            move |_| fired.borrow_mut().push("end")
        })),
        ..GestureCallbacks::default()
    };
    let mut recognizer = PointerGestureRecognizer::new(callbacks);
    let base = Instant::now();
    let at = Offset::new(10., 20.);
    recognizer.observe(event(at, PointerPhase::Down, base, 0));
    let decision = recognizer.poll(base + LONG_PRESS_TIMEOUT);
    let GestureDecision::Accept(action) = decision else {
        panic!("deadline accepts the long press");
    };
    recognizer.dispatch(action);
    assert_eq!(*fired.borrow(), vec!["start", "long_press"]);

    let released = recognizer.observe(event(
        at,
        PointerPhase::Up,
        base,
        LONG_PRESS_TIMEOUT.as_millis() as u64 + 10,
    ));
    assert!(!matches!(
        released,
        GestureDecision::Accept(GestureAction::LongPress)
    ));
    assert_eq!(*fired.borrow(), vec!["start", "long_press", "end"]);
}

/// gestures-002: a double-tap-only recognizer fires after two presses that
/// share the first-tap record, without any `onTap` callback.
#[test]
fn double_tap_only_completes_across_presses() {
    let doubles = Rc::new(Cell::new(0));
    let callbacks = GestureCallbacks {
        on_double_tap: Some(Rc::new({
            let doubles = doubles.clone();
            move || doubles.set(doubles.get() + 1)
        })),
        ..GestureCallbacks::default()
    };
    let memory = Rc::new(Cell::new(TapMemory::default()));
    let base = Instant::now();
    let at = Offset::new(10., 20.);

    let mut first = PointerGestureRecognizer::new(callbacks.clone());
    first.set_tap_memory(memory.clone());
    first.observe(event(at, PointerPhase::Down, base, 0));
    assert!(matches!(
        first.observe(event(at, PointerPhase::Up, base, 50)),
        GestureDecision::Reject
    ));

    let mut second = PointerGestureRecognizer::new(callbacks);
    second.set_tap_memory(memory);
    second.observe(event(at, PointerPhase::Down, base, 150));
    let decision = second.observe(event(at, PointerPhase::Up, base, 200));
    let GestureDecision::Accept(action) = decision else {
        panic!("second tap completes the double tap, got {decision:?}");
    };
    second.dispatch(action);
    assert_eq!(doubles.get(), 1);
}
