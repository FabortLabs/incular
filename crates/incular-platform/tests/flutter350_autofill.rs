use std::hash::{DefaultHasher, Hash, Hasher};

use incular_platform::{
    TextInputClientId,
    autofill::{
        AutofillConfiguration, AutofillEditingValue, AutofillFieldConfiguration,
        AutofillMessageError, AutofillSelectionAffinity, FlutterAutofillScope,
    },
    message_codecs::{JsonMethodCodec, MethodCall},
};
use serde_json::{Value, json};

fn field(tag: &str, text: &str) -> AutofillFieldConfiguration {
    let mut input = json!({
        "inputType": {"name": "TextInputType.text", "signed": false, "decimal": false},
        "inputAction": "TextInputAction.done",
        "obscureText": false,
        "autocorrect": true,
        "enableSuggestions": true,
        "enableDeltaModel": false,
    })
    .as_object()
    .unwrap()
    .clone();
    // The focus-specific options for the active client stay intact.
    input.insert("clientOption".into(), json!(tag));
    AutofillFieldConfiguration {
        input,
        autofill: AutofillConfiguration::new(tag, [tag], AutofillEditingValue::new(text)),
    }
}

fn encoded(method: &str, arguments: Value) -> Vec<u8> {
    JsonMethodCodec::encode_method_call(&MethodCall {
        method: method.into(),
        arguments,
    })
    .unwrap()
}

#[test]
fn empty_hints_remain_enabled_and_disabled_omits_autofill() {
    let config = AutofillConfiguration::new("id", [] as [&str; 0], AutofillEditingValue::default());
    assert!(config.enabled);
    let json = config.to_json().unwrap().unwrap();
    assert_eq!(json["hints"], json!([]));
    assert!(json.get("hintText").is_none());
    assert_eq!(json["editingValue"]["selectionBase"], -1);
    assert!(!AutofillConfiguration::disabled().enabled);
    assert!(
        AutofillConfiguration::disabled()
            .to_json()
            .unwrap()
            .is_none()
    );
}

#[test]
fn configuration_equality_and_hash_include_ordered_hints_state_and_optional_label() {
    let create = || {
        AutofillConfiguration::new(
            "id1".to_owned(),
            vec!["client1".to_owned()],
            AutofillEditingValue::default(),
        )
        .hint_text("hint")
    };
    let one = create();
    let two = create();
    assert_eq!(one, two);
    let hash = |config: &AutofillConfiguration| {
        let mut state = DefaultHasher::new();
        config.hash(&mut state);
        state.finish()
    };
    assert_eq!(hash(&one), hash(&two));
    assert_eq!(one.to_json().unwrap().unwrap()["hintText"], "hint");
    let mut other = two.clone();
    other.current_editing_value.text = "different".into();
    assert_ne!(one, other);
    assert_ne!(one, two.hint_text("other hint"));
}

#[test]
fn sends_all_enabled_fields_then_routes_a_nonfocused_tag_with_sender_zero() {
    let mut scope = FlutterAutofillScope::default();
    let client1 = field("client1", "test1");
    let client2 = field("client2", "test2");
    assert!(scope.register(client1.clone()));
    assert!(scope.register(client2.clone()));
    let payload = scope
        .encode_set_client(TextInputClientId::new(1), &client1)
        .unwrap();
    let call = JsonMethodCodec::decode_method_call(&payload).unwrap();
    assert_eq!(call.method, "TextInput.setClient");
    assert_eq!(call.arguments[0], 1);
    let config = &call.arguments[1];
    assert_eq!(config["autofill"]["uniqueIdentifier"], "client1");
    assert_eq!(config["clientOption"], "client1");
    assert_eq!(config["fields"].as_array().unwrap().len(), 2);
    for (index, (tag, text)) in [("client1", "test1"), ("client2", "test2")]
        .into_iter()
        .enumerate()
    {
        let item = &config["fields"][index];
        assert_eq!(item["autofill"]["uniqueIdentifier"], tag);
        assert_eq!(item["autofill"]["hints"], json!([tag]));
        assert_eq!(
            item["autofill"]["editingValue"],
            AutofillEditingValue::new(text).to_json().unwrap()
        );
        assert_eq!(item["clientOption"], tag);
        assert_eq!(item["enableDeltaModel"], false);
    }

    let mut seen = Vec::new();
    assert!(
        scope
            .handle_tagged_update(
                &encoded(
                    "TextInputClient.updateEditingStateWithTag",
                    json!([0, {"client2": AutofillEditingValue::new("Text 2").to_json().unwrap()}]),
                ),
                |tag, value| seen.push((tag.to_owned(), value))
            )
            .unwrap()
    );
    assert_eq!(
        seen,
        [("client2".into(), AutofillEditingValue::new("Text 2"))]
    );
}

#[test]
fn removed_unknown_and_disabled_clients_are_never_updated() {
    let mut scope = FlutterAutofillScope::default();
    let live = field("live", "");
    let mut disabled = field("disabled", "");
    disabled.autofill.enabled = false;
    assert!(scope.register(live.clone()));
    assert!(scope.register(field("removed", "")));
    assert!(scope.register(disabled));
    assert!(!scope.register(live.clone()));
    assert!(scope.unregister("removed"));
    assert!(!scope.unregister("removed"));

    let method = "TextInputClient.updateEditingStateWithTag";
    let mut received = Vec::new();
    scope
        .handle_tagged_update(
            &encoded(
                method,
                json!([999, {
                    "live": {"text":"okay"}, "removed":{"text":"hidden"},
                    "unknown":{"text":"hidden"}, "disabled":{"text":"hidden"}
                }]),
            ),
            |tag, value| received.push((tag.to_owned(), value.text)),
        )
        .unwrap();
    assert_eq!(received, [("live".into(), "okay".into())]);
    assert!(
        !scope
            .handle_tagged_update(
                &encoded("TextInputClient.updateEditingState", json!([])),
                |_, _| panic!("wrong method")
            )
            .unwrap()
    );
    let err = scope
        .handle_tagged_update(
            &encoded(
                method,
                json!([0, {
                    "live":{"text":"valid"}, "malformed":{"text":"bad", "selectionBase":99}
                }]),
            ),
            |_, _| panic!("malformed batch must not partially apply"),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        AutofillMessageError::InvalidField("selection range")
    ));
}

#[test]
fn wire_values_preserve_utf16_backward_selection_and_only_log_redacted_text() {
    let state = AutofillEditingValue {
        text: "A😃z".into(),
        selection: (4, 1),
        composing: (1, 3),
        selection_affinity: AutofillSelectionAffinity::Upstream,
        selection_is_directional: true,
    };
    let wire = state.to_json().unwrap();
    assert_eq!(wire["selectionBase"], 4);
    assert_eq!(wire["selectionExtent"], 1);
    assert_eq!(wire["selectionAffinity"], "TextAffinity.upstream");
    assert_eq!(AutofillEditingValue::from_json(&wire).unwrap(), state);
    assert!(!format!("{state:?}").contains("A😃z"));
    for save in [false, true] {
        let call = JsonMethodCodec::decode_method_call(
            &FlutterAutofillScope::encode_finish(save).unwrap(),
        )
        .unwrap();
        assert_eq!(call.method, "TextInput.finishAutofillContext");
        assert_eq!(call.arguments, Value::Bool(save));
    }
}
