//! Exact native StandardMessageCodec cases from Flutter 3.50.0.
use incular_platform::message_codecs::{
    MessageCodecError, MessageValue as V, StandardMessageCodec as Codec,
};

fn same_message(left: &V, right: &V) -> bool {
    fn scalar(a: f64, b: f64) -> bool {
        a.to_bits() == b.to_bits() || a.is_nan() && b.is_nan()
    }
    match (left, right) {
        (V::Float(a), V::Float(b)) => scalar(*a, *b),
        (V::Float64List(a), V::Float64List(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| scalar(x, y))
        }
        (V::Float32List(a), V::Float32List(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| scalar(x.into(), y.into()))
        }
        (V::List(a), V::List(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same_message(x, y))
        }
        (V::Map(a), V::Map(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((ka, va), (kb, vb))| same_message(ka, kb) && same_message(va, vb))
        }
        _ => left == right,
    }
}

fn round_trip(value: V) {
    let encoded = Codec::encode_message(&value).unwrap();
    let decoded = Codec::decode_message(encoded.as_deref()).unwrap();
    assert!(same_message(&decoded, &value), "{decoded:?} vs {value:?}");
}

#[test]
fn byte_list_lengths_use_the_pinned_253_254_65535_65536_boundaries() {
    for (length, expected) in [
        (253, vec![8, 253]),
        (254, [vec![8, 254], 254_u16.to_ne_bytes().to_vec()].concat()),
        (65_535, vec![8, 254, 255, 255]),
        (
            65_536,
            [vec![8, 255], 65_536_u32.to_ne_bytes().to_vec()].concat(),
        ),
    ] {
        let value = V::Bytes(vec![0; length]);
        let encoded = Codec::encode_message(&value).unwrap().unwrap();
        assert_eq!(encoded.len(), expected.len() + length);
        assert_eq!(&encoded[..expected.len()], &expected);
        assert!(encoded[expected.len()..].iter().all(|&byte| byte == 0));
        assert_eq!(Codec::decode_message(Some(&encoded)), Ok(value));
    }
}

#[test]
fn simple_values_round_trip_without_losing_numeric_or_string_types() {
    for value in [
        V::Null,
        V::Bool(true),
        V::Bool(false),
        V::Int(7),
        V::Int(-7),
        V::Int(98_742_923_489),
        V::Int(-98_742_923_489),
        V::Float(314.0 / 100.0),
        V::Float(f64::INFINITY),
        V::Float(f64::NAN),
        V::String(String::new()),
        V::String("hello".into()),
        V::String("special chars >☺😂<".into()),
    ] {
        round_trip(value);
    }
    assert!(Codec::encode_message(&V::Null).unwrap().is_none());
    let encoded = |v| Codec::encode_message(&V::Int(v)).unwrap().unwrap();
    assert_eq!(encoded(i32::MAX.into())[0], 3);
    assert_eq!(encoded(i64::from(i32::MAX) + 1)[0], 4);
}

#[test]
fn composite_values_preserve_types_and_heterogeneous_keys() {
    let subnormal = f64::from_bits(1);
    let doubles = [
        f64::NEG_INFINITY,
        -f64::MAX,
        -subnormal,
        -0.0,
        0.0,
        subnormal,
        f64::MAX,
        f64::INFINITY,
        f64::NAN,
    ];
    round_trip(V::List(vec![
        V::Null,
        V::Bool(true),
        V::Bool(false),
        V::Int(-707),
        V::Int(-7_000_000_007),
        V::Float(-314.0 / 100.0),
        V::String(String::new()),
        V::String("hello".into()),
        V::Bytes(vec![0xba, 0x5e, 0xba, 0x11]),
        V::Int32List(vec![i32::MIN, 0, i32::MAX]),
        V::Null,
        V::Null,
        V::Float64List(doubles.to_vec()),
        V::Float32List(doubles.into_iter().map(|value| value as f32).collect()),
        V::List(vec![V::String("nested".into()), V::List(vec![])]),
        V::Map(vec![
            (V::String("a".into()), V::String("nested".into())),
            (V::Null, V::Map(vec![])),
        ]),
        V::String("world".into()),
    ]));
}

#[test]
fn a_double_tag_is_padded_to_eight_bytes_before_the_value() {
    let bytes = Codec::encode_message(&V::Float(1.0)).unwrap().unwrap();
    assert_eq!(&bytes[..8], &[6, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(&bytes[8..], &1.0_f64.to_ne_bytes());
    assert_eq!(Codec::decode_message(Some(&bytes)), Ok(V::Float(1.0)));
}

#[test]
fn nested_null_and_utf8_have_the_exact_reference_wire_bytes() {
    let value = V::List(vec![V::Null, V::Bool(true), V::String("☺".into())]);
    let mut storage = vec![0xee];
    storage.extend([12, 3, 0, 1, 7, 3, 0xe2, 0x98, 0xba]);
    storage.push(0xee);
    assert_eq!(
        Codec::encode_message(&value).unwrap().unwrap(),
        storage[1..10]
    );
    assert_eq!(Codec::decode_message(Some(&storage[1..10])), Ok(value));
}

#[test]
fn malformed_input_is_rejected_without_large_declared_length_allocation() {
    assert_eq!(
        Codec::decode_message(Some(&[1, 0])),
        Err(MessageCodecError::TrailingBytes)
    );
    assert_eq!(
        Codec::decode_message(Some(&[91])),
        Err(MessageCodecError::UnknownType(91))
    );
    assert_eq!(
        Codec::decode_message(Some(&[7, 1, 0xff])),
        Err(MessageCodecError::InvalidUtf8)
    );
    assert_eq!(
        Codec::decode_message(Some(&[12, 255, 255, 255, 255, 255])),
        Err(MessageCodecError::InvalidLength)
    );
}
