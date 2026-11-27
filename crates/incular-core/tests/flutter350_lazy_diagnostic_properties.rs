use std::{cell::Cell, rc::Rc};

use incular_core::{
    Offset, Rect, Size,
    diagnostic_property::{
        DiagnosticFailure, DiagnosticLevel as Level, DiagnosticValue, DiagnosticsProperty,
    },
};
use serde_json::{Value, json};

#[test]
fn lazy_double_values_and_errors_use_the_normal_description_and_wire_format() {
    let calls = Rc::new(Cell::new(0));
    let safe = DiagnosticsProperty::lazy_double("name", {
        let calls = calls.clone();
        move || {
            calls.set(calls.get() + 1);
            Ok(Some(42.))
        }
    });
    let copy = safe.clone();
    assert_eq!(calls.get(), 0);
    assert_eq!(safe.to_string(), "name: 42.0");
    assert!(!safe.is_filtered(Level::Info));
    assert_eq!(safe.value(), Some(&42.));
    assert_eq!(copy.to_json()["numberToString"], "42.0");
    assert_eq!(copy.to_json()["value"], json!(42.));
    assert_eq!(calls.get(), 1);
    assert_eq!(
        DiagnosticsProperty::lazy_double("name", || Ok(Some(1.3333))).to_string(),
        "name: 1.3"
    );

    let missing = DiagnosticsProperty::lazy_double("name", || Ok(None));
    assert_eq!(missing.to_string(), "name: null");
    assert!(!missing.is_filtered(Level::Info));

    let error = DiagnosticFailure::new("FlutterError", "Invalid constraints");
    let bad = DiagnosticsProperty::lazy_double("name", {
        let error = error.clone();
        let calls = calls.clone();
        move || {
            calls.set(calls.get() + 1);
            Err(error)
        }
    });
    assert_eq!(bad.value(), None);
    assert!(!bad.is_filtered(Level::Info));
    assert_eq!(bad.to_string(), "name: EXCEPTION (FlutterError)");
    assert_eq!(bad.effective_level(), Level::Error);
    assert_eq!(bad.exception(), Some(&error));
    let wire = bad.to_json();
    assert_eq!(wire["value"], Value::Null);
    assert_eq!(wire["exception"], "Invalid constraints");
    assert_eq!(wire["numberToString"], "null");
    assert_eq!(wire["type"], "DoubleProperty");
    assert_eq!(wire["propertyType"], "double");
    assert_eq!(wire["defaultLevel"], "info");
    assert_eq!(wire["level"], "error");
    assert_eq!(calls.get(), 2);
}

#[derive(PartialEq)]
struct Object;

impl DiagnosticValue for Object {
    const TYPE_NAME: &'static str = "Object";
    fn diagnostic_text(&self) -> String {
        "object".to_owned()
    }
    fn to_diagnostic_json(&self) -> Value {
        Value::Null
    }
    fn has_json_value(&self) -> bool {
        false
    }
}

#[test]
fn lazy_objects_preserve_descriptions_defaults_and_failure_metadata() {
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(20., 20.));
    let simple =
        DiagnosticsProperty::lazy("name", move || Ok(Some(rect))).description("small rect");
    assert_eq!(simple.value(), Some(&rect));
    assert_eq!(simple.effective_level(), Level::Info);
    assert_eq!(simple.to_string(), "name: small rect");
    assert_eq!(simple.to_json()["description"], "small rect");
    assert_eq!(simple.to_json()["type"], "DiagnosticsProperty<Rect>");
    assert_eq!(simple.to_json()["propertyType"], "Rect");
    assert!(simple.to_json().get("value").is_none());
    let nameless = simple.show_name(false);
    assert_eq!(nameless.value(), Some(&rect));
    assert!(!nameless.is_filtered(Level::Info));
    assert_eq!(nameless.to_string(), "small rect");
    assert_eq!(nameless.to_json()["showName"], false);

    let absent = DiagnosticsProperty::<Object>::lazy("name", || Ok(None)).description("missing");
    assert!(absent.value().is_none());
    assert!(!absent.is_filtered(Level::Info));
    assert_eq!(absent.to_string(), "name: missing");
    assert_eq!(absent.to_json()["value"], Value::Null);
    let expected_absence = absent.default_value(None);
    assert!(expected_absence.value().is_none());
    assert!(expected_absence.is_filtered(Level::Info));
    assert_eq!(expected_absence.to_string(), "name: missing");
    assert_eq!(expected_absence.to_json()["defaultValue"], "null");

    for description in [None, Some("missing")] {
        let error = DiagnosticFailure::new("FlutterError", "Property not available");
        let property = DiagnosticsProperty::<Object>::lazy("name", {
            let error = error.clone();
            move || Err(error)
        })
        .default_value(None);
        let property = match description {
            Some(desc) => property.description(desc),
            None => property,
        };
        assert!(property.value().is_none());
        assert_eq!(property.exception(), Some(&error));
        assert!(!property.is_filtered(Level::Info));
        let expected = description.unwrap_or("EXCEPTION (FlutterError)");
        assert_eq!(property.to_string(), format!("name: {expected}"));
        let wire: Value =
            serde_json::from_str(&serde_json::to_string(&property.to_json()).unwrap()).unwrap();
        assert_eq!(wire["description"], expected);
        assert_eq!(wire["defaultLevel"], "info");
        assert_eq!(wire["exception"], "Property not available");
        assert_eq!(wire["value"], Value::Null);
        assert_eq!(wire["level"], "error");
    }
}

#[test]
fn reentrant_observation_sees_no_value_until_the_one_computation_finishes() {
    use std::cell::RefCell;
    let slot: Rc<RefCell<Option<DiagnosticsProperty<i64>>>> = Rc::new(RefCell::new(None));
    let weak = Rc::downgrade(&slot);
    let lazy = DiagnosticsProperty::lazy("value", move || {
        let held = weak.upgrade().unwrap();
        let held = held.borrow();
        let observer = held.as_ref().unwrap();
        assert_eq!(observer.value(), None);
        assert_eq!(observer.exception(), None);
        Ok(Some(42_i64))
    });
    slot.replace(Some(lazy.clone()));
    assert_eq!(lazy.value(), Some(&42));
    assert_eq!(slot.borrow().as_ref().unwrap().value(), Some(&42));
}
