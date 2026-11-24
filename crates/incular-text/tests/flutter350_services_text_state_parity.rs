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
