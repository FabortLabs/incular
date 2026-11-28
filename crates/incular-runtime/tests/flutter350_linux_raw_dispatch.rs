use incular_core::{Code, LogicalKeyboardKey as L};
use incular_runtime::{KeyMessage, Runtime};
use incular_widgets::{
    LinuxKeyToolkit::{self, Glfw, Gtk},
    LinuxRawKeyData, SizedBox,
};

fn send(
    runtime: &mut Runtime,
    toolkit: LinuxKeyToolkit,
    is_down: bool,
    scan_code: u32,
    key_code: u32,
    modifiers: u32,
    unicode_scalar_values: u32,
) {
    let packet = LinuxRawKeyData {
        toolkit,
        is_down,
        scan_code,
        key_code,
        modifiers,
        unicode_scalar_values,
        ..LinuxRawKeyData::default()
    };
    runtime.dispatch_key_message(KeyMessage::Raw(packet.into_event(false)));
}

fn check(runtime: &Runtime, keys: &[L]) {
    assert_eq!(
        runtime.legacy_raw_keyboard().logical_keys_pressed(),
        keys.iter().copied().collect()
    );
}

#[test]
fn glfw_shift_snapshot_promotes_both_sides_and_release_clears_them() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    check(&runtime, &[]);
    send(&mut runtime, Glfw, true, 38, 65, 1, 97);
    check(&runtime, &[L::KEY_A, L::SHIFT_LEFT, L::SHIFT_RIGHT]);
    assert!(
        runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::ShiftLeft)
    );
    assert!(
        runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::ShiftRight)
    );
    send(&mut runtime, Glfw, false, 50, 340, 1, 0);
    check(&runtime, &[L::KEY_A]);
    assert!(
        !runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::ShiftLeft)
    );
    assert!(
        !runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::ShiftRight)
    );
    send(&mut runtime, Glfw, false, 38, 65, 0, 97);
    check(&runtime, &[]);
}

#[test]
fn gtk_group_switch_sequence_preserves_alt_right_when_its_bit_is_missing() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    check(&runtime, &[]);
    for (down, scan, key, modifiers) in [
        (true, 0x6c, 0xffea, 0x2000000),
        (true, 0x32, 0xfe08, 0x2000008),
        (false, 0x6c, 0xfe03, 0x2002008),
        (true, 0x6c, 0xfe03, 0x2002000),
    ] {
        send(&mut runtime, Gtk, down, scan, key, modifiers, 0);
    }
    check(&runtime, &[L::ALT_RIGHT]);
    assert_eq!(
        runtime.legacy_raw_keyboard().look_up_layout(0x700e6),
        Some(L::ALT_RIGHT)
    );
    send(&mut runtime, Gtk, false, 0x6c, 0xfe03, 0x2002000, 0);
    check(&runtime, &[]);
}

#[test]
fn gtk_caps_lock_mapped_to_backspace_survives_modifier_synchronization() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    check(&runtime, &[]);
    send(&mut runtime, Gtk, true, 0x42, 0xff08, 0x2000000, 0);
    check(&runtime, &[L::BACKSPACE]);
    assert_eq!(
        runtime.legacy_raw_keyboard().look_up_layout(0x70039),
        Some(L::BACKSPACE)
    );
    send(&mut runtime, Gtk, true, 38, 97, 0, 97);
    check(&runtime, &[L::BACKSPACE, L::KEY_A]);
    send(&mut runtime, Gtk, false, 0x42, 0xff08, 0, 0);
    check(&runtime, &[L::KEY_A]);
}
