use incular_platform::{
    ClipboardChannelError as Error, FlutterClipboardCodec as Clipboard,
    message_codecs::JsonMethodCodec,
};
use serde_json::json;

#[test]
fn get_and_set_plain_text_use_the_actual_flutter_platform_methods() {
    assert_eq!(Clipboard::CHANNEL, "flutter/platform");
    let request = Clipboard::get_data_request(Clipboard::TEXT_PLAIN).unwrap();
    let request = JsonMethodCodec::decode_method_call(&request).unwrap();
    assert_eq!(request.method, "Clipboard.getData");
    assert_eq!(request.arguments, json!("text/plain"));
    assert_eq!(
        Clipboard::decode_get_data_reply(Some(br#"[{"text":"Hello world"}]"#)).unwrap(),
        Some("Hello world".to_owned())
    );

    let set = Clipboard::set_data_request("Hello world").unwrap();
    let set = JsonMethodCodec::decode_method_call(&set).unwrap();
    assert_eq!(set.method, "Clipboard.setData");
    assert_eq!(set.arguments, json!({"text": "Hello world"}));
}

#[test]
fn a_null_reply_is_empty_but_missing_or_null_fields_are_invalid() {
    assert_eq!(Clipboard::decode_get_data_reply(None).unwrap(), None);
    assert_eq!(
        Clipboard::decode_get_data_reply(Some(b"[null]")).unwrap(),
        None
    );
    assert!(matches!(
        Clipboard::decode_get_data_reply(Some(b"[{}]")),
        Err(Error::MissingText)
    ));
    assert!(matches!(
        Clipboard::decode_get_data_reply(Some(br#"[{"text":null}]"#)),
        Err(Error::NullText)
    ));
    assert!(matches!(
        Clipboard::decode_get_data_reply(Some(br#"[{"text":123}]"#)),
        Err(Error::InvalidText)
    ));
    assert_eq!(
        Clipboard::decode_get_data_reply(Some(br#"[{"text":""}]"#)).unwrap(),
        Some(String::new())
    );
}

#[test]
fn has_strings_has_a_boolean_result_not_a_truthiness_coercion() {
    let request = Clipboard::has_strings_request().unwrap();
    let request = JsonMethodCodec::decode_method_call(&request).unwrap();
    assert_eq!(request.method, "Clipboard.hasStrings");
    assert_eq!(request.arguments, json!("text/plain"));
    assert!(!Clipboard::decode_has_strings_reply(None).unwrap());
    assert!(!Clipboard::decode_has_strings_reply(Some(b"[null]")).unwrap());
    assert!(Clipboard::decode_has_strings_reply(Some(br#"[{"value":true}]"#)).unwrap());
    assert!(!Clipboard::decode_has_strings_reply(Some(br#"[{"value":false}]"#)).unwrap());
    assert!(matches!(
        Clipboard::decode_has_strings_reply(Some(br#"[{"value":"true"}]"#)),
        Err(Error::InvalidPresence)
    ));
}
