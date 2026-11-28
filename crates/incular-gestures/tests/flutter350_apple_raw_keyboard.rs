use incular_core::{
    Code, KeyState, KeyboardKey, LogicalKeyboardKey as L, NamedKey, PhysicalKeyboardKey as P,
};
use incular_gestures::{
    AppleKeyPlatform::{self, Ios, MacOs},
    AppleRawKeyData as A, RawKeyboardModifier as M, RawKeyboardSide as S,
};

const MODIFIERS: [M; 9] = [
    M::Alt,
    M::Shift,
    M::Control,
    M::Meta,
    M::CapsLock,
    M::NumLock,
    M::ScrollLock,
    M::Function,
    M::Symbol,
];
const SIDED: [(M, u32, u32, u32); 4] = [
    (M::Alt, 0x80000, 0x20, 0x40),
    (M::Shift, 0x20000, 2, 4),
    (M::Control, 0x40000, 1, 0x2000),
    (M::Meta, 0x100000, 8, 16),
];

fn packet(
    platform: AppleKeyPlatform,
    code: u32,
    character: Option<&str>,
    label: &str,
    mask: u32,
) -> A {
    A {
        platform,
        key_code: code,
        characters: character.map(str::to_owned),
        characters_ignoring_modifiers: label.to_owned(),
        modifiers: mask,
    }
}

#[test]
fn individual_apple_masks_map_both_sides_without_claiming_unrelated_modifiers() {
    for platform in [AppleKeyPlatform::MAC_OS, Ios] {
        for (tested, generic, left, right) in SIDED {
            for (mask, pressed, expected_side) in [
                (generic | left, true, S::Left),
                (generic | right, true, S::Right),
                (generic, true, S::All),
                (generic | left | right, true, S::All),
                (left | right, false, S::All),
                (left, false, S::Left),
                (right, false, S::Right),
            ] {
                let data = packet(platform, 0, Some("a"), "a", mask);
                for modifier in MODIFIERS {
                    let expected = modifier == tested && pressed;
                    assert_eq!(data.is_modifier_pressed(modifier, S::Any), expected);
                    if modifier == tested {
                        assert_eq!(data.modifier_side(modifier), Some(expected_side));
                        assert_eq!(
                            data.is_modifier_pressed(modifier, S::Left),
                            expected && expected_side != S::Right
                        );
                        assert_eq!(
                            data.is_modifier_pressed(modifier, S::Right),
                            expected && expected_side != S::Left
                        );
                        assert_eq!(
                            data.is_modifier_pressed(modifier, S::All),
                            expected && expected_side == S::All
                        );
                    }
                }
            }
        }
        let data = packet(platform, 0, Some("a"), "a", 0x10000 | 0x200000 | 0x800000);
        for modifier in MODIFIERS {
            assert_eq!(
                data.is_modifier_pressed(modifier, S::All),
                modifier == M::CapsLock
            );
        }
    }
}

#[test]
fn apple_masks_keep_caps_lock_and_a_sided_modifier_active_together() {
    for platform in [AppleKeyPlatform::MAC_OS, Ios] {
        for (tested, generic, left, right) in SIDED {
            for (bits, expected_side) in [(left, S::Left), (right, S::Right), (0, S::All)] {
                let data = packet(platform, 0, Some("A"), "a", generic | bits | 0x10000);
                for modifier in MODIFIERS {
                    let active = modifier == tested || modifier == M::CapsLock;
                    assert_eq!(data.is_modifier_pressed(modifier, S::Any), active);
                    if active {
                        let side = if modifier == tested {
                            expected_side
                        } else {
                            S::All
                        };
                        assert!(data.is_modifier_pressed(modifier, side));
                        assert_eq!(data.modifier_side(modifier), Some(side));
                    }
                }
            }
        }
    }
}

