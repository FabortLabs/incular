use incular_core::{
    DiagnosticEnum, DiagnosticLevel as Level, DiagnosticValue, DiagnosticsProperty, Offset, Rect,
    Size, describe_enum, describe_legacy_enum,
};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq)]
enum ExampleEnum {
    Hello,
    World,
    DeferToChild,
}

impl DiagnosticValue for ExampleEnum {
    const TYPE_NAME: &'static str = "ExampleEnum";
    fn diagnostic_text(&self) -> String {
        format!("ExampleEnum.{}", self.variant_name())
    }
    fn to_diagnostic_json(&self) -> Value {
        Value::Null
    }
    fn has_json_value(&self) -> bool {
        false
    }
}

impl DiagnosticEnum for ExampleEnum {
    fn variant_name(&self) -> &str {
        match self {
            Self::Hello => "hello",
            Self::World => "world",
            Self::DeferToChild => "deferToChild",
        }
    }
}

const MEMBERS: [(ExampleEnum, &str); 3] = [
    (ExampleEnum::Hello, "hello"),
    (ExampleEnum::World, "world"),
    (ExampleEnum::DeferToChild, "deferToChild"),
];

#[test]
fn real_and_legacy_enums_describe_the_selected_member() {
    for (value, name) in MEMBERS {
        assert_eq!(describe_enum(&value), name);
        assert_eq!(
            describe_legacy_enum(&value.diagnostic_text()).unwrap(),
            name
        );
    }
    let error = describe_legacy_enum("Hello World").unwrap_err();
    assert_eq!(
        error.to_string(),
        "The provided object \"Hello World\" is not an enum."
    );
    assert!(describe_legacy_enum("ExampleEnum.").is_err());
}

#[test]
fn enum_properties_keep_the_member_but_display_only_its_short_name() {
    let prop = |member| DiagnosticsProperty::enumeration("name", member);
    for (value, name) in MEMBERS {
        let property = prop(Some(value));
        assert_eq!(property.effective_level(), Level::Info);
        assert_eq!(property.value(), Some(&value));
        assert_eq!(property.to_string(), format!("name: {name}"));
        let wire: Value =
            serde_json::from_str(&serde_json::to_string(&property.to_json()).unwrap()).unwrap();
        assert_eq!(wire["name"], "name");
        assert_eq!(wire["description"], name);
        assert_eq!(wire["type"], "EnumProperty<ExampleEnum>");
        assert_eq!(wire["propertyType"], "ExampleEnum");
        assert_eq!(wire["missingIfNull"], false);
        assert!(wire.get("value").is_none());
    }
    let missing = prop(None);
    assert_eq!(missing.effective_level(), Level::Info);
    assert_eq!(missing.value(), None);
    assert_eq!(missing.to_string(), "name: null");
    assert_eq!(missing.to_json()["value"], Value::Null);

    let matching_default = prop(Some(ExampleEnum::Hello)).default_value(Some(ExampleEnum::Hello));
    assert_eq!(matching_default.to_string(), "name: hello");
    assert_eq!(matching_default.value(), Some(&ExampleEnum::Hello));
    assert!(matching_default.is_filtered(Level::Info));
    assert_eq!(
        matching_default.to_json()["defaultValue"],
        "ExampleEnum.hello"
    );
    assert_eq!(
        prop(Some(ExampleEnum::Hello))
            .level(Level::Hidden)
            .effective_level(),
        Level::Hidden
    );
}

#[test]
fn integer_properties_report_null_defaults_units_and_visibility() {
    let int = |value| DiagnosticsProperty::integer("name", value);
    for (property, value, expected, level) in [
        (int(Some(42)), Some(42), "name: 42", Level::Info),
        (int(None), None, "name: null", Level::Info),
        (
            int(None).default_value(None),
            None,
            "name: null",
            Level::Fine,
        ),
        (
            int(None).if_null("missing"),
            None,
            "name: missing",
            Level::Info,
        ),
        (int(Some(42)).show_name(false), Some(42), "42", Level::Info),
        (
            int(Some(42)).unit("pt"),
            Some(42),
            "name: 42pt",
            Level::Info,
        ),
        (
            int(Some(42)).default_value(Some(42)),
            Some(42),
            "name: 42",
            Level::Fine,
        ),
        (
            int(Some(43)).default_value(Some(42)),
            Some(43),
            "name: 43",
            Level::Info,
        ),
        (
            int(Some(42)).level(Level::Hidden),
            Some(42),
            "name: 42",
            Level::Hidden,
        ),
    ] {
        assert_eq!(property.to_string(), expected);
        assert_eq!(property.value().copied(), value);
        assert_eq!(property.effective_level(), level);
        assert_eq!(property.is_filtered(Level::Info), level < Level::Info);
        assert_eq!(property.to_json()["value"], json!(value));
    }
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
fn object_properties_retain_alternate_descriptions_and_separator_settings() {
    let rect = Rect::from_origin_size(Offset::ZERO, Size::new(20., 20.));
    let text = "Rect.fromLTRB(0.0, 0.0, 20.0, 20.0)";
    let prop = || DiagnosticsProperty::new("name", Some(rect));
    let plain = prop();
    assert_eq!(plain.value(), Some(&rect));
    assert_eq!(plain.effective_level(), Level::Info);
    assert_eq!(plain.to_string(), format!("name: {text}"));
    assert!(plain.to_json().get("value").is_none());
    let alternative = prop().description("small rect");
    assert_eq!(alternative.value(), Some(&rect));
    assert_eq!(alternative.effective_level(), Level::Info);
    assert_eq!(alternative.to_string(), "name: small rect");
    assert_eq!(alternative.to_json()["description"], "small rect");

    let null = || DiagnosticsProperty::<Object>::new("name", None);
    for (property, text, level) in [
        (null(), "name: null", Level::Info),
        (null().default_value(None), "name: null", Level::Fine),
        (null().if_null("missing"), "name: missing", Level::Info),
    ] {
        assert!(property.value().is_none());
        assert_eq!(property.effective_level(), level);
        assert_eq!(property.is_filtered(Level::Info), level < Level::Info);
        assert_eq!(property.to_string(), text);
        assert_eq!(property.to_json()["value"], Value::Null);
    }

    let nameless = prop().show_name(false).level(Level::Warning);
    assert_eq!(nameless.value(), Some(&rect));
    assert_eq!(nameless.effective_level(), Level::Warning);
    assert_eq!(nameless.to_string(), text);
    assert_eq!(nameless.to_json()["showName"], false);
    let no_colon = DiagnosticsProperty::new("Creator", Some(rect)).show_separator(false);
    assert_eq!(no_colon.value(), Some(&rect));
    assert_eq!(no_colon.effective_level(), Level::Info);
    assert_eq!(no_colon.to_string(), format!("Creator {text}"));
    assert_eq!(no_colon.to_json()["showSeparator"], false);
}
