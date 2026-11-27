//! Inline placeholder boxes (Flutter `PlaceholderSpan`) and paragraph struts
//! (Flutter `StrutStyle`).

use super::LineBox;
use crate::WidgetSpanAlignment;
use incular_core::{Rect, Size};
use parley::{AlignmentBaseline, BaselineShift, InlineBox, InlineBoxKind, Layout, VerticalAlign};

/// The object replacement character a placeholder occupies in plain text.
pub(super) const PLACEHOLDER_CHAR: char = '\u{fffc}';

/// The character shaped in a placeholder's place: zero-width and as long in
/// UTF-8 as U+FFFC, so every byte offset of the paragraph is unchanged and the
/// inline box alone supplies the placeholder's advance (SkParagraph shapes a
/// placeholder as its box, not as a glyph).
pub(super) const SHAPED_PLACEHOLDER_CHAR: char = '\u{200b}';

/// One inline box a paragraph reserves, as Flutter's `PlaceholderDimensions`
/// describe a `WidgetSpan`: its size, its vertical alignment and, for
/// baseline alignment, the distance from its top to its baseline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextPlaceholder {
    /// Caller identity, reported back on the placed box.
    pub id: u64,
    /// Byte index of the placeholder's U+FFFC in the paragraph text.
    pub index: usize,
    pub size: Size,
    pub alignment: WidgetSpanAlignment,
    /// Distance from the box top to its baseline for
    /// [`WidgetSpanAlignment::Baseline`]; `None` puts the bottom edge on the
    /// text baseline, as a child without a baseline does in Flutter.
    pub baseline: Option<f32>,
}

impl TextPlaceholder {
    #[must_use]
    pub const fn new(id: u64, index: usize, size: Size, alignment: WidgetSpanAlignment) -> Self {
        Self {
            id,
            index,
            size,
            alignment,
            baseline: None,
        }
    }

    #[must_use]
    pub const fn baseline(mut self, baseline: Option<f32>) -> Self {
        self.baseline = baseline;
        self
    }
}

/// Where a laid-out paragraph placed one [`TextPlaceholder`] (Flutter
/// `Paragraph.getBoxesForPlaceholders`), in paragraph coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextPlaceholderBox {
    pub id: u64,
    /// Byte index of the placeholder's U+FFFC in the paragraph text.
    pub index: usize,
    pub rect: Rect,
    /// Visual line holding the box.
    pub line: usize,
}

/// A paragraph strut resolved to line metrics (Flutter `StrutStyle`, applied
/// by SkParagraph's `TextLine` metrics): every line is at least this tall
/// above and below its baseline, or exactly this tall when `force` is set
/// (`forceStrutHeight`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStrut {
    /// Distance from the line top to the baseline.
    pub ascent: f32,
    /// Distance from the baseline to the line bottom.
    pub descent: f32,
    pub force: bool,
    /// Corrected font ascent and descent before distributing extra leading.
    pub(crate) box_ascent: f32,
    pub(crate) box_descent: f32,
    pub(crate) raw_ascent: f32,
}

impl TextStrut {
    /// Creates a strut with the supplied metrics and no extra leading.
    #[must_use]
    pub const fn from_metrics(ascent: f32, descent: f32, force: bool) -> Self {
        Self { ascent, descent, force, box_ascent: ascent, box_descent: descent, raw_ascent: ascent }
    }

    pub(super) fn key(self) -> ([u32; 5], bool) {
        (
            [
                self.ascent.to_bits(),
                self.descent.to_bits(),
                self.box_ascent.to_bits(),
                self.box_descent.to_bits(),
                self.raw_ascent.to_bits(),
            ],
            self.force,
        )
    }
}

/// Applies a strut to unrounded line boxes: SkParagraph's
/// `InternalLineMetrics::updateLineMetrics` widens a line's ascent and
/// descent to the strut's, and `forceStrutHeight` replaces them.
pub(super) fn apply_strut(boxes: &mut [LineBox], strut: TextStrut) {
    for line_box in boxes {
        let (ascent, descent) = if strut.force {
            (strut.ascent, strut.descent)
        } else {
            (
                (line_box.baseline - line_box.top).max(strut.ascent),
                (line_box.bottom - line_box.baseline).max(strut.descent),
            )
        };
        line_box.top = line_box.baseline - ascent;
        line_box.bottom = line_box.baseline + descent;
    }
}

