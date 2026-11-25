use std::collections::HashSet;

use incular_core::{
    Code, Color, HardwareKeyboardState, InputEvent, KeyboardEvent, KeyboardKey as Key, Modifiers,
    NamedKey, Size,
};
use incular_runtime::Runtime;
use incular_widgets::Widget;

fn runtime() -> Runtime {
    Runtime::new(Widget::box_(Size::new(10.0, 10.0), Color::BLACK)).unwrap()
}

const NUM: Key = Key::Named(NamedKey::NumLock);

fn one() -> Key {
    Key::Character("1".to_owned())
}

fn num_down() -> KeyboardEvent {
    KeyboardEvent::key_down(NUM, Code::NumLock)
}

fn num_up() -> KeyboardEvent {
    KeyboardEvent::key_up(NUM, Code::NumLock)
}

fn one_down() -> KeyboardEvent {
    KeyboardEvent::key_down(one(), Code::Numpad1)
}

fn one_up() -> KeyboardEvent {
    KeyboardEvent::key_up(one(), Code::Numpad1)
}

fn one_repeat() -> KeyboardEvent {
    KeyboardEvent {
        repeat: true,
        ..one_down()
    }
}

fn send(runtime: &mut Runtime, event: KeyboardEvent) {
    let _ = runtime.handle_input(InputEvent::Key(event));
}

fn assert_state(state: &HardwareKeyboardState, codes: &[Code], keys: &[Key], num_lock: bool) {
    assert_eq!(
        state.physical_keys_pressed(),
        HashSet::from_iter(codes.iter().copied())
    );
    assert_eq!(
        state.logical_keys_pressed(),
        HashSet::from_iter(keys.iter().cloned())
    );
    assert_eq!(
        state.lock_modes_enabled(),
        if num_lock {
            Modifiers::NUM_LOCK
        } else {
            Modifiers::empty()
        }
    );
}

#[test]
fn records_pressed_positions_logical_values_and_toggled_locks() {
    let mut runtime = runtime();
    for (event, codes, keys, expected_lock) in [
        (num_down(), vec![Code::NumLock], vec![NUM], true),
        (
            one_down(),
            vec![Code::NumLock, Code::Numpad1],
            vec![NUM, one()],
            true,
        ),
        (
            one_repeat(),
            vec![Code::NumLock, Code::Numpad1],
            vec![NUM, one()],
            true,
        ),
        (num_up(), vec![Code::Numpad1], vec![one()], true),
        (
            num_down(),
            vec![Code::NumLock, Code::Numpad1],
            vec![NUM, one()],
            false,
        ),
        (one_up(), vec![Code::NumLock], vec![NUM], false),
        (num_up(), vec![], vec![], false),
    ] {
        send(&mut runtime, event);
        assert_state(runtime.hardware_keyboard(), &codes, &keys, expected_lock);
    }
}

#[test]
fn membership_queries_follow_down_repeat_and_up() {
    let mut runtime = runtime();
    for (event, code, key, pressed) in [
        (num_down(), Code::NumLock, NUM, true),
        (one_down(), Code::Numpad1, one(), true),
        (one_repeat(), Code::Numpad1, one(), true),
        (num_up(), Code::Numpad1, one(), true),
        (num_down(), Code::NumLock, NUM, true),
        (one_up(), Code::Numpad1, one(), false),
        (num_up(), Code::NumLock, NUM, false),
    ] {
        send(&mut runtime, event);
        let state = runtime.hardware_keyboard();
        assert_eq!(state.is_physical_key_pressed(code), pressed);
        assert_eq!(state.is_logical_key_pressed(&key), pressed);
    }
}

#[test]
fn press_owns_its_position_and_repeats_do_not_toggle_a_lock() {
    let mut state = HardwareKeyboardState::default();
    state.observe(&num_down());
    state.observe(&KeyboardEvent {
        repeat: true,
        ..num_down()
    });
    assert!(state.lock_modes_enabled().contains(Modifiers::NUM_LOCK));
    state.observe(&KeyboardEvent::key_up(
        Key::Named(NamedKey::Escape),
        Code::NumLock,
    ));
    assert!(!state.is_physical_key_pressed(Code::NumLock));
    assert!(!state.is_logical_key_pressed(&NUM));
    assert!(state.lock_modes_enabled().contains(Modifiers::NUM_LOCK));
}

fn letter(label: &str, down: bool) -> KeyboardEvent {
    let logical = Key::Character(label.to_owned());
    if down {
        KeyboardEvent::key_down(logical, Code::KeyA)
    } else {
        KeyboardEvent::key_up(logical, Code::KeyA)
    }
}

#[test]
fn opt_in_keyboard_diagnostics_report_the_event_and_both_pressed_states() {
    use std::{cell::RefCell, rc::Rc};
    let mut runtime = runtime();
    let logs = Rc::new(RefCell::new(Vec::new()));
    runtime.set_keyboard_event_debug_listener({
        let logs = logs.clone();
        move |line| logs.borrow_mut().push(line.to_owned())
    });
    assert!(!runtime.dispatch_keyboard_event(letter("a", true)));
    let captured = logs.take().join("\n");
    assert!(captured.contains("KEYBOARD: Key event received: "));
    assert!(captured.contains("KEYBOARD: Pressed state before processing the event:"));
    assert!(captured.contains("KEYBOARD: Pressed state after processing the event:"));
    runtime.clear_keyboard_event_debug_listener();
    assert!(!runtime.dispatch_keyboard_event(letter("a", false)));
    assert!(logs.borrow().is_empty());
}

#[test]
fn irregular_keys_are_still_delivered_and_update_the_pressed_state() {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    let mut runtime = runtime();
    let logs = Rc::new(RefCell::new(Vec::new()));
    let delivered = Rc::new(Cell::new(0));
    runtime.set_keyboard_event_debug_listener({
        let logs = logs.clone();
        move |line| logs.borrow_mut().push(line.to_owned())
    });
    let _handler = runtime.add_hardware_key_handler({
        let delivered = delivered.clone();
        move |_| {
            delivered.set(delivered.get() + 1);
            false
        }
    });
    for (event, irregular, key) in [
        (letter("a", true), false, Some("a")),
        (letter("b", true), true, Some("b")),
        (letter("b", false), false, None),
        (letter("a", false), true, None),
        (
            KeyboardEvent {
                repeat: true,
                ..letter("a", true)
            },
            true,
            Some("a"),
        ),
        (letter("b", true), true, Some("b")),
        (letter("a", false), true, None),
    ] {
        assert!(!runtime.dispatch_keyboard_event(event));
        assert_eq!(
            logs.take().iter().any(|line| line.contains("ERROR")),
            irregular
        );
        let expected = key.map(|value| Key::Character(value.to_owned()));
        let keys: Vec<_> = expected.into_iter().collect();
        let codes = if keys.is_empty() {
            vec![]
        } else {
            vec![Code::KeyA]
        };
        assert_state(runtime.hardware_keyboard(), &codes, &keys, false);
    }
    assert_eq!(delivered.get(), 7);
}
