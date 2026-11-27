use incular_core::diagnostic_property::{
    DiagnosticLevel as Level, DiagnosticValue, DiagnosticsProperty,
};
use serde_json::{Value, json};

fn inspect<T: DiagnosticValue>(
    property: &DiagnosticsProperty<T>,
    name: &str,
    type_name: &str,
    description: &str,
    value: Value,
) -> Value {
    let encoded = serde_json::to_string(&property.to_json()).unwrap();
    let json: Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(json["name"], name);
    assert_eq!(json["type"], type_name);
    assert_eq!(json["propertyType"], T::TYPE_NAME);
    assert_eq!(json["description"], description);
    assert_eq!(json["value"], value);
    assert_eq!(json["missingIfNull"], false);
    assert_eq!(json["defaultLevel"], "info");
    assert!(json.get("isDiagnosticableValue").is_none());
    assert!(json.get("exception").is_none());
    json
}

#[test]
fn string_properties_quote_only_the_value_and_keep_raw_data() {
    let prop = |value: Option<&str>| DiagnosticsProperty::string("name", value.map(str::to_owned));
    assert_eq!(prop(Some("value")).quoted(false).to_string(), "name: value");
    let override_name = prop(Some("value"))
        .description("VALUE")
        .if_empty("<hidden>")
        .quoted(false);
    assert_eq!(override_name.to_string(), "name: VALUE");
    let json = inspect(
        &override_name,
        "name",
        "StringProperty",
        "VALUE",
        json!("value"),
    );
    assert_eq!(json["ifEmpty"], "<hidden>");
    assert_eq!(json["quoted"], false);
    assert_eq!(
        prop(Some("value"))
            .quoted(false)
            .show_name(false)
            .if_empty("<hidden>")
            .to_string(),
        "value"
    );
    for show_name in [true, false] {
        assert_eq!(
            prop(Some(""))
                .if_empty("<hidden>")
                .show_name(show_name)
                .to_string(),
            if show_name {
                "name: <hidden>"
            } else {
                "<hidden>"
            }
        );
    }
    assert!(!prop(None).is_filtered(Level::Info));
    assert!(
        prop(Some("value"))
            .level(Level::Hidden)
            .is_filtered(Level::Info)
    );
    assert!(prop(None).default_value(None).is_filtered(Level::Info));

    let quoted = prop(Some("value"));
    assert_eq!(quoted.to_string(), "name: \"value\"");
    let json = inspect(
        &quoted,
        "name",
        "StringProperty",
        "\"value\"",
        json!("value"),
    );
    assert_eq!(json["quoted"], true);
    assert!(json.get("ifEmpty").is_none());
    assert_eq!(quoted.show_name(false).to_string(), "\"value\"");
    assert_eq!(prop(None).show_name(false).to_string(), "null");
}

#[test]
fn booleans_are_not_filtered_unless_hidden_or_at_the_explicit_default() {
    let prop = |value| DiagnosticsProperty::new("name", value);
    for value in [true, false] {
        let property = prop(Some(value));
        assert_eq!(property.value(), Some(&value));
        assert_eq!(property.to_string(), format!("name: {value}"));
        assert!(!property.is_filtered(Level::Info));
        let json = inspect(
            &property,
            "name",
            "DiagnosticsProperty<bool>",
            &value.to_string(),
            json!(value),
        );
        assert!(json.get("defaultValue").is_none());
        assert!(json.get("level").is_none());
    }
    let truthy = prop(Some(true)).description("truthy");
    assert_eq!(truthy.to_string(), "name: truthy");
    inspect(
        &truthy,
        "name",
        "DiagnosticsProperty<bool>",
        "truthy",
        json!(true),
    );
    assert_eq!(prop(Some(true)).show_name(false).to_string(), "true");
    assert!(!prop(None).is_filtered(Level::Info));
    assert!(
        prop(Some(true))
            .level(Level::Hidden)
            .is_filtered(Level::Info)
    );
    assert!(prop(None).default_value(None).is_filtered(Level::Info));
    assert_eq!(
        prop(None).default_value(None).to_json()["defaultValue"],
        "null"
    );
    let missing = prop(None).if_null("missing");
    assert_eq!(missing.to_string(), "name: missing");
    let json = inspect(
        &missing,
        "name",
        "DiagnosticsProperty<bool>",
        "missing",
        Value::Null,
    );
    assert_eq!(json["ifNull"], "missing");
}

