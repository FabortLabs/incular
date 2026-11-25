//! Direct EditableText selection, not a keyboard shortcut or menu discovery.
use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_text::{SelectionChangedCause, TextEditingController, TextSelection};
use incular_widgets::{EditableText, FocusNode, TextDirection, Widget, internal::Key};

#[test]
fn explicit_select_all_obeys_read_only_obscuring_not_keyboard_configuration() {
    const FIELD: Key = Key::Value(6);
    for (read_only, obscured, interactive, allowed) in [
        (true, true, None, false),
        (true, true, Some(true), false),
        (true, false, Some(false), true),
        (false, true, Some(false), true),
    ] {
        let controller = TextEditingController::with_text("blah blah");
        controller.set_selection(TextSelection::collapsed(2));
        let focus = FocusNode::new();
        let mut field = EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .focus_node(focus.clone())
            .obscure_text(obscured)
            .read_only(read_only);
        if let Some(interactive) = interactive {
            field = field.enable_interactive_selection(interactive);
        }
        assert!(!field.select_all_enabled(), "the menu command is separate");
        let mut runtime = Runtime::new(Widget::from(field).with_key(FIELD)).unwrap();
        let constraints = Constraints::tight(Size::new(800., 600.));
        runtime.run_frame(constraints).unwrap();
        let field = runtime.tree().element_with_key(&FIELD).unwrap();
        assert!(!focus.has_focus());
        assert_eq!(
            runtime.select_all_text(field, SelectionChangedCause::Toolbar),
            allowed
        );
        runtime.run_frame(constraints).unwrap();
        let expected = if allowed {
            TextSelection::new(0, 9)
        } else {
            TextSelection::collapsed(2)
        };
        assert_eq!(controller.selection(), expected);
        assert_eq!(focus.has_focus(), allowed);
    }
}

#[test]
fn explicit_select_all_uses_utf8_boundaries_and_leaves_text_unchanged() {
    let controller = TextEditingController::with_text("é😀a");
    let mut runtime = Runtime::new(
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .into(),
    )
    .unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(constraints).unwrap();
    let field = runtime.tree().root().unwrap();
    assert!(runtime.select_all_text(field, SelectionChangedCause::Programmatic));
    assert_eq!(controller.selection(), TextSelection::new(0, "é😀a".len()));
    assert_eq!(controller.text(), "é😀a");
}
