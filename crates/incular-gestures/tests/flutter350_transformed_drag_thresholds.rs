use std::{rc::Rc, time::Duration};

use incular_core::{DragStartBehavior, Offset, PointerPhase, Transform, time::Instant};
use incular_gestures::{
    GestureAction, GestureCallbacks, GestureDecision, GestureRecognizer, PointerEvent,
    PointerGestureRecognizer,
};

#[test]
fn strict_drag_threshold_is_physical_even_when_the_local_axis_is_scaled() {
    for x_scale in [4., 0.25, -4.] {
        let world = Transform::translation(Offset::new(20., 30.))
            .then(Transform::scale_non_uniform(x_scale, 0.5));
        let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
            on_horizontal_drag_start: Some(Rc::new(|_| {})),
            ..GestureCallbacks::default()
        });
        recognizer.set_local_transform(world.inverse().unwrap());
        let base = Instant::now();
        let origin = Offset::new(50., 50.);
        let observe = |recognizer: &mut PointerGestureRecognizer, phase, dx, now| {
            recognizer.observe(PointerEvent {
                pointer: 1,
                position: origin + Offset::new(dx, 0.),
                phase,
                time: base + Duration::from_millis(now),
            })
        };
        let _ = observe(&mut recognizer, PointerPhase::Down, 0., 0);
        assert!(matches!(
            observe(&mut recognizer, PointerPhase::Move, 17., 1),
            GestureDecision::Pending
        ));
        assert!(matches!(
            observe(&mut recognizer, PointerPhase::Move, 18., 2),
            GestureDecision::Pending
        ));
        let GestureDecision::Accept(GestureAction::HorizontalDragStart(details)) =
            observe(&mut recognizer, PointerPhase::Move, 19., 3)
        else {
            panic!("19 physical pixels must cross the 18px threshold at scale {x_scale}");
        };
        assert_eq!(details.global_position, origin + Offset::new(19., 0.));
        assert_eq!(
            details.local_position,
            world
                .inverse_transform_point(details.global_position)
                .unwrap()
        );
        assert!(recognizer.take_start_update().is_none());
    }
}

#[test]
fn down_behavior_projects_the_first_update_and_both_positions_onto_the_local_axis() {
    // World motion (20, 10) is local (5, 20). A horizontal drag uses
    // its local x component, physically 20px, and drops world y.
    let world =
        Transform::translation(Offset::new(20., 30.)).then(Transform::scale_non_uniform(4., 0.5));
    let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_horizontal_drag_start: Some(Rc::new(|_| {})),
        on_horizontal_drag_update: Some(Rc::new(|_| {})),
        drag_start_behavior: DragStartBehavior::Down,
        ..GestureCallbacks::default()
    });
    recognizer.set_local_transform(world.inverse().unwrap());
    let base = Instant::now();
    let down = Offset::new(40., 50.);
    let _ = recognizer.observe(PointerEvent {
        pointer: 1,
        position: down,
        phase: PointerPhase::Down,
        time: base,
    });
    let GestureDecision::Accept(GestureAction::HorizontalDragStart(start)) =
        recognizer.observe(PointerEvent {
            pointer: 1,
            position: down + Offset::new(20., 10.),
            phase: PointerPhase::Move,
            time: base + Duration::from_millis(10),
        })
    else {
        panic!("world x of 20px crosses slop despite local x measuring 5");
    };
    assert_eq!(start.global_position, down);
    assert_eq!(start.local_position, Offset::new(5., 40.));
    let Some(GestureAction::HorizontalDrag(update)) = recognizer.take_start_update() else {
        panic!("Down forwards the pending projected movement");
    };
    assert_eq!(update.delta, Offset::new(5., 0.));
    assert_eq!(update.primary_delta, Some(5.));
    assert_eq!(update.global_position, Offset::new(60., 50.));
    assert_eq!(update.local_position, Offset::new(10., 40.));

    let GestureDecision::Accept(GestureAction::HorizontalDrag(update)) =
        recognizer.observe(PointerEvent {
            pointer: 1,
            position: Offset::new(68., 65.),
            phase: PointerPhase::Move,
            time: base + Duration::from_millis(20),
        })
    else {
        panic!("the accepted next move should be an update");
    };
    assert_eq!(update.delta, Offset::new(2., 0.));
    assert_eq!(update.global_position, Offset::new(68., 65.));
    assert_eq!(update.local_position, Offset::new(12., 70.));
}