#[test]
fn tooltips_decorate_the_description_but_not_the_original_value() {
    let property = DiagnosticsProperty::new("name", Some("value".to_owned())).tooltip("tooltip");
    assert_eq!(property.to_string(), "name: value (tooltip)");
    assert_eq!(property.value().map(String::as_str), Some("value"));
    assert!(!property.is_filtered(Level::Fine));
    let json = inspect(
        &property,
        "name",
        "DiagnosticsProperty<String>",
        "value (tooltip)",
        json!("value"),
    );
    assert_eq!(json["tooltip"], "tooltip");
}

#[test]
fn double_properties_share_precision_and_append_units_only_to_display() {
    let prop = |value| DiagnosticsProperty::double("name", value);
    let plain = prop(Some(42.));
    assert_eq!(plain.to_string(), "name: 42.0");
    assert!(!plain.is_filtered(Level::Info));
    assert_eq!(plain.value(), Some(&42.));
    let json = inspect(&plain, "name", "DoubleProperty", "42.0", json!(42.));
    assert!(json.get("unit").is_none());
    assert_eq!(json["numberToString"], "42.0");
    assert_eq!(prop(Some(1.3333)).to_string(), "name: 1.3");
    assert_eq!(prop(None).to_string(), "name: null");
    assert!(!prop(None).is_filtered(Level::Info));
    assert_eq!(prop(None).if_null("missing").to_string(), "name: missing");
    let px = prop(Some(42.)).unit("px");
    assert_eq!(px.to_string(), "name: 42.0px");
    let json = inspect(&px, "name", "DoubleProperty", "42.0px", json!(42.));
    assert_eq!(json["unit"], "px");
    assert_eq!(json["numberToString"], "42.0");
}

#[test]
fn basic_inspector_values_stay_native_json_scalars() {
    for (property, name, expected) in [
        (
            DiagnosticsProperty::new("int1", Some(10_i64)).to_json(),
            "int1",
            json!(10),
        ),
        (
            DiagnosticsProperty::integer("int2", Some(20)).to_json(),
            "int2",
            json!(20),
        ),
        (
            DiagnosticsProperty::new("double", Some(33.3_f64)).to_json(),
            "double",
            json!(33.3),
        ),
        (
            DiagnosticsProperty::double("double2", Some(33.3)).to_json(),
            "double2",
            json!(33.3),
        ),
        (
            DiagnosticsProperty::new("bool", Some(true)).to_json(),
            "bool",
            json!(true),
        ),
        (
            DiagnosticsProperty::new("string1", Some("hello".to_owned())).to_json(),
            "string1",
            json!("hello"),
        ),
        (
            DiagnosticsProperty::string("string2", Some("world".to_owned())).to_json(),
            "string2",
            json!("world"),
        ),
    ] {
        let wire: Value = serde_json::from_str(&serde_json::to_string(&property).unwrap()).unwrap();
        assert_eq!(wire["name"], name);
        assert_eq!(wire["value"], expected);
    }
}

#[test]
fn unlabeled_flag_states_are_hidden_but_keep_their_boolean_value() {
    for value in [true, false] {
        let flag = DiagnosticsProperty::flag("myFlag", Some(value), Some("myFlag"), None);
        assert_eq!(flag.value(), Some(&value));
        assert_eq!(flag.is_filtered(Level::Fine), !value);
        let description = if value { "myFlag" } else { "false" };
        let json = inspect(&flag, "myFlag", "FlagProperty", description, json!(value));
        assert_eq!(json["ifTrue"], "myFlag");
        assert_eq!(json["defaultValue"], "null");
        assert!(json.get("ifFalse").is_none());
        if value {
            assert_eq!(flag.to_string(), "myFlag");
            assert_eq!(json["showName"], false);
        } else {
            assert_eq!(flag.to_string(), "myFlag: false");
            assert_eq!(json["level"], "hidden");
        }
    }
}

#[test]
fn either_flag_state_can_have_a_suppressed_default_label() {
    for (value, if_true, if_false, label, level) in [
        (
            true,
            Some("layout computed"),
            None,
            "layout computed",
            Level::Info,
        ),
        (
            false,
            Some("layout computed"),
            None,
            "wasLayout: false",
            Level::Hidden,
        ),
        (
            true,
            None,
            Some("no layout computed"),
            "wasLayout: true",
            Level::Hidden,
        ),
    ] {
        let flag = DiagnosticsProperty::flag("wasLayout", Some(value), if_true, if_false);
        assert_eq!(flag.value(), Some(&value));
        assert_eq!(flag.to_string(), label);
        assert_eq!(flag.effective_level(), level);
        let json = inspect(
            &flag,
            "wasLayout",
            "FlagProperty",
            &flag.to_description(),
            json!(value),
        );
        assert_eq!(
            json.get("ifTrue"),
            if_true.map(|label| json!(label)).as_ref()
        );
        assert_eq!(
            json.get("ifFalse"),
            if_false.map(|label| json!(label)).as_ref()
        );
    }
}

