use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TargetPlatform};
use incular_core::{
    Code, HardwareKeyOrigin as Origin, KeyState, LogicalKeyboardKey as L, PointerDeviceKind, Size,
};
use incular_runtime::{KeyMessage, Runtime};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{
    AndroidRawKeyData as A, EditableText, Focus, FocusHighlightManager, FocusHighlightMode,
    FocusHighlightStrategy, FocusNode, KeyEventResult, RawKeyPlatformData, SizedBox, TextDirection,
};

const KEYBOARD: u32 = 0x101;

fn a_mask(meta_state: u32) -> A {
    A {
        key_code: 29,
        scan_code: 30,
        plain_code_point: 'a'.into(),
        code_point: 'a'.into(),
        meta_state,
        event_source: KEYBOARD,
        device_id: 1,
        ..A::default()
    }
}

fn send(runtime: &mut Runtime, data: A, state: KeyState, character: Option<&str>) -> bool {
    runtime.dispatch_key_message(KeyMessage::Raw(
        data.into_event(state, character.map(str::to_owned)),
    ))
}

#[test]
fn android_raw_press_synchronizes_hardware_shift_before_its_primary_a() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    let events = Rc::new(RefCell::new(Vec::new()));
    let trace = events.clone();
    let _handler = runtime.add_hardware_key_handler_with_origin(move |event, origin| {
        trace.borrow_mut().push((event.code, event.state, origin));
        false
    });
    assert!(
        runtime
            .legacy_raw_keyboard()
            .logical_keys_pressed()
            .is_empty()
    );
    send(
        &mut runtime,
        a_mask(A::SHIFT | A::LEFT_SHIFT),
        KeyState::Down,
        Some("a"),
    );
    assert_eq!(
        runtime.legacy_raw_keyboard().logical_keys_pressed(),
        [L::KEY_A, L::SHIFT_LEFT].into()
    );
    assert_eq!(
        events.take(),
        [
            (
                Code::ShiftLeft,
                KeyState::Down,
                Origin::LegacyModifierReconciliation
            ),
            (Code::KeyA, KeyState::Down, Origin::Supplied),
        ]
    );
    send(&mut runtime, a_mask(0), KeyState::Up, None);
    assert_eq!(
        events.take(),
        [
            (
                Code::ShiftLeft,
                KeyState::Up,
                Origin::LegacyModifierReconciliation
            ),
            (Code::KeyA, KeyState::Up, Origin::Supplied),
        ]
    );
    assert!(
        runtime
            .legacy_raw_keyboard()
            .logical_keys_pressed()
            .is_empty()
    );
}

#[test]
fn android_generic_modifiers_sync_both_sides_without_fabricating_raw_packets() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    let count = Rc::new(RefCell::new(0));
    let captured = count.clone();
    let _raw = runtime.add_raw_keyboard_listener(move |_| *captured.borrow_mut() += 1);
    send(
        &mut runtime,
        a_mask(A::SHIFT | A::ALT | A::CONTROL | A::META),
        KeyState::Down,
        Some("a"),
    );
    assert_eq!(
        runtime.legacy_raw_keyboard().logical_keys_pressed(),
        [
            L::KEY_A,
            L::SHIFT_LEFT,
            L::SHIFT_RIGHT,
            L::ALT_LEFT,
            L::ALT_RIGHT,
            L::CONTROL_LEFT,
            L::CONTROL_RIGHT,
            L::META_LEFT,
            L::META_RIGHT,
        ]
        .into()
    );
    assert_eq!(*count.borrow(), 1);
}

#[test]
fn android_accepts_a_caps_lock_down_without_the_modifier_flag() {
    let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let captured = seen.clone();
    let _raw = runtime.add_raw_keyboard_listener(move |raw| {
        captured
            .borrow_mut()
            .push((raw.physical_key(), raw.logical_key().clone()));
    });
    let caps = A {
        key_code: 115,
        scan_code: 58,
        event_source: KEYBOARD,
        device_id: 1,
        ..A::default()
    };
    send(&mut runtime, caps, KeyState::Down, None);
    assert_eq!(seen.borrow().len(), 1);
    assert_eq!(seen.borrow()[0].0, Code::CapsLock);
    assert_eq!(
        runtime.legacy_raw_keyboard().logical_keys_pressed(),
        [L::CAPS_LOCK].into()
    );
}

#[test]
fn android_packet_response_changes_only_when_a_focused_raw_handler_accepts() {
    let focus = FocusNode::new();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let trace = seen.clone();
    let widget = Focus::new(SizedBox::shrink())
        .node(focus.clone())
        .on_key(move |_, raw| {
            let Some(RawKeyPlatformData::Android(data)) = raw.data() else {
                panic!("wrong platform")
            };
            trace.borrow_mut().push((raw.repeat(), data.repeat_count));
            KeyEventResult::Handled
        });
    let mut runtime = Runtime::new(widget.into()).unwrap();
    let bounds = Constraints::tight(Size::new(120., 100.));
    runtime.run_frame(bounds).unwrap();
    assert!(!send(&mut runtime, a_mask(0), KeyState::Down, Some("a")));
    assert!(seen.borrow().is_empty());
    focus.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(send(&mut runtime, a_mask(0), KeyState::Down, Some("a")));
    assert_eq!(seen.take(), [(true, 0)]);
}

#[test]
fn android_legacy_virtual_backspace_edits_without_leaving_touch_highlights() {
    let manager = FocusHighlightManager::new();
    let controller = TextEditingController::with_text("abc");
    let mut runtime = Runtime::new(
        EditableText::new(controller.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .size(Size::new(40., 40.))
            .into(),
    )
    .unwrap();
    Runtime::set_focus_highlight_platform_default(TargetPlatform::Android);
    manager.set_strategy(FocusHighlightStrategy::Automatic);
    runtime
        .run_frame(Constraints::tight(Size::new(40., 40.)))
        .unwrap();
    manager.note_pointer_input(PointerDeviceKind::Touch);
    controller.set_selection(TextSelection::collapsed(3));
    assert_eq!(manager.mode(), FocusHighlightMode::Touch);
    for (flags, device, expected) in [(2, -1, "ab"), (2, 5, "a"), (0, -1, "")] {
        let packet = A {
            flags,
            device_id: device,
            key_code: 67,
            scan_code: 14,
            event_source: KEYBOARD,
            ..A::default()
        };
        send(&mut runtime, packet, KeyState::Down, None);
        send(&mut runtime, packet, KeyState::Up, None);
        assert_eq!(controller.text(), expected);
        assert_eq!(manager.mode(), FocusHighlightMode::Touch);
    }
    send(&mut runtime, a_mask(0), KeyState::Down, Some("a"));
    assert_eq!(manager.mode(), FocusHighlightMode::Traditional);
}
