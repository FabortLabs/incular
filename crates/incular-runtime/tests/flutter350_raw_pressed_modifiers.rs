use incular_core::{Code, KeyState, KeyboardEvent, KeyboardKey, LogicalKeyboardKey as L};
use incular_runtime::{KeyMessage, Runtime};
use incular_widgets::{FuchsiaRawKeyData as F, SizedBox, WebRawKeyData as W};

fn fuchsia(runtime: &mut Runtime, hid_usage: u32, ch: char, modifiers: u32, state: KeyState) {
    let packet = F {
        hid_usage,
        code_point: ch.into(),
        modifiers,
    };
    runtime.dispatch_key_message(KeyMessage::Raw(packet.into_event(state, false)));
}

fn web(runtime: &mut Runtime, code: &str, key: &str, location: u32, mask: u32, state: KeyState) {
    let packet = W {
        code: code.into(),
        key: key.into(),
        location,
        meta_state: mask,
        key_code: 0,
    };
    runtime.dispatch_key_message(KeyMessage::Raw(packet.into_event(state, false, false)));
}

fn check(runtime: &Runtime, expected: &[L]) {
    assert_eq!(
        runtime.legacy_raw_keyboard().logical_keys_pressed(),
        expected.iter().copied().collect()
    );
}

#[test]
fn fuchsia_pressed_keys_adopt_and_clear_sided_packet_modifiers() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    check(&runtime, &[]);
    fuchsia(
        &mut runtime,
        0x0007_0004,
        'a',
        F::LEFT_SHIFT,
        KeyState::Down,
    );
    check(&runtime, &[L::KEY_A, L::SHIFT_LEFT]);
    assert_eq!(
        runtime.legacy_raw_keyboard().look_up_layout(0x0007_00e1),
        Some(L::SHIFT_LEFT)
    );
    assert!(
        runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::ShiftLeft)
    );

    let typed = KeyboardEvent::key_down(KeyboardKey::Character("c".into()), Code::KeyC);
    runtime.dispatch_key_message(KeyMessage::Typed(typed));
    check(&runtime, &[L::KEY_A, L::SHIFT_LEFT]);

    // A layout change alters the release character, not the physical key.
    fuchsia(&mut runtime, 0x0007_0004, 'A', F::RIGHT_META, KeyState::Up);
    check(&runtime, &[L::META_RIGHT]);
    assert_eq!(
        runtime
            .legacy_raw_keyboard()
            .physical_keys_pressed()
            .collect::<Vec<_>>(),
        [0x0007_00e7]
    );

    fuchsia(
        &mut runtime,
        0x0007_0005,
        'b',
        F::LEFT_SHIFT | F::RIGHT_SHIFT,
        KeyState::Down,
    );
    check(&runtime, &[L::KEY_B, L::SHIFT_LEFT, L::SHIFT_RIGHT]);
    runtime.clear_legacy_raw_keyboard();
    check(&runtime, &[]);
    assert!(
        runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::KeyB)
    );
}

#[test]
fn web_undirected_modifier_preserves_observed_side_and_falls_back_left() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    check(&runtime, &[]);
    web(&mut runtime, "KeyA", "a", 0, W::SHIFT, KeyState::Down);
    check(&runtime, &[L::KEY_A, L::SHIFT_LEFT]);
    web(&mut runtime, "KeyA", "a", 0, 0, KeyState::Up);
    check(&runtime, &[]);

    web(
        &mut runtime,
        "ShiftRight",
        "Shift",
        2,
        W::SHIFT,
        KeyState::Down,
    );
    check(&runtime, &[L::SHIFT_RIGHT]);
    web(&mut runtime, "KeyA", "A", 0, W::SHIFT, KeyState::Down);
    check(&runtime, &[L::KEY_A, L::SHIFT_RIGHT]);
    web(&mut runtime, "ShiftRight", "Shift", 2, 0, KeyState::Up);
    check(&runtime, &[L::KEY_A]);
    web(&mut runtime, "KeyA", "a", 0, 0, KeyState::Up);
    check(&runtime, &[]);
}

#[test]
fn web_altgraph_and_remapped_caps_lock_keep_their_layout_identities() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    web(&mut runtime, "AltRight", "AltGraph", 2, 0, KeyState::Down);
    check(&runtime, &[L::ALT_GRAPH]);
    web(&mut runtime, "AltRight", "Alt", 2, 0, KeyState::Up);
    check(&runtime, &[]);

    web(&mut runtime, "CapsLock", "Escape", 0, 0, KeyState::Down);
    web(&mut runtime, "KeyA", "a", 0, 0, KeyState::Down);
    check(&runtime, &[L::ESCAPE, L::KEY_A]);
    web(&mut runtime, "CapsLock", "Escape", 0, 0, KeyState::Up);
    check(&runtime, &[L::KEY_A]);
}