#[test]
fn infinity_is_reported_as_named_json_text_not_null() {
    for (name, value, expected) in [
        ("double1", f64::INFINITY, "Infinity"),
        ("double2", f64::NEG_INFINITY, "-Infinity"),
    ] {
        let property = DiagnosticsProperty::double(name, Some(value));
        assert_eq!(property.to_string(), format!("{name}: {expected}"));
        assert_eq!(property.value(), Some(&value));
        let json = inspect(&property, name, "DoubleProperty", expected, json!(expected));
        assert_eq!(json["numberToString"], expected);
        assert!(json.get("unit").is_none());
    }
}

#[test]
fn percent_clamps_only_the_description_and_separates_its_unit() {
    let prop = |value| DiagnosticsProperty::percent("name", value);
    for (value, expected) in [
        (Some(0.4), "name: 40.0%"),
        (Some(0.), "name: 0.0%"),
        (Some(-10.), "name: 0.0%"),
        (Some(1.), "name: 100.0%"),
        (Some(3.), "name: 100.0%"),
        (None, "name: null"),
    ] {
        let property = prop(value);
        assert_eq!(property.to_string(), expected);
        assert_eq!(property.value().copied(), value);
        assert_eq!(property.to_json()["value"], json!(value));
    }
    let decorated = prop(Some(0.99))
        .unit("invisible")
        .tooltip("almost transparent");
    assert_eq!(
        decorated.to_string(),
        "name: 99.0% invisible (almost transparent)"
    );
    let json = inspect(
        &decorated,
        "name",
        "PercentProperty",
        "99.0% invisible (almost transparent)",
        json!(0.99),
    );
    assert_eq!(json["unit"], "invisible");
    assert_eq!(json["numberToString"], "99.0%");
    assert_eq!(
        prop(None).unit("invisible").tooltip("!").to_string(),
        "name: null (!)"
    );
    assert_eq!(prop(None).if_null("missing").to_string(), "name: missing");
    assert_eq!(
        prop(None).if_null("missing").show_name(false).to_string(),
        "missing"
    );
    assert_eq!(prop(Some(0.5)).show_name(false).to_string(), "50.0%");
}

#[test]
fn an_integer_property_retains_the_base_visibility_and_display_options() {
    use incular_core::diagnostic_property::DiagnosticsTreeStyle;

    let property = DiagnosticsProperty::integer("Example", Some(0))
        .if_null("is null")
        .show_name(false)
        .default_value(Some(1))
        .style(DiagnosticsTreeStyle::None)
        .level(Level::Off);
    assert_eq!(property.value(), Some(&0));
    assert!(!property.shows_name());
    assert_eq!(property.effective_level(), Level::Off);
    let json = property.to_json();
    assert_eq!(json["ifNull"], "is null");
    assert_eq!(json["showName"], false);
    assert_eq!(json["defaultValue"], "1");
    assert_eq!(json["style"], "none");
    assert_eq!(json["level"], "off");
}

#[test]
fn color_property_publishes_packed_channels_only_for_a_present_color() {
    use incular_core::Color;
    let color = Color::from_argb32(0x0a141e28);
    let property = DiagnosticsProperty::color("foo", Some(color));
    let json = property.to_json();
    assert_eq!(json["type"], "ColorProperty");
    assert_eq!(json["propertyType"], "Color");
    assert_eq!(
        json["valueProperties"],
        json!({"alpha": 10, "red": 20, "green": 30, "blue": 40})
    );
    assert!(json.get("value").is_none());

    let absent = DiagnosticsProperty::color("foo", None).to_json();
    assert!(absent.get("valueProperties").is_none());
    let transparent = DiagnosticsProperty::color("foo", Some(Color::TRANSPARENT)).to_json();
    assert_eq!(
        transparent["valueProperties"],
        json!({"alpha": 0, "red": 0, "green": 0, "blue": 0})
    );
    let ordinary_color = DiagnosticsProperty::new("foo", Some(color)).to_json();
    assert!(ordinary_color.get("valueProperties").is_none());
}
