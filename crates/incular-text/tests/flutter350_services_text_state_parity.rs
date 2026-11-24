//! Text-state comparisons against pinned Flutter 3.50.0 services tests.
//! These exercise public value operations, not Flutter's channel transport.

use incular_text::{TextEditingDelta, TextEditingValue, TextRange, TextSelection};

const TEXT: &str = "From a false proposition, anything follows.";

fn reversed_selection() -> TextEditingValue {
    TextEditingValue::new(TEXT).with_selection(TextSelection::new(13, 5))
}

// Flutter services/text_input_test.dart:55.
#[test]
fn reversed_selection_deletion_collapses_to_the_start_of_the_removed_range() {
    let edit = reversed_selection();
    let range = edit.selection.range();
    let result = edit.replace(range, "");
    assert_eq!(
        result,
        TextEditingValue::new("From proposition, anything follows.")
            .with_selection(TextSelection::collapsed(5))
    );
}

// Flutter services/text_input_test.dart:78.
#[test]
fn replacing_before_selection_shifts_both_offsets_without_losing_direction() {
    let result = reversed_selection().replace(TextRange::new(4, 5), "AA");
    assert_eq!(result.text, "FromAAa false proposition, anything follows.");
    assert_eq!(result.selection, TextSelection::new(14, 6));
    assert_eq!(result.composing, None);
}

// Flutter services/text_input_test.dart:94.
#[test]
fn replacing_after_selection_keeps_both_offsets_and_direction() {
    let result = reversed_selection().replace(TextRange::new(13, 14), "AA");
    assert_eq!(result.text, "From a false AAroposition, anything follows.");
    assert_eq!(result.selection, TextSelection::new(13, 5));
    assert_eq!(result.composing, None);
}

#[test]
fn composing_and_reversed_selection_keep_the_same_utf16_offsets_in_utf8() {
    let value = TextEditingValue::new("éaq")
        .with_selection(TextSelection::new(3, 2))
        .with_composing(Some(TextRange::new(2, 3)));
    let edited = value.replace(TextRange::new(0, 3), "b中");
    assert_eq!(edited.text, "b中q");
    assert_eq!(edited.selection, TextSelection::new(4, 1));
    assert_eq!(edited.composing, Some(TextRange::new(1, 4)));
}

#[test]
fn insertion_before_a_composing_range_shifts_both_of_its_ends() {
    let value = TextEditingValue::new("éa🙂")
        .with_selection(TextSelection::collapsed(3))
        .with_composing(Some(TextRange::new(2, 7)));
    let edited = value.replace(TextRange::collapsed(2), "中");
    assert_eq!(edited.text, "é中a🙂");
    assert_eq!(edited.selection, TextSelection::collapsed(6));
    assert_eq!(edited.composing, Some(TextRange::new(5, 10)));
}

#[test]
fn equal_bytes_can_change_utf16_length_and_replace_an_interior_caret() {
    let value = TextEditingValue::new("abcd!")
        .with_selection(TextSelection::collapsed(2))
        .with_composing(Some(TextRange::new(1, 3)));
    let edited = value.replace(TextRange::new(0, 4), "🙂");
    assert_eq!(edited.text, "🙂!");
    assert_eq!(edited.selection, TextSelection::collapsed(4));
    assert_eq!(edited.composing, Some(TextRange::collapsed(4)));
}

// Payload behavior from services/text_editing_delta_test.dart:40. Rust
// computes the smallest delta from whole values rather than decoding JSON.
// The old text is an argument, not a field stored on the resulting delta.
#[test]
fn delta_at_end_of_composition_inserts_only_the_added_character() {
    let old = TextEditingValue::new("hello worl");
    let new = TextEditingValue::new("hello world").with_composing(Some(TextRange::new(6, 11)));
    let delta = TextEditingDelta::between(&old.text, &new);

    assert_eq!(delta.range, TextRange::collapsed(10));
    assert_eq!(delta.replacement, "d");
    assert_eq!(delta.selection, Some(TextSelection::collapsed(11)));
    assert_eq!(delta.composing, Some(TextRange::new(6, 11)));
    assert_eq!(delta.apply(&old), new);
}
