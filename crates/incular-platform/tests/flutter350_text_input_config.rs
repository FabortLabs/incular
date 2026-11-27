use incular_config::Brightness;
use incular_platform::{
    FlutterTextInputConfiguration, SmartDashesType, TextCapitalization, TextInputAction,
    TextInputType,
};
use serde_json::{Value, json};

#[test]
fn keyboard_wire_defaults_match_flutter_without_opting_into_deltas() {
    let config = FlutterTextInputConfiguration::default();
    assert_eq!(config.input_type, TextInputType::Text);
    assert!(!config.read_only && !config.obscure_text && !config.enable_delta_model);
    assert!(config.autocorrect);
    assert_eq!(config.action_label, None);
    assert_eq!(config.text_capitalization, TextCapitalization::None);
    assert_eq!(config.keyboard_appearance, Brightness::Light);
    assert_eq!(config.enable_inline_prediction, None);
    assert_eq!(
        Value::Object(config.to_json()),
        json!({
            "viewId": null,
            "inputType": {"name":"TextInputType.text","signed":null,"decimal":null,"password":null},
            "readOnly": false,
            "obscureText": false,
            "autocorrect": true,
            "smartDashesType": "1",
            "smartQuotesType": "1",
            "enableSuggestions": true,
            "enableInteractiveSelection": true,
            "actionLabel": null,
            "inputAction": "TextInputAction.done",
            "textCapitalization": "TextCapitalization.none",
            "keyboardAppearance": "Brightness.light",
            "enableIMEPersonalizedLearning": true,
            "contentCommitMimeTypes": [],
            "enableDeltaModel": false,
            "hintLocales": [],
            "enableInlinePrediction": null,
        })
    );
}

#[test]
fn text_serializes_null_type_flags_and_resolves_password_substitutions() {
    let mut config = FlutterTextInputConfiguration {
        read_only: true,
        obscure_text: true,
        autocorrect: false,
        action_label: Some("xyzzy".into()),
        ..FlutterTextInputConfiguration::default()
    };
    let fields = config.to_json();
    assert_eq!(
        fields["inputType"],
        json!({"name":"TextInputType.text","signed":null,"decimal":null,"password":null})
    );
    assert_eq!(fields["readOnly"], json!(true));
    assert_eq!(fields["obscureText"], json!(true));
    assert_eq!(fields["autocorrect"], json!(false));
    assert_eq!(fields["actionLabel"], json!("xyzzy"));
    assert_eq!(fields["enableInlinePrediction"], Value::Null);
    assert_eq!(fields["smartDashesType"], json!("0"));
    assert_eq!(fields["smartQuotesType"], json!("0"));
    config.smart_dashes_type = Some(SmartDashesType::Enabled);
    assert_eq!(
        config.to_json()["smartDashesType"],
        json!("1"),
        "explicit override"
    );
}

#[test]
fn numeric_type_keeps_decimal_flag_separate_from_obscured_content() {
    let fields = FlutterTextInputConfiguration {
        input_type: TextInputType::number_with_options(false, true),
        obscure_text: true,
        autocorrect: false,
        action_label: Some("xyzzy".into()),
        ..FlutterTextInputConfiguration::default()
    }
    .to_json();
    assert_eq!(
        fields["inputType"],
        json!({"name":"TextInputType.number","signed":false,"decimal":true,"password":false})
    );
    assert_eq!(fields["readOnly"], json!(false));
    assert_eq!(fields["obscureText"], json!(true));
    assert_eq!(fields["autocorrect"], json!(false));
    assert_eq!(fields["actionLabel"], json!("xyzzy"));
}

#[test]
fn numeric_password_changes_the_keyboard_policy_not_the_editor_content_flag() {
    let fields = FlutterTextInputConfiguration {
        input_type: TextInputType::number_password(false, false),
        input_action: TextInputAction::Go,
        ..FlutterTextInputConfiguration::default()
    }
    .to_json();
    assert_eq!(
        fields["inputType"],
        json!({"name":"TextInputType.number","signed":false,"decimal":false,"password":true})
    );
    assert_eq!(fields["obscureText"], json!(false));
    assert_eq!(fields["inputAction"], json!("TextInputAction.go"));
}

#[test]
fn explicit_noncompletion_actions_keep_their_wire_identity() {
    use TextInputAction::*;
    for (action, wire) in [
        (Unspecified, "TextInputAction.unspecified"),
        (None, "TextInputAction.none"),
        (ContinueAction, "TextInputAction.continueAction"),
        (Join, "TextInputAction.join"),
        (Route, "TextInputAction.route"),
        (EmergencyCall, "TextInputAction.emergencyCall"),
    ] {
        let config = FlutterTextInputConfiguration {
            input_action: action,
            ..Default::default()
        };
        assert_eq!(config.to_json()["inputAction"], wire, "{action:?}");
    }
}

#[test]
fn android_and_web_project_every_keyboard_action_to_its_native_vocabulary() {
    use TextInputAction::*;
    for (action, android, web) in [
        (Unspecified, 0, Option::None),
        (None, 1, Option::None),
        (Newline, 1, Some("enter")),
        (Go, 2, Some("go")),
        (Search, 3, Some("search")),
        (Send, 4, Some("send")),
        (Next, 5, Some("next")),
        (Done, 6, Some("done")),
        (Previous, 7, Some("previous")),
        (ContinueAction, 0, Some("next")),
        (Join, 0, Option::None),
        (Route, 0, Option::None),
        (EmergencyCall, 0, Option::None),
    ] {
        assert_eq!(action.android_ime_action(), android, "{action:?}");
        assert_eq!(action.web_enter_key_hint(), web, "{action:?}");
    }
}

