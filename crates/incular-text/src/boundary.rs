//! Text boundaries used by keyboard caret movement and deletion.
//!
//! These are ports of Flutter 3.41's `TextBoundary` family
//! (`services/text_boundary.dart`) and of `WordBoundary.moveByWordBoundary`
//! (`painting/text_painter.dart`), over UTF-8 byte offsets instead of UTF-16
//! code units. Line boundaries depend on the laid-out text and live with the
//! text layout; everything here is a pure function of the string.

use std::{cell::RefCell, rc::Rc};

use icu_properties::{CodePointMapData, props::GeneralCategory};
use icu_segmenter::options::WordBreakInvariantOptions;
use icu_segmenter::{GraphemeClusterSegmenter, WordSegmenter};

type WordBreaks = Rc<dyn Fn(&str) -> Option<Vec<usize>>>;

thread_local! {
    static WORD_SEGMENTER: RefCell<Option<WordBreaks>> = const { RefCell::new(None) };
}

/// Replaces word segmentation on the calling (UI) thread. `segment` returns
/// the ascending UTF-8 byte offsets of a text's word boundaries, including
/// `0` and its length, or `None` to use the built-in segmenter.
///
/// The built-in segmenter follows UAX #29 everywhere. On native targets it
/// also segments Chinese, Japanese, Thai, Lao, Khmer and Burmese with ICU's
/// dictionaries and models; a browser build leaves those out (they would add
/// about 2 MB to every page) and its host supplies the browser's segmenter
/// here instead.
pub fn set_word_segmenter(segment: impl Fn(&str) -> Option<Vec<usize>> + 'static) {
    WORD_SEGMENTER.set(Some(Rc::new(segment)));
}

fn word_boundaries(text: &str) -> Vec<usize> {
    if let Some(segment) = WORD_SEGMENTER.with_borrow(Clone::clone)
        && let Some(boundaries) = segment(text)
    {
        return boundaries;
    }
    #[cfg(not(target_arch = "wasm32"))]
    let segmenter = WordSegmenter::new_auto(WordBreakInvariantOptions::default());
    #[cfg(target_arch = "wasm32")]
    let segmenter =
        WordSegmenter::new_for_non_complex_scripts(WordBreakInvariantOptions::default());
    segmenter.segment_str(text).collect()
}

/// The string-only boundary kinds a caret can move or delete by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBoundaryKind {
    /// Grapheme clusters (`CharacterBoundary`).
    Character,
    /// Words with spaces and punctuation skipped (`moveByWordBoundary`).
    Word,
    /// Paragraphs ended by a line terminator (`ParagraphBoundary`).
    Paragraph,
    /// The whole text (`DocumentBoundary`).
    Document,
}

impl TextBoundaryKind {
    /// Flutter's `TextBoundary.getLeadingTextBoundaryAt`: the boundary at or
    /// before `position`, or `None` when there is none.
    #[must_use]
    pub fn leading_at(self, text: &str, position: isize) -> Option<usize> {
        match self {
            Self::Character => character_leading(text, position),
            Self::Word => word_leading(text, position),
            Self::Paragraph => paragraph_leading(text, position),
            Self::Document => (position >= 0).then_some(0),
        }
    }

    /// Flutter's `TextBoundary.getTrailingTextBoundaryAt`: the boundary after
    /// `position`, or `None` when there is none.
    #[must_use]
    pub fn trailing_at(self, text: &str, position: isize) -> Option<usize> {
        match self {
            Self::Character => character_trailing(text, position),
            Self::Word => word_trailing(text, position),
            Self::Paragraph => paragraph_trailing(text, position),
            Self::Document => ((position.max(0) as usize) < text.len()).then_some(text.len()),
        }
    }

    /// Flutter's `_moveBeyondTextBoundary`: the closest boundary to `extent`
    /// that is not `extent` itself (unless already at the start or end of the
    /// text), in the given direction.
    #[must_use]
    pub fn move_beyond(self, text: &str, extent: usize, forward: bool) -> usize {
        let extent = floor_char_boundary(text, extent);
        if forward {
            self.trailing_at(text, extent as isize)
                .unwrap_or(text.len())
        } else {
            // x - 1 avoids getting stuck at a boundary that maps to itself.
            self.leading_at(text, previous_unit(text, extent))
                .unwrap_or(0)
        }
    }
}

/// The byte offset one character before `offset`, or -1 at the start.
fn previous_unit(text: &str, offset: usize) -> isize {
    text[..offset]
        .char_indices()
        .next_back()
        .map_or(-1, |(index, _)| index as isize)
}

fn floor_char_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn clamp_position(text: &str, position: isize) -> usize {
    floor_char_boundary(text, position.max(0) as usize)
}

// ----- Character -----

fn grapheme_boundaries(text: &str) -> impl Iterator<Item = usize> + '_ {
    GraphemeClusterSegmenter::new().segment_str(text)
}

