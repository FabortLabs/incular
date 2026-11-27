use std::{cell::Cell, rc::Rc};

use incular_core::{Color, DiagnosticCallback, DiagnosticLevel as Level, DiagnosticsProperty};
use serde_json::{Value, json};

fn callback_wire(property: &DiagnosticsProperty<DiagnosticCallback>) -> Value {
    let wire: Value =
        serde_json::from_str(&serde_json::to_string(&property.to_json()).unwrap()).unwrap();
    assert_eq!(wire["name"], "onClick");
    assert_eq!(wire["type"], "ObjectFlagProperty<Function>");
    assert_eq!(wire["propertyType"], "Function");
    assert_eq!(wire["defaultLevel"], "info");
    assert_eq!(wire["missingIfNull"], false);
    assert_eq!(wire["description"], property.to_description());
    if property.value().is_some() {
        assert!(wire.get("value").is_none());
    } else {
        assert_eq!(wire["value"], Value::Null);
    }
    wire
}

#[test]
fn a_present_callback_uses_its_configured_label_without_invoking_it() {
    let calls = Rc::new(Cell::new(0));
    let callback = DiagnosticCallback::new({
        let calls = calls.clone();
        move || calls.set(calls.get() + 1)
    });
    let present = DiagnosticsProperty::object_flag(
        "onClick",
        Some(callback.clone()),
        Some("clickable"),
        None,
    );
    let missing = DiagnosticsProperty::<DiagnosticCallback>::object_flag(
        "onClick",
        None,
        Some("clickable"),
        None,
    );
    assert_eq!(present.to_string(), "clickable");
    assert_eq!(present.value(), Some(&callback));
    assert!(!present.is_filtered(Level::Info));
    assert_eq!(missing.to_string(), "onClick: null");
    assert!(missing.is_filtered(Level::Fine));
    for property in [&present, &missing] {
        let wire = callback_wire(property);
        assert_eq!(wire["ifPresent"], "clickable");
        assert!(wire.get("ifNull").is_none());
    }
    assert_eq!(calls.get(), 0);
    present.value().unwrap().invoke();
    assert_eq!(calls.get(), 1);
}

#[test]
fn a_missing_callback_can_be_reported_as_disabled() {
    let callback = DiagnosticCallback::new(|| {});
    let present =
        DiagnosticsProperty::object_flag("onClick", Some(callback.clone()), None, Some("disabled"));
    let missing = DiagnosticsProperty::<DiagnosticCallback>::object_flag(
        "onClick",
        None,
        None,
        Some("disabled"),
    );
    assert_eq!(present.to_string(), "onClick: Closure: () => void");
    assert!(present.is_filtered(Level::Fine));
    assert_eq!(present.value(), Some(&callback));
    assert_eq!(missing.to_string(), "disabled");
    assert!(!missing.is_filtered(Level::Info));
    for property in [&present, &missing] {
        let wire = callback_wire(property);
        assert_eq!(wire["ifNull"], "disabled");
        assert!(wire.get("ifPresent").is_none());
    }
}

#[test]
fn a_presence_shorthand_includes_the_actual_object_name() {
    let callback = DiagnosticCallback::new(|| {});
    let present = DiagnosticsProperty::has("onClick", Some(callback.clone()));
    let missing = DiagnosticsProperty::<DiagnosticCallback>::has("onClick", None);
    assert_eq!(present.value(), Some(&callback));
    assert!(!present.is_filtered(Level::Info));
    assert_eq!(present.to_string(), "has onClick");
    assert!(missing.value().is_none());
    assert!(missing.is_filtered(Level::Info));
    assert_eq!(missing.to_string(), "onClick: null");
    for property in [&present, &missing] {
        assert_eq!(callback_wire(property)["ifPresent"], "has onClick");
    }
}

#[test]
fn boolean_labels_can_include_or_exclude_the_property_name() {
    for (value, label) in [(true, "YES"), (false, "NO")] {
        let independent = DiagnosticsProperty::flag("name", Some(value), Some("YES"), Some("NO"));
        let named = independent.clone().show_name(true);
        assert_eq!(named.to_string(), format!("name: {label}"));
        assert_eq!(named.effective_level(), Level::Info);
        assert_eq!(named.value(), Some(&value));
        let wire = named.to_json();
        assert_eq!(wire["value"], value);
        assert_eq!(wire["ifTrue"], "YES");
        assert_eq!(wire["ifFalse"], "NO");
        assert_eq!(wire["description"], label);
        assert_eq!(independent.to_string(), label);
        assert_eq!(independent.to_json()["showName"], false);
    }
    assert_eq!(
        DiagnosticsProperty::flag("name", Some(true), Some("YES"), Some("NO"))
            .show_name(true)
            .level(Level::Hidden)
            .effective_level(),
        Level::Hidden
    );
}

#[test]
fn a_color_property_renders_channel_values_without_inventing_a_json_primitive() {
    let property = DiagnosticsProperty::new("name", Some(Color::WHITE));
    let description = "Color(alpha: 1.0000, red: 1.0000, green: 1.0000, blue: 1.0000, colorSpace: ColorSpace.sRGB)";
    assert_eq!(property.value(), Some(&Color::WHITE));
    assert!(!property.is_filtered(Level::Info));
    assert_eq!(property.to_string(), format!("name: {description}"));
    let wire: Value =
        serde_json::from_str(&serde_json::to_string(&property.to_json()).unwrap()).unwrap();
    assert_eq!(wire["name"], "name");
    assert_eq!(wire["description"], description);
    assert_eq!(wire["type"], "DiagnosticsProperty<Color>");
    assert_eq!(wire["propertyType"], "Color");
    assert_eq!(wire["defaultLevel"], "info");
    assert_eq!(wire["missingIfNull"], json!(false));
    assert!(wire.get("value").is_none());
}
