//! Flutter `DragStartBehavior` for the drag recognizers
//! (`gestures/monodrag.dart` `_checkDrag`).

use std::{cell::RefCell, rc::Rc, time::Duration, time::Instant};

use incular_core::{DragStartBehavior, Offset, PointerPhase};
use incular_gestures::*;

#[derive(Debug, PartialEq)]
enum Seen {
    Start(Offset),
    Update {
        delta: Offset,
        primary: Option<f32>,
        position: Offset,
    },
}

fn horizontal(behavior: DragStartBehavior) -> (GestureDetector, Rc<RefCell<Vec<Seen>>>) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let detector = GestureDetector::new(GestureCallbacks {
        on_horizontal_drag_start: Some({
            let seen = seen.clone();
            Rc::new(move |details: DragStartDetails| {
                seen.borrow_mut().push(Seen::Start(details.global_position));
            })
        }),
        on_horizontal_drag_update: Some({
            let seen = seen.clone();
            Rc::new(move |details: DragUpdateDetails| {
                seen.borrow_mut().push(Seen::Update {
                    delta: details.delta,
                    primary: details.primary_delta,
                    position: details.global_position,
                });
            })
        }),
        drag_start_behavior: behavior,
        ..GestureCallbacks::default()
    });
    (detector, seen)
}

fn drive(detector: &mut GestureDetector) {
    let base = Instant::now();
    // Unknown device kinds use touch slops: a horizontal drag accepts once
    // the horizontal travel exceeds kTouchSlop (18).
    for (phase, x, y, millis) in [
        (PointerPhase::Down, 0., 0., 0),
        (PointerPhase::Move, 10., 0., 10),
        (PointerPhase::Move, 20., 5., 20),
        (PointerPhase::Move, 25., 5., 30),
    ] {
        detector.handle(PointerEvent {
            pointer: 1,
            position: Offset::new(x, y),
            phase,
            time: base + Duration::from_millis(millis),
        });
    }
}

#[test]
fn gesture_detector_defaults_to_start_behavior() {
    assert_eq!(
        GestureCallbacks::default().drag_start_behavior,
        DragStartBehavior::Start
    );
}

#[test]
fn start_behavior_begins_at_the_accepting_position_without_an_update() {
    let (mut detector, seen) = horizontal(DragStartBehavior::Start);
    drive(&mut detector);
    assert_eq!(
        *seen.borrow(),
        vec![
            Seen::Start(Offset::new(20., 5.)),
            Seen::Update {
                delta: Offset::new(5., 0.),
                primary: Some(5.),
                position: Offset::new(25., 5.),
            },
        ]
    );
}

#[test]
fn down_behavior_begins_at_the_press_and_reports_the_slop_as_an_update() {
    let (mut detector, seen) = horizontal(DragStartBehavior::Down);
    drive(&mut detector);
    // `_checkDrag` with `DragStartBehavior.down`: onStart at the press, then
    // an update for the pending (20, 5) restricted to the horizontal axis,
    // positioned at the press plus that delta.
    assert_eq!(
        *seen.borrow(),
        vec![
            Seen::Start(Offset::ZERO),
            Seen::Update {
                delta: Offset::new(20., 0.),
                primary: Some(20.),
                position: Offset::new(20., 0.),
            },
            Seen::Update {
                delta: Offset::new(5., 0.),
                primary: Some(5.),
                position: Offset::new(25., 5.),
            },
        ]
    );
}
