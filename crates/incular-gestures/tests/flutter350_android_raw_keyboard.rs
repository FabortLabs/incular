use incular_core::{Code, KeyState, LogicalKeyboardKey as L, PhysicalKeyboardKey as P};
use incular_gestures::{
    AndroidRawKeyData as A, RawKeyPlatformData, RawKeyboardModifier as M, RawKeyboardSide as S,
};

const KEYBOARD_SOURCE: u32 = 0x101;

#[test]
fn android_individual_modifier_masks_and_function_combination() {
    let cases = [
        (A::ALT | A::LEFT_ALT, M::Alt, S::Left),
        (A::ALT | A::RIGHT_ALT, M::Alt, S::Right),
        (A::SHIFT | A::LEFT_SHIFT, M::Shift, S::Left),
        (A::SHIFT | A::RIGHT_SHIFT, M::Shift, S::Right),
        (A::SYMBOL, M::Symbol, S::All),
        (A::FUNCTION, M::Function, S::All),
        (A::CONTROL | A::LEFT_CONTROL, M::Control, S::Left),
        (A::CONTROL | A::RIGHT_CONTROL, M::Control, S::Right),
        (A::META | A::LEFT_META, M::Meta, S::Left),
        (A::META | A::RIGHT_META, M::Meta, S::Right),
        (A::CAPS_LOCK, M::CapsLock, S::All),
        (A::NUM_LOCK, M::NumLock, S::All),
        (A::SCROLL_LOCK, M::ScrollLock, S::All),
    ];
    for (mask, modifier, side) in cases {
        for meta_state in [mask, mask | A::FUNCTION] {
            let data = A {
                meta_state,
                ..A::default()
            };
            for key in [
                M::Alt,
                M::Shift,
                M::Symbol,
                M::Function,
                M::Control,
                M::Meta,
                M::CapsLock,
                M::NumLock,
                M::ScrollLock,
            ] {
                let expect = if key == modifier {
                    Some(side)
                } else if key == M::Function && meta_state & A::FUNCTION != 0 {
                    Some(S::All)
                } else {
                    None
                };
                assert_eq!(data.pressed_modifier_side(key), expect);
                assert_eq!(data.is_modifier_pressed(key, side), expect.is_some());
            }
        }
    }
    let any_shift = A {
        meta_state: A::SHIFT,
        ..A::default()
    };
    assert_eq!(any_shift.pressed_modifier_side(M::Shift), Some(S::All));
    assert!(!any_shift.is_modifier_pressed(M::Shift, S::Left));
    assert!(!any_shift.is_modifier_pressed(M::Shift, S::Right));
}

fn packet(key_code: u32, scan_code: u32, event_source: u32) -> A {
    A {
        key_code,
        scan_code,
        event_source,
        device_id: 1,
        ..A::default()
    }
}

#[test]
fn android_printable_and_named_keys_keep_key_label_and_event_character_separate() {
    let printable = A {
        code_point: 'A'.into(),
        plain_code_point: 'a'.into(),
        ..packet(29, 30, KEYBOARD_SOURCE)
    };
    assert_eq!(
        printable.flutter_physical_key_id(),
        u64::from(P::KEY_A.usb_hid_usage())
    );
    assert_eq!(printable.flutter_logical_key(), L::KEY_A);
    assert_eq!(printable.key_label(), Some('a'));
    let event = printable.into_event(KeyState::Down, Some("A".into()));
    assert_eq!(event.physical_key(), Code::KeyA);
    assert_eq!(event.character().as_deref(), Some("A"));
    assert_eq!(printable.into_event(KeyState::Down, None).character(), None);
    assert_eq!(
        printable
            .into_event(KeyState::Up, Some("A".into()))
            .character(),
        None
    );

    for (data, physical, logical, code) in [
        (
            packet(111, 1, KEYBOARD_SOURCE),
            P::ESCAPE,
            L::ESCAPE,
            Code::Escape,
        ),
        (
            A {
                meta_state: A::LEFT_SHIFT,
                ..packet(59, 42, KEYBOARD_SOURCE)
            },
            P::SHIFT_LEFT,
            L::SHIFT_LEFT,
            Code::ShiftLeft,
        ),
    ] {
        assert_eq!(
            data.flutter_physical_key_id(),
            u64::from(physical.usb_hid_usage())
        );
        assert_eq!(data.flutter_logical_key(), logical);
        assert_eq!(data.key_label(), None);
        assert_eq!(data.into_event(KeyState::Down, None).physical_key(), code);
    }
    let combining = A {
        plain_code_point: 0x8000_0301,
        ..printable
    };
    assert_eq!(combining.key_label(), Some('\u{301}'));
    assert_eq!(combining.flutter_logical_key().key_id(), 0x301);
}

