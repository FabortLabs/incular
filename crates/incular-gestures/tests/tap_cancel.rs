//! Flutter `BaseTapGestureRecognizer`: `onTapCancel` follows only a reported
//! `onTapDown` (`_sentTapDown`), whether the tap loses to movement, another
//! recognizer, or a cancelled pointer.

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_core::{Offset, PointerPhase};
use incular_gestures::*;

struct Log(Rc<RefCell<Vec<&'static str>>>);

impl Log {
    fn take(&self) -> Vec<&'static str> {
        std::mem::take(&mut self.0.borrow_mut())
    }
}

fn recorder(long_press: bool) -> (PointerGestureRecognizer, Log, Instant) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let push = |name: &'static str| {
        let log = log.clone();
        Some(Rc::new(move || log.borrow_mut().push(name)) as Rc<dyn Fn()>)
    };
    let mut callbacks = GestureCallbacks {
        on_tap: push("tap"),
        on_tap_cancel: push("cancel"),
        on_tap_down: Some({
            let log = log.clone();
            Rc::new(move |_| log.borrow_mut().push("down"))
        }),
        on_tap_up: Some({
            let log = log.clone();
            Rc::new(move |_| log.borrow_mut().push("up"))
        }),
        ..GestureCallbacks::default()
    };
    if long_press {
        callbacks.on_long_press = push("long press");
    }
    (
        PointerGestureRecognizer::new(callbacks),
        Log(log),
        Instant::now(),
    )
}

fn event(base: Instant, phase: PointerPhase, x: f32, millis: u64) -> PointerEvent {
    PointerEvent {
        pointer: 1,
        position: Offset::new(x, 0.),
        phase,
        time: base + Duration::from_millis(millis),
    }
}

#[test]
fn a_quick_tap_reports_down_up_and_tap_without_cancel() {
    let (mut tap, log, base) = recorder(false);
    tap.handle(event(base, PointerPhase::Down, 0., 0));
    tap.handle(event(base, PointerPhase::Up, 0., 50));
    assert_eq!(log.take(), ["down", "up", "tap"]);
}

#[test]
fn moving_past_the_slop_before_the_press_deadline_reports_nothing() {
    let (mut tap, log, base) = recorder(false);
    tap.handle(event(base, PointerPhase::Down, 0., 0));
    // kTouchSlop 18 for the unknown device kind.
    tap.handle(event(base, PointerPhase::Move, 19., 50));
    tap.handle(event(base, PointerPhase::Up, 19., 200));
    assert!(log.take().is_empty());
}

#[test]
fn moving_past_the_slop_after_tap_down_cancels_at_that_move() {
    let (mut tap, log, base) = recorder(false);
    tap.handle(event(base, PointerPhase::Down, 0., 0));
    // The kPressTimeout (100 ms) deadline has passed: tap down was sent.
    let _ = tap.poll(base + Duration::from_millis(120));
    assert_eq!(log.take(), ["down"]);
    tap.handle(event(base, PointerPhase::Move, 10., 130));
    assert!(log.take().is_empty(), "still within the slop");
    tap.handle(event(base, PointerPhase::Move, 19., 140));
    assert_eq!(log.take(), ["cancel"]);
    tap.handle(event(base, PointerPhase::Up, 19., 200));
    assert!(log.take().is_empty());
}

#[test]
fn a_late_move_reports_the_elapsed_deadline_before_cancelling() {
    let (mut tap, log, base) = recorder(false);
    tap.handle(event(base, PointerPhase::Down, 0., 0));
    // No frame polled the deadline; the move at 150 ms comes after it.
    tap.handle(event(base, PointerPhase::Move, 30., 150));
    assert_eq!(log.take(), ["down", "cancel"]);
}

#[test]
fn a_cancelled_pointer_cancels_only_a_reported_tap_down() {
    let (mut tap, log, base) = recorder(false);
    tap.handle(event(base, PointerPhase::Down, 0., 0));
    tap.handle(event(base, PointerPhase::Cancel, 0., 50));
    assert!(log.take().is_empty());

    tap.handle(event(base, PointerPhase::Down, 0., 1_000));
    let _ = tap.poll(base + Duration::from_millis(1_110));
    tap.handle(event(base, PointerPhase::Cancel, 0., 1_150));
    assert_eq!(log.take(), ["down", "cancel"]);
}

#[test]
fn losing_the_arena_cancels_only_a_reported_tap_down() {
    let (mut tap, log, base) = recorder(false);
    tap.handle(event(base, PointerPhase::Down, 0., 0));
    tap.reject();
    assert!(log.take().is_empty());

    tap.handle(event(base, PointerPhase::Down, 0., 1_000));
    let _ = tap.poll(base + Duration::from_millis(1_100));
    tap.reject();
    assert_eq!(log.take(), ["down", "cancel"]);
}

#[test]
fn a_long_hold_without_long_press_callbacks_is_still_a_tap() {
    // Tap recognizers have no deadline beyond kPressTimeout.
    let (mut tap, log, base) = recorder(false);
    tap.handle(event(base, PointerPhase::Down, 0., 0));
    tap.handle(event(base, PointerPhase::Up, 0., 900));
    assert_eq!(log.take(), ["down", "up", "tap"]);
}

#[test]
fn a_winning_long_press_cancels_the_tap() {
    let (mut tap, log, base) = recorder(true);
    tap.handle(event(base, PointerPhase::Down, 0., 0));
    let _ = tap.poll(base + Duration::from_millis(100));
    // kLongPressTimeout (500 ms): the long press wins the arena.
    assert!(matches!(
        tap.poll(base + Duration::from_millis(500)),
        GestureDecision::Accept(GestureAction::LongPressStart(_))
    ));
    assert_eq!(log.take(), ["down", "cancel"]);
    tap.handle(event(base, PointerPhase::Up, 0., 700));
    assert_eq!(log.take(), ["long press"]);
}
