//! Native binary, string and JSON equivalents for Flutter 3.50.0 codecs.
use incular_platform::message_codecs::{BinaryCodec, JsonMessageCodec, StringCodec};
use serde_json::{Value, json};

#[test]
fn binary_codec_is_an_exact_borrowed_view() {
    let signed = (-7_i32).to_be_bytes();
    for value in [None, Some([].as_slice()), Some(signed.as_slice())] {
        let result = BinaryCodec::decode_message(BinaryCodec::encode_message(value));
        assert_eq!(result, value);
        if let (Some(actual), Some(expected)) = (result, value) {
            assert!(std::ptr::eq(actual, expected));
        }
    }
}

#[test]
fn utf8_codec_distinguishes_absent_empty_and_unicode_values() {
    for value in [None, Some(""), Some("hello"), Some("special chars >☺😂<")] {
        let bytes = StringCodec::encode_message(value);
        assert_eq!(bytes, value.map(str::as_bytes));
        assert_eq!(StringCodec::decode_message(bytes).unwrap(), value);
    }
    assert_eq!(
        StringCodec::decode_message(Some(b"\xef\xbb\xbfhello")).unwrap(),
        Some("hello")
    );
    assert!(StringCodec::decode_message(Some(&[0xff])).is_err());
}

#[test]
fn string_codec_does_not_decode_outside_a_slice() {
    let wire = StringCodec::encode_message(Some("hello world")).unwrap();
    assert_eq!(
        StringCodec::decode_message(Some(&wire[5..])).unwrap(),
        Some(" world")
    );
}

fn assert_json_roundtrip(value: Value) {
    let bytes = JsonMessageCodec::encode_message(&value).unwrap();
    assert_eq!(
        JsonMessageCodec::decode_message(bytes.as_deref()).unwrap(),
        value
    );
}

#[test]
fn json_roundtrips_pinned_scalar_values() {
    for value in [
        Value::Null,
        json!(true),
        json!(false),
        json!(7),
        json!(-7),
        json!(98_742_923_489_i64),
        json!(-98_742_923_489_i64),
        json!(314.0 / 100.0),
        json!(""),
        json!("hello"),
        json!("special chars >☺😂<"),
    ] {
        assert_json_roundtrip(value);
    }
    assert_eq!(
        JsonMessageCodec::encode_message(&Value::Null).unwrap(),
        None
    );
}

#[test]
fn json_roundtrips_pinned_composite_values() {
    assert_json_roundtrip(json!([
        null, true, false, -707, -7_000_000_007_i64, -314.0 / 100.0, "", "hello",
        ["nested", []], {"a": "nested", "b": {}}, "world"
    ]));
}

#[test]
fn json_utf8_crosses_the_reference_chunk_boundary_and_honors_slice_bounds() {
    let string = format!("{}😂", "a".repeat(254));
    let value = json!(string);
    let bytes = JsonMessageCodec::encode_message(&value).unwrap().unwrap();
    let reference = format!("\"{string}\"").into_bytes();
    assert_eq!(bytes, reference);
    let mut padded = vec![b'['];
    padded.extend_from_slice(&reference);
    padded.push(b']');
    assert_eq!(
        JsonMessageCodec::decode_message(Some(&padded[1..1 + reference.len()])).unwrap(),
        value
    );
}
