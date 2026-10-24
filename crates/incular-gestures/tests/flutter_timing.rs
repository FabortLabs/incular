//! Flutter 3.41.7 `gestures/constants.dart` values and the timing rules built
//! on them: `kPressTimeout`, `kLongPressTimeout`, the double-tap window,
//! slop and minimum time, and the fling thresholds a scrollable overrides.

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

fn at(base: Instant, millis: u64) -> Instant {
    base + Duration::from_millis(millis)
}

#[test]
fn constants_match_flutter() {
    assert_eq!(PRESS_TIMEOUT, Duration::from_millis(100));
    assert_eq!(HOVER_TAP_TIMEOUT, Duration::from_millis(150));
    assert_eq!(HOVER_TAP_SLOP, 20.0);
    assert_eq!(LONG_PRESS_TIMEOUT, Duration::from_millis(500));
    assert_eq!(DOUBLE_TAP_TIMEOUT, Duration::from_millis(300));
    assert_eq!(DOUBLE_TAP_MIN_TIME, Duration::from_millis(40));
    assert_eq!(DOUBLE_TAP_TOUCH_SLOP, 18.0);
    assert_eq!(DOUBLE_TAP_SLOP, 100.0);
    assert_eq!(ZOOM_CONTROLS_TIMEOUT, Duration::from_millis(3000));
    assert_eq!(TOUCH_SLOP, 18.0);
    assert_eq!(PAGING_TOUCH_SLOP, 36.0);
    assert_eq!(PAN_SLOP, 36.0);
    assert_eq!(SCALE_SLOP, 18.0);
    assert_eq!(WINDOW_TOUCH_SLOP, 16.0);
    assert_eq!(MIN_FLING_VELOCITY, 50.0);
    assert_eq!(MAX_FLING_VELOCITY, 8000.0);
    assert_eq!(JUMP_TAP_TIMEOUT, Duration::from_millis(500));
    assert_eq!(PRECISE_POINTER_HIT_SLOP, 1.0);
    assert_eq!(PRECISE_POINTER_PAN_SLOP, 2.0);
    assert_eq!(PRECISE_POINTER_SCALE_SLOP, 1.0);
    assert_eq!(PointerGestureRecognizer::PAN_SLOP, PAN_SLOP);
}

fn tap_logger() -> (PointerGestureRecognizer, Rc<RefCell<Vec<&'static str>>>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_tap_down: Some({
            let log = log.clone();
            Rc::new(move |_| log.borrow_mut().push("down"))
        }),
        on_tap_up: Some({
            let log = log.clone();
            Rc::new(move |_| log.borrow_mut().push("up"))
        }),
        on_tap: Some({
            let log = log.clone();
            Rc::new(move || log.borrow_mut().push("tap"))
        }),
        ..GestureCallbacks::default()
    });
    (recognizer, log)
}

#[test]
fn tap_down_waits_for_the_press_timeout() {
    // TapGestureRecognizer reports onTapDown after kPressTimeout (100 ms).
    let (mut recognizer, log) = tap_logger();
    let base = Instant::now();
    recognizer.observe(event(Offset::ZERO, PointerPhase::Down, base, 0));
    assert!(log.borrow().is_empty(), "no tap down at the press itself");
    assert!(recognizer.has_pending_deadline());
    recognizer.poll(at(base, 99));
    assert!(log.borrow().is_empty());
    recognizer.poll(at(base, 100));
    assert_eq!(*log.borrow(), ["down"]);
    // It is reported once.
    recognizer.poll(at(base, 300));
    assert_eq!(*log.borrow(), ["down"]);
    assert!(!recognizer.has_pending_deadline());
    let decision = recognizer.observe(event(Offset::ZERO, PointerPhase::Up, base, 320));
    assert!(matches!(
        decision,
        GestureDecision::Accept(GestureAction::Tap)
    ));
    assert_eq!(*log.borrow(), ["down", "up"]);
}

#[test]
fn quick_tap_reports_tap_down_when_it_is_accepted() {
    // Released before the deadline, the tap is accepted and onTapDown comes
    // right before onTapUp.
    let (mut recognizer, log) = tap_logger();
    let base = Instant::now();
    recognizer.observe(event(Offset::ZERO, PointerPhase::Down, base, 0));
    let decision = recognizer.observe(event(Offset::ZERO, PointerPhase::Up, base, 30));
    assert!(matches!(
        decision,
        GestureDecision::Accept(GestureAction::Tap)
    ));
    assert_eq!(*log.borrow(), ["down", "up"]);
}

