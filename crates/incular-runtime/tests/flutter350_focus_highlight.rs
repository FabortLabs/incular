//! Focus highlight transitions driven by the retained runtime, not direct
//! calls into the modality manager.

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{
    Code, KeyState, KeyboardEvent, KeyboardKey, NamedKey, Offset, PointerPhase, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{FocusHighlightManager, FocusHighlightMode, FocusHighlightStrategy};
use incular_widgets::{
    SizedBox,
    internal::{PointerDeviceKind, RawPointerEvent},
};

fn pointer(runtime: &mut Runtime, kind: PointerDeviceKind, phase: PointerPhase) {
    let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
        1,
        1,
        kind,
        0,
        Offset::new(10., 10.),
        phase,
        Instant::now(),
    ));
}

fn touch(runtime: &mut Runtime) {
    pointer(runtime, PointerDeviceKind::Touch, PointerPhase::Down);
    pointer(runtime, PointerDeviceKind::Touch, PointerPhase::Up);
}

fn key(runtime: &mut Runtime, name: NamedKey, code: Code) {
    let mut key = KeyboardEvent::key_down(KeyboardKey::Named(name), code);
    let _ = runtime.dispatch_keyboard_event(key.clone());
    key.state = KeyState::Up;
    let _ = runtime.dispatch_keyboard_event(key);
}

#[test]
fn actual_key_and_touch_events_change_highlight_mode_before_listeners_read_it() {
    use FocusHighlightMode::{Touch, Traditional};
    use FocusHighlightStrategy::{AlwaysTouch, AlwaysTraditional, Automatic};

    let manager = FocusHighlightManager::new();
    let mut runtime = Runtime::new(SizedBox::new().width(40.).height(40.).into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(40., 40.)))
        .unwrap();
    manager.set_strategy(Automatic);
    Runtime::set_focus_highlight_platform_default(incular_config::TargetPlatform::Android);
    assert_eq!(manager.mode(), Touch);
    let modes = Rc::new(RefCell::new(Vec::new()));
    let _listen = manager.observe({
        let modes = modes.clone();
        move |mode| {
            assert_eq!(manager.mode(), mode);
            modes.borrow_mut().push(mode);
        }
    });

    key(&mut runtime, NamedKey::Meta, Code::MetaLeft);
    touch(&mut runtime);
    pointer(&mut runtime, PointerDeviceKind::Mouse, PointerPhase::Down);
    pointer(&mut runtime, PointerDeviceKind::Mouse, PointerPhase::Up);
    touch(&mut runtime);
    manager.set_strategy(AlwaysTraditional);
    manager.set_strategy(AlwaysTouch);
    assert_eq!(*modes.borrow(), [Traditional, Touch, Traditional, Touch]);
}

#[test]
fn all_platforms_choose_the_initial_mode_and_update_when_a_mouse_first_arrives() {
    use FocusHighlightMode::{Touch, Traditional};
    use incular_config::TargetPlatform::{Android, Fuchsia, Ios, Linux, MacOs, Windows};
    for (platform, initial) in [
        (Android, Touch),
        (Fuchsia, Touch),
        (Ios, Touch),
        (Linux, Traditional),
        (MacOs, Traditional),
        (Windows, Traditional),
    ] {
        std::thread::spawn(move || {
            let manager = FocusHighlightManager::new();
            let mut runtime = Runtime::new(SizedBox::new().width(40.).height(40.).into()).unwrap();
            Runtime::set_focus_highlight_platform_default(platform);
            runtime
                .run_frame(Constraints::tight(Size::new(40., 40.)))
                .unwrap();
            assert_eq!(manager.mode(), initial, "{platform:?}");
            pointer(&mut runtime, PointerDeviceKind::Mouse, PointerPhase::Move);
            assert_eq!(manager.mode(), Traditional, "{platform:?}");
        })
        .join()
        .unwrap();
    }
}

#[test]
fn a_real_keyboard_key_selects_traditional_on_each_platform() {
    use FocusHighlightMode::{Touch, Traditional};
    use incular_config::TargetPlatform::{Android, Fuchsia, Ios, Linux, MacOs, Windows};
    for (platform, initial) in [
        (Android, Touch),
        (Fuchsia, Touch),
        (Ios, Touch),
        (Linux, Traditional),
        (MacOs, Traditional),
        (Windows, Traditional),
    ] {
        std::thread::spawn(move || {
            let manager = FocusHighlightManager::new();
            let mut runtime = Runtime::new(SizedBox::shrink().into()).unwrap();
            Runtime::set_focus_highlight_platform_default(platform);
            assert_eq!(manager.mode(), initial, "{platform:?}");
            key(&mut runtime, NamedKey::Enter, Code::Enter);
            assert_eq!(manager.mode(), Traditional, "{platform:?}");
        })
        .join()
        .unwrap();
    }
}

#[test]
fn virtual_android_keys_edit_without_switching_touch_focus_highlights() {
    use FocusHighlightMode::{Touch, Traditional};
    use incular_platform::{KeyboardInputSource, PlatformEvent};
    use incular_text::{TextEditingController, TextSelection};
    use incular_widgets::{EditableText, TextDirection};

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
    Runtime::set_focus_highlight_platform_default(incular_config::TargetPlatform::Android);
    manager.set_strategy(FocusHighlightStrategy::Automatic);
    runtime
        .run_frame(Constraints::tight(Size::new(40., 40.)))
        .unwrap();
    touch(&mut runtime);
    controller.set_selection(TextSelection::collapsed(3));
    assert_eq!(manager.mode(), Touch);

    for (flags, id, expected) in [(0x2, -1, "ab"), (0x2, 5, "a"), (0, -1, "")] {
        let source = KeyboardInputSource::from_android(flags, id);
        assert_eq!(source, KeyboardInputSource::Virtual);
        let down =
            KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Backspace), Code::Backspace);
        let _ = runtime.handle_platform_event(PlatformEvent::KeyboardInput {
            event: down.clone(),
            source,
        });
        let mut up = down;
        up.state = KeyState::Up;
        let _ = runtime.handle_platform_event(PlatformEvent::KeyboardInput { event: up, source });
        assert_eq!(manager.mode(), Touch);
        assert_eq!(controller.text(), expected);
    }

    let source = KeyboardInputSource::from_android(0, 5);
    assert_eq!(source, KeyboardInputSource::Physical);
    let event = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::ArrowLeft), Code::ArrowLeft);
    let _ = runtime.handle_platform_event(PlatformEvent::KeyboardInput { event, source });
    assert_eq!(manager.mode(), Traditional);
}
