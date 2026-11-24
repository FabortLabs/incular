//! Additional exact Flutter 3.50.0 TextEditingValue.replaced fixtures.
//! The original strings and all compared offsets are ASCII.

use incular_text::{TextEditingValue, TextRange, TextSelection};

const TEXT: &str = "From a false proposition, anything follows.";

fn with_selection(base: usize, extent: usize) -> TextEditingValue {
    TextEditingValue::new(TEXT).with_selection(TextSelection::new(base, extent))
}

fn check(
    initial: TextEditingValue,
    replaced: TextRange,
    insert: &str,
    expected_text: &str,
    expected_selection: TextSelection,
) {
    let result = initial.replace(replaced, insert);
    assert_eq!(result.text, expected_text);
    assert_eq!(result.selection, expected_selection);
    assert_eq!(result.composing, None);
}

#[test]
fn deleting_forward_selection_collapses_at_its_start() {
    let initial = with_selection(5, 13);
    let range = initial.selection.range();
    check(
        initial,
        range,
        "",
        "From proposition, anything follows.",
        TextSelection::collapsed(5),
    );
}

#[test]
fn inserting_at_caret_advances_to_the_insert_end() {
    let initial = with_selection(5, 5);
    let range = initial.selection.range();
    check(
        initial,
        range,
        "AA",
        "From AAa false proposition, anything follows.",
        TextSelection::collapsed(7),
    );
}

#[test]
fn replacing_text_at_selection_start_keeps_the_reversed_selection() {
    check(
        with_selection(13, 5),
        TextRange::new(5, 6),
        "AA",
        "From AA false proposition, anything follows.",
        TextSelection::new(14, 5),
    );
}

#[test]
fn replacing_text_at_selection_end_keeps_the_reversed_selection() {
    check(
        with_selection(13, 5),
        TextRange::new(12, 13),
        "AA",
        "From a falseAAproposition, anything follows.",
        TextSelection::new(14, 5),
    );
}

#[test]
fn deleting_after_selection_keeps_the_reversed_selection() {
    check(
        with_selection(13, 5),
        TextRange::new(13, 14),
        "",
        "From a false roposition, anything follows.",
        TextSelection::new(13, 5),
    );
}

#[test]
fn deleting_at_selection_start_shortens_the_reversed_selection() {
    check(
        with_selection(13, 5),
        TextRange::new(5, 6),
        "",
        "From  false proposition, anything follows.",
        TextSelection::new(12, 5),
    );
}

#[test]
fn deleting_at_selection_end_shortens_the_reversed_selection() {
    check(
        with_selection(13, 5),
        TextRange::new(12, 13),
        "",
        "From a falseproposition, anything follows.",
        TextSelection::new(12, 5),
    );
}
