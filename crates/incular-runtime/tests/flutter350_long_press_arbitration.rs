use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{GestureDetector, HitTestBehavior, SizedBox};

#[test]
fn a_pending_tap_reports_down_before_the_competing_hold_is_accepted() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |value| {
        let trace = trace.clone();
        move || trace.borrow_mut().push(value)
    };
    let down = record("down");
    let tap_down = record("tap_down");
    let long_press = GestureDetector::new(SizedBox::new().width(100.).height(100.))
        .behavior(HitTestBehavior::Opaque)
        .on_long_press_down(move |_| down())
        .on_long_press(record("start"));
    let tap = GestureDetector::new(long_press)
        .behavior(HitTestBehavior::Opaque)
        .on_tap_down(move |_| tap_down());
    let mut runtime = Runtime::new(tap.into()).unwrap();
    let size = Constraints::tight(Size::new(100., 100.));
    let at = Instant::now();
    runtime.run_frame_at(size, at).unwrap();
    assert!(trace.borrow().is_empty());

    runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 5,
            device: 1,
            kind: PointerDeviceKind::Touch,
            button: Some(PRIMARY_POINTER_BUTTON),
            phase: PointerPhase::Down,
            buttons: PRIMARY_POINTER_BUTTON,
            position: Offset::new(10., 10.),
            sample: PointerSampleMetadata::EMPTY,
        },
        at,
    );
    assert_eq!(trace.borrow().as_slice(), ["down"]);
    runtime
        .run_frame_at(size, at + Duration::from_millis(300))
        .unwrap();
    assert_eq!(trace.borrow().as_slice(), ["down", "tap_down"]);
    runtime
        .run_frame_at(size, at + Duration::from_secs(1))
        .unwrap();
    assert_eq!(trace.borrow().as_slice(), ["down", "tap_down", "start"]);
    drop(runtime);
    assert_eq!(trace.borrow().as_slice(), ["down", "tap_down", "start"]);
}

fn button_competition(button: u32) {
    use incular_core::SECONDARY_POINTER_BUTTON;
    let trace = Rc::new(RefCell::new(Vec::new()));
    let hold = GestureDetector::new(SizedBox::new().width(100.).height(100.))
        .behavior(HitTestBehavior::Opaque)
        .on_long_press_start({
            let trace = trace.clone();
            move |_| trace.borrow_mut().push("hold")
        });
    let tap = GestureDetector::new(hold)
        .behavior(HitTestBehavior::Opaque)
        .on_tap_down({
            let trace = trace.clone();
            move |_| trace.borrow_mut().push("primary")
        })
        .on_secondary_tap_down({
            let trace = trace.clone();
            move |_| trace.borrow_mut().push("secondary")
        });
    let mut runtime = Runtime::new(tap.into()).unwrap();
    let size = Constraints::tight(Size::new(100., 100.));
    let at = Instant::now();
    runtime.run_frame_at(size, at).unwrap();
    let pointer = if button == PRIMARY_POINTER_BUTTON {
        5
    } else {
        7
    };
    let event = |phase| InputEvent::PointerWithMetadata {
        pointer,
        device: 1,
        kind: PointerDeviceKind::Touch,
        button: Some(button),
        phase,
        buttons: if phase == PointerPhase::Down {
            button
        } else {
            0
        },
        position: if button == PRIMARY_POINTER_BUTTON {
            Offset::new(10., 10.)
        } else {
            Offset::new(30., 30.)
        },
        sample: PointerSampleMetadata::EMPTY,
    };
    runtime.handle_input_at(event(PointerPhase::Down), at);
    // Flutter's gesture tester drains deferred decisions after routing.
    runtime
        .run_frame_at(size, at + Duration::from_millis(1))
        .unwrap();
    if button == SECONDARY_POINTER_BUTTON {
        assert_eq!(trace.borrow().as_slice(), ["secondary"]);
    } else {
        assert!(trace.borrow().is_empty());
    }
    runtime.handle_input_at(event(PointerPhase::Up), at + Duration::from_millis(10));
    runtime
        .run_frame_at(size, at + Duration::from_millis(11))
        .unwrap();
    let expected = if button == PRIMARY_POINTER_BUTTON {
        "primary"
    } else {
        "secondary"
    };
    assert_eq!(trace.borrow().as_slice(), [expected]);
}

#[test]
fn a_primary_hold_does_not_delay_a_secondary_tap_down() {
    button_competition(incular_core::SECONDARY_POINTER_BUTTON);
}

#[test]
fn a_primary_hold_keeps_a_primary_tap_pending_until_release() {
    button_competition(PRIMARY_POINTER_BUTTON);
}
