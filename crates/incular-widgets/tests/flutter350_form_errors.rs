//! Flutter 3.50.0 FormField forced errors and neutral error presentation.
use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_widgets::{
    AutovalidateMode, Column, Directionality, Form, FormFieldState, GenericFormField, Text, Widget,
    internal::{ElementId, Key, WidgetTree},
};

type Field = FormFieldState<String>;

fn field(state: &Field) -> GenericFormField<String> {
    let state_for_view = state.clone();
    GenericFormField::new(move |value: Option<&String>| {
        let label = |text: String| -> Widget {
            Widget::from(Text::new(text.clone())).with_key(Key::from(text.as_str()))
        };
        Column::new(
            value
                .cloned()
                .map(label)
                .into_iter()
                .chain(state_for_view.error_text().map(label)),
        )
    })
    .state(state.clone())
}

struct Harness {
    tree: WidgetTree,
    root: ElementId,
}

impl Harness {
    fn scene(form: &Form, children: &[Widget]) -> Widget {
        Directionality::new(
            TextDirection::Ltr,
            form.clone().child(Column::new(children.iter().cloned())),
        )
        .into()
    }

    fn new(form: &Form, children: &[Widget]) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(Self::scene(form, children)).unwrap();
        tree.layout(Constraints::loose(Size::new(400., 300.)))
            .unwrap();
        Self { tree, root }
    }

    fn update(&mut self, form: &Form, children: &[Widget]) {
        self.tree
            .update(self.root, Self::scene(form, children))
            .unwrap();
        self.tree
            .layout(Constraints::loose(Size::new(400., 300.)))
            .unwrap();
    }

    fn shows(&self, text: &str) -> bool {
        self.tree.element_with_key(&Key::from(text)).is_some()
    }
}

#[test]
fn a_forced_error_appears_on_first_layout_without_autovalidation() {
    let form = Form::new();
    let state = form.register_value(Some("foo".into()));
    let harness = Harness::new(
        &form,
        &[field(&state).force_error_text("Forcing error.").into()],
    );
    assert!(harness.shows("Forcing error."));
    assert!(!state.is_valid());
}

#[test]
fn forced_errors_override_a_validator_that_returns_no_error() {
    let form = Form::new();
    let state = form.register_value(Some("foo".into()));
    let build = || {
        field(&state)
            .force_error_text("Forcing error")
            .validator(|_| None)
            .into()
    };
    let mut harness = Harness::new(&form, &[build()]);
    assert!(harness.shows("Forcing error"));
    assert!(!form.validate());
    harness.update(&form, &[build()]);
    assert!(harness.shows("Forcing error"));
}

#[test]
fn a_changed_forced_error_updates_the_existing_field_in_both_directions() {
    let form = Form::new();
    let state = form.register_value(Some("foo".into()));
    let mut harness = Harness::new(&form, &[field(&state).into()]);
    assert!(form.validate());
    assert!(!harness.shows("Forcing Error Text"));
    harness.update(
        &form,
        &[field(&state).force_error_text("Forcing Error Text").into()],
    );
    assert!(harness.shows("Forcing Error Text"));
    harness.update(&form, &[field(&state).into()]);
    assert!(!harness.shows("Forcing Error Text"));
    assert!(state.is_valid());
}

#[test]
fn reset_temporarily_clears_the_forced_presentation_and_validation_bypasses_the_callback() {
    let form = Form::new();
    let state = form.register_value(Some("foo".into()));
    let calls = Rc::new(Cell::new(0));
    let build = |mode| {
        let calls = calls.clone();
        field(&state)
            .force_error_text("Forcing error.")
            .autovalidate_mode(mode)
            .validator(move |_| {
                calls.set(calls.get() + 1);
                Some("validator error".into())
            })
            .into()
    };
    let disabled = AutovalidateMode::Disabled;
    let mut harness = Harness::new(&form, &[build(disabled)]);
    assert!(harness.shows("Forcing error."));
    assert!(!harness.shows("validator error"));
    form.reset();
    harness.update(&form, &[build(disabled)]);
    assert!(!harness.shows("Forcing error."));
    assert!(!harness.shows("validator error"));
    assert!(!form.validate());
    harness.update(&form, &[build(disabled)]);
    assert!(harness.shows("Forcing error."));
    assert!(!harness.shows("validator error"));
    harness.update(&form, &[build(AutovalidateMode::Always)]);
    assert!(harness.shows("Forcing error."));
    assert!(!harness.shows("validator error"));
    assert_eq!(calls.get(), 0);
}

fn verify_clear(all: bool) {
    let form = Form::new();
    let states: Vec<_> = ["foo", "bar"]
        .into_iter()
        .take(if all { 2 } else { 1 })
        .map(|value| form.register_value(Some(value.to_owned())))
        .collect();
    let build = || {
        states
            .iter()
            .map(|state| {
                field(state)
                    .validator(|value| Some(format!("{}/error", value.unwrap())))
                    .into()
            })
            .collect::<Vec<_>>()
    };
    let mut harness = Harness::new(&form, &build());
    assert!(!form.validate());
    harness.update(&form, &build());
    for state in &states {
        assert!(harness.shows(&format!("{}/error", state.value().unwrap())));
    }
    if all {
        form.clear_error();
    } else {
        states[0].clear_error();
    }
    harness.update(&form, &build());
    for state in &states {
        let text = state.value().unwrap();
        assert!(!harness.shows(&format!("{text}/error")));
        assert!(harness.shows(&text));
    }
}

#[test]
fn clearing_one_field_error_keeps_its_value() {
    verify_clear(false);
}

#[test]
fn clearing_form_errors_keeps_every_field_value() {
    verify_clear(true);
}
