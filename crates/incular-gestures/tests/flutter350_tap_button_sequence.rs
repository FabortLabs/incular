use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, SECONDARY_POINTER_BUTTON,
    time::Instant,
};
use incular_gestures::{
    GestureCallbacks, GestureDecision, GestureRecognizer, PointerGestureRecognizer, RawPointerEvent,
};

type Log = Rc<RefCell<Vec<&'static str>>>;

fn route(
    recognizer: &mut PointerGestureRecognizer,
    base: Instant,
    pointer: u64,
    kind: PointerDeviceKind,
    phase: PointerPhase,
    buttons: u32,
    position: Offset,
    elapsed: u64,
) {
    let event = RawPointerEvent::new(
        pointer,
        1,
        kind,
        buttons,
        position,
        phase,
        base + Duration::from_millis(elapsed),
    );
    if let GestureDecision::Accept(action) = recognizer.observe_raw(event) {
        recognizer.dispatch(action);
    }
}

fn callbacks() -> (PointerGestureRecognizer, Log) {
    let log = Log::default();
    let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_tap_down: Some(Rc::new({
            let log = log.clone();
            move |_| log.borrow_mut().push("down")
        })),
        on_tap_up: Some(Rc::new({
            let log = log.clone();
            move |_| log.borrow_mut().push("up")
        })),
        on_tap_cancel: Some(Rc::new({
            let log = log.clone();
            move || log.borrow_mut().push("cancel")
        })),
        ..GestureCallbacks::default()
    });
    recognizer.set_supported_devices([PointerDeviceKind::Touch]);
    (recognizer, log)
}

fn check_button_change(after_down: bool, check_next_press: bool) {
    let (mut recognizer, log) = callbacks();
    let base = Instant::now();
    let send =
        |recognizer: &mut PointerGestureRecognizer, pointer, phase, buttons, position, ms| {
            route(
                recognizer,
                base,
                pointer,
                PointerDeviceKind::Touch,
                phase,
                buttons,
                position,
                ms,
            );
        };
    let origin = Offset::new(10., 10.);
    send(
        &mut recognizer,
        1,
        PointerPhase::Down,
        PRIMARY_POINTER_BUTTON,
        origin,
        0,
    );
    assert!(log.borrow().is_empty());
    let changed = if after_down {
        let _ = recognizer.poll(base + Duration::from_millis(1000));
        assert_eq!(*log.borrow(), ["down"]);
        1001
    } else {
        10
    };
    let expected: &[&str] = if after_down { &["down", "cancel"] } else { &[] };
    for (phase, buttons, position, ms) in [
        (
            PointerPhase::Move,
            PRIMARY_POINTER_BUTTON | SECONDARY_POINTER_BUTTON,
            origin,
            changed,
        ),
        (
            PointerPhase::Move,
            SECONDARY_POINTER_BUTTON,
            origin,
            changed + 1,
        ),
        (PointerPhase::Up, 0, Offset::new(11., 9.), changed + 2),
    ] {
        send(&mut recognizer, 1, phase, buttons, position, ms);
        assert_eq!(&*log.borrow(), expected, "{phase:?} {buttons:#x}");
    }
    if check_next_press {
        send(
            &mut recognizer,
            2,
            PointerPhase::Down,
            PRIMARY_POINTER_BUTTON,
            Offset::new(30., 30.),
            2000,
        );
        let _ = recognizer.poll(base + Duration::from_millis(3000));
        send(
            &mut recognizer,
            2,
            PointerPhase::Up,
            0,
            Offset::new(31., 29.),
            3001,
        );
        assert_eq!(*log.borrow(), [expected, &["down", "up"]].concat());
    }
}

#[test]
fn changing_buttons_before_tap_down_is_silent() {
    check_button_change(false, false);
}

#[test]
fn changing_buttons_after_tap_down_reports_one_cancel() {
    check_button_change(true, false);
}

#[test]
fn changing_buttons_before_tap_down_does_not_poison_the_next_press() {
    check_button_change(false, true);
}

#[test]
fn changing_buttons_after_tap_down_does_not_poison_the_next_press() {
    check_button_change(true, true);
}

#[test]
fn only_supported_raw_devices_can_tap_even_with_the_same_pointer_id() {
    let taps = Rc::new(RefCell::new(0));
    let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
        on_tap: Some(Rc::new({
            let taps = taps.clone();
            move || *taps.borrow_mut() += 1
        })),
        ..GestureCallbacks::default()
    });
    recognizer.set_supported_devices([PointerDeviceKind::Mouse, PointerDeviceKind::Stylus]);
    let base = Instant::now();
    for (index, (device, expected)) in [
        (PointerDeviceKind::Touch, 0),
        (PointerDeviceKind::Mouse, 1),
        (PointerDeviceKind::Stylus, 2),
    ]
    .into_iter()
    .enumerate()
    {
        let at = (index as u64) * 100;
        route(
            &mut recognizer,
            base,
            1,
            device,
            PointerPhase::Down,
            PRIMARY_POINTER_BUTTON,
            Offset::new(10., 10.),
            at,
        );
        assert_eq!(*taps.borrow(), index.saturating_sub(1));
        route(
            &mut recognizer,
            base,
            1,
            device,
            PointerPhase::Up,
            0,
            Offset::new(11., 9.),
            at + 5,
        );
        assert_eq!(*taps.borrow(), expected);
    }
}
