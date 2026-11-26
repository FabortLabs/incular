use incular_core::{
    Offset, PRIMARY_POINTER_BUTTON, PointerPhase, SECONDARY_POINTER_BUTTON, time::Instant,
};
use incular_gestures::{
    GestureArena, GestureArenaKey, GestureCallbacks, GestureDecision, GestureDisposition,
    GestureRecognizer, PointerDeviceKind, PointerGestureRecognizer, RawPointerEvent,
};
use std::{cell::RefCell, rc::Rc};

fn down(mask: u32) -> RawPointerEvent {
    RawPointerEvent::new(
        6,
        4,
        PointerDeviceKind::Touch,
        mask,
        Offset::new(10.0, 10.0),
        PointerPhase::Down,
        Instant::now(),
    )
}

fn compete(mask: u32) -> Vec<&'static str> {
    let press = down(mask);
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut arena = GestureArena::new();
    let key = GestureArenaKey::pointer_device(1, press.device, press.pointer);
    let mut active = Vec::new();
    for (accepted, label) in [
        (PRIMARY_POINTER_BUTTON, "onStartPrimary"),
        (SECONDARY_POINTER_BUTTON, "onStartSecondary"),
    ] {
        let mut drag = PointerGestureRecognizer::new(GestureCallbacks {
            on_horizontal_drag_start: Some(Rc::new({
                let log = log.clone();
                move |_| log.borrow_mut().push(label)
            })),
            ..Default::default()
        });
        drag.set_allowed_buttons_filter(Some(Rc::new(move |buttons| buttons == accepted)));
        match drag.observe_raw(press) {
            GestureDecision::Pending => active.push((arena.add(key, false), drag)),
            GestureDecision::Reject => assert!(drag.accept_on_press().is_none()),
            _ => panic!("start must wait for the arena to close"),
        }
    }
    assert!(log.borrow().is_empty());
    let decisions = arena.close_sole_member(key);
    for (member, mut recognizer) in active {
        assert!(decisions.iter().any(|entry| {
            entry.member == member && entry.disposition == GestureDisposition::Accepted
        }));
        let start = recognizer.accept_on_press().expect("sole drag won");
        recognizer.dispatch(start);
    }
    log.take()
}

#[test]
fn only_the_primary_drag_enters_for_a_primary_press() {
    assert_eq!(compete(PRIMARY_POINTER_BUTTON), ["onStartPrimary"]);
}

#[test]
fn only_the_secondary_drag_enters_for_a_secondary_press() {
    assert_eq!(compete(SECONDARY_POINTER_BUTTON), ["onStartSecondary"]);
}

#[test]
fn a_button_chord_is_not_treated_as_either_single_button() {
    assert!(compete(PRIMARY_POINTER_BUTTON | SECONDARY_POINTER_BUTTON).is_empty());
}
