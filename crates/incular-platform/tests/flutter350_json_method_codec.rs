use incular_platform::message_codecs::{
    JsonMethodCodec as Codec, JsonMethodCodecError as Error, MethodCall, PlatformException,
};
use serde_json::{Value, json};

fn wanted(trace: Option<&str>) -> PlatformException<Value> {
    PlatformException {
        code: "errorCode".into(),
        message: Some("errorMessage".into()),
        details: json!("errorDetails"),
        stacktrace: trace.map(str::to_owned),
    }
}

fn decode_error(bytes: &[u8]) -> PlatformException<Value> {
    match Codec::decode_envelope(bytes) {
        Err(Error::Platform(error)) => *error,
        other => panic!("expected a platform error, got {other:?}"),
    }
}

#[test]
fn json_envelope_decodes_without_native_stacktrace() {
    let expected = wanted(None);
    let bytes = Codec::encode_error_envelope(&expected).unwrap();
    assert_eq!(bytes, br#"["errorCode","errorMessage","errorDetails"]"#);
    assert_eq!(decode_error(&bytes), expected);
}

#[test]
fn json_envelope_decodes_a_native_stacktrace() {
    let wire = br#"["errorCode","errorMessage","errorDetails","errorStacktrace"]"#;
    let expected = wanted(Some("errorStacktrace"));
    assert_eq!(decode_error(wire), expected);
    assert_eq!(Codec::encode_error_envelope(&expected).unwrap(), wire);
}

#[test]
fn method_call_accepts_missing_arguments_and_successful_quoteable_values() {
    let call = Codec::decode_method_call(br#"{"method":"lookup"}"#).unwrap();
    assert_eq!(
        call,
        MethodCall {
            method: "lookup".into(),
            arguments: Value::Null
        }
    );
    let wire = Codec::encode_method_call(&call).unwrap();
    assert_eq!(Codec::decode_method_call(&wire).unwrap(), call);
    let result = json!({"name":"☺", "values":[1,2,3]});
    assert_eq!(
        Codec::decode_envelope(&Codec::encode_success_envelope(&result).unwrap()).unwrap(),
        result
    );
    assert!(matches!(
        Codec::decode_envelope(br#"["Error", 9, null]"#),
        Err(Error::InvalidEnvelope)
    ));
    assert!(matches!(
        Codec::decode_method_call(br#"{"method":42}"#),
        Err(Error::InvalidMethodCall)
    ));
}
