use incular_text::{TextEditingValue, TextRange};

#[test]
fn composing_validity_requires_a_present_ordered_range_within_the_text() {
    assert!(!TextEditingValue::empty().is_composing_range_valid());
    for (range, expected) in [
        (TextRange::new(1, 0), false),
        (TextRange::new(1, 5), false),
        // Rust represents Flutter's negative or otherwise unrepresentable
        // IME endpoints as absent, or rejects them at the byte boundary.
        (TextRange::new(usize::MAX, 4), false),
        (TextRange::new(1, 4), true),
        (TextRange::collapsed(4), true),
    ] {
        let value = TextEditingValue {
            composing: Some(range),
            ..TextEditingValue::new("test")
        };
        assert_eq!(value.is_composing_range_valid(), expected, "{range:?}");
    }
}

#[test]
fn composition_requires_representable_byte_boundaries() {
    let text = "a😀b";
    for (range, valid) in [
        (TextRange::new(1, 5), true),
        (TextRange::collapsed(5), true),
        (TextRange::collapsed(6), true),
        (TextRange::new(1, 4), false),
        (TextRange::collapsed(2), false),
    ] {
        let value = TextEditingValue {
            composing: Some(range),
            ..TextEditingValue::new(text)
        };
        assert_eq!(value.is_composing_range_valid(), valid, "{range:?}");
        assert_eq!(range.is_valid_for(text), valid);
    }
}
