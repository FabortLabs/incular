use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyState, KeyboardEvent, KeyboardKey, LogicalKeyboardKey as L, Size};
use incular_runtime::{KeyMessage, Runtime};
use incular_widgets::{Focus, FocusNode, KeyEventResult, SizedBox, WindowsRawKeyData as W};

fn send(
    runtime: &mut Runtime,
    key_code: u32,
    scan_code: u32,
    modifiers: u32,
    state: KeyState,
) -> bool {
    let event = W {
        key_code,
        scan_code,
        character_code_point: 0,
        modifiers,
    }
    .into_event(state, false);
    runtime.dispatch_key_message(KeyMessage::Raw(event))
}

#[test]
fn process_key_and_the_corresponding_ordinary_release_do_not_dispatch() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let scoped = events.clone();
    let focus = FocusNode::new();
    let widget = Focus::new(SizedBox::shrink())
        .node(focus.clone())
        .autofocus(true)
        .on_key(move |_, raw| {
            scoped.borrow_mut().push((1, raw.is_key_down()));
            KeyEventResult::Ignored
        });
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(120., 60.)))
        .unwrap();
    assert!(focus.has_primary_focus());
    let global = events.clone();
    let _listener = runtime.add_raw_keyboard_listener(move |raw| {
        global.borrow_mut().push((2, raw.is_key_down()));
    });

    assert!(!send(&mut runtime, 65, 30, 0, KeyState::Down));
    assert!(!send(&mut runtime, 65, 30, 0, KeyState::Up));
    assert_eq!(
        events.take(),
        [(2, true), (1, true), (2, false), (1, false)]
    );
    assert!(send(&mut runtime, W::VK_PROCESSKEY, 30, 0, KeyState::Down));
    assert!(
        !runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::KeyA)
    );
    assert_eq!(
        runtime.legacy_raw_keyboard().physical_keys_pressed().len(),
        0
    );
    assert!(send(&mut runtime, 65, 30, 0, KeyState::Up));
    assert!(events.borrow().is_empty());

    // A different key is independent, and a real down on the suppressed
    // key also cancels the marker before handling its real release.
    assert!(send(&mut runtime, W::VK_PROCESSKEY, 30, 0, KeyState::Down));
    assert!(!send(&mut runtime, 66, 48, 0, KeyState::Down));
    assert!(!send(&mut runtime, 66, 48, 0, KeyState::Up));
    assert!(!send(&mut runtime, 65, 30, 0, KeyState::Down));
    assert!(!send(&mut runtime, 65, 30, 0, KeyState::Up));
    assert_eq!(events.take().len(), 8);
}

#[test]
fn repeated_raw_down_is_inferred_from_the_scan_code_and_up_is_not_repeat() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let captured = seen.clone();
    let _listener = runtime.add_raw_keyboard_listener(move |raw| {
        captured
            .borrow_mut()
            .push((raw.is_key_down(), raw.repeat()));
    });
    for state in [KeyState::Down, KeyState::Down, KeyState::Up, KeyState::Down] {
        send(&mut runtime, 65, 30, 0, state);
    }
    assert_eq!(
        seen.take(),
        [(true, false), (true, true), (false, false), (true, false)]
    );
}

#[test]
fn windows_reconciles_exact_sides_or_both_for_a_generic_only_mask() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    send(
        &mut runtime,
        65,
        30,
        W::SHIFT | W::LEFT_SHIFT,
        KeyState::Down,
    );
    assert_eq!(
        runtime.legacy_raw_keyboard().logical_keys_pressed(),
        [L::KEY_A, L::SHIFT_LEFT].into()
    );
    send(&mut runtime, 65, 30, 0, KeyState::Up);
    assert!(
        runtime
            .legacy_raw_keyboard()
            .logical_keys_pressed()
            .is_empty()
    );
    send(
        &mut runtime,
        65,
        30,
        W::SHIFT | W::CONTROL | W::ALT,
        KeyState::Down,
    );
    assert_eq!(
        runtime.legacy_raw_keyboard().logical_keys_pressed(),
        [
            L::KEY_A,
            L::SHIFT_LEFT,
            L::SHIFT_RIGHT,
            L::CONTROL_LEFT,
            L::CONTROL_RIGHT,
            L::ALT_LEFT,
            L::ALT_RIGHT
        ]
        .into()
    );
    assert!(
        runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::ShiftLeft)
    );
}

#[test]
fn ime_raw_suppression_does_not_discard_independently_supplied_typed_packet() {
    let raw_events = Rc::new(RefCell::new(Vec::new()));
    let events = raw_events.clone();
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    let _listener = runtime.add_raw_keyboard_listener(move |raw| {
        events.borrow_mut().push(raw.physical_key());
    });
    let raw = W {
        key_code: W::VK_PROCESSKEY,
        scan_code: 30,
        ..W::default()
    }
    .into_event(KeyState::Down, false);
    let typed = KeyboardEvent::key_down(KeyboardKey::Character("b".into()), Code::KeyB);
    assert!(!runtime.dispatch_key_message(KeyMessage::TypedAndRaw { typed, raw }));
    assert!(raw_events.borrow().is_empty());
    assert!(
        runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::KeyB)
    );
    assert!(
        !runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::KeyA)
    );
    assert_eq!(
        runtime.legacy_raw_keyboard().physical_keys_pressed().len(),
        0
    );
    assert!(send(&mut runtime, 65, 30, 0, KeyState::Up));
}
