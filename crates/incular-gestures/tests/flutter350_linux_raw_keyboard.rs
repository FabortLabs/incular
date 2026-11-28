use incular_core::{
    Code, KeyboardKey, LogicalKeyboardKey as L, NamedKey, PhysicalKeyboardKey as P,
};
use incular_gestures::{
    LinuxKeyToolkit::{self, Glfw, Gtk},
    LinuxRawKeyData as Packet, RawKeyPlatformData, RawKeyboardModifier as M, RawKeyboardSide as S,
};

const MODIFIERS: [M; 9] = [
    M::Shift,
    M::Control,
    M::Alt,
    M::Meta,
    M::CapsLock,
    M::NumLock,
    M::ScrollLock,
    M::Function,
    M::Symbol,
];
const GTK_KEYS: [(M, u32, [u32; 2]); 6] = [
    (M::Shift, 1, [0xffe1, 0xffe2]),
    (M::Control, 4, [0xffe3, 0xffe4]),
    (M::Alt, 8, [0xffe9, 0xffea]),
    (M::Meta, 1 << 26, [0xffeb, 0xffec]),
    (M::CapsLock, 2, [0xffe5, 0xffe6]),
    (M::NumLock, 16, [0xff7f, 0xff7f]),
];
const GLFW_KEYS: [(M, u32, [u32; 2]); 6] = [
    (M::Shift, 1, [340, 344]),
    (M::Control, 2, [341, 345]),
    (M::Alt, 4, [342, 346]),
    (M::Meta, 8, [343, 347]),
    (M::CapsLock, 16, [280, 280]),
    (M::NumLock, 32, [282, 282]),
];

fn packet(toolkit: LinuxKeyToolkit, scan_code: u32, key_code: u32, scalar: u32) -> Packet {
    Packet {
        toolkit,
        scan_code,
        key_code,
        unicode_scalar_values: scalar,
        is_down: true,
        ..Packet::default()
    }
}

#[test]
fn each_toolkit_merges_both_keys_actual_down_and_up_into_the_prior_mask() {
    for (toolkit, keys) in [(Gtk, GTK_KEYS), (Glfw, GLFW_KEYS)] {
        for (modifier, flag, codes) in keys {
            for key_code in codes {
                for is_down in [true, false] {
                    let data = Packet {
                        modifiers: if is_down { 0 } else { flag },
                        is_down,
                        ..packet(toolkit, 38, key_code, 97)
                    };
                    let raw = data.into_event(false);
                    assert_eq!(raw.is_key_down(), is_down);
                    assert_eq!(raw.data(), Some(&RawKeyPlatformData::Linux(data)));
                    for other in MODIFIERS {
                        assert_eq!(data.modifier_side(other), S::All);
                        for side in [S::Any, S::All, S::Left, S::Right] {
                            assert_eq!(
                                data.is_modifier_pressed(other, side),
                                is_down && other == modifier,
                                "{toolkit:?}, code {key_code:#x}, {other:?}, {side:?}, down={is_down}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn combined_mask_survives_a_non_modifier_key_packet() {
    for (toolkit, keys) in [(Gtk, GTK_KEYS), (Glfw, GLFW_KEYS)] {
        for (modifier, flag, _) in keys {
            if modifier == M::Control {
                continue;
            }
            let data = Packet {
                modifiers: flag | keys[1].1,
                ..packet(toolkit, 38, 65, 97)
            };
            for other in MODIFIERS {
                assert_eq!(data.modifier_side(other), S::All);
                assert_eq!(
                    data.is_modifier_pressed(other, S::All),
                    other == modifier || other == M::Control
                );
            }
        }
    }
}

#[test]
fn xkb_position_and_layout_character_are_independent_in_both_toolkits() {
    for toolkit in [Gtk, Glfw] {
        for (scalar, expected) in [(113, 'q'), (0x10ffff, '\u{10ffff}')] {
            let data = packet(toolkit, 38, 65, scalar);
            assert_eq!(
                data.flutter_physical_key_id(),
                u64::from(P::KEY_A.usb_hid_usage())
            );
            assert_eq!(data.flutter_logical_key(), L::new(u64::from(scalar)));
            assert_eq!(data.key_label(), Some(expected));
            let raw = data.into_event(false);
            assert_eq!(raw.physical_key(), Code::KeyA);
            assert_eq!(
                raw.character().as_deref(),
                Some(expected.to_string().as_str())
            );
        }
    }
}

#[test]
fn toolkit_specific_escape_and_shift_keep_xkb_positions_without_labels() {
    for (toolkit, escape, shift) in [(Gtk, 65307, 65505), (Glfw, 256, 340)] {
        for (scan, code, physical, logical, portable) in [
            (9, escape, P::ESCAPE, L::ESCAPE, NamedKey::Escape),
            (50, shift, P::SHIFT_LEFT, L::SHIFT_LEFT, NamedKey::Shift),
        ] {
            let data = packet(toolkit, scan, code, 0);
            assert_eq!(
                data.flutter_physical_key_id(),
                u64::from(physical.usb_hid_usage())
            );
            assert_eq!(data.flutter_logical_key(), logical);
            assert_eq!(data.key_label(), None);
            assert_eq!(
                data.into_event(false).logical_key(),
                &KeyboardKey::Named(portable)
            );
        }
    }
}

#[test]
fn override_precedes_unicode_without_replacing_raw_character() {
    let data = Packet {
        specified_logical_key: Some(0x61),
        ..packet(Gtk, 0x26, 0x6c6, 0x424)
    };
    assert_eq!(
        data.flutter_physical_key_id(),
        u64::from(P::KEY_A.usb_hid_usage())
    );
    assert_eq!(data.flutter_logical_key(), L::KEY_A);
    let raw = data.into_event(false);
    assert_eq!(raw.logical_key(), &KeyboardKey::Character("a".to_owned()));
    assert_eq!(raw.character().as_deref(), Some("Ф"));
    assert_eq!(
        Packet {
            is_down: false,
            ..data
        }
        .into_event(false)
        .character(),
        None
    );
}

#[test]
fn numpad_precedes_unicode_and_unknown_id_uses_each_toolkit_plane() {
    for (toolkit, numpad_one, plane) in [(Gtk, 0xffb1, L::GTK_PLANE), (Glfw, 321, L::GLFW_PLANE)] {
        let data = packet(toolkit, 87, numpad_one, 49);
        assert_eq!(data.flutter_logical_key(), L::NUMPAD1);
        assert_eq!(data.into_event(false).character().as_deref(), Some("1"));
        let unknown = packet(toolkit, 0xffff, 0x10000000, 0);
        assert_eq!(unknown.flutter_physical_key_id(), L::WEB_PLANE + 0xffff);
        assert_eq!(unknown.flutter_logical_key(), L::new(plane | 0x10000000));
    }
}
