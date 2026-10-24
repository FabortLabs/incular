//! Flutter `ScaleGestureRecognizer` acceptance: `computeScaleSlop` for span
//! changes and `computePanSlop` for focal-point movement, by pointer kind.

use std::{cell::RefCell, rc::Rc, time::Instant};

use incular_core::{DragStartBehavior, Offset, PointerPhase};
use incular_gestures::constants::{PRECISE_POINTER_SCALE_SLOP, SCALE_SLOP};
use incular_gestures::*;

#[derive(Debug, PartialEq)]
enum Seen {
    Start(usize),
    Update(f32, usize),
    End(usize),
}

fn recorder() -> (ScaleGestureDetector, Rc<RefCell<Vec<Seen>>>) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let detector = ScaleGestureDetector::with_callbacks(
        Some({
            let seen = seen.clone();
            Rc::new(move |details: ScaleUpdateDetails| {
                seen.borrow_mut()
                    .push(Seen::Update(details.scale, details.pointer_count));
            })
        }),
        Some({
            let seen = seen.clone();
            Rc::new(move |details: ScaleStartDetails| {
                seen.borrow_mut().push(Seen::Start(details.pointer_count));
            })
        }),
        Some({
            let seen = seen.clone();
            Rc::new(move |details: ScaleEndDetails| {
                seen.borrow_mut().push(Seen::End(details.pointer_count));
            })
        }),
    );
    (detector, seen)
}

fn raw(pointer: u64, kind: PointerDeviceKind, x: f32, phase: PointerPhase) -> RawPointerEvent {
    RawPointerEvent::new(
        pointer,
        0,
        kind,
        0,
        Offset::new(x, 0.),
        phase,
        Instant::now(),
    )
}

#[test]
fn scale_slop_follows_flutter_compute_scale_slop() {
    assert_eq!(compute_scale_slop(PointerDeviceKind::Mouse), 1.0);
    assert_eq!(PRECISE_POINTER_SCALE_SLOP, 1.0);
    for kind in [
        PointerDeviceKind::Touch,
        PointerDeviceKind::Stylus,
        PointerDeviceKind::InvertedStylus,
        PointerDeviceKind::Trackpad,
        PointerDeviceKind::Unknown,
    ] {
        assert_eq!(compute_scale_slop(kind), SCALE_SLOP);
        assert_eq!(compute_scale_slop(kind), 18.0);
    }
}

#[test]
fn touch_span_must_change_by_more_than_scale_slop() {
    let (mut detector, seen) = recorder();
    let touch = PointerDeviceKind::Touch;
    // Contacts at 0 and 100: focal 50, span 50.
    detector.handle_raw(raw(1, touch, 0., PointerPhase::Down));
    detector.handle_raw(raw(2, touch, 100., PointerPhase::Down));
    // Contact 2 to 136: focal 68 (moved 18 < kPanSlop 36), span 68, exactly
    // kScaleSlop (18) past 50, which is not more than the slop.
    detector.handle_raw(raw(2, touch, 136., PointerPhase::Move));
    assert!(seen.borrow().is_empty());
    // Contact 2 to 138: span 69, 19 past 50: accepted. The bare recognizer
    // measures from the press (`DragStartBehavior.down`): 69 / 50.
    detector.handle_raw(raw(2, touch, 138., PointerPhase::Move));
    assert_eq!(
        *seen.borrow(),
        vec![Seen::Start(2), Seen::Update(69. / 50., 2)]
    );
}

#[test]
fn mouse_span_uses_precise_pointer_scale_slop() {
    let (mut detector, seen) = recorder();
    let mouse = PointerDeviceKind::Mouse;
    detector.handle_raw(raw(1, mouse, 0., PointerPhase::Down));
    detector.handle_raw(raw(2, mouse, 100., PointerPhase::Down));
    // Contact 2 to 102: focal 51 (moved 1 < kPrecisePointerPanSlop 2), span
    // 51, exactly kPrecisePointerScaleSlop (1) past 50: not accepted.
    detector.handle_raw(raw(2, mouse, 102., PointerPhase::Move));
    assert!(seen.borrow().is_empty());
    // Contact 2 to 103: focal 51.5 (moved 1.5 < 2), span 51.5, 1.5 > 1.
    detector.handle_raw(raw(2, mouse, 103., PointerPhase::Move));
    assert_eq!(
        *seen.borrow(),
        vec![Seen::Start(2), Seen::Update(51.5 / 50., 2)]
    );
}

#[test]
fn one_contact_scales_once_its_focal_point_passes_pan_slop() {
    let (mut detector, seen) = recorder();
    let touch = PointerDeviceKind::Touch;
    detector.handle_raw(raw(1, touch, 0., PointerPhase::Down));
    // Moving exactly kPanSlop (36) is not past it.
    detector.handle_raw(raw(1, touch, 36., PointerPhase::Move));
    assert!(seen.borrow().is_empty());
    // A lone contact has span 0, so Flutter reports scale 1.
    detector.handle_raw(raw(1, touch, 37., PointerPhase::Move));
    assert_eq!(*seen.borrow(), vec![Seen::Start(1), Seen::Update(1., 1)]);
}

