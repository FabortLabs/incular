use incular_core::{
    Code, KeyState, KeyboardKey, LogicalKeyboardKey, NamedKey, PhysicalKeyboardKey,
};
use incular_gestures::{
    FuchsiaRawKeyData as Data, RawKeyPlatformData, RawKeyboardModifier as M, RawKeyboardSide as S,
};

const MODIFIERS: [M; 9] = [
    M::Alt,
    M::Control,
    M::Shift,
    M::Meta,
    M::CapsLock,
    M::NumLock,
    M::ScrollLock,
    M::Function,
    M::Symbol,
];
const CASES: [(u32, M, S); 13] = [
    (Data::LEFT_ALT | Data::RIGHT_ALT, M::Alt, S::Any),
    (Data::LEFT_ALT, M::Alt, S::Left),
    (Data::RIGHT_ALT, M::Alt, S::Right),
    (Data::LEFT_SHIFT | Data::RIGHT_SHIFT, M::Shift, S::Any),
    (Data::LEFT_SHIFT, M::Shift, S::Left),
    (Data::RIGHT_SHIFT, M::Shift, S::Right),
    (Data::LEFT_CONTROL | Data::RIGHT_CONTROL, M::Control, S::Any),
    (Data::LEFT_CONTROL, M::Control, S::Left),
    (Data::RIGHT_CONTROL, M::Control, S::Right),
    (Data::LEFT_META | Data::RIGHT_META, M::Meta, S::Any),
    (Data::LEFT_META, M::Meta, S::Left),
    (Data::RIGHT_META, M::Meta, S::Right),
    (Data::CAPS_LOCK, M::CapsLock, S::Any),
];

fn check_modifiers(combined: bool) {
    for (mask, expected, side) in CASES {
        if combined && expected == M::CapsLock {
            continue;
        }
        let modifiers = mask | if combined { Data::CAPS_LOCK } else { 0 };
        let packet = Data {
            hid_usage: 0x04,
            code_point: 0x64,
            modifiers,
        };
        let raw = packet.into_event(KeyState::Down, false);
        let Some(RawKeyPlatformData::Fuchsia(data)) = raw.data() else {
            panic!("missing raw packet")
        };
        assert_eq!(data.modifiers, modifiers);
        for modifier in MODIFIERS {
            let enabled = modifier == expected || (combined && modifier == M::CapsLock);
            assert_eq!(
                data.is_modifier_pressed(modifier, side),
                enabled,
                "{mask:#x} {modifier:?} {side:?}"
            );
        }
        if side == S::Left {
            assert!(!data.is_modifier_pressed(expected, S::Right));
        } else if side == S::Right {
            assert!(!data.is_modifier_pressed(expected, S::Left));
        }
    }
}

#[test]
fn individual_fuchsia_modifier_masks_preserve_identity_and_side() {
    check_modifiers(false);
}

#[test]
fn fuchsia_caps_lock_combines_without_synthesizing_unrelated_modifiers() {
    check_modifiers(true);
}

fn matches_key(
    data: Data,
    code: Code,
    physical: PhysicalKeyboardKey,
    logical: LogicalKeyboardKey,
    portable: KeyboardKey,
    label: Option<char>,
) {
    let raw = data.into_event(KeyState::Down, false);
    assert_eq!(raw.data(), Some(&RawKeyPlatformData::Fuchsia(data)));
    assert_eq!(
        data.flutter_physical_key_id(),
        u64::from(physical.usb_hid_usage())
    );
    assert_eq!(data.flutter_logical_key(), logical);
    assert_eq!(raw.physical_key(), code);
    assert_eq!(raw.logical_key(), &portable);
    assert_eq!(data.key_label(), label);
}

#[test]
fn printable_fuchsia_keys_keep_the_code_point_label_and_usb_identity() {
    matches_key(
        Data {
            hid_usage: 0x00070004,
            code_point: 'a' as u32,
            modifiers: 0,
        },
        Code::KeyA,
        PhysicalKeyboardKey::KEY_A,
        LogicalKeyboardKey::KEY_A,
        KeyboardKey::Character("a".into()),
        Some('a'),
    );
}

#[test]
fn fuchsia_escape_remains_a_named_control_without_a_text_label() {
    matches_key(
        Data {
            hid_usage: 0x00070029,
            ..Data::default()
        },
        Code::Escape,
        PhysicalKeyboardKey::ESCAPE,
        LogicalKeyboardKey::ESCAPE,
        KeyboardKey::Named(NamedKey::Escape),
        None,
    );
}

#[test]
fn fuchsia_left_shift_keeps_sided_logical_and_physical_ids() {
    matches_key(
        Data {
            hid_usage: 0x000700e1,
            ..Data::default()
        },
        Code::ShiftLeft,
        PhysicalKeyboardKey::SHIFT_LEFT,
        LogicalKeyboardKey::SHIFT_LEFT,
        KeyboardKey::Named(NamedKey::Shift),
        None,
    );
}

#[test]
fn fuchsia_raw_value_equality_preserves_every_platform_field() {
    let data = Data {
        hid_usage: 0x00070004,
        code_point: 97,
        modifiers: 0x10,
    };
    let event = data.into_event(KeyState::Down, false);
    assert_eq!(event.data(), Some(&RawKeyPlatformData::Fuchsia(data)));
    assert_ne!(
        event.data(),
        Some(&RawKeyPlatformData::Fuchsia(Data::default()))
    );
    let diagnostic = format!("{data:?}");
    for field in ["hid_usage: 458756", "code_point: 97", "modifiers: 16"] {
        assert!(diagnostic.contains(field));
    }
}
