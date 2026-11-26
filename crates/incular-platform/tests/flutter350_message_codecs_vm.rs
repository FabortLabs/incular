use incular_platform::message_codecs::{JsonMessageCodec, MessageValue as V, StandardMessageCodec};
use serde_json::{Value, json};

const BIG: i64 = -7_000_000_000_000_000_007;

fn json_roundtrip(message: Value) {
    let encoded = JsonMessageCodec::encode_message(&message).unwrap().unwrap();
    assert_eq!(
        JsonMessageCodec::decode_message(Some(&encoded)).unwrap(),
        message
    );
}

fn standard_roundtrip(message: V) {
    let encoded = StandardMessageCodec::encode_message(&message)
        .unwrap()
        .unwrap();
    assert_eq!(
        StandardMessageCodec::decode_message(Some(&encoded)).unwrap(),
        message
    );
}

#[test]
fn json_keeps_signed_vm_endpoints_without_float_conversion() {
    for value in [i64::MAX, -i64::MAX] {
        json_roundtrip(json!(value));
        let encoded = JsonMessageCodec::encode_message(&json!(value))
            .unwrap()
            .unwrap();
        assert_eq!(std::str::from_utf8(&encoded).unwrap(), value.to_string());
    }
}

#[test]
fn json_list_keeps_the_exact_large_negative_integer() {
    json_roundtrip(json!([BIG]));
}

#[test]
fn standard_integer_wire_tags_and_bytes_match_the_closed_i32_interval() {
    for (value, compact) in [
        (i64::from(i32::MIN), true),
        (i64::from(i32::MIN) - 1, false),
        (i64::from(i32::MAX), true),
        (i64::from(i32::MAX) + 1, false),
        (i64::MIN, false),
        (i64::MIN.wrapping_sub(1), false),
        (i64::MAX, false),
        (i64::MAX.wrapping_add(1), false),
    ] {
        let mut expected = vec![if compact { 3 } else { 4 }];
        if compact {
            expected.extend_from_slice(&(value as i32).to_ne_bytes());
        } else {
            expected.extend_from_slice(&value.to_ne_bytes());
        }

        let bytes = StandardMessageCodec::encode_message(&V::Int(value))
            .unwrap()
            .unwrap();
        assert_eq!(bytes, expected, "{value}");
        assert_eq!(
            StandardMessageCodec::decode_message(Some(&bytes)),
            Ok(V::Int(value))
        );
    }
}

#[test]
fn standard_roundtrips_signed_vm_endpoints() {
    standard_roundtrip(V::Int(i64::MAX));
    standard_roundtrip(V::Int(-i64::MAX));
}

#[test]
fn standard_list_preserves_large_values_and_signed_64_bit_typed_storage() {
    standard_roundtrip(V::List(vec![
        V::Int(BIG),
        V::Int64List(vec![i64::MIN, 0, i64::MAX]),
    ]));
}
