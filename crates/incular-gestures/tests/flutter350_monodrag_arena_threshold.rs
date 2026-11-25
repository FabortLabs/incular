use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::{Offset, PointerPhase, time::Instant};
use incular_gestures::{
    GestureAction, GestureCallbacks, GestureDecision, GestureRecognizer, PointerEvent,
    PointerGestureRecognizer,
};

fn event(phase: PointerPhase, y: f32, time: Instant) -> PointerEvent {
    PointerEvent {
        pointer: 6,
        position: Offset::new(10., y),
        phase,
        time,
    }
}

fn recognizer() -> (PointerGestureRecognizer, Rc<RefCell<Vec<&'static str>>>) {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let mut callbacks = GestureCallbacks::default();
    let record = |phase| {
        let trace = trace.clone();
        move || trace.borrow_mut().push(phase)
    };
    let start = record("onStart");
    callbacks.on_vertical_drag_start = Some(Rc::new(move |_| start()));
    let update = record("onUpdate");
    callbacks.on_vertical_drag_update = Some(Rc::new(move |_| update()));
    let end = record("onEnd");
    callbacks.on_vertical_drag_end = Some(Rc::new(move |_| end()));
    (PointerGestureRecognizer::new(callbacks), trace)
}

#[test]
fn accepting_at_press_preserves_its_original_timestamp() {
    let (mut drag, _) = recognizer();
    let down = Instant::now();
    let _ = drag.observe(event(PointerPhase::Down, 10., down));
    let Some(GestureAction::VerticalDragStart(details)) = drag.accept_on_press() else {
        panic!("the drag wins at the press");
    };
    assert_eq!(details.source_time, Some(down));
}

#[test]
fn an_arena_win_does_not_start_an_explicit_threshold_drag_without_motion() {
    for threshold in [true, false] {
        let (mut drag, trace) = recognizer();
        drag.set_only_accept_drag_on_threshold(threshold);
        let down = Instant::now();
        let _ = drag.observe(event(PointerPhase::Down, 10., down));
        assert!(drag.can_accept_on_press(), "arena eligibility is unchanged");
        if let Some(action) = drag.accept_on_press() {
            drag.dispatch(action);
        }
        if let GestureDecision::Accept(action) = drag.observe(event(
            PointerPhase::Up,
            10.,
            down + Duration::from_millis(10),
        )) {
            drag.dispatch(action);
        }
        let expected: &[_] = if threshold {
            &[]
        } else {
            &["onStart", "onEnd"]
        };
        assert_eq!(trace.borrow().as_slice(), expected);
    }
}

#[test]
fn accepted_threshold_drag_starts_on_the_crossing_without_replaying_a_move() {
    let (mut drag, trace) = recognizer();
    drag.set_only_accept_drag_on_threshold(true);
    let down = Instant::now();
    let _ = drag.observe(event(PointerPhase::Down, 10., down));
    assert!(drag.accept_on_press().is_none());
    for (y, time) in [(27., 1), (28., 2)] {
        assert!(matches!(
            drag.observe(event(
                PointerPhase::Move,
                y,
                down + Duration::from_millis(time)
            )),
            GestureDecision::Pending
        ));
    }
    let time = down + Duration::from_millis(3);
    let GestureDecision::Accept(action @ GestureAction::VerticalDragStart(details)) =
        drag.observe(event(PointerPhase::Move, 29., time))
    else {
        panic!("the accepted drag starts at the first crossing");
    };
    assert_eq!(details.source_time, Some(time));
    assert_eq!(details.global_position, Offset::new(10., 29.));
    assert!(drag.take_start_update().is_none());
    drag.dispatch(action);
    assert!(
        drag.accept_on_press().is_none(),
        "duplicate acceptance cannot replay start"
    );
    let GestureDecision::Accept(release) = drag.observe(event(
        PointerPhase::Up,
        29.,
        time + Duration::from_millis(1),
    )) else {
        panic!("the active drag ends");
    };
    drag.dispatch(release);
    assert_eq!(*trace.borrow(), ["onStart", "onEnd"]);
}

#[test]
fn acceptance_clears_the_pending_timestamp_and_tolerates_a_second_call() {
    let (mut drag, trace) = recognizer();
    let down = Instant::now();
    assert_eq!(drag.debug_last_pending_drag_timestamp(), None);
    let _ = drag.observe(event(PointerPhase::Down, 10., down));
    assert_eq!(drag.debug_last_pending_drag_timestamp(), Some(down));
    let start = drag.accept_on_press().unwrap();
    drag.dispatch(start);
    assert_eq!(drag.debug_last_pending_drag_timestamp(), None);
    drag.reject();
    assert_eq!(drag.debug_last_pending_drag_timestamp(), None);
    assert!(drag.accept_on_press().is_none());
    assert_eq!(drag.debug_last_pending_drag_timestamp(), None);
    assert_eq!(*trace.borrow(), ["onStart"]);

    // Reusing the same recognizer must not carry the old arena win into
    // the next press if that stream has not been accepted yet.
    let _ = drag.observe(event(
        PointerPhase::Down,
        10.,
        down + Duration::from_millis(2),
    ));
    assert!(matches!(
        drag.observe(event(
            PointerPhase::Move,
            11.,
            down + Duration::from_millis(3)
        )),
        GestureDecision::Pending
    ));
}
