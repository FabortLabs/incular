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
