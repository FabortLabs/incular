use incular_core::{
    Code, KeyState, KeyboardKey, Location, LogicalKeyboardKey as L, NamedKey,
    PhysicalKeyboardKey as P,
};
use incular_gestures::{
    RawKeyPlatformData, RawKeyboardModifier as M, RawKeyboardSide as S, WindowsRawKeyData as W,
};

fn packet(key_code: u32, scan_code: u32, character_code_point: u32, modifiers: u32) -> W {
    W {
        key_code,
        scan_code,
        character_code_point,
        modifiers,
    }
}

#[test]
fn individual_windows_masks_and_combined_caps_report_their_own_side() {
    for (mask, modifier, side) in [
        (W::LEFT_ALT, M::Alt, S::Left),
        (W::RIGHT_ALT, M::Alt, S::Right),
        (W::LEFT_SHIFT, M::Shift, S::Left),
        (W::RIGHT_SHIFT, M::Shift, S::Right),
        (W::LEFT_CONTROL, M::Control, S::Left),
        (W::RIGHT_CONTROL, M::Control, S::Right),
        (W::LEFT_META, M::Meta, S::Left),
        (W::RIGHT_META, M::Meta, S::Right),
        (W::SHIFT, M::Shift, S::All),
        (W::CONTROL, M::Control, S::All),
        (W::ALT, M::Alt, S::All),
        (W::CAPS_LOCK, M::CapsLock, S::All),
        (W::NUM_LOCK, M::NumLock, S::All),
        (W::SCROLL_LOCK, M::ScrollLock, S::All),
    ] {
        for combined in [mask, mask | W::CAPS_LOCK] {
            let data = packet(4, 4, 0, combined);
            for key in [
                M::Alt,
                M::Control,
                M::Shift,
                M::Meta,
                M::CapsLock,
                M::NumLock,
                M::ScrollLock,
                M::Function,
                M::Symbol,
            ] {
                let expected = if key == modifier {
                    Some(side)
                } else if key == M::CapsLock && combined & W::CAPS_LOCK != 0 {
                    Some(S::All)
                } else {
                    None
                };
                assert_eq!(data.pressed_modifier_side(key), expected);
                assert_eq!(data.is_modifier_pressed(key, side), expected.is_some());
            }
        }
    }
    let shift = packet(0, 0, 0, W::SHIFT | W::RIGHT_SHIFT);
    assert!(!shift.is_modifier_pressed(M::Shift, S::Left));
    assert!(shift.is_modifier_pressed(M::Shift, S::Right));
}

#[test]
fn windows_known_scan_codes_do_not_depend_on_the_virtual_key_or_label() {
    for (data, physical, logical, label, code, portable) in [
        (
            packet(0, 0x1e, 97, 0),
            P::KEY_A,
            L::KEY_A,
            Some('a'),
            Code::KeyA,
            KeyboardKey::Character("a".into()),
        ),
        (
            packet(27, 0x01, 0, 0),
            P::ESCAPE,
            L::ESCAPE,
            None,
            Code::Escape,
            KeyboardKey::Named(NamedKey::Escape),
        ),
        (
            packet(160, 0x2a, 0, W::LEFT_SHIFT),
            P::SHIFT_LEFT,
            L::SHIFT_LEFT,
            None,
            Code::ShiftLeft,
            KeyboardKey::Named(NamedKey::Shift),
        ),
        (
            packet(37, 0xe04b, 0, 0),
            P::ARROW_LEFT,
            L::ARROW_LEFT,
            None,
            Code::ArrowLeft,
            KeyboardKey::Named(NamedKey::ArrowLeft),
        ),
    ] {
        assert_eq!(
            data.flutter_physical_key_id(),
            u64::from(physical.usb_hid_usage())
        );
        assert_eq!(data.flutter_logical_key(), logical);
        assert_eq!(data.key_label(), label);
        let event = data.into_event(KeyState::Down, false);
        assert_eq!(event.logical_key(), &portable);
        assert_eq!(event.physical_key(), code);
        assert_eq!(
            event.character().as_deref(),
            label.map(|ch| ch.to_string()).as_deref()
        );
    }
}

#[test]
fn numpad_location_takes_precedence_and_control_unicode_uses_virtual_key() {
    let two = packet(98, 0x50, '2'.into(), 0);
    assert_eq!(two.flutter_logical_key(), L::NUMPAD2);
    let event = two.into_event(KeyState::Down, false);
    assert_eq!(event.portable().location, Location::Numpad);
    assert_eq!(event.character().as_deref(), Some("2"));
    assert_eq!(two.into_event(KeyState::Up, false).character(), None);

    let enter = packet(13, 0x1c, 13, 0);
    assert_eq!(enter.flutter_logical_key(), L::ENTER);
    assert_eq!(enter.key_label(), Some('\r'));

    let unknown = packet(0xfffe, 0xf111, 0, 0);
    assert_eq!(unknown.flutter_physical_key_id(), L::WINDOWS_PLANE + 0xf111);
    assert_eq!(
        unknown.flutter_logical_key().key_id(),
        L::WINDOWS_PLANE | 0xfffe
    );
}

#[test]
fn windows_packet_equality_and_diagnostics_keep_the_wire_fields() {
    let data = packet(16, 0x1e, 10, 0x20);
    assert_eq!(
        data.into_event(KeyState::Down, false).data(),
        Some(&RawKeyPlatformData::Windows(data))
    );
    assert_ne!(data, W::default());
    let debug = format!("{data:?}");
    for field in [
        "key_code: 16",
        "scan_code: 30",
        "character_code_point: 10",
        "modifiers: 32",
    ] {
        assert!(debug.contains(field), "{debug}");
    }
}
