use incular_core::{KeyState, LogicalKeyboardKey as L};
use incular_runtime::{KeyMessage, Runtime};
use incular_widgets::{
    AppleKeyPlatform::{self, Ios, MacOs},
    AppleRawKeyData as A, RawKeyPlatformData, SizedBox,
};
use std::{cell::RefCell, rc::Rc};

fn packet(platform: AppleKeyPlatform, code: u32, characters: &str, mask: u32) -> A {
    A {
        platform,
        key_code: code,
        characters: Some(characters.to_owned()),
        characters_ignoring_modifiers: characters.to_owned(),
        modifiers: mask,
    }
}

fn send(runtime: &mut Runtime, packet: A, state: KeyState) -> bool {
    runtime.dispatch_key_message(KeyMessage::Raw(packet.into_event(state, false)))
}

fn check(runtime: &Runtime, expected: &[L]) {
    assert_eq!(
        runtime.legacy_raw_keyboard().logical_keys_pressed(),
        expected.iter().copied().collect()
    );
}

#[test]
fn generic_apple_mask_marks_each_side_and_releases_them() {
    for (platform, a) in [(AppleKeyPlatform::MAC_OS, 0), (Ios, 4)] {
        let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
        check(&runtime, &[]);
        send(
            &mut runtime,
            packet(platform, a, "a", 0x1e0000),
            KeyState::Down,
        );
        check(
            &runtime,
            &[
                L::KEY_A,
                L::SHIFT_LEFT,
                L::SHIFT_RIGHT,
                L::ALT_LEFT,
                L::ALT_RIGHT,
                L::CONTROL_LEFT,
                L::CONTROL_RIGHT,
                L::META_LEFT,
                L::META_RIGHT,
            ],
        );
        send(&mut runtime, packet(platform, a, "a", 0), KeyState::Up);
        check(&runtime, &[]);
    }
}

#[test]
fn sided_apple_snapshot_adds_only_the_reported_left_shift() {
    for (platform, a) in [(AppleKeyPlatform::MAC_OS, 0), (Ios, 4)] {
        let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
        check(&runtime, &[]);
        send(
            &mut runtime,
            packet(platform, a, "a", 0x20002),
            KeyState::Down,
        );
        check(&runtime, &[L::KEY_A, L::SHIFT_LEFT]);
        send(&mut runtime, packet(platform, a, "a", 0), KeyState::Up);
        check(&runtime, &[]);
    }
}

#[test]
fn ios_publishes_caps_lock_down_even_when_the_mask_omits_it() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let capture = Rc::clone(&seen);
    let _subscription = runtime.add_raw_keyboard_listener(move |event| {
        let Some(RawKeyPlatformData::Apple(data)) = event.data() else {
            panic!("expected Apple packet")
        };
        capture.borrow_mut().push((
            data.flutter_physical_key_id(),
            data.flutter_logical_key(),
            event.is_key_down(),
        ));
    });
    send(&mut runtime, packet(Ios, 0x39, "", 0), KeyState::Down);
    check(&runtime, &[L::CAPS_LOCK]);
    assert_eq!(&*seen.borrow(), &[(0x70039, L::CAPS_LOCK, true)]);
    send(&mut runtime, packet(Ios, 0x39, "", 0), KeyState::Up);
    check(&runtime, &[]);
    assert_eq!(seen.borrow().len(), 2);
}

#[test]
fn macos_fn_down_and_physical_release_are_filtered_without_suppressing_f12() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let capture = Rc::clone(&seen);
    let _subscription = runtime.add_raw_keyboard_listener(move |event| {
        let Some(RawKeyPlatformData::Apple(data)) = event.data() else {
            panic!("expected Apple packet")
        };
        capture
            .borrow_mut()
            .push((data.flutter_logical_key(), event.is_key_down()));
    });
    assert!(send(
        &mut runtime,
        packet(AppleKeyPlatform::MAC_OS, 0x3f, "", 0x800000),
        KeyState::Down
    ));
    check(&runtime, &[]);
    send(
        &mut runtime,
        packet(AppleKeyPlatform::MAC_OS, 0x6f, "", 0x800000),
        KeyState::Down,
    );
    check(&runtime, &[L::F12]);
    // Flutter balances suppression by physical identity, not by release's logical override.
    let changed = MacOs {
        specified_logical_key: Some(L::KEY_A.key_id()),
    };
    assert!(send(
        &mut runtime,
        packet(changed, 0x3f, "", 0),
        KeyState::Up
    ));
    check(&runtime, &[L::F12]);
    send(
        &mut runtime,
        packet(AppleKeyPlatform::MAC_OS, 0x6f, "", 0),
        KeyState::Up,
    );
    check(&runtime, &[]);
    assert_eq!(&*seen.borrow(), &[(L::F12, true), (L::F12, false)]);
}
