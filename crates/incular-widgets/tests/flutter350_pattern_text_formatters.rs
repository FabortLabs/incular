use incular_text::{TextEditingValue, TextRange, TextSelection};
use incular_widgets::{
    CallbackTextInputFormatter, FilteringTextInputFormatter, TextInputFormatter,
};
use std::cell::RefCell;

fn apply(formatter: &impl TextInputFormatter, value: &TextEditingValue) -> TextEditingValue {
    formatter.format_edit_update(&TextEditingValue::empty(), value)
}

#[test]
fn callback_formatter_receives_the_original_old_and_new_values() {
    let old = TextEditingValue::new("before");
    let new = TextEditingValue::new("after").with_selection(TextSelection::new(5, 2));
    let seen = RefCell::new(None);
    let callback = CallbackTextInputFormatter::new(|a: &TextEditingValue, b: &TextEditingValue| {
        *seen.borrow_mut() = Some((a.clone(), b.clone()));
        TextEditingValue::empty()
    });
    assert_eq!(
        callback.format_edit_update(&old, &new),
        TextEditingValue::empty()
    );
    assert_eq!(seen.take(), Some((old, new)));
}

#[test]
fn literal_and_regex_example_matches_each_gap_or_match_and_remaps_selection() {
    let source = TextEditingValue::new("Into the Woods");
    let selected = source.clone().with_selection(TextSelection::new(11, 14));
    for (formatter, text, base, extent) in [
        (
            FilteringTextInputFormatter::allow_literal("o"),
            "*o*oo*",
            4,
            6,
        ),
        (
            FilteringTextInputFormatter::deny_literal("o"),
            "Int* the W**ds",
            11,
            14,
        ),
        (
            FilteringTextInputFormatter::allow_regex("o+").unwrap(),
            "*o*oo*",
            4,
            6,
        ),
        (
            FilteringTextInputFormatter::deny_regex("o+").unwrap(),
            "Int* the W*ds",
            11,
            13,
        ),
        (
            FilteringTextInputFormatter::allow(|c| c == 'o'),
            "*o*oo*",
            4,
            6,
        ),
        (
            FilteringTextInputFormatter::deny(|c| c == 'o'),
            "Int* the W**ds",
            11,
            14,
        ),
    ] {
        let formatter = formatter.replacement_string("*");
        assert_eq!(apply(&formatter, &source).text, text);
        let edit = apply(&formatter, &selected);
        assert_eq!(edit.text, text);
        assert_eq!(edit.selection, TextSelection::new(base, extent));
    }
    assert!(FilteringTextInputFormatter::deny_regex("[").is_err());
}

#[test]
fn filtering_repeated_literal_does_not_depend_on_selection_or_composition() {
    let filter = FilteringTextInputFormatter::deny_literal("abc").replacement_string("*");
    let value = TextEditingValue::new("abcabcabc");
    assert_eq!(apply(&filter, &value).text, "***");
    for i in 0..value.text.len() {
        let selected = value.clone().with_selection(TextSelection::collapsed(i));
        let composing = value.clone().with_composing(Some(TextRange::new(i, i)));
        assert_eq!(apply(&filter, &selected).text, "***");
        assert_eq!(apply(&filter, &composing).text, "***");
    }
}

#[test]
fn deny_pattern_preserves_all_original_selection_cases() {
    for (replacement, original, expected) in [
        ("****", (6, 3), (7, 3)),
        ("****", (9, 3), (10, 3)),
        ("****", (9, 6), (10, 7)),
        ("***", (5, 4), (5, 4)),
        ("", (6, 3), (3, 3)),
        ("****", (5, 4), (7, 7)),
    ] {
        let filter =
            FilteringTextInputFormatter::deny_literal("BBB").replacement_string(replacement);
        let value = TextEditingValue::new("AAABBBCCC")
            .with_selection(TextSelection::new(original.0, original.1));
        assert_eq!(
            apply(&filter, &value).selection,
            TextSelection::new(expected.0, expected.1)
        );
    }
}

#[test]
fn allow_pattern_replaces_the_gaps_and_uses_non_overlapping_matches() {
    let filter = FilteringTextInputFormatter::allow_literal("BBB").replacement_string("****");
    for (original, expected) in [((6, 3), (7, 4)), ((9, 0), (11, 0)), ((9, 6), (11, 7))] {
        let value = TextEditingValue::new("AAABBBCCC")
            .with_selection(TextSelection::new(original.0, original.1));
        let result = apply(&filter, &value);
        assert_eq!(result.text, "****BBB****");
        assert_eq!(result.selection, TextSelection::new(expected.0, expected.1));
    }
    let only = FilteringTextInputFormatter::allow_literal("BBB");
    let value = TextEditingValue::new("AAABBBBBCCC").with_selection(TextSelection::new(8, 3));
    let result = apply(&only, &value);
    assert_eq!(result.text, "BBB");
    assert_eq!(result.selection, TextSelection::new(3, 0));
}

#[test]
fn deny_pattern_preserves_or_clears_the_original_composing_ranges() {
    for (replacement, original, expected) in [
        ("****", (3, 6), Some((3, 7))),
        ("****", (3, 9), Some((3, 10))),
        ("****", (6, 9), Some((7, 10))),
        ("***", (4, 5), Some((4, 5))),
        ("", (3, 6), None),
    ] {
        let filter =
            FilteringTextInputFormatter::deny_literal("BBB").replacement_string(replacement);
        let value = TextEditingValue::new("AAABBBCCC")
            .with_composing(Some(TextRange::new(original.0, original.1)));
        assert_eq!(
            apply(&filter, &value).composing,
            expected.map(|(a, b)| TextRange::new(a, b))
        );
    }
}

#[test]
fn unicode_replacement_keeps_equal_utf16_offsets_and_clamps_unrepresentable_offsets() {
    let to_ascii = FilteringTextInputFormatter::deny_literal("🦄a").replacement_string("XYZ");
    let value = TextEditingValue::new("x🦄ay")
        .with_selection(TextSelection::new(6, 5))
        .with_composing(Some(TextRange::new(1, 5)));
    let after = apply(&to_ascii, &value);
    assert_eq!(after.text, "xXYZy");
    assert_eq!(after.selection, TextSelection::new(4, 3));
    assert_eq!(after.composing, Some(TextRange::new(1, 3)));

    let to_emoji = FilteringTextInputFormatter::deny_literal("XY").replacement_string("🦄");
    let value = TextEditingValue::new("aXYz").with_selection(TextSelection::new(3, 2));
    let after = apply(&to_emoji, &value);
    assert_eq!(after.text, "a🦄z");
    assert_eq!(after.selection, TextSelection::new(5, 1));
}

#[test]
fn regex_filter_accepts_empty_matches_without_replacing_an_empty_region() {
    let filter = FilteringTextInputFormatter::allow_regex("[A-Za-z0-9.@-]*").unwrap();
    assert_eq!(
        apply(&filter, &TextEditingValue::new("ab&&ca@bcabc")).text,
        "abca@bcabc"
    );
    let empty = FilteringTextInputFormatter::deny_regex("^")
        .unwrap()
        .replacement_string("!");
    assert_eq!(apply(&empty, &TextEditingValue::new("abc")).text, "abc");
}
