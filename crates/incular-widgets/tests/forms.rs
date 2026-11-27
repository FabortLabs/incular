//! Form validation, persistence, and restoration behavior tests.

use incular_core::{RestorationKey, RestorationScope};
use incular_text::TextEditingController;
use incular_widgets::{
    AutovalidateMode, Form, FormFieldState, FormState, GenericFormField, Widget,
};

#[test]
fn form_errors_track_and_field_drop_removes_its_controller_observer() {
    let context = incular_core::BuildContext::new();
    let form = Form::new();
    let controller = TextEditingController::with_text("Ada");
    let field = form
        .register(controller.clone())
        .validator(|text| text.is_empty().then(|| "required".to_owned()))
        .autovalidate(AutovalidateMode::Always);
    context.build(|_| {
        assert!(form.errors().is_empty());
    });
    controller.set_text("");
    assert!(context.take_dirty());
    assert_eq!(form.errors().len(), 1);
    drop(field);
    assert!(context.take_dirty());
    assert_eq!(form.field_count(), 0);
    controller.set_text("Grace");
    assert!(!context.is_dirty());
}
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

#[derive(Default)]
struct MemoryRestorationBackend(RefCell<BTreeMap<Vec<RestorationKey>, Value>>);

impl incular_core::RestorationBackend for MemoryRestorationBackend {
    fn read_value(&self, path: &[RestorationKey]) -> Option<Value> {
        self.0.borrow().get(path).cloned()
    }

    fn write_value(&self, path: &[RestorationKey], value: Value) {
        self.0.borrow_mut().insert(path.to_vec(), value);
    }

    fn remove_value(&self, path: &[RestorationKey]) {
        self.0.borrow_mut().remove(path);
    }
}

fn restoration_key(value: &str) -> RestorationKey {
    RestorationKey::new(value).unwrap()
}

fn restoration_scope() -> RestorationScope {
    RestorationScope::root(Rc::new(MemoryRestorationBackend::default()))
        .child_unchecked(restoration_key("window"))
        .child_unchecked(restoration_key("main"))
}

#[test]
fn form_validate_save_reset() {
    let form = Form::new();
    let controller = TextEditingController::with_text("Ada");
    let submitted = Rc::new(Cell::new(0));
    let field = form
        .register(controller.clone())
        .validator(|value| (!value.contains('@')).then_some("email required".into()))
        .autovalidate(AutovalidateMode::OnUserInteraction)
        .on_submit({
            let submitted = submitted.clone();
            move |_| submitted.set(submitted.get() + 1)
        });
    assert!(!form.validate());
    assert_eq!(field.error().as_deref(), Some("email required"));
    assert!(!form.submit());
    field.set_text("ada@example.test");
    assert!(field.error().is_none());
    assert!(form.submit());
    assert_eq!(submitted.get(), 1);
    form.reset();
    assert_eq!(controller.text(), "Ada");
}

#[test]
fn form_controller_survives_rebuild() {
    let form: FormState = Form::new();
    let controller = TextEditingController::with_text("before");
    let saved = Rc::new(Cell::new(0));
    let field = form.register(controller.clone()).on_saved({
        let saved = saved.clone();
        move |_| saved.set(saved.get() + 1)
    });

    let rebuilt = form.clone();
    field.set_text("after");
    assert!(rebuilt.save());
    assert_eq!(saved.get(), 1);
    rebuilt.reset();
    assert_eq!(controller.text(), "before");
}

#[test]
fn form_unregisters_dropped_fields() {
    let form = Form::new();
    let field = form.register(TextEditingController::new());
    assert_eq!(form.field_count(), 1);
    drop(field);
    assert_eq!(form.field_count(), 0);
}

#[test]
fn form_field_restoration_keeps_the_declared_reset_default() {
    let scope = restoration_scope();
    let key = restoration_key("email");
    scope.set_json(
        &key,
        json!({
            "text": "restored@example.test",
            "selection": { "base": 8, "extent": 8 },
        }),
    );

    let form = Form::new();
    let controller = TextEditingController::with_text("default@example.test");
    let field = form.register_with_restoration(controller.clone(), scope.clone(), key.clone());

    assert_eq!(controller.text(), "restored@example.test");
    field.set_text("updated@example.test");
    assert_eq!(
        scope.get_json(&key),
        Some(json!({
            "text": "updated@example.test",
            "selection": {
                "base": 20,
                "extent": 20,
                "affinity": "downstream",
                "isDirectional": false,
                "isSet": false,
            },
        }))
    );
    field.reset();
    assert_eq!(controller.text(), "default@example.test");
}

