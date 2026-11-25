//! Exact pinned Flutter 3.50 widget spell-check span cases.

use incular_config::TargetPlatform;
use incular_core::Color;
use incular_text::{
    TextDecoration, TextDecorationStyle, TextEditingValue, TextRange, TextSelection, TextSpan,
    TextStyle,
};
use incular_widgets::{
    SpellCheckResults, SuggestionSpan, build_text_span_with_spell_check_suggestions,
};

#[derive(Clone, Copy)]
enum Style {
    Plain,
    Misspelled,
    Composing,
}
use Style::{Composing as C, Misspelled as M, Plain as P};

const MOBILE: &[TargetPlatform] = &[TargetPlatform::Android, TargetPlatform::Ios];
const ANDROID: &[TargetPlatform] = &[TargetPlatform::Android];

fn assert_spans(
    platforms: &[TargetPlatform],
    value: TextEditingValue,
    out_of_range: bool,
    checked: SpellCheckResults,
    expected: &[(&str, Style)],
) {
    let misspelled = TextStyle::new()
        .decoration(TextDecoration::UNDERLINE)
        .decoration_color(Color::rgba(255, 0, 0, 255))
        .decoration_style(TextDecorationStyle::Wavy);
    let composing = TextStyle::new().decoration(TextDecoration::UNDERLINE);
    let mut expected_tree = TextSpan::new("");
    expected_tree.children = expected
        .iter()
        .map(|&(text, style)| {
            let mut span = TextSpan::new(text);
            span.style = match style {
                P => None,
                M => Some(misspelled.clone()),
                C => Some(composing.clone()),
            };
            span.into()
        })
        .collect();

    for &platform in platforms {
        let actual = build_text_span_with_spell_check_suggestions(
            &value,
            out_of_range,
            None,
            &misspelled,
            &checked,
            platform,
        );
        assert_eq!(actual, expected_tree, "{platform:?}, text {:?}", value.text);
    }
}

fn results(text: &str, specs: &[(usize, usize, &[&str])]) -> SpellCheckResults {
    SpellCheckResults::new(
        text,
        specs
            .iter()
            .map(|&(start, end, words)| SuggestionSpan::new(start..end, words.iter().copied()))
            .collect(),
    )
}

fn stale(text: &str, checked: SpellCheckResults, expected: &[(&str, Style)]) {
    assert_spans(
        MOBILE,
        TextEditingValue::new(text),
        false,
        checked,
        expected,
    );
}

const WORLD: &[&str] = &["world", "word", "old"];
const WRONG: &[&str] = &["wrong", "worn", "wrung"];
const HELLO: &str = "Hello, wrold! Hey";
const HELLO_SPANS: &[(&str, Style)] = &[("Hello, ", P), ("wrold", M), ("! Hey", P)];

#[test]
fn out_of_range_composition_does_not_add_a_composing_decoration() {
    assert_spans(
        MOBILE,
        TextEditingValue::new(HELLO),
        true,
        results(HELLO, &[(7, 12, WORLD)]),
        HELLO_SPANS,
    );
}

#[test]
fn isolated_misspelling_and_separate_composition_have_different_underlines() {
    let mut value = TextEditingValue::new(HELLO);
    value.composing = Some(TextRange::new(14, 17));
    assert_spans(
        ANDROID,
        value,
        false,
        results(HELLO, &[(7, 12, WORLD)]),
        &[("Hello, ", P), ("wrold", M), ("! ", P), ("Hey", C)],
    );
}

#[test]
fn overlapping_misspelling_uses_the_composing_decoration_instead() {
    let text = "Right worng worng right";
    let mut value = TextEditingValue::new(text);
    value.composing = Some(TextRange::new(12, 17));
    assert_spans(
        ANDROID,
        value,
        false,
        results(text, &[(6, 11, WRONG), (12, 17, WRONG)]),
        &[
            ("Right ", P),
            ("worng", M),
            (" ", P),
            ("worng", C),
            (" right", P),
        ],
    );
}

#[test]
fn consecutive_misspellings_keep_the_unstyled_space_between_them() {
    let text = "Right worng worng right";
    assert_spans(
        MOBILE,
        TextEditingValue::new(text),
        true,
        results(text, &[(6, 11, WRONG), (12, 17, WRONG)]),
        &[
            ("Right ", P),
            ("worng", M),
            (" ", P),
            ("worng", M),
            (" right", P),
        ],
    );
}

