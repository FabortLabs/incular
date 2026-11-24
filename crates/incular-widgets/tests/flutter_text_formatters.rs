//! Flutter 3.50.0 text input formatter regressions (C29–C30 in the parity audit).

use incular_text::{TextEditingValue, TextSelection};
use incular_widgets::{
    FilteringTextInputFormatter, LengthLimitingTextInputFormatter, TextInputFormatter,
};

fn value(text: &str, caret: usize) -> TextEditingValue {
    TextEditingValue {
        text: text.into(),
        selection: TextSelection::collapsed(caret),
        composing: None,
    }
}

fn assert_digits_only_remaps_caret(input: &str, caret: usize) {
    let formatter = FilteringTextInputFormatter::digits_only();
    let output = formatter.format_edit_update(&value("12", 1), &value(input, caret));

    assert_eq!(output.text, "12");
    assert_eq!(output.selection, TextSelection::collapsed(1));
}

#[test]
fn digits_only_remaps_caret_after_removed_ascii() {
    assert_digits_only_remaps_caret("1a2", 2);
}

#[test]
fn digits_only_remaps_caret_after_removed_multibyte_character() {
    // Rust counts UTF-8 bytes: the caret immediately after the emoji is 5.
    assert_digits_only_remaps_caret("1😀2", "1😀".len());
}

#[test]
fn full_length_limit_rejects_a_middle_insert_without_truncating_existing_text() {
    let formatter = LengthLimitingTextInputFormatter::new(3);
    let old = value("abc", 1);

    assert_eq!(
        formatter.format_edit_update(&old, &value("aYc", 2)).text,
        "aYc"
    );
    let output = formatter.format_edit_update(&old, &value("aXbc", 2));
    assert_eq!((output.text, output.selection), (old.text, old.selection));
}
