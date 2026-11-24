//! Comparisons with pinned Flutter 3.50.0 services/text_formatter_test.dart.
//! Selection positions use the equivalent UTF-8 byte offsets in Incular.

use incular_text::{TextEditingValue, TextRange, TextSelection};
use incular_widgets::{
    FilteringTextInputFormatter, LengthLimitingTextInputFormatter, TextInputFormatter,
};

fn selected(text: &str, base: usize, extent: usize) -> TextEditingValue {
    TextEditingValue::new(text).with_selection(TextSelection::new(base, extent))
}

// Upstream :148. A character predicate and [a-z] select the same ASCII
// matches in this fixture; neither case uses a replacement string.
#[test]
fn deny_filter_preserves_the_selected_digits_after_removing_letters() {
    let formatter = FilteringTextInputFormatter::deny(|c| c.is_ascii_lowercase());
    let value = formatter.format_edit_update(
        &TextEditingValue::empty(),
        &selected("a1b2c3\nd4e5f6", 3, 9),
    );
    assert_eq!(value.text, "123\n456");
    assert_eq!(value.selection, TextSelection::new(1, 5));
    assert_eq!(value.composing, None);
}

// Upstream :210.
#[test]
fn allow_filter_preserves_the_selected_letters_after_removing_other_characters() {
    let formatter = FilteringTextInputFormatter::allow(|c| ('a'..='c').contains(&c));
    let value = formatter.format_edit_update(
        &TextEditingValue::empty(),
        &selected("a1b2c3\nd4e5f6", 3, 9),
    );
    assert_eq!(value.text, "abc");
    assert_eq!(value.selection, TextSelection::new(2, 3));
    assert_eq!(value.composing, None);
}

// Upstream :304. Both the input and the expected output place the caret at
// the end of the corresponding Unicode string, not at the same raw offset.
#[test]
fn length_limiting_counts_non_bmp_scalars_and_repositions_the_end_caret() {
    let formatter = LengthLimitingTextInputFormatter::new(2);
    let value = formatter.format_edit_update(
        &TextEditingValue::empty(),
        &TextEditingValue::new("🦄🦄🦄🦄"),
    );
    assert_eq!(value, TextEditingValue::new("🦄🦄"));
}

// Upstream :418 asserts the public static truncate operation, which Incular
// does not expose. Pair its text and both selection assertions through an
// enforced edit from empty, without claiming the static API is present.
#[test]
fn enforced_edit_counts_a_zwj_family_as_one_grapheme_and_clamps_the_end_caret() {
    let formatter = LengthLimitingTextInputFormatter::new(10);
    let value = formatter.format_edit_update(
        &TextEditingValue::empty(),
        &TextEditingValue::new("👨‍👩‍👦01234567890"),
    );
    let expected = "👨‍👩‍👦012345678";
    assert_eq!(value.text, expected);
    assert_eq!(value.selection.base, expected.len());
    assert_eq!(value.selection.extent, expected.len());
}

// Only the final assertion from upstream "Preserves composing region" :808,
// :856–865. The other four require a multi-character pattern and a
// nonempty replacement string, which the Rust character predicate lacks.
#[test]
fn denying_every_character_in_the_composing_range_clears_composition() {
    let formatter = FilteringTextInputFormatter::deny(|c| c == 'B');
    let edit = TextEditingValue::new("AAABBBCCC").with_composing(Some(TextRange::new(3, 6)));
    let value = formatter.format_edit_update(&TextEditingValue::empty(), &edit);
    assert_eq!(value.text, "AAACCC");
    assert_eq!(value.composing, None);
}