#[test]
fn restored_form_recomputes_validation_instead_of_reusing_stale_presentation() {
    let scope = restoration_scope();
    let key = restoration_key("email");
    scope.set_json(
        &key,
        json!({
            "text": "invalid",
            "selection": { "base": 7, "extent": 7 },
            "error": "obsolete validator message",
        }),
    );

    let field = Form::new()
        .register_restored(scope, key)
        .validator(|value| (!value.contains('@')).then_some("email is required".into()))
        .autovalidate(AutovalidateMode::Always);

    assert_eq!(field.error().as_deref(), Some("email is required"));
}

#[test]
fn form_pass_validate_visits_all_fields_and_clears_stale_errors() {
    for order in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
        let form = Form::new();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let controllers: Vec<_> = (0..3).map(|_| TextEditingController::new()).collect();
        let fields: Vec<_> = order
            .into_iter()
            .map(|index| {
                let calls = calls.clone();
                form.register(controllers[index].clone())
                    .validator(move |text| {
                        calls.borrow_mut().push(index);
                        text.is_empty().then(|| format!("required {index}"))
                    })
            })
            .collect();
        let notifications = Rc::new(Cell::new(0));
        let observer = form.observe({
            let notifications = notifications.clone();
            move || notifications.set(notifications.get() + 1)
        });
        for valid_count in 0..=3 {
            if valid_count > 0 {
                controllers[valid_count - 1].set_text("valid");
            }
            calls.borrow_mut().clear();
            let before = notifications.get();
            assert_eq!(form.validate(), valid_count == 3);
            let mut actual_calls = calls.borrow().clone();
            actual_calls.sort_unstable();
            assert_eq!(
                actual_calls,
                vec![0, 1, 2],
                "every field validates exactly once"
            );
            assert_eq!(
                form.errors().len(),
                3 - valid_count,
                "clear stale errors as well as adding errors"
            );
            for (field, index) in fields.iter().zip(order) {
                assert_eq!(field.error().is_none(), index < valid_count);
            }
            assert_eq!(
                notifications.get(),
                before + 1,
                "one notification per form pass"
            );
        }
        drop(observer);
    }
}

#[test]
fn form_save_invokes_every_callback_in_registration_order_even_when_invalid() {
    // Flutter FormState.save calls FormField.save on every field regardless of
    // validity (widgets/form.dart:314-319, 643-645).
    let form = Form::new();
    let validator_calls = Rc::new(Cell::new(0));
    let saved = Rc::new(RefCell::new(Vec::new()));
    let fields: Vec<_> = (0..3)
        .map(|index| {
            let validator_calls = validator_calls.clone();
            let saved = saved.clone();
            form.register(TextEditingController::with_text(format!("value {index}")))
                .validator(move |text| {
                    validator_calls.set(validator_calls.get() + 1);
                    (index == 1).then(|| format!("invalid {text}"))
                })
                .on_saved(move |text| saved.borrow_mut().push((index, text)))
        })
        .collect();
    assert!(!form.validate());
    assert_eq!(validator_calls.get(), 3);

    assert!(!form.save(), "save reports the recorded error");
    assert_eq!(
        validator_calls.get(),
        3,
        "save does not run validators again"
    );
    assert_eq!(
        *saved.borrow(),
        vec![
            (0, "value 0".to_owned()),
            (1, "value 1".to_owned()),
            (2, "value 2".to_owned()),
        ]
    );
    drop(fields);
}

#[test]
fn form_submit_validates_before_any_save_callback() {
    let form = Form::new();
    let controller = TextEditingController::new();
    let saved = Rc::new(Cell::new(0));
    let field = form
        .register(controller.clone())
        .validator(|text| text.is_empty().then(|| "required".to_owned()))
        .on_saved({
            let saved = saved.clone();
            move |_| saved.set(saved.get() + 1)
        });

    assert!(!form.submit());
    assert_eq!(saved.get(), 0, "submit never saves an invalid form");
    controller.set_text("ok");
    assert!(form.submit());
    assert_eq!(saved.get(), 1);
    drop(field);
}

#[test]
fn typed_form_field_state_registers_validates_saves_and_resets() {
    let form = Form::new();
    let state = form.register_value(Some(3_i32));
    let saved = Rc::new(Cell::new(None));
    state.set_validator(|value| match value {
        Some(value) if *value >= 5 => None,
        _ => Some("too small".to_owned()),
    });
    state.set_on_saved({
        let saved = saved.clone();
        move |value| saved.set(Some(value))
    });
    assert_eq!(form.field_count(), 1);

    assert!(!form.validate());
    assert_eq!(state.error_text().as_deref(), Some("too small"));
    assert_eq!(form.errors().len(), 1);

    state.did_change(7);
    assert!(form.validate());
    assert!(form.errors().is_empty());
    assert!(form.save());
    assert_eq!(saved.get(), Some(7));

    form.reset();
    assert_eq!(state.value(), Some(3));
    assert!(!state.has_error(), "reset clears presentation");
    assert!(
        !state.is_valid(),
        "the restored value still fails the validator"
    );

    drop(state);
    assert_eq!(form.field_count(), 0);
}

