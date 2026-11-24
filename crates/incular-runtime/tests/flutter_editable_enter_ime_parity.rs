//! On Flutter's Android-profile neutral EditableText, a raw Enter lets
//! the IME handle the key. A native Done action completes then submits.

use incular_config::Constraints;
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset, PointerPhase, Size,
};
use incular_platform::{PlatformEvent, TextInputAction, TextInputState};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::TextEditingController;
use incular_widgets::{EditableText, TextDirection};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, PartialEq, Eq)]
enum Call {
    Changed(String),
    Complete,
    Submitted(String),
}

struct Editor {
    runtime: Runtime,
    controller: TextEditingController,
    calls: Rc<RefCell<Vec<Call>>>,
}

fn editor_with_native_text(platform: TextShortcutPlatform) -> Editor {
    let controller = TextEditingController::new();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let changed = Rc::clone(&calls);
    let completed = Rc::clone(&calls);
    let submitted = Rc::clone(&calls);
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(200., 48.))
        .on_changed(move |text| changed.borrow_mut().push(Call::Changed(text.to_owned())))
        .on_editing_complete(move || completed.borrow_mut().push(Call::Complete))
        .on_submit(move |text| submitted.borrow_mut().push(Call::Submitted(text)));
    let mut runtime = Runtime::new(field.into()).expect("mount editable");
    runtime.set_text_shortcut_platform(platform);
    let bounds = Constraints::tight(Size::new(200., 48.));
    runtime.run_frame(bounds).expect("initial layout");
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(20., 20.),
        });
    }
    runtime.run_frame(bounds).expect("focus editable");
    assert!(runtime.focused_element().is_some());
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        text: "ab".into(),
        selection_start: 2,
        selection_end: 2,
        composing: None,
    }));
    assert_eq!(controller.text(), "ab");
    assert_eq!(*calls.borrow(), [Call::Changed("ab".into())]);
    Editor {
        runtime,
        controller,
        calls,
    }
}

fn raw_enter(editor: &mut Editor) {
    let enter = KeyboardKey::Named(NamedKey::Enter);
    for event in [
        KeyboardEvent::key_down(enter.clone(), Code::Enter),
        KeyboardEvent::key_up(enter, Code::Enter),
    ] {
        let _ = editor.runtime.handle_input(InputEvent::Key(event));
    }
}

#[test]
fn raw_enter_without_ime_action_does_not_complete_or_submit_mobile_editable() {
    for platform in [TextShortcutPlatform::Android, TextShortcutPlatform::Ios] {
        let mut editor = editor_with_native_text(platform);
        raw_enter(&mut editor);
        assert_eq!(editor.controller.text(), "ab");
        assert!(editor.runtime.focused_element().is_some());
        assert_eq!(
            *editor.calls.borrow(),
            [Call::Changed("ab".into())],
            "a raw key is not an IME action on {platform:?}"
        );
    }
}

#[test]
fn native_ime_done_completes_before_submitting_android_editable() {
    let mut editor = editor_with_native_text(TextShortcutPlatform::Android);
    let _ = editor
        .runtime
        .handle_platform_event(PlatformEvent::TextInputAction(TextInputAction::Done));

    assert_eq!(editor.controller.text(), "ab");
    assert!(editor.runtime.focused_element().is_some());
    assert_eq!(
        *editor.calls.borrow(),
        [
            Call::Changed("ab".into()),
            Call::Complete,
            Call::Submitted("ab".into())
        ]
    );
}

#[test]
fn raw_enter_does_not_change_active_mobile_composition() {
    let mut editor = editor_with_native_text(TextShortcutPlatform::Android);
    let _ = editor
        .runtime
        .handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
            text: "abあ".into(),
            selection_start: 5,
            selection_end: 5,
            composing: Some((2, 5)),
        }));
    let before = editor.controller.value();
    let event_count = editor.calls.borrow().len();
    raw_enter(&mut editor);
    assert_eq!(editor.controller.value(), before);
    assert_eq!(editor.calls.borrow().len(), event_count);
}

#[test]
fn desktop_keyboard_keeps_the_existing_submit_fallback() {
    let mut editor = editor_with_native_text(TextShortcutPlatform::Windows);
    raw_enter(&mut editor);
    assert_eq!(
        *editor.calls.borrow(),
        [
            Call::Changed("ab".into()),
            Call::Complete,
            Call::Submitted("ab".into())
        ]
    );
}
