//! Paired Flutter engine packets use the real hardware and focus routes.
use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{
    Code, HardwareKeyOrigin, HardwareKeySubscription, KeyState, KeyboardEvent, KeyboardKey,
    NamedKey, Size,
};
use incular_runtime::{
    FlutterEngineKeyData as Data, FlutterEngineKeyEvent, FlutterKeyTransit, FlutterKeyTransitMode,
    KeyMessage, RawKeyboardSubscription, Runtime,
};
use incular_widgets::{Focus, KeyEventResult, RawKeyEvent, SizedBox, WebRawKeyData};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Raw(Code, KeyState),
    Hardware(Code, KeyState, HardwareKeyOrigin),
    FocusTyped(Code, KeyState),
    FocusRaw(Code, KeyState),
}

struct Scene {
    runtime: Runtime,
    transit: FlutterKeyTransit,
    log: Rc<RefCell<Vec<Event>>>,
    _hardware: HardwareKeySubscription,
    _raw: RawKeyboardSubscription,
}

impl Scene {
    fn new() -> Self {
        let log = Rc::new(RefCell::new(Vec::new()));
        let root = Focus::new(SizedBox::shrink())
            .autofocus(true)
            .on_key_event({
                let log = log.clone();
                move |_, event| {
                    log.borrow_mut()
                        .push(Event::FocusTyped(event.code, event.state));
                    KeyEventResult::Ignored
                }
            })
            .on_key({
                let log = log.clone();
                move |_, event| {
                    log.borrow_mut().push(Event::FocusRaw(
                        event.physical_key(),
                        event.portable().state,
                    ));
                    KeyEventResult::Ignored
                }
            });
        let mut runtime = Runtime::new(root.into()).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(70., 70.)))
            .unwrap();
        assert!(runtime.focused_element().is_some());
        let hardware = runtime.add_hardware_key_handler_with_origin({
            let log = log.clone();
            move |event, origin| {
                log.borrow_mut()
                    .push(Event::Hardware(event.code, event.state, origin));
                false
            }
        });
        let raw = runtime.add_raw_keyboard_listener({
            let log = log.clone();
            move |event| {
                log.borrow_mut()
                    .push(Event::Raw(event.physical_key(), event.portable().state));
            }
        });
        Self {
            runtime,
            transit: FlutterKeyTransit::default(),
            log,
            _hardware: hardware,
            _raw: raw,
        }
    }

    fn key_data(&mut self, data: Data) -> bool {
        let runtime = &mut self.runtime;
        self.transit
            .handle_key_data(data, |message| runtime.dispatch_key_message(message))
    }

    fn raw(&mut self, raw: RawKeyEvent) -> bool {
        let runtime = &mut self.runtime;
        self.transit
            .handle_raw_key(raw, |message| runtime.dispatch_key_message(message))
    }
}

fn typed(event: KeyboardEvent, synthesized: bool) -> Data {
    Data::Event(FlutterEngineKeyEvent::new(event, synthesized))
}

fn raw_a() -> RawKeyEvent {
    WebRawKeyData {
        code: "KeyA".into(),
        key: "a".into(),
        key_code: 65,
        ..Default::default()
    }
    .into_event(KeyState::Down, false, false)
}

#[test]
fn a_solitary_synthesized_packet_reaches_hardware_and_the_focus_listener_immediately() {
    let mut scene = Scene::new();
    assert!(!scene.key_data(typed(
        KeyboardEvent::key_down(KeyboardKey::Character("a".into()), Code::KeyA),
        true,
    )));
    assert_eq!(
        scene.transit.mode(),
        Some(FlutterKeyTransitMode::KeyDataThenRaw)
    );
    assert_eq!(
        scene.log.take(),
        [
            Event::Hardware(
                Code::KeyA,
                KeyState::Down,
                HardwareKeyOrigin::EngineSynthesized
            ),
            Event::FocusTyped(Code::KeyA, KeyState::Down),
        ]
    );
    assert!(
        scene
            .runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::KeyA)
    );
}

#[test]
fn once_a_press_waits_for_raw_data_its_synthesized_release_waits_in_the_same_batch() {
    let mut scene = Scene::new();
    let caps = KeyboardKey::Named(NamedKey::CapsLock);
    let down = KeyboardEvent::key_down(caps.clone(), Code::CapsLock);
    let up = KeyboardEvent::key_up(caps, Code::CapsLock);
    assert!(!scene.key_data(typed(down.clone(), false)));
    assert!(scene.log.borrow().is_empty());
    assert!(!scene.key_data(typed(up, true)));
    assert!(scene.log.borrow().is_empty());
    assert!(
        !scene
            .runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::CapsLock)
    );
    assert!(!scene.raw(RawKeyEvent::from(down)));
    assert_eq!(
        scene.log.take(),
        [
            Event::Raw(Code::CapsLock, KeyState::Down),
            Event::Hardware(Code::CapsLock, KeyState::Down, HardwareKeyOrigin::Supplied),
            Event::Hardware(
                Code::CapsLock,
                KeyState::Up,
                HardwareKeyOrigin::EngineSynthesized
            ),
            Event::FocusTyped(Code::CapsLock, KeyState::Down),
            Event::FocusTyped(Code::CapsLock, KeyState::Up),
            Event::FocusRaw(Code::CapsLock, KeyState::Down),
        ]
    );
    assert!(
        !scene
            .runtime
            .hardware_keyboard()
            .is_physical_key_pressed(Code::CapsLock)
    );
}

