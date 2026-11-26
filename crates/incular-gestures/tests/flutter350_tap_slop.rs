use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::{
    DeviceGestureSettings, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    time::Instant,
};
use incular_gestures::{
    GestureCallbacks, GestureDecision, GestureRecognizer, PointerGestureRecognizer,
    RawPointerEvent, TapSlopTolerance,
};

fn route(
    tap: &mut PointerGestureRecognizer,
    kind: PointerDeviceKind,
    phase: PointerPhase,
    position: Offset,
    time: Instant,
) {
    let event = RawPointerEvent::new(
        1,
        1,
        kind,
        if phase == PointerPhase::Up {
            0
        } else {
            PRIMARY_POINTER_BUTTON
        },
        position,
        phase,
        time,
    );
    if let GestureDecision::Accept(action) = tap.observe_raw(event) {
        tap.dispatch(action);
    }
}

#[test]
fn accepting_a_tap_switches_to_the_optional_post_accept_tolerance() {
    let recorded = Rc::new(RefCell::new(Vec::new()));
    let mut tap = PointerGestureRecognizer::new(GestureCallbacks {
        on_tap_move: Some(Rc::new({
            let recorded = recorded.clone();
            move |details| {
                recorded
                    .borrow_mut()
                    .push((details.global_position, details.delta))
            }
        })),
        ..GestureCallbacks::default()
    });
    tap.set_tap_slop_tolerances(TapSlopTolerance::Default, TapSlopTolerance::Unlimited);
    let start = Instant::now();
    let kind = PointerDeviceKind::Touch;
    route(&mut tap, kind, PointerPhase::Down, Offset::ZERO, start);
    assert!(tap.accept_tap_on_arena_close(1));
    route(
        &mut tap,
        kind,
        PointerPhase::Move,
        Offset::new(50., 0.),
        start + Duration::from_millis(1),
    );
    assert_eq!(
        *recorded.borrow(),
        [(Offset::new(50., 0.), Offset::new(50., 0.))]
    );
    recorded.borrow_mut().clear();
    route(
        &mut tap,
        kind,
        PointerPhase::Move,
        Offset::new(60., 10.),
        start + Duration::from_millis(2),
    );
    assert_eq!(
        *recorded.borrow(),
        [(Offset::new(60., 10.), Offset::new(10., 10.))]
    );
}

#[test]
fn ordinary_mouse_taps_use_touch_slop_instead_of_drag_or_multitap_slop() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |name| {
        let trace = trace.clone();
        move || trace.borrow_mut().push(name)
    };
    let down = record("down");
    let up = record("up");
    let mut tap = PointerGestureRecognizer::new(GestureCallbacks {
        on_tap_down: Some(Rc::new(move |_| down())),
        on_tap_up: Some(Rc::new(move |_| up())),
        on_tap: Some(Rc::new(record("tap"))),
        on_tap_cancel: Some(Rc::new(record("cancel"))),
        ..GestureCallbacks::default()
    });
    for (slop, distance, expected) in [
        (None, 2., &["down", "up", "tap"][..]),
        (None, 18., &["down", "up", "tap"][..]),
        (None, 18.25, &["down", "cancel"][..]),
        (Some(4.), 4., &["down", "up", "tap"][..]),
        (Some(4.), 4.25, &["down", "cancel"][..]),
    ] {
        tap.set_gesture_settings(DeviceGestureSettings::new(slop));
        let start = Instant::now();
        let kind = PointerDeviceKind::Mouse;
        trace.borrow_mut().clear();
        route(&mut tap, kind, PointerPhase::Down, Offset::ZERO, start);
        assert!(tap.accept_tap_on_arena_close(1));
        let end = Offset::new(distance, 0.);
        route(
            &mut tap,
            kind,
            PointerPhase::Move,
            end,
            start + Duration::from_millis(1),
        );
        route(
            &mut tap,
            kind,
            PointerPhase::Up,
            end,
            start + Duration::from_millis(2),
        );
        assert_eq!(&*trace.borrow(), expected, "{slop:?} {distance}");
    }
}

#[test]
fn arena_acceptance_and_release_report_the_raw_mouse_kind() {
    let called = Rc::new(RefCell::new(Vec::new()));
    let mut tap = PointerGestureRecognizer::new(GestureCallbacks {
        on_tap_down: Some(Rc::new({
            let called = called.clone();
            move |details| called.borrow_mut().push(("down", details.kind))
        })),
        on_tap_up: Some(Rc::new({
            let called = called.clone();
            move |details| called.borrow_mut().push(("up", details.kind))
        })),
        ..GestureCallbacks::default()
    });
    let start = Instant::now();
    route(
        &mut tap,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        Offset::ZERO,
        start,
    );
    assert!(tap.accept_tap_on_arena_close(1));
    assert_eq!(*called.borrow(), [("down", PointerDeviceKind::Mouse)]);
    route(
        &mut tap,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        Offset::ZERO,
        start + Duration::from_millis(1),
    );
    assert_eq!(
        *called.borrow(),
        [
            ("down", PointerDeviceKind::Mouse),
            ("up", PointerDeviceKind::Mouse)
        ]
    );
}