#[test]
fn typed_form_field_autovalidates_on_did_change() {
    let form = Form::new();
    let state = form.register_value::<String>(None);
    state.set_validator(|value| {
        value
            .is_none_or(|text| text.is_empty())
            .then(|| "required".to_owned())
    });
    state.set_autovalidate_mode(AutovalidateMode::OnUserInteraction);
    assert!(
        !state.has_error(),
        "no displayed error before user interaction"
    );
    assert!(
        !state.is_valid(),
        "validity can be queried without displaying an error"
    );

    state.did_change("ada".to_owned());
    assert!(state.is_valid());
    state.did_change(String::new());
    assert_eq!(state.error_text().as_deref(), Some("required"));
    assert_eq!(form.errors().len(), 1);
}

#[test]
fn generic_form_field_connects_retained_state_to_builder_and_validation() {
    let form = Form::new();
    let state = form.register_value::<String>(None);
    let saved = Rc::new(RefCell::new(None::<String>));
    let rendered = Rc::new(RefCell::new(Vec::new()));
    let build = |state: FormFieldState<String>| {
        let rendered = rendered.clone();
        GenericFormField::new(move |value: Option<&String>| {
            rendered.borrow_mut().push(value.cloned());
            incular_widgets::SizedBox::shrink()
        })
        .state(state)
        .validator(|value| value.is_none().then(|| "required".to_owned()))
        .on_saved({
            let saved = saved.clone();
            move |value| *saved.borrow_mut() = Some(value)
        })
    };

    let _: Widget = build(state.clone()).into();
    assert_eq!(rendered.borrow().as_slice(), [None]);
    assert!(!form.validate(), "the conversion installed the validator");
    assert_eq!(state.error_text().as_deref(), Some("required"));

    state.did_change("Ada".to_owned());
    let _: Widget = build(state.clone()).into();
    assert_eq!(
        rendered.borrow().last().cloned(),
        Some(Some("Ada".to_owned())),
        "builder receives the retained value"
    );
    assert!(form.validate());
    assert!(form.save());
    assert_eq!(saved.borrow().as_deref(), Some("Ada"));
}

#[test]
fn generic_form_field_save_skips_absent_value_and_registration_ends_with_state() {
    let form = Form::new();
    let state = form.register_value::<u8>(None);
    let calls = Rc::new(Cell::new(0));
    state.set_on_saved({
        let calls = calls.clone();
        move |_| calls.set(calls.get() + 1)
    });
    assert!(form.save());
    assert_eq!(calls.get(), 0);
    state.did_change(1);
    assert!(form.save());
    assert_eq!(calls.get(), 1);
    drop(state);
    assert_eq!(form.field_count(), 0);
}

#[test]
fn form_validator_can_replace_itself_for_the_next_pass() {
    let form = Form::new();
    let registration = Rc::new(RefCell::new(None::<incular_widgets::FormField>));
    let weak = Rc::downgrade(&registration);
    let field = form
        .register(TextEditingController::new())
        .validator(move |_| {
            let slot = weak.upgrade().expect("registration is live");
            let field = slot.borrow_mut().take().expect("field installed");
            let field = field.validator(|_| None);
            slot.borrow_mut().replace(field);
            Some("original validator".into())
        });
    registration.borrow_mut().replace(field);
    assert!(!form.validate(), "current invocation keeps its result");
    assert!(
        form.validate(),
        "replacement is used on the next invocation"
    );
    assert!(form.errors().is_empty());
}

#[test]
fn form_validator_reentrant_validate_terminates_with_settled_result() {
    let form = Form::new();
    let nested = form.clone();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let field = form
        .register(TextEditingController::with_text("x"))
        .validator(move |value| {
            observed.set(observed.get() + 1);
            if observed.get() == 1 {
                let _ = nested.validate();
            }
            if value.is_empty() {
                Some("required".into())
            } else {
                None
            }
        });
    let _ = field;
    assert!(form.validate());
    assert_eq!(calls.get(), 1);
}

#[test]
fn form_save_callback_can_replace_itself_for_the_next_pass() {
    let form = Form::new();
    let registration = Rc::new(RefCell::new(None::<incular_widgets::FormField>));
    let weak = Rc::downgrade(&registration);
    let calls = Rc::new(Cell::new(0));
    let first_calls = calls.clone();
    let field = form
        .register(TextEditingController::new())
        .on_saved(move |_| {
            first_calls.set(first_calls.get() + 1);
            let slot = weak.upgrade().expect("registration is live");
            let field = slot.borrow_mut().take().expect("field installed");
            let next_calls = first_calls.clone();
            let field = field.on_saved(move |_| next_calls.set(next_calls.get() + 10));
            slot.borrow_mut().replace(field);
        });
    registration.borrow_mut().replace(field);
    assert!(form.save());
    assert_eq!(calls.get(), 1);
    assert!(form.save());
    assert_eq!(calls.get(), 11);
}