#[test]
fn macos_layout_label_and_character_survive_case_and_override_independently() {
    for (data, physical, logical, character) in [
        (
            packet(AppleKeyPlatform::MAC_OS, 0, Some("a"), "a", 0),
            P::KEY_A,
            L::KEY_A,
            "a",
        ),
        (
            packet(AppleKeyPlatform::MAC_OS, 0, Some("A"), "A", 0x20002),
            P::KEY_A,
            L::KEY_A,
            "A",
        ),
        (
            packet(AppleKeyPlatform::MAC_OS, 0, Some("án"), "án", 0),
            P::KEY_A,
            L::new(0x1400000000),
            "án",
        ),
        (
            packet(
                MacOs {
                    specified_logical_key: Some(0x31),
                },
                0x12,
                Some("&"),
                "&",
                0,
            ),
            P::DIGIT1,
            L::DIGIT1,
            "&",
        ),
    ] {
        assert_eq!(
            data.flutter_physical_key_id(),
            u64::from(physical.usb_hid_usage())
        );
        assert_eq!(data.flutter_logical_key(), logical);
        assert_eq!(data.key_label(), character);
        assert_eq!(
            data.into_event(KeyState::Down, false)
                .character()
                .as_deref(),
            Some(character)
        );
    }
}

#[test]
fn apple_native_function_codes_do_not_become_printable() {
    for (platform, escape, shift, arrow, label) in [
        (AppleKeyPlatform::MAC_OS, 0x35, 0x38, 0x7b, "\u{f702}"),
        (Ios, 0x29, 0xe1, 0x50, "UIKeyInputLeftArrow"),
    ] {
        for (code, mask, label, physical, logical, named) in [
            (escape, 0, "", P::ESCAPE, L::ESCAPE, NamedKey::Escape),
            (shift, 2, "", P::SHIFT_LEFT, L::SHIFT_LEFT, NamedKey::Shift),
            (
                arrow,
                0x800000,
                label,
                P::ARROW_LEFT,
                L::ARROW_LEFT,
                NamedKey::ArrowLeft,
            ),
        ] {
            let data = packet(platform, code, Some(""), label, mask);
            assert_eq!(
                data.flutter_physical_key_id(),
                u64::from(physical.usb_hid_usage())
            );
            assert_eq!(data.flutter_logical_key(), logical);
            assert_eq!(data.key_label(), label);
            assert!(!data.is_modifier_pressed(M::Function, S::Any));
            let event = data.into_event(KeyState::Down, false);
            assert_eq!(event.logical_key(), &KeyboardKey::Named(named));
            assert_eq!(
                event.character().as_deref(),
                platform.is_macos().then_some("")
            );
        }
    }
}

#[test]
fn ios_printable_label_and_raw_character_are_distinct() {
    let data = packet(Ios, 4, Some("a"), "a", 0);
    assert_eq!(
        data.flutter_physical_key_id(),
        u64::from(P::KEY_A.usb_hid_usage())
    );
    assert_eq!(data.flutter_logical_key(), L::KEY_A);
    assert_eq!(data.key_label(), "a");
    assert_eq!(
        data.into_event(KeyState::Down, false).physical_key(),
        Code::KeyA
    );

    let astral = packet(Ios, 4, Some("😊"), "😊", 0);
    assert_eq!(astral.flutter_logical_key(), L::new(0xd83d_de0a));
    assert_eq!(
        astral.clone().into_event(KeyState::Up, false).character(),
        None
    );
    assert_eq!(
        astral
            .into_event(KeyState::Down, false)
            .character()
            .as_deref(),
        Some("😊")
    );
    let special = packet(Ios, 0x50, None, "UIKeyInputEnd", 0);
    assert_eq!(special.flutter_logical_key(), L::ENTER);
}

#[test]
fn apple_packet_equality_uses_packet_values_not_the_layout_override() {
    for (platform, code) in [(AppleKeyPlatform::MAC_OS, 96), (Ios, 4)] {
        let data = packet(platform, code, Some("A"), "a", 0x10);
        assert_eq!(data, packet(platform, code, Some("A"), "a", 0x10));
        assert_ne!(data, packet(platform, 0, Some(""), "", 0));
        assert_ne!(data, packet(platform, code, Some("A"), "b", 0x10));
    }
    assert_eq!(
        packet(AppleKeyPlatform::MAC_OS, 0, None, "a", 0),
        packet(
            MacOs {
                specified_logical_key: Some(0x31)
            },
            0,
            Some(""),
            "a",
            0
        ),
    );
}
