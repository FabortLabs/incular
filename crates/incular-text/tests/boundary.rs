//! Flutter 3.41 `TextBoundary` behaviour (`services/text_boundary.dart` and
//! `WordBoundary.moveByWordBoundary` in `painting/text_painter.dart`).

use incular_text::TextBoundaryKind::{Character, Document, Paragraph, Word};

#[test]
fn word_forward_stops_at_the_end_of_the_next_word() {
    // moveByWordBoundary skips the spaces between words: from the end of
    // "hello" the caret lands at the end of "world", not at its start.
    let text = "hello world";
    assert_eq!(Word.move_beyond(text, 0, true), 5);
    assert_eq!(Word.move_beyond(text, 5, true), 11);
    assert_eq!(Word.move_beyond(text, 11, true), 11, "already at the end");
}

#[test]
fn word_backward_stops_at_the_start_of_the_previous_word() {
    let text = "hello world";
    assert_eq!(Word.move_beyond(text, 11, false), 6);
    assert_eq!(Word.move_beyond(text, 6, false), 0);
    assert_eq!(Word.move_beyond(text, 0, false), 0, "already at the start");
}

#[test]
fn word_moves_skip_punctuation_runs() {
    let text = "foo, bar";
    assert_eq!(Word.move_beyond(text, 3, true), 8);
    assert_eq!(Word.move_beyond(text, 5, false), 0);
}

#[test]
fn word_moves_stop_at_newlines() {
    // WB3a/WB3b: hard breaks around newlines take precedence over the
    // space-skipping rule.
    let text = "ab\ncd";
    assert_eq!(Word.move_beyond(text, 0, true), 2);
    assert_eq!(Word.move_beyond(text, 2, true), 3);
}

#[test]
fn paragraph_boundaries_end_after_the_line_terminator() {
    let text = "ab\ncd\n";
    assert_eq!(Paragraph.trailing_at(text, 0), Some(3));
    assert_eq!(Paragraph.trailing_at(text, 3), Some(6));
    assert_eq!(Paragraph.trailing_at(text, 6), None);
    assert_eq!(Paragraph.leading_at("ab\ncd", 4), Some(3));
    assert_eq!(Paragraph.leading_at("ab\ncd", 1), Some(0));
    assert_eq!(
        Paragraph.trailing_at("a\r\nb", 0),
        Some(3),
        "CRLF is one terminator"
    );
}

#[test]
fn document_boundaries_span_the_whole_text() {
    assert_eq!(Document.leading_at("abc", 2), Some(0));
    assert_eq!(Document.trailing_at("abc", 1), Some(3));
    assert_eq!(Document.trailing_at("abc", 3), None);
    assert_eq!(Document.move_beyond("abc", 1, true), 3);
    assert_eq!(Document.move_beyond("abc", 2, false), 0);
}

#[test]
fn character_boundaries_follow_grapheme_clusters() {
    // "e" + combining acute accent, then "x".
    let text = "e\u{301}x";
    assert_eq!(Character.leading_at(text, 1), Some(0));
    assert_eq!(Character.trailing_at(text, 0), Some(3));
    assert_eq!(Character.move_beyond(text, 0, true), 3);
    assert_eq!(Character.move_beyond(text, 3, false), 0);
    assert_eq!(Character.trailing_at(text, 4), None);
}

#[test]
fn a_host_word_segmenter_replaces_the_built_in_one() {
    // A host segmenting "東京都" as "東京" + "都", as a browser dictionary does.
    let text = "東京都";
    incular_text::set_word_segmenter(|text| (text == "東京都").then(|| vec![0, 6, 9]));
    assert_eq!(Word.move_beyond(text, 0, true), 6);
    assert_eq!(Word.move_beyond(text, 9, false), 6);
    // Declining a text falls back to the built-in segmenter.
    assert_eq!(Word.move_beyond("hello world", 0, true), 5);
}
