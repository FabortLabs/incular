use incular_text::TextBoundaryKind::{Character, Document, Paragraph, Word};

#[test]
fn character_trailing_before_any_text_is_the_start_not_the_first_character_end() {
    assert_eq!(Character.leading_at("abc", -1), None);
    assert_eq!(Character.trailing_at("abc", -1), Some(0));
}

#[test]
fn paragraph_boundary_at_lf_after_multibyte_content_does_not_split_utf8() {
    let text = "é\r\nβ";
    let line_feed = text.find('\n').unwrap() as isize;
    assert_eq!(Paragraph.leading_at(text, line_feed), Some(0));
    assert_eq!(Paragraph.trailing_at(text, line_feed), Some(4));
}

use incular_text::{
    LineBoundary, TextAlign, TextBoundaryKind, TextEngine, TextLayoutOptions, TextRange, TextStyle,
};

fn utf16_position_as_byte(text: &str, position: isize) -> isize {
    if position < 0 {
        return position;
    }
    let position = position as usize;
    let mut units = 0;
    for (byte, scalar) in text.char_indices() {
        units += scalar.len_utf16();
        if position < units {
            return byte as isize;
        }
    }
    (text.len() + position - units) as isize
}

fn case(kind: TextBoundaryKind, text: &str, position: isize, start: isize, end: isize) {
    let byte = |position| utf16_position_as_byte(text, position);
    let endpoint = |position| (position >= 0).then(|| byte(position) as usize);
    assert_eq!(
        kind.leading_at(text, byte(position)),
        endpoint(start),
        "{text:?} at unit {position}"
    );
    assert_eq!(
        kind.trailing_at(text, byte(position)),
        endpoint(end),
        "{text:?} at unit {position}"
    );
    assert_eq!(
        kind.at(text, byte(position)),
        (endpoint(start), endpoint(end)),
        "{text:?} at unit {position}"
    );
}

fn consistent(kind: TextBoundaryKind, text: &str) {
    for offset in -1..=text.len() as isize + 1 {
        assert_eq!(
            kind.at(text, offset),
            (
                kind.leading_at(text, offset),
                kind.trailing_at(text, offset)
            )
        );
    }
}

#[test]
fn character_boundary_works() {
    let text = "abc";
    consistent(Character, text);
    for (position, leading, trailing) in [
        (-1, -1, 0),
        (0, 0, 1),
        (1, 1, 2),
        (2, 2, 3),
        (3, 3, -1),
        (4, 3, -1),
    ] {
        case(Character, text, position, leading, trailing);
    }
}

#[test]
fn character_boundary_works_with_a_grapheme() {
    let text = "a❄︎c";
    consistent(Character, text);
    for (position, leading, trailing) in [
        (-1, -1, 0),
        (0, 0, 1),
        (1, 1, 3),
        (2, 1, 3),
        (3, 3, 4),
        (4, 4, -1),
    ] {
        case(Character, text, position, leading, trailing);
    }
}

#[test]
fn move_by_word_boundaries_handles_spaces_punctuation_and_surrogates() {
    let text = concat!(
        "ABC   ABC\n",
        "AÁ    Á\n",
        "         \n",
        "ABC!!!ABC\n",
        "  !ABC !!\n",
        "A  𑗋𑗋 A\n",
    );
    for (position, leading, trailing) in [
        (4, 0, 9),
        (14, 10, 19),
        (24, 20, 29),
        (34, 30, 39),
        (44, 43, 46),
        (54, 50, 59),
    ] {
        case(Word, text, position, leading, trailing);
    }
}