#[test]
fn joystick_dpad_keyboard_arrows_and_gamepad_center_keep_physical_identity() {
    let arrow = u64::from(P::ARROW_DOWN.usb_hid_usage());
    for data in [
        packet(20, 0, A::SOURCE_JOYSTICK),
        packet(20, 108, KEYBOARD_SOURCE),
    ] {
        assert_eq!(data.flutter_physical_key_id(), arrow);
        assert_eq!(data.flutter_logical_key(), L::ARROW_DOWN);
        assert_eq!(data.key_label(), None);
        assert_eq!(
            data.into_event(KeyState::Down, None).physical_key(),
            Code::ArrowDown
        );
    }
    let center = packet(23, 317, 0x501);
    assert_eq!(
        center.flutter_physical_key_id(),
        u64::from(P::GAME_BUTTON_THUMB_LEFT.usb_hid_usage())
    );
    assert_eq!(center.flutter_logical_key(), L::SELECT);
    assert_eq!(center.key_label(), None);
    assert_eq!(
        packet(20, 0, KEYBOARD_SOURCE).flutter_physical_key_id(),
        L::ANDROID_PLANE
    );
}

#[test]
fn android_repeat_and_device_metadata_survive_even_when_ignored_by_equality() {
    let data = A {
        code_point: 65,
        plain_code_point: 97,
        key_code: 29,
        scan_code: 30,
        vendor_id: 21,
        product_id: 33,
        device_id: 10,
        repeat_count: 42,
        event_source: KEYBOARD_SOURCE,
        ..A::default()
    };
    let event = data.into_event(KeyState::Down, Some("A".into()));
    let Some(RawKeyPlatformData::Android(found)) = event.data() else {
        panic!("missing packet")
    };
    assert_eq!(
        (
            found.vendor_id,
            found.product_id,
            found.device_id,
            found.repeat_count,
            found.event_source
        ),
        (21, 33, 10, 42, KEYBOARD_SOURCE)
    );
    assert_eq!(
        *found,
        A {
            code_point: 65,
            plain_code_point: 97,
            key_code: 29,
            scan_code: 30,
            ..A::default()
        }
    );
    assert_ne!(*found, A::default());
    assert!(!data.is_virtual_keyboard());
    assert!(
        A {
            device_id: -1,
            ..data
        }
        .is_virtual_keyboard()
    );
    assert!(
        A {
            flags: A::FLAG_SOFT_KEYBOARD,
            ..data
        }
        .is_virtual_keyboard()
    );
}

#[test]
fn android_numpad_identity_takes_precedence_over_the_plain_label() {
    let data = A {
        plain_code_point: '2'.into(),
        ..packet(146, 80, KEYBOARD_SOURCE)
    };
    assert_eq!(data.flutter_logical_key(), L::NUMPAD2);
    let event = data.into_event(KeyState::Down, Some("2".into()));
    assert_eq!(event.physical_key(), Code::Numpad2);
    assert_eq!(event.portable().location, incular_core::Location::Numpad);
    assert_eq!(event.character().as_deref(), Some("2"));
}
