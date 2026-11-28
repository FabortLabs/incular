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
        focus_form_input(&mut self.runtime, key);
        self.frame();
    }

    fn replace_text(&mut self, value: &str) {
        replace_form_input(&mut self.runtime, value);
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

fn required_field(state: &incular_widgets::FormFieldState<String>) -> GenericFormField<String> {
    GenericFormField::new({
        let state = state.clone();
        move |_: Option<&String>| {
            state.error_text().map_or_else(
                || incular_widgets::SizedBox::shrink().into(),
                |error| Widget::from(Text::new(error.clone())).with_key(error.as_str()),
            )
        }
    })
    .state(state.clone())
    .validator(|value| {
        value
            .is_none_or(String::is_empty)
            .then(|| "Required".into())
    })
}

fn form_frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(400., 300.)))
        .unwrap();
}

fn focus_form_input(runtime: &mut Runtime, key: &str) {
    let node = runtime.tree().element_with_key(&Key::from(key)).unwrap();
    let position = runtime.tree().element_bounds(node).unwrap().origin + Offset::new(8., 8.);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
    }
    assert_eq!(runtime.focused_element(), Some(node));
}

fn replace_form_input(runtime: &mut Runtime, value: &str) {
    let mut ctrl_a = KeyboardEvent::key_down(KeyboardKey::Character("a".into()), Code::KeyA);
    ctrl_a.modifiers = Modifiers::CONTROL;
    let _ = runtime.handle_input(InputEvent::Key(ctrl_a));
    if value.is_empty() {
        let _ = runtime.handle_input(InputEvent::Key(KeyboardEvent::key_down(
            KeyboardKey::Named(NamedKey::Backspace),
            Code::Backspace,
        )));
    } else {
        let _ = runtime.handle_input(InputEvent::Text(value.into()));
    }
}

fn runtime_shows(runtime: &Runtime, key: &str) -> bool {
    runtime.tree().element_with_key(&Key::from(key)).is_some()
}

#[test]
fn if_error_mode_does_not_validate_initially_with_a_valid_value() {
    let state = incular_widgets::FormFieldState::new(Some("foo".to_owned()));
    let calls = Rc::new(std::cell::Cell::new(0));
    let field = required_field(&state)
        .autovalidate_mode(AutovalidateMode::OnUserInteractionIfError)
        .validator({
            let calls = calls.clone();
            move |_| {
                calls.set(calls.get() + 1);
                None
            }
        });
    let mut runtime = Runtime::new(field.into()).unwrap();
    form_frame(&mut runtime);
    assert_eq!(calls.get(), 0);
    assert!(!state.has_error());
    assert!(!runtime_shows(&runtime, "Required"));
}

#[test]
fn if_error_mode_starts_silent_even_if_empty_and_explicit_validate_shows_the_error() {
    let state = incular_widgets::FormFieldState::new(None);
    let widget = Directionality::new(
        TextDirection::Ltr,
        required_field(&state).autovalidate_mode(AutovalidateMode::OnUserInteractionIfError),
    );
    let mut runtime = Runtime::new(widget.into()).unwrap();
    form_frame(&mut runtime);
    assert!(!state.has_error());
    assert!(!runtime_shows(&runtime, "Required"));
    assert!(!state.validate());
    form_frame(&mut runtime);
    assert!(state.has_error());
    assert!(runtime_shows(&runtime, "Required"));
}