#[test]
fn an_engine_placeholder_selects_key_data_mode_without_making_a_typed_key() {
    let mut scene = Scene::new();
    assert!(!scene.key_data(Data::Empty));
    assert_eq!(
        scene.transit.mode(),
        Some(FlutterKeyTransitMode::KeyDataThenRaw)
    );
    assert!(!scene.raw(raw_a()));
    assert_eq!(
        scene.log.take(),
        [
            Event::Raw(Code::KeyA, KeyState::Down),
            Event::FocusRaw(Code::KeyA, KeyState::Down),
        ]
    );
    assert!(
        scene
            .runtime
            .hardware_keyboard()
            .physical_keys_pressed()
            .is_empty()
    );

    let quote = KeyboardEvent::key_down(KeyboardKey::Character("\"".into()), Code::Quote);
    assert!(!scene.key_data(typed(quote, false)));
    assert!(scene.log.borrow().is_empty());
    assert!(!scene.raw(raw_a()));
    assert_eq!(
        scene.log.take(),
        [
            Event::Raw(Code::KeyA, KeyState::Down),
            Event::Hardware(Code::Quote, KeyState::Down, HardwareKeyOrigin::Supplied),
            Event::FocusTyped(Code::Quote, KeyState::Down),
            Event::FocusRaw(Code::KeyA, KeyState::Down),
        ]
    );
}

#[test]
fn an_engine_that_starts_with_a_raw_packet_keeps_legacy_conversion() {
    let mut scene = Scene::new();
    assert!(!scene.raw(raw_a()));
    assert_eq!(scene.transit.mode(), Some(FlutterKeyTransitMode::RawOnly));
    assert_eq!(
        scene.log.take(),
        [
            Event::Raw(Code::KeyA, KeyState::Down),
            Event::Hardware(Code::KeyA, KeyState::Down, HardwareKeyOrigin::Supplied),
            Event::FocusTyped(Code::KeyA, KeyState::Down),
            Event::FocusRaw(Code::KeyA, KeyState::Down),
        ]
    );
}

#[test]
fn a_custom_receiver_observes_the_actual_batch_instead_of_one_selected_event() {
    let mut scene = Scene::new();
    let seen = Rc::new(RefCell::new(Vec::new()));
    scene.runtime.set_key_message_handler({
        let seen = seen.clone();
        move |message| {
            let KeyMessage::FlutterBatch { typed, raw } = message else {
                panic!("not an engine batch")
            };
            assert_eq!(raw.as_ref().unwrap().physical_key(), Code::KeyA);
            seen.borrow_mut()
                .extend(typed.iter().map(|key| (key.event.state, key.synthesized)));
            true
        }
    });
    let logical = KeyboardKey::Character("a".into());
    assert!(!scene.key_data(typed(
        KeyboardEvent::key_down(logical.clone(), Code::KeyA),
        false
    )));
    assert!(!scene.key_data(typed(KeyboardEvent::key_up(logical, Code::KeyA), true)));
    assert!(scene.raw(raw_a()));
    assert_eq!(
        *seen.borrow(),
        [(KeyState::Down, false), (KeyState::Up, true)]
    );
}

#[test]
fn a_windows_ime_only_down_and_matching_release_do_not_flush_the_waiting_batch() {
    use incular_widgets::WindowsRawKeyData;

    let mut scene = Scene::new();
    assert!(!scene.key_data(typed(
        KeyboardEvent::key_down(KeyboardKey::Character("a".into()), Code::KeyA),
        false,
    )));
    for (state, key_code) in [
        (KeyState::Down, WindowsRawKeyData::VK_PROCESSKEY),
        (KeyState::Up, 0x41),
    ] {
        let raw = WindowsRawKeyData {
            key_code,
            scan_code: 0x1e,
            ..Default::default()
        }
        .into_event(state, false);
        assert!(scene.raw(raw));
        assert!(scene.log.borrow().is_empty());
    }
    assert!(!scene.raw(raw_a()));
    assert_eq!(
        scene.log.take(),
        [
            Event::Raw(Code::KeyA, KeyState::Down),
            Event::Hardware(Code::KeyA, KeyState::Down, HardwareKeyOrigin::Supplied),
            Event::FocusTyped(Code::KeyA, KeyState::Down),
            Event::FocusRaw(Code::KeyA, KeyState::Down),
        ]
    );
}
