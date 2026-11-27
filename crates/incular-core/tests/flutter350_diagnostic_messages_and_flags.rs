use incular_core::{
    DiagnosticCallback, DiagnosticLevel, DiagnosticsNode, DiagnosticsProperty,
    DiagnosticsSerializationOptions, DiagnosticsTreeStyle,
};
use indexmap::IndexMap;
use serde_json::{Value, json};

fn wire<T: incular_core::DiagnosticValue>(property: &DiagnosticsProperty<T>) -> Value {
    let object = DiagnosticsNode::property(property)
        .to_json(DiagnosticsSerializationOptions::default(), &());
    let bytes = serde_json::to_vec(&object).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
fn flags_preserve_input_order_and_skip_absent_values_without_calling_callbacks() {
    let callback = || {
        Some(DiagnosticCallback::new(|| {
            panic!("diagnostics must not invoke callbacks")
        }))
    };
    for (entries, expected) in [
        (
            IndexMap::from([("click".into(), callback()), ("move".into(), callback())]),
            vec!["click", "move"],
        ),
        (
            IndexMap::from([("move".into(), callback()), ("click".into(), callback())]),
            vec!["move", "click"],
        ),
        (
            IndexMap::from([("move".into(), None), ("click".into(), callback())]),
            vec!["click"],
        ),
    ] {
        let property = DiagnosticsProperty::flags_summary("listeners", entries.clone());
        assert_eq!(property.value().unwrap().entries(), &entries);
        assert!(!property.is_filtered(DiagnosticLevel::Info));
        assert_eq!(
            property.to_string(),
            format!("listeners: {}", expected.join(", "))
        );
        let json = wire(&property);
        assert_eq!(json["name"], "listeners");
        assert_eq!(json["description"], expected.join(", "));
        assert_eq!(json["type"], "FlagsSummary<Function>");
        assert_eq!(json["propertyType"], "Map<String, Function?>");
        assert_eq!(json["style"], "singleLine");
        assert_eq!(json["defaultLevel"], "info");
        assert_eq!(json["values"], json!(expected));
        assert!(json.get("value").is_none());
    }
}

#[test]
fn empty_flag_input_and_all_absent_input_have_distinct_json() {
    let all_absent = || IndexMap::from([("enter".into(), None::<DiagnosticCallback>)]);
    let hidden = DiagnosticsProperty::flags_summary("listeners", all_absent());
    assert!(hidden.is_filtered(DiagnosticLevel::Info));
    assert_eq!(wire(&hidden)["values"], json!([]));
    assert_eq!(wire(&hidden)["level"], "hidden");

    let shown = DiagnosticsProperty::flags_summary("listeners", all_absent()).if_empty("<none>");
    assert!(!shown.is_filtered(DiagnosticLevel::Info));
    assert_eq!(shown.to_string(), "listeners: <none>");
    let json = wire(&shown);
    assert_eq!(json["ifEmpty"], "<none>");
    assert_eq!(json["values"], json!([]));

    let empty = DiagnosticsProperty::flags_summary(
        "listeners",
        IndexMap::<String, Option<DiagnosticCallback>>::new(),
    );
    assert!(empty.is_filtered(DiagnosticLevel::Info));
    assert!(wire(&empty).get("values").is_none());
}

#[test]
fn diagnostic_message_retains_text_but_has_no_concrete_value() {
    let plain = DiagnosticsProperty::message("hello world");
    assert_eq!(plain.to_string(), "hello world");
    assert!(!plain.shows_name());
    assert_eq!(plain.value(), None);
    let json = wire(&plain);
    assert_eq!(json["description"], "hello world");
    assert_eq!(json["name"], "");
    assert_eq!(json["showName"], false);
    assert_eq!(json["type"], "DiagnosticsProperty<void>");
    assert_eq!(json["style"], "singleLine");
    assert_eq!(json.get("value"), Some(&Value::Null));

    let named = DiagnosticsProperty::message_property("diagnostics", "hello world");
    assert_eq!(named.to_string(), "diagnostics: hello world");
    assert!(named.shows_name());
    assert_eq!(named.value(), None);
    let json = wire(&named);
    assert_eq!(json["description"], "hello world");
    assert_eq!(json["name"], "diagnostics");
    assert!(json.get("showName").is_none());
    assert_eq!(json["type"], "MessageProperty");
    assert_eq!(json["propertyType"], "void");
    assert_eq!(json["style"], "singleLine");
    assert_eq!(json["defaultLevel"], "info");
    assert_eq!(json["missingIfNull"], false);
    assert_eq!(json.get("value"), Some(&Value::Null));
    assert_eq!(
        wire(&plain.style(DiagnosticsTreeStyle::Sparse)).get("style"),
        None
    );
}
