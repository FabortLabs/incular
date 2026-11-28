use incular_core::{
    Code, KeyState, KeyboardKey, LogicalKeyboardKey as L, NamedKey, PhysicalKeyboardKey as P,
};
use incular_gestures::{
    RawKeyPlatformData, RawKeyboardModifier as M, RawKeyboardSide, WebRawKeyData as Data,
};

fn packet(code: &str, key: &str, location: u32, meta_state: u32, key_code: u32) -> Data {
    Data {
        code: code.into(),
        key: key.into(),
        location,
        meta_state,
        key_code,
    }
}

const MASKS: [(u32, M); 7] = [
    (Data::ALT, M::Alt),
    (Data::SHIFT, M::Shift),
    (Data::CONTROL, M::Control),
    (Data::META, M::Meta),
    (Data::CAPS_LOCK, M::CapsLock),
    (Data::NUM_LOCK, M::NumLock),
    (Data::SCROLL_LOCK, M::ScrollLock),
];

#[test]
fn web_known_modifier_flags_stay_independent_even_when_combined() {
    for extra_meta in [false, true] {
        for (requested, expected) in MASKS {
            if extra_meta && expected == M::Meta {
                continue;
            }
            let raw = packet(
                "RandomCode",
                "RandomKey",
                0,
                requested | if extra_meta { Data::META } else { 0 },
                0,
            );
            for actual in [
                M::Alt,
                M::Shift,
                M::Control,
                M::Meta,
                M::CapsLock,
                M::NumLock,
                M::ScrollLock,
                M::Function,
                M::Symbol,
            ] {
                let supposed = actual == expected || (extra_meta && actual == M::Meta);
                assert_eq!(
                    raw.is_modifier_pressed(actual, RawKeyboardSide::Any),
                    supposed,
                    "{actual:?}"
                );
            }
            assert_eq!(
                raw.clone().into_event(KeyState::Down, false, false).data(),
                Some(&RawKeyPlatformData::Web(raw))
            );
        }
    }
}

#[test]
fn missing_modifier_location_uses_the_left_id_and_numpad_keeps_its_own_id() {
    for (name, left, right) in [
        ("Alt", L::ALT_LEFT, L::ALT_RIGHT),
        ("Shift", L::SHIFT_LEFT, L::SHIFT_RIGHT),
        ("Control", L::CONTROL_LEFT, L::CONTROL_RIGHT),
        ("Meta", L::META_LEFT, L::META_RIGHT),
    ] {
        assert_eq!(packet("", name, 0, 0, 0).flutter_logical_key(), left);
        assert_eq!(packet("", name, 2, 0, 0).flutter_logical_key(), right);
    }
    assert_eq!(
        packet("Numpad2", "ArrowDown", 3, 0, 0).flutter_logical_key(),
        L::NUMPAD2
    );
}

#[test]
fn printable_web_labels_keep_case_but_logical_letter_ids_do_not() {
    for (label, flags) in [("a", 0), ("A", Data::SHIFT)] {
        let data = packet("KeyA", label, 0, flags, 0x41);
        assert_eq!(
            data.flutter_physical_key_id(),
            u64::from(P::KEY_A.usb_hid_usage())
        );
        assert_eq!(data.flutter_logical_key(), L::KEY_A);
        assert_eq!(data.key_label(), Some(label));
        let raw = data.into_event(KeyState::Down, false, false);
        let Some(RawKeyPlatformData::Web(original)) = raw.data() else {
            panic!("missing DOM payload");
        };
        assert_eq!(original.key_code, 0x41);
        assert_eq!(raw.logical_key(), &KeyboardKey::Character(label.into()));
        assert_eq!(raw.physical_key(), Code::KeyA);
    }
}

#[test]
fn named_and_legacy_web_key_spellings_keep_their_physical_and_logical_identity() {
    for (name, label, location, modifiers, keycode, physical, logical, portable, code) in [
        (
            "Escape",
            "Escape",
            0,
            0,
            0x1b,
            P::ESCAPE,
            L::ESCAPE,
            NamedKey::Escape,
            Code::Escape,
        ),
        (
            "Esc",
            "Esc",
            0,
            0,
            0x1b,
            P::ESCAPE,
            L::ESCAPE,
            NamedKey::Escape,
            Code::Escape,
        ),
        (
            "ShiftLeft",
            "Shift",
            1,
            Data::SHIFT,
            0x10,
            P::SHIFT_LEFT,
            L::SHIFT_LEFT,
            NamedKey::Shift,
            Code::ShiftLeft,
        ),
        (
            "ArrowDown",
            "ArrowDown",
            0,
            0,
            0x28,
            P::ARROW_DOWN,
            L::ARROW_DOWN,
            NamedKey::ArrowDown,
            Code::ArrowDown,
        ),
    ] {
        let data = packet(name, label, location, modifiers, keycode);
        assert_eq!(
            data.flutter_physical_key_id(),
            u64::from(physical.usb_hid_usage())
        );
        assert_eq!(data.flutter_logical_key(), logical);
        assert_eq!(data.key_label(), None);
        let raw = data.into_event(KeyState::Down, false, false);
        let Some(RawKeyPlatformData::Web(original)) = raw.data() else {
            panic!("missing DOM payload");
        };
        assert_eq!(original.key_code, keycode);
        assert_eq!(raw.logical_key(), &KeyboardKey::Named(portable));
        assert_eq!(raw.physical_key(), code);
    }
}

#[test]
fn unknown_dom_strings_are_minted_in_the_web_plane_from_the_physical_code() {
    let data = packet("Unrecog1", "Unrecog2", 0, 0, 0);
    let physical = data.flutter_physical_key_id();
    let logical = data.flutter_logical_key().key_id();
    for id in [physical, logical] {
        assert!(L::WEB_PLANE < id && id < L::GLFW_PLANE);
    }
    assert_eq!(data.key_label(), None);
    assert_eq!(data.key_code, 0);
    let unlabelled = packet("Unrecog1", "UnknownDOMKey", 0, 0, 0);
    assert_eq!(unlabelled.flutter_logical_key().key_id(), logical);
    assert_ne!(
        packet("other", "UnknownDOMKey", 0, 0, 0)
            .flutter_logical_key()
            .key_id(),
        logical
    );
}

#[test]
fn web_payload_equality_and_diagnostics_keep_all_five_fields() {
    let data = packet("KeyA", "a", 2, 0x10, 0x41);
    let raw = data.clone().into_event(KeyState::Down, false, false);
    assert_eq!(raw.data(), Some(&RawKeyPlatformData::Web(data.clone())));
    assert_ne!(
        raw.data(),
        Some(&RawKeyPlatformData::Web(packet("KeyA", "a", 0, 0, 0)))
    );
    let diagnostic = format!("{data:?}");
    for field in [
        "code: \"KeyA\"",
        "key: \"a\"",
        "location: 2",
        "meta_state: 16",
        "key_code: 65",
    ] {
        assert!(diagnostic.contains(field));
    }
}
