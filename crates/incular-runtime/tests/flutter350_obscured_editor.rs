//! Pinned Flutter obscured editor code-point behavior.

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset, PointerPhase, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    EditableText,
    internal::{TextEditingController, TextSelection},
};

fn key(runtime: &mut Runtime, name: NamedKey, code: Code) {
    assert!(
        runtime.dispatch_keyboard_event(KeyboardEvent::key_down(KeyboardKey::Named(name), code))
    );
    let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(KeyboardKey::Named(name), code));
}

fn editor(obscured: bool) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::new();
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .obscure_text(obscured);
    let mut runtime = Runtime::new(field.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    runtime
        .run_frame(Constraints::tight(Size::new(240., 50.)))
        .unwrap();
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(50., 10.),
        });
    }
    assert!(runtime.focused_element().is_some());
    (runtime, controller)
}

#[test]
fn obscured_family_emoji_walks_and_deletes_one_code_point_at_a_time() {
    let (mut runtime, controller) = editor(true);
    let _ = runtime.handle_input(InputEvent::Text("👨‍👩‍👦".into()));
    let caret = || {
        let text = controller.text();
        assert!(controller.selection().is_collapsed());
        text[..controller.selection().extent].encode_utf16().count()
    };
    assert_eq!(controller.text(), "👨‍👩‍👦");
    assert_eq!(caret(), 8);
    for column in [6, 5, 3, 2, 0] {
        key(&mut runtime, NamedKey::ArrowLeft, Code::ArrowLeft);
        assert_eq!(caret(), column);
    }
    for column in [2, 3, 5, 6, 8] {
        key(&mut runtime, NamedKey::ArrowRight, Code::ArrowRight);
        assert_eq!(caret(), column);
    }
    for expected in ["👨‍👩‍", "👨‍👩", "👨‍", "👨", ""] {
        key(&mut runtime, NamedKey::Backspace, Code::Backspace);
        assert_eq!(controller.text(), expected);
        assert_eq!(caret(), expected.encode_utf16().count());
    }
}

#[test]
fn focused_desktop_replacement_and_explicit_caret_delete_different_ranges() {
    let (mut runtime, controller) = editor(false);
    controller.set_text("é👩‍💻");
    assert_eq!(
        controller.selection(),
        TextSelection::new(0, controller.text().len())
    );
    key(&mut runtime, NamedKey::Backspace, Code::Backspace);
    assert!(controller.text().is_empty());

    controller.set_text("é👩‍💻");
    controller.set_selection(TextSelection::collapsed(controller.text().len()));
    key(&mut runtime, NamedKey::Backspace, Code::Backspace);
    assert_eq!(controller.text(), "é");
    controller.set_selection(TextSelection::collapsed(0));
    key(&mut runtime, NamedKey::Delete, Code::Delete);
    assert!(controller.text().is_empty());
}
