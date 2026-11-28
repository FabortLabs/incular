//! The neutral editor drives Flutter-style FormField interaction and saving.
use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset, PointerPhase, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::TextEditingController;
use incular_widgets::{
    AutovalidateMode, Column, Directionality, EditableText, Form, FormField, GenericFormField,
    Text, TextInputActionHint, Widget, internal::Key,
};

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

struct BlurForm {
    runtime: Runtime,
    form: Form,
}

impl BlurForm {
    fn new(values: [&str; 2], modes: [AutovalidateMode; 2]) -> Self {
        let form = Form::new().autovalidate_mode(AutovalidateMode::OnUnfocus);
        let inputs = ["blur-first", "blur-second"];
        let fields: [Widget; 2] = std::array::from_fn(|index| {
            let state = form.register_value(Some(values[index].to_owned()));
            let controller = TextEditingController::with_text(values[index]);
            let editor = Widget::from(
                EditableText::new(controller)
                    .input_action(if index == 0 {
                        TextInputActionHint::Next
                    } else {
                        TextInputActionHint::Unspecified
                    })
                    .on_changed({
                        let state = state.clone();
                        move |text| state.did_change(text.to_owned())
                    }),
            )
            .with_key(inputs[index]);
            GenericFormField::new({
                let state = state.clone();
                move |_: Option<&String>| {
                    Column::new(
                        std::iter::once(editor.clone()).chain(state.error_text().map(|error| {
                            Widget::from(Text::new(error.clone())).with_key(error.as_str())
                        })),
                    )
                }
            })
            .state(state)
            .autovalidate_mode(modes[index])
            .validator(|value| value.map(|text: &String| format!("{text}/error")))
            .into()
        });
        let widget =
            Directionality::new(TextDirection::Ltr, form.clone().child(Column::new(fields))).into();
        let mut runtime = Runtime::new(widget).unwrap();
        runtime.set_text_shortcut_platform(TextShortcutPlatform::Windows);
        let mut result = Self { runtime, form };
        result.frame();
        result
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame(Constraints::tight(Size::new(400., 300.)))
            .unwrap();
    }

    fn focus(&mut self, key: &str) {
        let node = self
            .runtime
            .tree()
            .element_with_key(&Key::from(key))
            .unwrap();
        let position =
            self.runtime.tree().element_bounds(node).unwrap().origin + Offset::new(8., 8.);
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            let _ = self
                .runtime
                .handle_input(InputEvent::Pointer { phase, position });
        }
        self.frame();
        assert_eq!(self.runtime.focused_element(), Some(node));
    }

    fn replace_text(&mut self, value: &str) {
        let mut ctrl_a = KeyboardEvent::key_down(KeyboardKey::Character("a".into()), Code::KeyA);
        ctrl_a.modifiers = Modifiers::CONTROL;
        let _ = self.runtime.handle_input(InputEvent::Key(ctrl_a));
        let _ = self.runtime.handle_input(InputEvent::Text(value.into()));
        self.frame();
    }

    fn shows(&self, key: &str) -> bool {
        self.runtime
            .tree()
            .element_with_key(&Key::from(key))
            .is_some()
    }
}

#[test]
fn an_on_unfocus_form_revalidates_only_each_editor_that_loses_focus() {
    let mut ui = BlurForm::new(["bar", "bar"], [AutovalidateMode::Disabled; 2]);
    assert!(!ui.shows("foo/error"));
    assert!(!ui.shows("bar/error"));
    ui.focus("blur-first");
    ui.replace_text("foo");
    assert!(!ui.shows("foo/error"));
    ui.focus("blur-second");
    assert!(ui.shows("foo/error"));
    assert!(!ui.shows("bar/error"));
    ui.focus("blur-first");
    assert!(ui.shows("foo/error"));
    assert!(ui.shows("bar/error"));
}

#[test]
fn a_field_with_always_validation_can_share_an_on_unfocus_form() {
    let mut ui = BlurForm::new(
        ["foo", "bar"],
        [AutovalidateMode::Always, AutovalidateMode::Disabled],
    );
    assert!(ui.shows("foo/error"));
    assert!(!ui.shows("bar/error"));
    ui.focus("blur-first");
    ui.replace_text("foo");
    ui.focus("blur-second");
    assert!(!ui.shows("bar/error"));
    ui.focus("blur-first");
    assert!(ui.shows("bar/error"));
}

#[test]
fn explicit_validation_updates_every_on_unfocus_field_even_while_focused() {
    let mut ui = BlurForm::new(["foo", "bar"], [AutovalidateMode::Disabled; 2]);
    ui.focus("blur-first");
    assert!(!ui.shows("foo/error"));
    assert!(!ui.shows("bar/error"));
    assert!(!ui.form.validate());
    ui.frame();
    assert!(ui.shows("foo/error"));
    assert!(ui.shows("bar/error"));
}

#[test]
fn platform_next_action_moves_to_the_second_editor_and_blurs_the_first() {
    use incular_platform::{PlatformEvent, TextInputAction};

    let mut ui = BlurForm::new(["foo", "bar"], [AutovalidateMode::Disabled; 2]);
    ui.focus("blur-first");
    let next = ui
        .runtime
        .tree()
        .element_with_key(&Key::from("blur-second"))
        .unwrap();
    assert!(!ui.shows("foo/error"));
    let _ = ui
        .runtime
        .handle_platform_event(PlatformEvent::TextInputAction(TextInputAction::Next));
    ui.frame();
    assert_eq!(ui.runtime.focused_element(), Some(next));
    assert!(ui.shows("foo/error"));
    assert!(!ui.shows("bar/error"));
}