#[test]
fn line_boundary_reads_the_real_shaped_line_and_excludes_the_hard_break() {
    let text = "abcdefghij\nk";
    let layout =
        TextEngine::new().layout(text, &TextStyle::new().size(14.), None, TextAlign::Start);
    let line = LineBoundary::new(&layout);
    assert_eq!(line.leading_at(3), Some(0));
    assert_eq!(line.trailing_at(3), Some(10));
    assert_eq!(line.at(3), Some(TextRange::new(0, 10)));
    assert_eq!(line.at(10), Some(TextRange::new(0, 10)));
    assert_eq!(line.at(11), Some(TextRange::new(11, 12)));
    assert_eq!(line.at(-1), Some(TextRange::new(0, 10)));
    assert_eq!(line.leading_at(-1), None);
    assert_eq!(line.at(13), None);

    let layout = TextEngine::new().layout_paragraphs(
        "abcdefghij\nk\nthird",
        &TextStyle::new().size(14.),
        TextLayoutOptions::default(),
    );
    let line = LineBoundary::new(&layout);
    assert_eq!(line.at(10), Some(TextRange::new(0, 10)));
    assert_eq!(line.at(12), Some(TextRange::new(11, 12)));
    assert_eq!(line.at(13), Some(TextRange::new(13, 18)));
}

#[test]
fn paragraph_boundaries_include_one_complete_terminator() {
    case(Paragraph, "abcd efg hi\njklmno\npqrstuv", 10, 0, 12);
    case(Paragraph, "abcd efg hi\r\njklmno\npqrstuv", 10, 0, 13);
    case(
        Paragraph,
        "Now is the time for\nall good people\nto come to the aid\nof their country.",
        11,
        0,
        20,
    );
}

#[test]
fn paragraph_boundaries_split_at_consecutive_terminators_involving_crlf() {
    let text = "Now is the time for\nall good people\n\r\nto come to the aid\nof their country.";
    for (position, start, end) in [(56, 38, 57), (38, 38, 57), (37, 36, 38)] {
        case(Paragraph, text, position, start, end);
    }
}

#[test]
fn paragraph_boundaries_split_at_consecutive_lf() {
    let text = "Now is the time for\nall good people\n\nto come to the aid\nof their country.";
    for (position, start, end) in [(55, 37, 56), (37, 37, 56), (36, 36, 37)] {
        case(Paragraph, text, position, start, end);
    }
}

#[test]
fn leading_paragraph_boundary_in_consecutive_crlf() {
    case(
        Paragraph,
        "abcd efg hi\r\n\r\n\r\n\r\n\r\n\r\n\r\n\n\n\n\n\njklmno\npqrstuv",
        18,
        17,
        19,
    );
}

#[test]
fn trailing_paragraph_boundary_in_consecutive_crlf() {
    case(
        Paragraph,
        "abcd efg hi\r\n\n\n\n\n\n\r\n\r\n\r\n\r\n\n\n\n\n\njklmno\npqrstuv",
        18,
        18,
        20,
    );
}

#[test]
fn paragraph_position_is_between_two_crlf_breaks() {
    case(Paragraph, "abcd efg hi\r\nhello\r\n\n", 16, 13, 20);
}

#[test]
fn paragraph_boundary_with_many_terminators() {
    let text = format!("abcd efg hi\r\n{}jklmno\npqrstuv", "\n".repeat(11));
    case(Paragraph, &text, 18, 18, 19);
    case(Paragraph, "abcd efg hi\r\n\n\n\n", 14, 14, 15);
}

#[test]
fn document_boundary_is_the_complete_document_even_outside_its_end() {
    let text = "abcd efg hi\njklmno\npqrstuv";
    consistent(Document, text);
    let len = text.len() as isize;
    for (position, start, end) in [
        (-1, -1, len),
        (0, 0, len),
        (10, 0, len),
        (len, 0, -1),
        (len + 1, 0, -1),
    ] {
        case(Document, text, position, start, end);
    }
}

#[test]
fn empty_string_boundaries_distinguish_the_negative_input() {
    for kind in [Character, Document] {
        assert_eq!(kind.at("", -1), (None, Some(0)));
        assert_eq!(kind.at("", 0), (Some(0), None));
    }
    assert_eq!(Paragraph.at("", -1), (None, None));
}