fn character_leading(text: &str, position: isize) -> Option<usize> {
    if position < 0 {
        return None;
    }
    let position = clamp_position(text, position);
    grapheme_boundaries(text)
        .take_while(|boundary| *boundary <= position)
        .last()
        .or(Some(0))
}

fn character_trailing(text: &str, position: isize) -> Option<usize> {
    let position = clamp_position(text, position);
    if position >= text.len() {
        return None;
    }
    grapheme_boundaries(text)
        .find(|boundary| *boundary > position)
        .or(Some(text.len()))
}

// ----- Word -----

fn is_newline(c: char) -> bool {
    // WordBoundary._isNewline: a carriage return is not a hard line break.
    matches!(
        c,
        '\u{000A}' | '\u{0085}' | '\u{000B}' | '\u{000C}' | '\u{2028}' | '\u{2029}'
    )
}

fn is_space_separator_or_punctuation(c: char) -> bool {
    matches!(
        CodePointMapData::<GeneralCategory>::new().get(c),
        GeneralCategory::SpaceSeparator
            | GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::ClosePunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::OtherPunctuation
    )
}

/// `Paragraph.getWordBoundary`: the UAX #29 word containing `position`.
fn word_range_at(text: &str, position: usize) -> (usize, usize) {
    let mut start = 0;
    let mut end = text.len();
    for boundary in word_boundaries(text) {
        if boundary <= position {
            start = boundary;
        } else {
            end = boundary;
            break;
        }
    }
    if position >= text.len() {
        start = text.len();
    }
    (start, end)
}

/// `WordBoundary._skipSpacesAndPunctuations`: whether a word-break `offset`
/// is a place a word move may stop at. Breaks next to spaces or punctuation
/// are skipped, except where a hard break (text edge, newline) applies.
fn stops_at(text: &str, offset: usize, forward: bool) -> bool {
    let before = text[..offset].chars().next_back();
    let after = text[offset..].chars().next();
    // "inner" is the character before the break in the search direction.
    let (inner, outer) = if forward {
        (before, after)
    } else {
        (after, before)
    };
    let hard_break = match (inner, outer) {
        (Some(inner), Some(outer)) => is_newline(inner) || is_newline(outer),
        _ => true,
    };
    hard_break || inner.is_some_and(|c| !is_space_separator_or_punctuation(c))
}

fn word_leading(text: &str, position: isize) -> Option<usize> {
    let mut position = position;
    loop {
        if position < 0 {
            return None;
        }
        let (start, _) = word_range_at(text, clamp_position(text, position));
        if stops_at(text, start, false) {
            return Some(start);
        }
        position = start as isize - 1;
    }
}

fn word_trailing(text: &str, position: isize) -> Option<usize> {
    let mut position = clamp_position(text, position);
    loop {
        let (_, end) = word_range_at(text, position);
        if stops_at(text, end, true) {
            return Some(end);
        }
        position = end;
    }
}

// ----- Paragraph -----

fn is_line_terminator(c: char) -> bool {
    matches!(
        c,
        '\u{000A}' | '\u{000B}' | '\u{000C}' | '\u{000D}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}

fn paragraph_leading(text: &str, position: isize) -> Option<usize> {
    if position < 0 || text.is_empty() {
        return None;
    }
    let position = clamp_position(text, position);
    if position >= text.len() {
        return Some(text.len());
    }
    if position == 0 {
        return Some(0);
    }
    let mut index = position;
    let char_at = |index: usize| text[index..].chars().next();
    let char_before = |index: usize| text[..index].chars().next_back();
    // A CRLF pair counts as one terminator: step back over both.
    if index > 1 && char_at(index) == Some('\n') && char_before(index) == Some('\r') {
        index -= 2;
    } else if char_at(index).is_some_and(is_line_terminator) {
        index = previous_unit(text, index).max(0) as usize;
    }
    while index > 0 {
        if let Some(c) = char_at(index)
            && is_line_terminator(c)
        {
            return Some(index + c.len_utf8());
        }
        index = previous_unit(text, index).max(0) as usize;
    }
    Some(0)
}

fn paragraph_trailing(text: &str, position: isize) -> Option<usize> {
    if text.is_empty() {
        return None;
    }
    if position < 0 {
        return Some(0);
    }
    let position = clamp_position(text, position);
    if position >= text.len() {
        return None;
    }
    let mut index = position;
    loop {
        let c = text[index..].chars().next()?;
        if is_line_terminator(c) {
            // A CRLF pair ends the paragraph after both characters.
            let next = index + c.len_utf8();
            return Some(if c == '\r' && text[next..].starts_with('\n') {
                next + 1
            } else {
                next
            });
        }
        index += c.len_utf8();
        if index == text.len() {
            return Some(index);
        }
    }
}
