//! Flutter 3.50.0 neutral Form/FormField state behavior.
use std::{cell::Cell, collections::BTreeSet, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::{
    AutovalidateMode, Column, Form, FormFieldState, GenericFormField, SizedBox, Widget,
    internal::WidgetTree,
};

fn mount(form: &Form, fields: impl IntoIterator<Item = Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(form.clone().child(Column::new(fields)).into())
        .unwrap();
    tree.layout(Constraints::loose(Size::new(200., 120.)))
        .unwrap();
    tree
}

fn field(state: FormFieldState<String>, mode: AutovalidateMode) -> GenericFormField<String> {
    GenericFormField::new(|_| SizedBox::shrink())
        .state(state)
        .autovalidate_mode(mode)
        .validator(|value: Option<&String>| {
            (!value.is_some_and(|value| value == "Valid string")).then(|| "Error text".into())
        })
}

#[test]
fn form_reset_invokes_the_mounted_field_callback_after_resetting_state() {
    let form = Form::new();
    let state = form.register_value(Some(String::from("Valid string")));
    let count = Rc::new(Cell::new(0));
    let scene = field(state.clone(), AutovalidateMode::Disabled).on_reset({
        let count = count.clone();
        move || {
            count.set(count.get() + 1);
        }
    });
    let mut tree = mount(&form, [scene.into()]);
    assert_eq!(count.get(), 0);
    state.did_change("bad".into());
    assert!(!state.validate());
    form.reset();
    tree.layout(Constraints::loose(Size::new(200., 120.)))
        .unwrap();
    assert_eq!(count.get(), 1);
    assert_eq!(state.value().as_deref(), Some("Valid string"));
    assert!(!state.has_error());
    assert!(!state.has_interacted_by_user());
}

#[test]
fn a_mounted_valid_field_returns_true_from_the_read_only_query() {
    let form = Form::new();
    let first = form.register_value(Some(String::from("Valid string")));
    let second = form.register_value(Some(String::from("Valid string")));
    let _tree = mount(
        &form,
        [
            field(first.clone(), AutovalidateMode::Always).into(),
            field(second.clone(), AutovalidateMode::Always).into(),
        ],
    );
    assert!(first.is_valid());
    assert!(second.is_valid());
}

#[test]
fn validity_read_does_not_turn_on_error_presentation_or_clear_a_stale_error() {
    let form = Form::new();
    let first = form.register_value(Some(String::from("Valid string")));
    let second = form.register_value(Some(String::new()));
    let _tree = mount(
        &form,
        [
            field(first.clone(), AutovalidateMode::Disabled).into(),
            field(second.clone(), AutovalidateMode::Disabled).into(),
        ],
    );
    assert!(first.is_valid());
    assert!(!second.is_valid());
    assert!(!second.has_error());
    assert!(!second.validate());
    assert_eq!(second.error_text().as_deref(), Some("Error text"));
    second.did_change("Valid string".into());
    assert!(second.is_valid());
    assert!(
        second.has_error(),
        "only validate updates the displayed error"
    );
}

#[test]
fn granular_validation_returns_only_invalid_registered_fields() {
    let form = Form::new();
    let valid = form.register_value(Some(String::from("Valid string")));
    let invalid = [
        form.register_value(Some(String::from("Invalid string"))),
        form.register_value(Some(String::from("Invalid string"))),
    ];
    let _tree = mount(
        &form,
        [
            field(valid.clone(), AutovalidateMode::Disabled).into(),
            field(invalid[0].clone(), AutovalidateMode::Disabled).into(),
            field(invalid[1].clone(), AutovalidateMode::Disabled).into(),
        ],
    );
    let result = form.validate_granularly();
    assert_eq!(
        result,
        BTreeSet::from([invalid[0].id().unwrap(), invalid[1].id().unwrap()])
    );
    assert!(!result.contains(&valid.id().unwrap()));
    assert!(invalid.iter().all(FormFieldState::has_error));
    assert!(!valid.has_error());
}
