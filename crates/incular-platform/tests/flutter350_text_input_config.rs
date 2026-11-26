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
