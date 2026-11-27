//! The neutral editor drives Flutter-style FormField interaction and saving.
use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset, PointerPhase, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::TextEditingController;
use incular_widgets::{Directionality, EditableText, Form, FormField, Widget, internal::Key};

struct Editor {
    runtime: Runtime,
    form: Form,
    field: Rc<FormField>,
    controller: TextEditingController,
    saved: Rc<RefCell<Vec<String>>>,
}

impl Editor {
    fn new() -> Self {
        let controller = TextEditingController::new();
        let form = Form::new();
        let saved = Rc::new(RefCell::new(Vec::new()));
        let field = Rc::new(form.register(controller.clone()).on_saved({
            let saved = saved.clone();
            move |value| saved.borrow_mut().push(value)
        }));
        let editing = Widget::from(EditableText::new(controller.clone()).on_changed({
            let field = field.clone();
            move |_| field.notify_user_interaction()
        }))
        .with_key("form input");
        let mut runtime = Runtime::new(
            Directionality::new(TextDirection::Ltr, form.clone().child(editing)).into(),
        )
        .unwrap();
        runtime.set_text_shortcut_platform(TextShortcutPlatform::Windows);
        runtime
            .run_frame(Constraints::tight(Size::new(200., 120.)))
            .unwrap();
        Self {
            runtime,
            form,
            field,
            controller,
            saved,
        }
    }

    fn focus(&mut self) {
        let node = self
            .runtime
            .tree()
            .element_with_key(&Key::from("form input"))
            .unwrap();
        let rect = self.runtime.tree().element_bounds(node).unwrap();
        let position = rect.origin + Offset::new(8., 8.);
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            let _ = self
                .runtime
                .handle_input(InputEvent::Pointer { phase, position });
        }
        assert_eq!(self.runtime.focused_element(), Some(node));
    }

    fn type_text(&mut self, text: &str) {
        let _ = self.runtime.handle_input(InputEvent::Text(text.into()));
        assert_eq!(self.controller.text(), text);
    }

    fn key(&mut self, code: Code, modifiers: Modifiers) {
        let mut event = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Unidentified), code);
        event.modifiers = modifiers;
        let _ = self.runtime.handle_input(InputEvent::Key(event));
    }
}

#[test]
fn an_unedited_field_has_not_interacted_even_if_focused() {
    let mut editor = Editor::new();
    assert!(!editor.field.has_interacted_by_user());
    editor.focus();
    assert!(!editor.field.has_interacted_by_user());
}

#[test]
fn entering_text_records_user_interaction_but_programmatic_changes_do_not() {
    let mut editor = Editor::new();
    editor.controller.set_text("programmatic");
    assert!(!editor.field.has_interacted_by_user());
    editor.controller.set_text("");
    editor.focus();
    editor.type_text("foo");
    assert!(editor.field.has_interacted_by_user());
}

#[test]
fn reset_clears_the_interaction_flag_on_a_live_editor() {
    let mut editor = Editor::new();
    editor.focus();
    editor.type_text("foo");
    assert!(editor.field.has_interacted_by_user());
    editor.field.reset();
    assert!(!editor.field.has_interacted_by_user());
    assert!(editor.controller.text().is_empty());
}

#[test]
fn save_calls_the_field_with_current_text_immediately_including_empty() {
    let mut editor = Editor::new();
    editor.focus();
    assert!(editor.saved.borrow().is_empty());
    editor.type_text("Test");
    assert!(editor.form.save());
    assert_eq!(*editor.saved.borrow(), ["Test"]);
    editor.key(Code::KeyA, Modifiers::CONTROL);
    editor.key(Code::Backspace, Modifiers::empty());
    assert!(editor.controller.text().is_empty());
    assert!(editor.form.save());
    assert_eq!(*editor.saved.borrow(), ["Test", ""]);
}