#[test]
fn moving_past_the_slop_before_the_deadline_never_reports_tap_down() {
    let (mut recognizer, log) = tap_logger();
    let base = Instant::now();
    recognizer.observe(event(Offset::ZERO, PointerPhase::Down, base, 0));
    recognizer.observe(event(Offset::new(30., 0.), PointerPhase::Move, base, 40));
    recognizer.poll(at(base, 150));
    assert!(log.borrow().is_empty(), "{:?}", log.borrow());
}

#[test]
fn long_press_starts_at_its_timeout_without_any_movement() {
    let started = Rc::new(Cell::new(0));
    let ended = Rc::new(Cell::new(0));
    let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_long_press_start: Some({
            let started = started.clone();
            Rc::new(move |_| started.set(started.get() + 1))
        }),
        on_long_press_end: Some({
            let ended = ended.clone();
            Rc::new(move |_| ended.set(ended.get() + 1))
        }),
        ..GestureCallbacks::default()
    });
    let base = Instant::now();
    recognizer.observe(event(Offset::ZERO, PointerPhase::Down, base, 0));
    assert!(recognizer.has_pending_deadline());
    assert_eq!(recognizer.poll(at(base, 499)), GestureDecision::Pending);
    let GestureDecision::Accept(action) = recognizer.poll(at(base, 500)) else {
        panic!("kLongPressTimeout accepts the long press");
    };
    assert!(matches!(action, GestureAction::LongPressStart(_)));
    recognizer.dispatch(action);
    assert_eq!(started.get(), 1);
    assert!(!recognizer.has_pending_deadline());
    // Releasing later ends the press without starting it a second time.
    let end = recognizer.observe(event(Offset::ZERO, PointerPhase::Up, base, 700));
    assert!(matches!(
        end,
        GestureDecision::Accept(GestureAction::LongPress)
    ));
    assert_eq!(started.get(), 1);
    assert_eq!(ended.get(), 1);
}

/// Runs one press and release, dispatching an accepted action, and names it.
fn tap_at(
    recognizer: &mut PointerGestureRecognizer,
    base: Instant,
    position: Offset,
    down: u64,
    up: u64,
) -> &'static str {
    recognizer.observe(event(position, PointerPhase::Down, base, down));
    match recognizer.observe(event(position, PointerPhase::Up, base, up)) {
        GestureDecision::Accept(GestureAction::DoubleTap) => "double",
        GestureDecision::Accept(GestureAction::Tap) => "tap",
        GestureDecision::Accept(_) => "other",
        GestureDecision::Reject => "rejected",
        GestureDecision::Pending | GestureDecision::Cancelled => "pending",
    }
}

fn double_tap_recognizer() -> PointerGestureRecognizer {
    PointerGestureRecognizer::new(GestureCallbacks {
        on_tap: Some(Rc::new(|| {})),
        on_double_tap: Some(Rc::new(|| {})),
        ..GestureCallbacks::default()
    })
}

#[test]
fn double_tap_needs_the_second_press_within_the_timeout_of_the_first_release() {
    let base = Instant::now();
    // Released at 50 ms; pressed again 200 ms later is inside kDoubleTapTimeout.
    let mut recognizer = double_tap_recognizer();
    assert_eq!(tap_at(&mut recognizer, base, Offset::ZERO, 0, 50), "tap");
    assert_eq!(
        tap_at(&mut recognizer, base, Offset::ZERO, 250, 300),
        "double"
    );
    // Pressed 301 ms after the release is outside it.
    let mut recognizer = double_tap_recognizer();
    assert_eq!(tap_at(&mut recognizer, base, Offset::ZERO, 0, 50), "tap");
    assert_eq!(tap_at(&mut recognizer, base, Offset::ZERO, 351, 380), "tap");
}

#[test]
fn double_tap_is_decided_when_the_second_tap_goes_down() {
    // The window runs from the first release to the second *press*; a second
    // tap held for a while after pressing in time is still a double tap.
    let base = Instant::now();
    let mut recognizer = double_tap_recognizer();
    assert_eq!(tap_at(&mut recognizer, base, Offset::ZERO, 0, 50), "tap");
    assert_eq!(
        tap_at(&mut recognizer, base, Offset::ZERO, 200, 450),
        "double"
    );
}

#[test]
fn double_tap_requires_the_two_taps_within_the_double_tap_slop() {
    let base = Instant::now();
    // kDoubleTapSlop is 100 logical pixels between the two presses.
    let mut recognizer = double_tap_recognizer();
    assert_eq!(tap_at(&mut recognizer, base, Offset::ZERO, 0, 50), "tap");
    assert_eq!(
        tap_at(&mut recognizer, base, Offset::new(99., 0.), 200, 250),
        "double"
    );
    let mut recognizer = double_tap_recognizer();
    assert_eq!(tap_at(&mut recognizer, base, Offset::ZERO, 0, 50), "tap");
    assert_eq!(
        tap_at(&mut recognizer, base, Offset::new(101., 0.), 200, 250),
        "tap"
    );
}

