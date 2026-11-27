use incular_platform::{FlutterTextInputConfiguration, SmartDashesType, SmartQuotesType};

#[test]
fn equality_uses_effective_smart_punctuation_defaults() {
    let implicit = FlutterTextInputConfiguration::default();
    let explicit = FlutterTextInputConfiguration {
        smart_dashes_type: Some(SmartDashesType::Enabled),
        smart_quotes_type: Some(SmartQuotesType::Enabled),
        ..FlutterTextInputConfiguration::default()
    };
    assert_eq!(implicit.to_json(), explicit.to_json());
    assert_eq!(implicit, explicit);
}

fn upstream_example() -> FlutterTextInputConfiguration {
    FlutterTextInputConfiguration {
        view_id: Some(1),
        action_label: Some("label1".into()),
        smart_dashes_type: Some(SmartDashesType::Enabled),
        smart_quotes_type: Some(SmartQuotesType::Enabled),
        allowed_mime_types: vec!["text/plain".into(), "application/pdf".into()],
        ..FlutterTextInputConfiguration::default()
    }
}

fn hash(config: &FlutterTextInputConfiguration) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    config.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn no_op_copy_keeps_original_fields_and_wire_values() {
    let original = upstream_example();
    let copy = original.copy_with(|_| {});
    assert_eq!(original, copy);
    assert_eq!(original.to_json(), copy.to_json());
    assert_eq!(copy.view_id, Some(1));
    assert_eq!(copy.action_label.as_deref(), Some("label1"));
    assert_eq!(copy.allowed_mime_types, ["text/plain", "application/pdf"]);
    assert_eq!(copy.smart_dashes_type, Some(SmartDashesType::Enabled));
    assert_eq!(copy.smart_quotes_type, Some(SmartQuotesType::Enabled));
}

#[test]
fn equal_configurations_have_equal_hashes_and_are_interchangeable_keys() {
    let original = upstream_example();
    assert_eq!(hash(&original), hash(&original.copy_with(|_| {})));

    let implicit = FlutterTextInputConfiguration::default();
    let explicit = implicit.copy_with(|_| {});
    assert_eq!(hash(&implicit), hash(&explicit));
    let by_configuration = std::collections::HashMap::from([(implicit, "original")]);
    assert_eq!(by_configuration.get(&explicit), Some(&"original"));

    let reversed = original.copy_with(|copy| copy.allowed_mime_types.reverse());
    assert_ne!(original, reversed);
}

#[test]
fn changing_obscure_mode_in_a_copy_keeps_source_punctuation_unless_overridden() {
    let original = FlutterTextInputConfiguration::default();
    let copy = original.copy_with(|copy| copy.obscure_text = true);
    let fresh_obscured = FlutterTextInputConfiguration {
        obscure_text: true,
        ..FlutterTextInputConfiguration::default()
    };

    assert_eq!(copy.to_json()["smartDashesType"], "1");
    assert_eq!(copy.to_json()["smartQuotesType"], "1");
    assert_eq!(fresh_obscured.to_json()["smartDashesType"], "0");
    assert_eq!(fresh_obscured.to_json()["smartQuotesType"], "0");
    assert_ne!(copy, fresh_obscured);
    assert!(!original.obscure_text);

    let cleared = copy.copy_with(|copy| {
        copy.smart_dashes_type = None;
        copy.smart_quotes_type = None;
    });
    assert_eq!(cleared, fresh_obscured);
    assert_eq!(hash(&cleared), hash(&fresh_obscured));

    let visible_again = fresh_obscured.copy_with(|copy| copy.obscure_text = false);
    assert_eq!(visible_again.to_json()["smartDashesType"], "0");
    assert_eq!(visible_again.to_json()["smartQuotesType"], "0");
}