#[test]
fn start_behavior_measures_from_the_accepting_span_and_reconfigures_on_lift() {
    let (detector, seen) = recorder();
    let mut detector = detector.with_drag_start_behavior(DragStartBehavior::Start);
    let touch = PointerDeviceKind::Touch;
    detector.handle_raw(raw(1, touch, 0., PointerPhase::Down));
    detector.handle_raw(raw(2, touch, 100., PointerPhase::Down));
    // Span 69 accepts; `start` resets the initial span to 69.
    detector.handle_raw(raw(2, touch, 138., PointerPhase::Move));
    // Contact 2 to 276: focal 138, span 138 = 2 x 69.
    detector.handle_raw(raw(2, touch, 276., PointerPhase::Move));
    // Lifting contact 1 ends the scale with the one remaining contact, and
    // the next move restarts it (`_ScaleState.accepted` -> `started`).
    detector.handle_raw(raw(1, touch, 0., PointerPhase::Up));
    detector.handle_raw(raw(2, touch, 280., PointerPhase::Move));
    assert_eq!(
        *seen.borrow(),
        vec![
            Seen::Start(2),
            Seen::Update(1., 2),
            Seen::Update(2., 2),
            Seen::End(1),
            Seen::Start(1),
            Seen::Update(1., 1),
        ]
    );
}

#[test]
fn an_arena_owner_must_accept_before_updates_flow() {
    let (mut detector, seen) = recorder();
    let touch = PointerDeviceKind::Touch;
    let _ = detector.observe_raw(raw(1, touch, 0., PointerPhase::Down));
    let _ = detector.observe_raw(raw(2, touch, 100., PointerPhase::Down));
    assert_eq!(
        detector.observe_raw(raw(2, touch, 138., PointerPhase::Move)),
        None
    );
    assert!(detector.wants_acceptance());
    assert!(!detector.is_started());
    let details = detector.accept().expect("accepting starts the scale");
    assert_eq!(details.pointer_count, 2);
    assert!(detector.is_started());
    assert_eq!(*seen.borrow(), vec![Seen::Start(2)]);
}

#[test]
fn update_details_report_axis_scales_rotation_and_focal_delta() {
    let updates = Rc::new(RefCell::new(Vec::new()));
    let ends = Rc::new(RefCell::new(Vec::new()));
    let mut detector = ScaleGestureDetector::with_callbacks(
        Some({
            let updates = updates.clone();
            Rc::new(move |details: ScaleUpdateDetails| updates.borrow_mut().push(details))
        }),
        None,
        Some({
            let ends = ends.clone();
            Rc::new(move |details: ScaleEndDetails| ends.borrow_mut().push(details))
        }),
    );
    let base = Instant::now();
    let at = |pointer, x: f32, y: f32, phase, millis| {
        RawPointerEvent::new(
            pointer,
            0,
            PointerDeviceKind::Touch,
            0,
            Offset::new(x, y),
            phase,
            base + std::time::Duration::from_millis(millis),
        )
    };
    detector.handle_raw(at(1, 0., 0., PointerPhase::Down, 0));
    detector.handle_raw(at(2, 100., 0., PointerPhase::Down, 0));
    // Contact 2 swings to (0, 100): the focal point moves from (50, 0) to
    // (0, 50), past kPanSlop, so the scale is accepted on this move.
    detector.handle_raw(at(2, 0., 100., PointerPhase::Move, 10));
    let first = updates.borrow()[0];
    // Spans: 50 -> 50 overall, 50 -> 0 horizontally, 0 -> 50 vertically
    // (a zero initial span reports 1).
    assert_eq!(first.scale, 1.);
    assert_eq!(first.horizontal_scale, 0.);
    assert_eq!(first.vertical_scale, 1.);
    // `_computeRotationFactor`: atan2(0 - 100, 0 - 0) - atan2(0 - 0, 0 - 100).
    let rotation = (-100f32).atan2(0.) - 0f32.atan2(-100.);
    assert_eq!(first.rotation, rotation);
    assert_eq!(first.focal_point_delta, Offset::new(-50., 50.));
    assert_eq!(
        first.source_time,
        Some(base + std::time::Duration::from_millis(10))
    );

    detector.handle_raw(at(2, 0., 130., PointerPhase::Move, 20));
    detector.handle_raw(at(2, 0., 170., PointerPhase::Move, 30));
    detector.handle_raw(at(2, 0., 170., PointerPhase::Up, 30));
    // The lifted contact's samples 100, 130, 170 at -20, -10, 0 ms fit
    // y = 170 + 4.5 t + 0.05 t^2: 4.5 px/ms, a fling.
    let end = ends.borrow()[0];
    assert_eq!(end.pointer_count, 1);
    assert_eq!(end.velocity.pixels_per_second.x, 0.);
    assert!((end.velocity.pixels_per_second.y - 4500.).abs() < 1.);
}