#[test]
fn form_if_error_starts_silent_then_revalidates_after_an_error_exists() {
    let form = Form::new().autovalidate_mode(AutovalidateMode::OnUserInteractionIfError);
    let state = form.register_value(Some("foo".to_owned()));
    let controller = TextEditingController::with_text("foo");
    let editor = Widget::from(EditableText::new(controller.clone()).on_changed({
        let state = state.clone();
        move |text| state.did_change(text.to_owned())
    }))
    .with_key("conditional input");
    let form_body = Column::new([editor, Widget::from(required_field(&state))]);
    let widget = Directionality::new(TextDirection::Ltr, form.clone().child(form_body));
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Windows);
    form_frame(&mut runtime);
    focus_form_input(&mut runtime, "conditional input");
    assert!(!runtime_shows(&runtime, "Required"));

    replace_form_input(&mut runtime, "bar");
    form_frame(&mut runtime);
    assert!(!runtime_shows(&runtime, "Required"));
    replace_form_input(&mut runtime, "");
    form_frame(&mut runtime);
    assert!(!runtime_shows(&runtime, "Required"));

    assert!(controller.text().is_empty());
    assert!(!form.validate());
    assert!(!runtime_shows(&runtime, "Required"));
    form_frame(&mut runtime);
    assert!(runtime_shows(&runtime, "Required"));

    replace_form_input(&mut runtime, "baz");
    form_frame(&mut runtime);
    assert!(!runtime_shows(&runtime, "Required"));
    assert!(form.validate());
    form_frame(&mut runtime);
    assert!(!runtime_shows(&runtime, "Required"));

    replace_form_input(&mut runtime, "");
    form_frame(&mut runtime);
    assert!(
        !runtime_shows(&runtime, "Required"),
        "gate closes when no error remains"
    );
}

#[test]
fn inherited_always_mode_shows_initial_errors_on_the_second_frame() {
    let form = Form::new().autovalidate_mode(AutovalidateMode::Always);
    let fields = ["foo", "bar"].map(|value| {
        let state = form.register_value(Some(value.to_owned()));
        let field = GenericFormField::new({
            let state = state.clone();
            move |_: Option<&String>| {
                state.error_text().map_or_else(
                    || incular_widgets::SizedBox::shrink().into(),
                    |error| Widget::from(Text::new(error.clone())).with_key(error.as_str()),
                )
            }
        });
        field
            .state(state)
            .validator(|value| value.map(|text: &String| format!("{text}/error")))
    });
    let widget = Directionality::new(TextDirection::Ltr, form.child(Column::new(fields)));
    let mut runtime = Runtime::new(widget.into()).unwrap();
    form_frame(&mut runtime);
    assert!(!runtime_shows(&runtime, "foo/error"));
    assert!(!runtime_shows(&runtime, "bar/error"));
    assert!(
        runtime.frame_requested(),
        "post-frame validation requests a follow-up"
    );
    form_frame(&mut runtime);
    assert!(runtime_shows(&runtime, "foo/error"));
    assert!(runtime_shows(&runtime, "bar/error"));
}

#[test]
fn form_validation_updates_the_existing_focusable_editors_semantics() {
    use incular_semantics::{SemanticRole, SemanticValidationResult};

    let form = Form::new().autovalidate_mode(AutovalidateMode::Always);
    let state = form.register_value(Some(String::new()));
    let invalid = Rc::new(std::cell::Cell::new(false));
    let editor = Widget::from(EditableText::new(TextEditingController::new()).on_changed({
        let state = state.clone();
        move |value| state.did_change(value.to_owned())
    }))
    .with_key("accessible editor");
    let field = GenericFormField::new(move |_: Option<&String>| editor.clone())
        .state(state.clone())
        .validator({
            let invalid = invalid.clone();
            move |_| invalid.get().then(|| "Error".into())
        });
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, form.child(field)).into()).unwrap();
    form_frame(&mut runtime);
    focus_form_input(&mut runtime, "accessible editor");

    fn editor_semantics(runtime: &Runtime) -> &incular_semantics::SemanticNode {
        runtime
            .tree()
            .semantics()
            .iter()
            .find_map(|(_, node)| (node.role == SemanticRole::TextField).then_some(node))
            .expect("the real editor has a text field semantic role")
    }

    let mut original = None;
    for (invalid_next, text, expected) in [
        (false, "Hello", SemanticValidationResult::Valid),
        (true, "Again", SemanticValidationResult::Invalid),
        (false, "Fixed", SemanticValidationResult::Valid),
    ] {
        invalid.set(invalid_next);
        assert!(
            runtime.focused_element().is_some(),
            "editing focus vanished before entering {text}"
        );
        let _ = runtime.handle_input(InputEvent::Text(text.into()));
        assert!(
            state.value().is_some_and(|value| value.contains(text)),
            "editable callback did not deliver {text}; value={:?}",
            state.value()
        );
        form_frame(&mut runtime);
        assert_eq!(
            state.has_error(),
            invalid_next,
            "validator ran after the live edit"
        );
        let semantics = editor_semantics(&runtime);
        assert!(semantics.state.focusable);
        assert_eq!(semantics.state.effective_validation_result(), expected);
        assert_eq!(semantics.id, *original.get_or_insert(semantics.id));
    }
    assert!(
        runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, node)| node.role == SemanticRole::Form)
    );
}