#[test]
fn every_keyboard_type_keeps_its_flutter_public_index_and_nullable_number_flags() {
    use incular_platform::TextInputType as Type;
    let roles = [
        (Type::Text, "text"),
        (Type::Multiline, "multiline"),
        (Type::NUMBER, "number"),
        (Type::Phone, "phone"),
        (Type::DateTime, "datetime"),
        (Type::Email, "emailAddress"),
        (Type::Url, "url"),
        (Type::Password, "visiblePassword"),
        (Type::Name, "name"),
        (Type::StreetAddress, "address"),
        (Type::None, "none"),
        (Type::WebSearch, "webSearch"),
        (Type::Twitter, "twitter"),
    ];
    for (index, (role, name)) in roles.into_iter().enumerate() {
        assert_eq!(usize::from(role.flutter_index()), index);
        assert_eq!(
            role.to_flutter_json()["name"],
            format!("TextInputType.{name}")
        );
        let flags = if role == Type::NUMBER {
            "false"
        } else {
            "null"
        };
        assert_eq!(
            role.to_string(),
            format!(
                "TextInputType(name: TextInputType.{name}, signed: {flags}, decimal: {flags}, password: {flags})"
            )
        );
    }
    for (input, signed, decimal, password) in [
        (Type::NUMBER, false, false, false),
        (Type::number_with_options(true, false), true, false, false),
        (Type::number_with_options(false, true), false, true, false),
        (Type::number_with_options(true, true), true, true, false),
        (Type::number_password(false, false), false, false, true),
        (Type::number_password(true, true), true, true, true),
    ] {
        assert_eq!(input.flutter_index(), 2);
        let json = input.to_flutter_json();
        assert_eq!(json["signed"], signed);
        assert_eq!(json["decimal"], decimal);
        assert_eq!(json["password"], password);
        assert_eq!(
            input.to_string(),
            format!(
                "TextInputType(name: TextInputType.number, signed: {signed}, decimal: {decimal}, password: {password})"
            )
        );
    }
}

#[test]
fn numeric_keyboard_equality_and_hash_keep_each_option_independent() {
    use incular_platform::TextInputType as Type;

    let unsigned = Type::NUMBER;
    let signed = Type::number_with_options(true, false);
    let decimal = Type::number_with_options(false, true);
    let signed_decimal = Type::number_with_options(true, true);
    let secret = Type::number_password(false, false);
    let signed_decimal_secret = Type::number_password(true, true);
    let hash = |value: Type| {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&value, &mut hasher);
        std::hash::Hasher::finish(&hasher)
    };
    for (a, b) in [
        (unsigned, Type::NUMBER),
        (signed, Type::number_with_options(true, false)),
        (secret, Type::number_password(false, false)),
    ] {
        assert_eq!(a, b);
        assert_eq!(hash(a), hash(b));
    }
    for (a, b) in [
        (Type::Text, unsigned),
        (unsigned, signed),
        (unsigned, secret),
        (signed, decimal),
        (signed, signed_decimal),
        (decimal, signed_decimal),
        (secret, signed_decimal_secret),
        (signed_decimal, signed_decimal_secret),
    ] {
        assert_ne!(a, b);
        assert_ne!(hash(a), hash(b));
    }
}

#[test]
fn wire_editor_debug_description_uses_signed_sentinels_without_changing_default_redaction() {
    use incular_platform::autofill::AutofillEditingValue;
    for text in ["", "Sample Text"] {
        let editor = AutofillEditingValue::new(text);
        assert_eq!(
            editor.flutter_debug_description(),
            format!(
                "TextEditingValue(text: ┤{text}├, selection: TextSelection.invalid, composing: TextRange(start: -1, end: -1))"
            )
        );
        assert!(format!("{editor:?}").contains("[redacted]"));
        assert_eq!(editor.selection, (-1, -1));
    }
}

#[test]
fn native_keyboard_roles_use_flutter_none_search_and_twitter_mappings() {
    use incular_platform::{
        TextInputAction, TextInputClientId, TextInputConfiguration, TextInputTraits,
        TextInputType as Type,
    };
    let mut configuration = TextInputConfiguration {
        client: TextInputClientId::new(1),
        input_type: Type::None,
        action: TextInputAction::Done,
        multiline: false,
        enabled: true,
        read_only: false,
        obscure_text: false,
        traits: TextInputTraits {
            autocorrect: false,
            ..TextInputTraits::default()
        },
    };
    for (input, expected, mode, soft) in [
        (Type::None, 0x0, "none", false),
        (Type::Twitter, 0x21, "text", true),
        (Type::WebSearch, 0x11, "text", true),
        (Type::Email, 0x21, "email", true),
        (Type::Url, 0x11, "url", true),
    ] {
        configuration.input_type = input;
        assert_eq!(
            incular_platform::flutter_android_input_type(&configuration),
            expected
        );
        assert_eq!(input.flutter_html_input_mode(), mode);
        assert_eq!(input.uses_system_keyboard(), soft);
    }
}
