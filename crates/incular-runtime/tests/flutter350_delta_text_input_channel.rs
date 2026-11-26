//! Pinned Flutter 3.50 services/delta_text_input_test.dart channel behavior.

use incular_platform::{
    TextInputClientId,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use incular_runtime::{FlutterDeltaTextInputConnection, FlutterDeltaTextInputError};
use incular_text::{FlutterTextDeltaError, FlutterTextEdit, TextEditingValue};
use serde_json::{Value, json};

fn connection() -> FlutterDeltaTextInputConnection {
    FlutterDeltaTextInputConnection::new(TextInputClientId::new(1))
}

fn call(method: &str, args: Value) -> Vec<u8> {
    JsonMethodCodec::encode_method_call(&MethodCall {
        method: method.into(),
        arguments: args,
    })
    .unwrap()
}

fn update(sender: i64, deltas: Vec<Value>) -> Vec<u8> {
    call(
        "TextInputClient.updateEditingStateWithDeltas",
        json!([sender, {"deltas": deltas}]),
    )
}

fn delta(old: &str, text: &str, span: (i64, i64), caret: i64, composing: (i64, i64)) -> Value {
    json!({
        "oldText": old, "deltaText": text, "deltaStart": span.0, "deltaEnd": span.1,
        "selectionBase": caret, "selectionExtent": caret, "composingBase": composing.0,
        "composingExtent": composing.1, "selectionAffinity": "TextAffinity.downstream",
        "selectionIsDirectional": false,
    })
}

#[test]
fn attach_opts_in_and_a_platform_message_reaches_the_delta_client() {
    assert_eq!(
        FlutterDeltaTextInputConnection::CHANNEL,
        "flutter/textinput"
    );
    let connection = connection();
    let mut configuration = json!({"obscureText": true, "enableDeltaModel": false});
    let bytes = connection
        .encode_set_client(configuration.as_object_mut().unwrap().clone())
        .unwrap();
    let attach = JsonMethodCodec::decode_method_call(&bytes).unwrap();
    assert_eq!(attach.method, "TextInput.setClient");
    assert_eq!(
        attach.arguments,
        json!([1, {"enableDeltaModel": true, "obscureText": true}])
    );

    let mut delivered = None;
    let message = update(
        1,
        vec![delta("", "let there be text", (0, 0), 17, (-1, -1))],
    );
    assert!(
        connection
            .handle_delta_call(&message, |deltas| delivered = Some(deltas))
            .unwrap()
    );
    let batch = delivered.unwrap();
    assert_eq!(batch.len(), 1);
    assert_eq!(
        batch[0].edit,
        FlutterTextEdit::Insertion {
            offset: 0,
            text: "let there be text".into()
        }
    );
    assert_eq!(
        batch[0].try_apply(&TextEditingValue::empty()).unwrap().text,
        "let there be text"
    );
}

#[test]
fn malformed_ranges_identify_the_field_without_delivering_a_partial_batch() {
    for (invalid, field) in [
        (delta("1", "", (-1, -1), 3, (-1, -1)), "selection"),
        (delta("", "hello", (0, 1), 5, (-1, -1)), "replacement"),
        (delta("", "hello", (0, 0), 6, (-1, -1)), "selection"),
        (delta("worl", "world", (0, 4), 5, (0, 6)), "composing"),
    ] {
        let message = update(1, vec![delta("", "h", (0, 0), 1, (-1, -1)), invalid]);
        let result = connection().handle_delta_call(&message, |_| panic!("partial delivery"));
        assert!(
            matches!(result, Err(FlutterDeltaTextInputError::InvalidDelta {
            index: 1, source: FlutterTextDeltaError::InvalidRange(range)
        }) if range == field)
        );
    }
}

#[test]
fn stale_or_unrelated_calls_do_not_invoke_the_active_client() {
    let stale = update(2, vec![Value::Null]);
    assert!(
        !connection()
            .handle_delta_call(&stale, |_| panic!("stale delta"))
            .unwrap()
    );
    let method = call("TextInputClient.performAction", Value::Null);
    assert!(
        !connection()
            .handle_delta_call(&method, |_| panic!("not a delta"))
            .unwrap()
    );
    assert!(matches!(
        connection().handle_delta_call(&update(1, vec![Value::Null]), |_| panic!("bad delta")),
        Err(FlutterDeltaTextInputError::InvalidDelta { index: 0, .. })
    ));
}

#[test]
fn negative_wildcard_id_is_debug_only() {
    let message = update(-1, vec![]);
    let mut called = false;
    let handled = connection()
        .handle_delta_call(&message, |_| called = true)
        .unwrap();
    assert_eq!(handled, cfg!(debug_assertions));
    assert_eq!(called, handled);
}

#[test]
fn typed_configuration_reaches_the_channel_with_delta_model_enabled() {
    use incular_platform::{FlutterTextInputConfiguration, TextInputType};
    let config = FlutterTextInputConfiguration {
        input_type: TextInputType::number_password(false, false),
        action_label: Some("Unlock".into()),
        ..FlutterTextInputConfiguration::default()
    };
    let bytes = connection().encode_set_client_with_config(&config).unwrap();
    let call = JsonMethodCodec::decode_method_call(&bytes).unwrap();
    assert_eq!(call.method, "TextInput.setClient");
    assert_eq!(call.arguments[0], json!(1));
    assert_eq!(call.arguments[1]["actionLabel"], json!("Unlock"));
    assert_eq!(call.arguments[1]["inputType"]["password"], json!(true));
    assert_eq!(call.arguments[1]["enableDeltaModel"], json!(true));
    assert!(!config.enable_delta_model, "the attach does not modify caller configuration");
}
