//! Retained Flutter text formatting compatibility examples omitted from the first slice.
use incular_text::{TextEditingValue, TextSelection};
use incular_widgets::{
    FilteringTextInputFormatter, LengthLimitingTextInputFormatter, TextInputFormatter,
};

fn edit(text: &str, base: usize, extent: usize) -> TextEditingValue {
    TextEditingValue {
        text: text.into(),
        selection: TextSelection::new(base, extent),
        composing: None,
    }
}
fn caret(text: &str, offset: usize) -> TextEditingValue {
    edit(text, offset, offset)
}
#[test]
fn digits_only_removes_a_non_allowed_append_without_changing_text() {
    let f = FilteringTextInputFormatter::digits_only();
    assert_eq!(
        f.format_edit_update(&caret("12345", 5), &caret("12345@", 6))
            .text,
        "12345"
    );
}
#[test]
fn digits_only_preserves_end_and_middle_cursor_when_no_text_is_filtered() {
    let f = FilteringTextInputFormatter::digits_only();
    let old = caret("123", 0);
    let a = caret("123456", 6);
    assert_eq!(f.format_edit_update(&old, &a).selection.base, 6);
    let b = caret("1239456", 4);
    assert_eq!(
        f.format_edit_update(&caret("123456", 4), &b).selection.base,
        4
    );
}
#[test]
fn basic_ascii_filter_keeps_letters_numbers_and_approved_punctuation() {
    let f = FilteringTextInputFormatter::allow(|c| {
        c.is_ascii_alphanumeric() || matches!(c, '.' | '@' | '-')
    });
    assert_eq!(
        f.format_edit_update(&caret("", 0), &caret("ab&&ca@bcabc", 0))
            .text,
        "abca@bcabc"
    );
}
#[test]
fn truncation_via_normal_formatting_preserves_emoji_caret_boundary() {
    let f = LengthLimitingTextInputFormatter::new(10);
    for (text, expected) in [
        ("01234567890", "0123456789"),
        ("😆01234567890", "😆012345678"),
    ] {
        let new = caret(text, text.len());
        assert_eq!(
            f.format_edit_update(&caret("", 0), &new),
            caret(expected, expected.len())
        );
    }
}
#[test]
fn deny_subcase_removes_bbb_and_collapses_reverse_selection_at_replacement_boundary() {
    let f = FilteringTextInputFormatter::deny(|c| c == 'B');
    let new = edit("AAABBBCCC", 6, 3);
    assert_eq!(
        f.format_edit_update(&caret("", 0), &new),
        caret("AAACCC", 3)
    );
}
#[test]
fn digits_only_erases_nonmatching_text_and_clamps_stranded_caret() {
    let f = FilteringTextInputFormatter::digits_only();
    let old = caret("42", 2);
    assert_eq!(f.format_edit_update(&old, &caret("X", 1)), caret("", 0));
}