#[test]
fn older_shorter_results_preserve_the_remaining_misspelling() {
    stale(
        HELLO,
        results("Hello, wrold!", &[(7, 12, WORLD)]),
        HELLO_SPANS,
    );
}

#[test]
fn older_longer_results_discard_a_removed_misspelled_word() {
    stale(
        HELLO,
        results(
            "Hello, wrold Hey feirnd!",
            &[(7, 12, WORLD), (17, 23, &["friend", "fiend", "fern"])],
        ),
        HELLO_SPANS,
    );
}

#[test]
fn stale_result_drops_a_word_that_no_longer_matches() {
    stale(
        HELLO,
        results("Hello, wrild! Hey", &[(7, 12, &["wild", "world"])]),
        &[(HELLO, P)],
    );
}

#[test]
fn stale_result_moves_forward_with_an_unchanged_misspelling() {
    stale(
        "Hello, there wrold! Hey",
        results(HELLO, &[(7, 12, WORLD)]),
        &[("Hello, there ", P), ("wrold", M), ("! Hey", P)],
    );
}

#[test]
fn stale_result_moves_back_with_an_unchanged_misspelling() {
    stale(
        HELLO,
        results("Hello, great wrold! Hey", &[(13, 18, WORLD)]),
        HELLO_SPANS,
    );
}

#[test]
fn stale_ranges_advance_and_recede_independently() {
    stale(
        "Hello, wrold! And Hye!",
        results(
            "Hello, great wrold! Hye!",
            &[(13, 18, WORLD), (20, 23, &["Hey", "He"])],
        ),
        &[
            ("Hello, ", P),
            ("wrold", M),
            ("! And ", P),
            ("Hye", M),
            ("!", P),
        ],
    );
}

#[test]
fn editing_inside_a_misspelled_word_removes_the_stale_result() {
    let text = "Hello, wroldd!";
    stale(
        text,
        results("Hello, wrold!", &[(7, 12, WORLD)]),
        &[(text, P)],
    );
}

#[test]
fn regex_reserved_and_non_ascii_characters_do_not_corrupt_stale_results() {
    let text = "Hello, *ãresaa";
    // Flutter's stale range is 7..12 UTF-16. The same substring, '*ãres',
    // uses one additional UTF-8 byte in Incular.
    stale(
        text,
        results("Hello, *ãresa", &[(7, 13, &["*rangesa"])]),
        &[(text, P)],
    );
}

#[test]
fn deletion_keeps_three_previous_spans_and_the_trailing_space() {
    let text = "Lorem ipsum dolor ";
    let mut value = TextEditingValue::new(text);
    value.selection = TextSelection::collapsed(text.len());
    let suggestions: &[&str] = &["suggestion1", "suggestion2"];
    assert_spans(
        MOBILE,
        value,
        false,
        results(
            "Lorem ipsum dolor sit ",
            &[
                (0, 5, suggestions),
                (6, 11, suggestions),
                (12, 17, suggestions),
                (18, 21, suggestions),
            ],
        ),
        &[
            ("Lorem", M),
            (" ", P),
            ("ipsum", M),
            (" ", P),
            ("dolor", M),
            (" ", P),
        ],
    );
}

// Incular's service API accepts UTF-8 byte ranges. Bad ranges from a custom
// checker must never drop or repeat characters in the editable field.
#[test]
fn malformed_service_ranges_leave_unicode_text_intact() {
    let text = "é worng β end";
    assert_spans(
        MOBILE,
        TextEditingValue::new(text),
        false,
        results(
            text,
            &[
                (1, 2, &[]), // starts inside é
                (2, 2, &[]), // empty
                (3, 8, WRONG),
                (6, 9, &[]),   // overlaps the accepted word
                (9, 10, &[]),  // ends inside β
                (12, 11, &[]), // reversed
                (40, 41, &[]), // outside the text
            ],
        ),
        &[("é ", P), ("worng", M), (" β end", P)],
    );
}

#[test]
fn service_range_with_an_oversized_end_is_still_clamped() {
    let text = "é worng";
    let mut value = TextEditingValue::new(text);
    value.selection = TextSelection::collapsed(0);
    assert_spans(
        MOBILE,
        value,
        false,
        results(text, &[(3, 80, WRONG)]),
        &[("é ", P), ("worng", M)],
    );
}
