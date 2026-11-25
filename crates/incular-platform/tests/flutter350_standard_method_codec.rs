//! Standard method codec registrations in pinned Flutter 3.50.0.
use incular_platform::message_codecs::{
    MessageValue as V, MethodCall, MethodCodecError, PlatformException,
    StandardMessageCodec as MessageCodec, StandardMethodCodec as MethodCodec,
};

#[test]
fn encoded_object_retains_distinct_string_and_integer_keys() {
    let value = V::Map(vec![
        (V::String("foo".into()), V::Bool(true)),
        (V::Int(3), V::String("fizz".into())),
    ]);
    let bytes = MessageCodec::encode_message(&value).unwrap().unwrap();
    assert_eq!(MessageCodec::decode_message(Some(&bytes)), Ok(value));
}

fn failure(message: Option<&str>, stacktrace: Option<&str>) -> PlatformException {
    PlatformException {
        code: "errorCode".into(),
        message: message.map(str::to_owned),
        details: V::String("errorDetails".into()),
        stacktrace: stacktrace.map(str::to_owned),
    }
}

#[test]
fn error_envelope_preserves_code_message_and_details() {
    let wanted = failure(Some("errorMessage"), None);
    let encoded = MethodCodec::encode_error_envelope(&wanted).unwrap();
    assert_eq!(encoded[0], 1);
    assert_eq!(
        MethodCodec::decode_envelope(&encoded),
        Err(MethodCodecError::Platform(Box::new(wanted)))
    );
}

#[test]
fn error_envelope_reads_a_native_stacktrace_from_wire() {
    let mut wire = vec![1];
    for value in [
        "errorCode",
        "errorMessage",
        "errorDetails",
        "errorStacktrace",
    ] {
        wire.extend([7, value.len() as u8]);
        wire.extend_from_slice(value.as_bytes());
    }
    let expected = failure(Some("errorMessage"), Some("errorStacktrace"));
    assert_eq!(MethodCodec::encode_error_envelope(&expected).unwrap(), wire);
    assert_eq!(
        MethodCodec::decode_envelope(&wire),
        Err(MethodCodecError::Platform(Box::new(expected)))
    );
}

#[test]
fn error_envelope_can_have_no_message() {
    let wanted = failure(None, None);
    let encoded = MethodCodec::encode_error_envelope(&wanted).unwrap();
    assert_eq!(encoded[12], 0, "message field uses the null tag");
    assert_eq!(
        MethodCodec::decode_envelope(&encoded),
        Err(MethodCodecError::Platform(Box::new(wanted)))
    );
}

#[test]
fn nested_method_arguments_preserve_envelope_alignment() {
    let call = MethodCall {
        method: "foo".into(),
        arguments: V::Float(1.0),
    };
    let encoded = MethodCodec::encode_method_call(&call).unwrap();
    assert_eq!(&encoded[..8], &[7, 3, b'f', b'o', b'o', 6, 0, 0]);
    assert_eq!(&encoded[8..], &1.0_f64.to_ne_bytes());
    assert_eq!(MethodCodec::decode_method_call(&encoded), Ok(call));
    let encoded = MethodCodec::encode_success_envelope(&V::Null).unwrap();
    assert_eq!(encoded, [0, 0]);
    assert_eq!(MethodCodec::decode_envelope(&encoded), Ok(V::Null));
}

#[test]
fn invalid_method_call_and_error_fields_are_rejected() {
    assert_eq!(
        MethodCodec::decode_method_call(&[1, 0]),
        Err(MethodCodecError::InvalidMethodCall)
    );
    assert_eq!(
        MethodCodec::decode_envelope(&[]),
        Err(MethodCodecError::InvalidEnvelope)
    );
    assert_eq!(
        MethodCodec::decode_envelope(&[1, 0, 0, 0]),
        Err(MethodCodecError::InvalidEnvelope)
    );
}

#[test]
fn call_and_exception_displays_keep_the_pinned_null_diagnostics() {
    use incular_platform::message_codecs::MissingPluginException;
    assert_eq!(
        MethodCall {
            method: "sample method".into(),
            arguments: V::Null
        }
        .to_string(),
        "MethodCall(sample method, null)",
    );
    assert_eq!(
        PlatformException {
            code: "100".into(),
            message: None,
            details: V::Null,
            stacktrace: None,
        }
        .to_string(),
        "PlatformException(100, null, null, null)",
    );
    assert_eq!(
        MissingPluginException::default().to_string(),
        "MissingPluginException(null)"
    );
}