#[test]
fn a_second_press_within_the_minimum_time_restarts_the_sequence() {
    // kDoubleTapMinTime: pressing again within 40 ms of the first press is not
    // a double tap; that press becomes the new first tap.
    let base = Instant::now();
    let mut recognizer = double_tap_recognizer();
    assert_eq!(tap_at(&mut recognizer, base, Offset::ZERO, 0, 10), "tap");
    assert_eq!(tap_at(&mut recognizer, base, Offset::ZERO, 30, 35), "tap");
    // The restarted sequence can still complete.
    assert_eq!(
        tap_at(&mut recognizer, base, Offset::ZERO, 120, 130),
        "double"
    );
}

#[test]
fn the_second_tap_may_move_up_to_the_double_tap_touch_slop() {
    // Moving up to kDoubleTapTouchSlop (18) keeps the double tap; beyond it the
    // tap is rejected.
    let base = Instant::now();
    let mut recognizer = double_tap_recognizer();
    assert_eq!(tap_at(&mut recognizer, base, Offset::ZERO, 0, 50), "tap");
    recognizer.observe(event(Offset::ZERO, PointerPhase::Down, base, 200));
    recognizer.observe(event(Offset::new(17., 0.), PointerPhase::Move, base, 210));
    assert!(matches!(
        recognizer.observe(event(Offset::new(17., 0.), PointerPhase::Up, base, 230)),
        GestureDecision::Accept(GestureAction::DoubleTap)
    ));
    let mut recognizer = double_tap_recognizer();
    assert_eq!(tap_at(&mut recognizer, base, Offset::ZERO, 0, 50), "tap");
    recognizer.observe(event(Offset::ZERO, PointerPhase::Down, base, 200));
    recognizer.observe(event(Offset::new(19., 0.), PointerPhase::Move, base, 210));
    assert_eq!(
        recognizer.observe(event(Offset::new(19., 0.), PointerPhase::Up, base, 230)),
        GestureDecision::Reject
    );
}

/// A horizontal drag moving `step` px every 20 ms for ten moves, released
/// 5 ms after the last one. Returns the reported primary velocity.
fn release_velocity(step: f32, limits: Option<FlingLimits>) -> Option<f32> {
    let ended = Rc::new(Cell::new(None));
    let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_horizontal_drag_update: Some(Rc::new(|_| {})),
        on_horizontal_drag_end: Some({
            let ended = ended.clone();
            Rc::new(move |details: DragEndDetails| ended.set(Some(details.primary_velocity)))
        }),
        ..GestureCallbacks::default()
    });
    if let Some(limits) = limits {
        recognizer.set_fling_limits(limits);
    }
    let base = Instant::now();
    recognizer.observe(event(Offset::ZERO, PointerPhase::Down, base, 0));
    for index in 1..=10u64 {
        recognizer.observe(event(
            Offset::new(step * index as f32, 0.),
            PointerPhase::Move,
            base,
            index * 20,
        ));
    }
    let decision = recognizer.observe(event(
        Offset::new(step * 10., 0.),
        PointerPhase::Up,
        base,
        205,
    ));
    if let GestureDecision::Accept(action) = decision {
        recognizer.dispatch(action);
    }
    ended.get().flatten()
}

#[test]
fn fling_thresholds_default_to_flutter_and_follow_overrides() {
    // 10 px every 20 ms is 500 px/s over about 100 px.
    let default = release_velocity(10., None).expect("drag end reports a velocity");
    assert!((default - 500.).abs() < 25., "{default}");

    // A scrollable's physics raise the minimum velocity: 500 px/s no longer
    // counts as a fling and the release reports zero.
    let slow = FlingLimits {
        min_velocity: Some(600.),
        ..FlingLimits::default()
    };
    assert_eq!(release_velocity(10., Some(slow)), Some(0.));

    // A larger minimum distance than the displacement across the velocity
    // estimate also makes it a plain release.
    let far = FlingLimits {
        min_distance: Some(500.),
        ..FlingLimits::default()
    };
    assert_eq!(release_velocity(10., Some(far)), Some(0.));

    // The maximum velocity clamps the report.
    let capped = FlingLimits {
        max_velocity: Some(300.),
        ..FlingLimits::default()
    };
    assert_eq!(release_velocity(10., Some(capped)), Some(300.));
}