/// The placeholders that belong to `text`: those whose U+FFFC is still in it
/// (an ellipsized candidate keeps only a prefix), with their positions.
pub(super) fn placeholders_in<'a>(
    text: &'a str,
    placeholders: &'a [TextPlaceholder],
) -> impl Iterator<Item = (usize, &'a TextPlaceholder)> + 'a {
    placeholders
        .iter()
        .enumerate()
        .filter(move |(_, placeholder)| {
            text.get(placeholder.index..)
                .is_some_and(|rest| rest.starts_with(PLACEHOLDER_CHAR))
        })
}

/// `text` with each placeholder's U+FFFC replaced by the zero-width shaped
/// character.
pub(super) fn shaped_placeholder_text(text: &str, placeholders: &[TextPlaceholder]) -> String {
    let mut shaped = text.to_owned();
    let mut buffer = [0; 4];
    let replacement = SHAPED_PLACEHOLDER_CHAR.encode_utf8(&mut buffer);
    for (_, placeholder) in placeholders_in(text, placeholders) {
        let end = placeholder.index + PLACEHOLDER_CHAR.len_utf8();
        shaped.replace_range(placeholder.index..end, replacement);
    }
    shaped
}

/// The Parley box reserving `placeholder`. SkParagraph's
/// `Run::updateMetrics` places baseline, above- and below-baseline boxes by
/// the box alone, and top and bottom boxes against the enclosing font's
/// ascent and descent (Parley's text-top and text-bottom). A middle box is
/// centred on the font's ascent-descent midpoint, which needs the font
/// metrics; it starts at text-top and [`center_middle_boxes`] shifts it once
/// the paragraph is shaped.
pub(super) fn inline_box(placeholder: &TextPlaceholder) -> InlineBox {
    let height = placeholder.size.height.max(0.);
    let (baseline, vertical_align) = match placeholder.alignment {
        WidgetSpanAlignment::Baseline => (placeholder.baseline, VerticalAlign::BASELINE),
        WidgetSpanAlignment::AboveBaseline => (Some(height), VerticalAlign::BASELINE),
        WidgetSpanAlignment::BelowBaseline => (Some(0.), VerticalAlign::BASELINE),
        WidgetSpanAlignment::TextTop | WidgetSpanAlignment::Middle => {
            (None, VerticalAlign::TEXT_TOP)
        }
        WidgetSpanAlignment::TextBottom => (None, VerticalAlign::TEXT_BOTTOM),
    };
    InlineBox {
        id: placeholder.id,
        kind: InlineBoxKind::InFlow,
        index: placeholder.index,
        width: placeholder.size.width.max(0.),
        height,
        baseline,
        vertical_align,
    }
}

/// Centres middle-aligned boxes as SkParagraph's `kMiddle` does: the box
/// midpoint sits `(ascent - descent) / 2` above the baseline, with the
/// ascent and descent of the font shaping the placeholder's own character.
/// Returns whether any box moved, so the caller breaks the lines again.
pub(super) fn center_middle_boxes(
    layout: &mut Layout<()>,
    placeholders: &[TextPlaceholder],
) -> bool {
    let middles: Vec<(usize, f32)> = placeholders
        .iter()
        .filter(|placeholder| placeholder.alignment == WidgetSpanAlignment::Middle)
        .filter_map(|placeholder| {
            let (ascent, descent) = layout.lines().find_map(|line| {
                line.runs()
                    .find(|run| run.text_range().contains(&placeholder.index))
                    .map(|run| (run.font_metrics().ascent, run.font_metrics().descent))
            })?;
            let height = placeholder.size.height.max(0.);
            // Text-top puts the box top at the ascent; raise it so the
            // midpoint lands at (ascent - descent) / 2.
            let shift = height / 2. - (ascent + descent) / 2.;
            Some((placeholder.index, shift))
        })
        .collect();
    if middles.is_empty() {
        return false;
    }
    for inline_box in layout.inline_boxes_mut() {
        if let Some((_, shift)) = middles.iter().find(|(index, _)| *index == inline_box.index) {
            inline_box.vertical_align =
                VerticalAlign::new(AlignmentBaseline::TextTop, BaselineShift::Length(*shift));
        }
    }
    true
}
