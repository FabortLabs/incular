//! A pure value replacement and a user edit have different selection rules.

use incular_config::{Constraints, TextDirection};
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextEditingValue, TextSelection};
use incular_widgets::{Directionality, EditableText};

fn editor(controller: &TextEditingController) -> Runtime {
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            EditableText::new(controller.clone()).autofocus(true),
        )
        .into(),
    )
    .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    runtime
        .run_frame(Constraints::tight(Size::new(200., 80.)))
        .unwrap();
    runtime
}

#[test]
fn paste_collapses_the_caret_while_value_replacement_preserves_direction() {
    // UTF-8 range 1..6 covers the same b and emoji that UTF-16 1..4 does.
    let selection = TextSelection::new(6, 1);
    let source = TextEditingValue::new("ab😀c").with_selection(selection);
    let replaced = source.clone().replace(selection.range(), "X");
    assert_eq!(replaced.text, "aXc");
    assert_eq!(replaced.selection, TextSelection::new(2, 1));

    let controller = TextEditingController::from_value(source);
    let mut runtime = editor(&controller);
    controller.set_selection(selection);
    runtime.set_clipboard_text("X");
    let mut paste = KeyboardEvent::key_down(KeyboardKey::Character("v".into()), Code::KeyV);
    paste.modifiers = Modifiers::CONTROL;
    assert!(runtime.dispatch_keyboard_event(paste));
    assert_eq!(controller.text(), "aXc");
    assert_eq!(controller.selection(), TextSelection::collapsed(2));
}

#[test]
fn backspace_with_a_nonempty_selection_collapses_at_its_earlier_edge() {
    let controller = TextEditingController::with_text("hello world");
    let mut runtime = editor(&controller);
    controller.set_selection(TextSelection::new(2, 7));
    let backspace =
        KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Backspace), Code::Backspace);
    assert!(runtime.dispatch_keyboard_event(backspace));
    assert_eq!(controller.text(), "heorld");
    assert_eq!(controller.selection(), TextSelection::collapsed(2));
}
