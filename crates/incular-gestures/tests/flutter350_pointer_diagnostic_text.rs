use std::{cell::Cell, rc::Rc, time::Duration};

use incular_core::{Offset, PRIMARY_POINTER_BUTTON, PointerPhase, time::Instant};
use incular_gestures::{
    GestureCallbacks, GestureRecognizer, PointerDeviceKind, PointerGestureRecognizer,
    RawPointerEvent, TapTrackingState, constants::PRESS_TIMEOUT,
};

fn press(at: Instant) -> RawPointerEvent {
    RawPointerEvent::new(
        1,
        0,
        PointerDeviceKind::Touch,
        PRIMARY_POINTER_BUTTON,
        Offset::new(10., 10.),
        PointerPhase::Down,
        at,
    )
}

#[test]
fn tap_down_diagnostics_only_report_down_on_the_actual_deadline() {
    let callbacks = Rc::new(Cell::new(0));
    let mut tap = PointerGestureRecognizer::new(GestureCallbacks {
        on_tap: Some(Rc::new(|| {})),
        on_tap_down: Some(Rc::new({
            let callbacks = callbacks.clone();
            move |_| callbacks.set(callbacks.get() + 1)
        })),
        ..Default::default()
    });
    let at = Instant::now();
    assert_eq!(
        tap.tap_diagnostics().to_string(),
        "TapGestureRecognizer(state: ready)"
    );
    tap.observe_raw(press(at));
    let before = tap.tap_diagnostics();
    assert_eq!(before.state, TapTrackingState::Possible);
    assert_eq!(before.buttons, Some(PRIMARY_POINTER_BUTTON));
    assert!(!before.tap_down_sent);
    tap.poll(at + PRESS_TIMEOUT - Duration::from_millis(1));
    assert!(!tap.tap_diagnostics().tap_down_sent);
    assert_eq!(callbacks.get(), 0);
    tap.poll(at + PRESS_TIMEOUT);
    assert_eq!(
        tap.tap_diagnostics().to_string(),
        "TapGestureRecognizer(state: possible, button: 1, sent tap down)"
    );
    assert_eq!(callbacks.get(), 1);
    tap.poll(at + PRESS_TIMEOUT + Duration::from_millis(1));
    assert_eq!(callbacks.get(), 1);
    let mut cancel = press(at + PRESS_TIMEOUT + Duration::from_millis(2));
    cancel.phase = PointerPhase::Cancel;
    cancel.down = false;
    tap.observe_raw(cancel);
    assert_eq!(
        tap.tap_diagnostics().to_string(),
        "TapGestureRecognizer(state: ready)"
    );
}

#[test]
fn raw_pointer_brief_and_full_descriptions_are_single_line_and_read_live_fields() {
    let at = Instant::now();
    let mut down = press(at);
    down.position = Offset::ZERO;
    down.local_position = Offset::ZERO;
    let brief = down.to_string();
    let full = down.to_string_full();
    assert_eq!(brief, "PointerDownEvent(position: Offset(0.0, 0.0))");
    assert_eq!(brief.lines().count(), 1);
    assert_eq!(full.lines().count(), 1);
    assert!(full.contains("pointer: 1, kind: touch, buttons: 1, down: true"));
    assert!(!full.contains("localPosition"));

    down.local_position = Offset::new(8., -2.);
    down.device = 27;
    down.synthesized = true;
    let transformed = down.to_string_full();
    assert_eq!(transformed.lines().count(), 1);
    assert!(transformed.contains("localPosition: Offset(8.0, -2.0)"));
    assert!(transformed.contains("kind: touch, device: 27, buttons: 1"));
    assert!(transformed.ends_with("down: true, synthesized)"));
}