#[test]
fn field_semantics_wrapper_emits_its_custom_validation_result_without_a_focus_wrapper() {
    use incular_semantics::SemanticValidationResult;
    use incular_widgets::{FormFieldState, Semantics};

    let state = FormFieldState::<String>::new(None);
    let field = GenericFormField::new(|_: Option<&String>| Text::new("field"))
        .state(state)
        .wrap_with_semantics(|child, default_result| {
            assert_eq!(default_result, SemanticValidationResult::Valid);
            Widget::from(Semantics::new(child).validation_result(SemanticValidationResult::Invalid))
                .with_key("semantics wrapper")
        })
        .wrap_with_focus(|focus| Widget::from(focus).with_key("focus wrapper"));
    let root = Directionality::new(TextDirection::Ltr, Form::of_child(field));
    let mut runtime = Runtime::new(root.into()).unwrap();
    form_frame(&mut runtime);
    assert!(runtime_shows(&runtime, "semantics wrapper"));
    assert!(!runtime_shows(&runtime, "focus wrapper"));
    let node = runtime
        .tree()
        .semantics()
        .iter()
        .find_map(|(_, node)| (node.label.as_deref() == Some("field")).then_some(node))
        .unwrap();
    assert_eq!(
        node.state.effective_validation_result(),
        SemanticValidationResult::Invalid
    );
}

#[test]
fn blur_mode_calls_the_focus_wrapper_in_addition_to_the_semantics_wrapper() {
    use incular_widgets::{FormFieldState, Semantics};

    let state = FormFieldState::<String>::new(None);
    let field = GenericFormField::new(|_: Option<&String>| Text::new("field"))
        .state(state)
        .wrap_with_semantics(|child, result| {
            Widget::from(Semantics::new(child).validation_result(result))
                .with_key("semantics wrapper")
        })
        .focus_includes_semantics(false)
        .wrap_with_focus(|focus| {
            assert!(!focus.includes_semantics());
            Widget::from(focus).with_key("focus wrapper")
        });
    let root = Directionality::new(
        TextDirection::Ltr,
        Form::of_child(field).autovalidate_mode(AutovalidateMode::OnUnfocus),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    form_frame(&mut runtime);
    assert!(runtime_shows(&runtime, "semantics wrapper"));
    assert!(runtime_shows(&runtime, "focus wrapper"));
}

#[test]
fn an_empty_form_may_have_zero_area() {
    use incular_widgets::{Center, SizedBox};

    let form = Widget::from(Form::of_child(Text::new("X"))).with_key("zero form");
    let root = Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::new().child(form).width(0.).height(0.)),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    form_frame(&mut runtime);
    let node = runtime
        .tree()
        .element_with_key(&Key::from("zero form"))
        .unwrap();
    assert_eq!(
        runtime.tree().element_bounds(node).unwrap().size,
        Size::ZERO
    );
}

#[test]
fn a_standalone_field_may_have_zero_area() {
    use incular_widgets::{Center, SizedBox};

    let field = Widget::from(GenericFormField::new(|_: Option<&String>| Text::new("X")))
        .with_key("zero form field");
    let root = Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::new().child(field).width(0.).height(0.)),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    form_frame(&mut runtime);
    let node = runtime
        .tree()
        .element_with_key(&Key::from("zero form field"))
        .unwrap();
    assert_eq!(
        runtime.tree().element_bounds(node).unwrap().size,
        Size::ZERO
    );
}
