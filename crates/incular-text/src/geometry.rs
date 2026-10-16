//! Line geometry shared by whole-text layouts and per-paragraph document
//! layouts, so editing code reads either through one interface.

use crate::{TextCaretPosition, TextClusterSpan, TextLayout, TextLine, TextMetrics};
use incular_core::Offset;
use incular_rendering::{GlyphPosition, GlyphRun};
use std::sync::Arc;

/// Line geometry of laid-out text: lines in layout coordinates, their caret
/// stops and the overall metrics.
///
/// [`TextLayout`] implements it for one shaped text, and
/// [`TextDocumentLayout`](crate::TextDocumentLayout) for a document laid out
/// paragraph by paragraph.
pub trait TextGeometry {
    fn metrics(&self) -> TextMetrics;

    fn line_count(&self) -> usize;

    fn get_line(&self, index: usize) -> Option<TextLineRef<'_>>;

    fn lines(&self) -> TextLines<'_>;

    /// Lines that may meet the vertical span `top..bottom`, plus one line on
    /// each side; pad the span for glyphs that overhang their line.
    fn lines_between(&self, top: f32, bottom: f32) -> TextLines<'_>;

    /// Index of the line whose `[start, end)` holds `byte`, or the last line
    /// when none does, like Flutter's `Paragraph.getLineBoundary`.
    fn line_for_offset(&self, byte: usize) -> usize;

    /// The visual insertion points of one line in left-to-right screen order.
    fn line_caret_positions(&self, line: usize) -> &[TextCaretPosition];

    /// The line at `index`.
    ///
    /// # Panics
    ///
    /// Panics when `index` is not below [`TextGeometry::line_count`].
    fn line(&self, index: usize) -> TextLineRef<'_> {
        self.get_line(index).expect("text line index out of range")
    }
}

/// Layouts are shared as `Arc`s; geometry reads through them unchanged.
impl<T: TextGeometry + ?Sized> TextGeometry for Arc<T> {
    fn metrics(&self) -> TextMetrics {
        (**self).metrics()
    }

    fn line_count(&self) -> usize {
        (**self).line_count()
    }

    fn get_line(&self, index: usize) -> Option<TextLineRef<'_>> {
        (**self).get_line(index)
    }

    fn lines(&self) -> TextLines<'_> {
        (**self).lines()
    }

    fn lines_between(&self, top: f32, bottom: f32) -> TextLines<'_> {
        (**self).lines_between(top, bottom)
    }

    fn line_for_offset(&self, byte: usize) -> usize {
        (**self).line_for_offset(byte)
    }

    fn line_caret_positions(&self, line: usize) -> &[TextCaretPosition] {
        (**self).line_caret_positions(line)
    }
}

impl TextGeometry for TextLayout {
    fn metrics(&self) -> TextMetrics {
        self.metrics
    }

    fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn get_line(&self, index: usize) -> Option<TextLineRef<'_>> {
        let line = self.lines.get(index)?;
        Some(TextLineRef::new(line, index, 0, 0.))
    }

    fn lines(&self) -> TextLines<'_> {
        TextLines::flat(&self.lines, 0, self.lines.len())
    }

    fn lines_between(&self, top: f32, bottom: f32) -> TextLines<'_> {
        let first = self
            .lines
            .partition_point(|line| line.baseline < top)
            .saturating_sub(1);
        let end = (self.lines.partition_point(|line| line.baseline <= bottom) + 1)
            .min(self.lines.len())
            .max(first);
        TextLines::flat(&self.lines, first, end)
    }

    fn line_for_offset(&self, byte: usize) -> usize {
        let last = self.lines.len().saturating_sub(1);
        let line = self.lines.partition_point(|line| line.start <= byte);
        match line.checked_sub(1) {
            Some(line) if byte < self.lines[line].end => line,
            _ => last,
        }
    }

    fn line_caret_positions(&self, line: usize) -> &[TextCaretPosition] {
        TextLayout::line_caret_positions(self, line)
    }
}

/// A line of laid-out text in layout coordinates: byte offsets index the
/// laid-out text and vertical positions are measured from its top.
#[derive(Clone, Copy, Debug)]
pub struct TextLineRef<'a> {
    line: &'a TextLine,
    index: usize,
    byte: usize,
    y: f32,
}

impl<'a> TextLineRef<'a> {
    pub(crate) const fn new(line: &'a TextLine, index: usize, byte: usize, y: f32) -> Self {
        Self {
            line,
            index,
            byte,
            y,
        }
    }

    /// The line's index in its layout.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    #[must_use]
    pub const fn start(&self) -> usize {
        self.line.start + self.byte
    }

    /// End of selectable/caret text on this visual line. This excludes a
    /// trailing hard line-break which has no shaped glyph of its own.
    #[must_use]
    pub const fn caret_end(&self) -> usize {
        self.line.caret_end + self.byte
    }

    #[must_use]
    pub const fn end(&self) -> usize {
        self.line.end + self.byte
    }

    #[must_use]
    pub fn baseline(&self) -> f32 {
        self.line.baseline + self.y
    }

    /// The unshifted advance; see [`TextLineRef::offset`].
    #[must_use]
    pub const fn width(&self) -> f32 {
        self.line.width
    }

    /// Horizontal alignment offset in text-area coordinates. Glyph and caret
    /// x positions already include it; consumers mapping absolute x
    /// positions add it to `width` instead of re-deriving the alignment.
    #[must_use]
    pub const fn offset(&self) -> f32 {
        self.line.offset
    }

    /// Glyph runs positioned relative to [`TextLineRef::paint_origin`].
    #[must_use]
    pub fn runs(&self) -> &'a [Arc<GlyphRun>] {
        &self.line.runs
    }

    /// Where the line's glyph runs are drawn: the top of its paragraph, or
    /// the layout's origin for a whole-text layout.
    #[must_use]
    pub const fn paint_origin(&self) -> Offset {
        Offset::new(0., self.y)
    }

    /// Positioned glyphs with clusters indexing the laid-out text and y in
    /// the renderer's negative space below the layout's top.
    pub fn glyphs(&self) -> impl ExactSizeIterator<Item = GlyphPosition> + 'a {
        let (byte, y) = (self.byte as u32, self.y);
        self.line.glyphs.iter().map(move |glyph| GlyphPosition {
            offset: Offset::new(glyph.offset.x, glyph.offset.y - y),
            cluster: glyph.cluster + byte,
            ..*glyph
        })
    }

    /// Shaped cluster coverage in visual order; see [`TextClusterSpan`].
    pub fn clusters(&self) -> impl ExactSizeIterator<Item = TextClusterSpan> + 'a {
        let byte = self.byte as u32;
        self.line.clusters.iter().map(move |span| TextClusterSpan {
            start: span.start + byte,
            end: span.end + byte,
            ..*span
        })
    }

    /// Visual x-intervals covered by a half-open byte range of the laid-out
    /// text; see [`TextLine::selection_spans`].
    #[must_use]
    pub fn selection_spans(&self, start: usize, end: usize) -> Vec<(f32, f32)> {
        if end < self.byte {
            return Vec::new();
        }
        self.line.selection_spans(
            start.saturating_sub(self.byte),
            end.saturating_sub(self.byte),
        )
    }

    /// The shaped line in the coordinates of the text it was shaped from.
    #[must_use]
    pub const fn shaped(&self) -> &'a TextLine {
        self.line
    }
}

/// One paragraph of a document layout and where it sits in the document.
#[derive(Clone, Debug)]
pub(crate) struct ParagraphSlot {
    /// Byte offset of the paragraph in the document's text.
    pub(crate) byte: usize,
    /// Vertical offset of the paragraph's top in the document.
    pub(crate) y: f32,
    /// Index of the paragraph's first line in the document.
    pub(crate) first_line: usize,
    /// The paragraph shaped on its own, in its own coordinates.
    pub(crate) layout: Arc<TextLayout>,
}

/// Lines sharing one origin: a paragraph, or every line of a whole layout.
#[derive(Clone, Copy, Debug)]
struct LineSegment<'a> {
    lines: &'a [TextLine],
    first_line: usize,
    byte: usize,
    y: f32,
}

impl<'a> LineSegment<'a> {
    fn of(slot: &'a ParagraphSlot) -> Self {
        Self {
            lines: &slot.layout.lines,
            first_line: slot.first_line,
            byte: slot.byte,
            y: slot.y,
        }
    }
}

/// Iterator over the lines of a [`TextGeometry`].
#[derive(Clone, Debug)]
pub struct TextLines<'a> {
    current: LineSegment<'a>,
    next: usize,
    rest: &'a [ParagraphSlot],
    remaining: usize,
}

impl<'a> TextLines<'a> {
    /// Lines `first..end` of a whole layout.
    fn flat(lines: &'a [TextLine], first: usize, end: usize) -> Self {
        Self {
            current: LineSegment {
                lines,
                first_line: 0,
                byte: 0,
                y: 0.,
            },
            next: first,
            rest: &[],
            remaining: end.saturating_sub(first),
        }
    }

    /// Every line of `slots`.
    pub(crate) fn paragraphs(slots: &'a [ParagraphSlot]) -> Self {
        let remaining = slots.iter().map(|slot| slot.layout.lines.len()).sum();
        match slots.split_first() {
            Some((first, rest)) => Self {
                current: LineSegment::of(first),
                next: 0,
                rest,
                remaining,
            },
            None => Self::flat(&[], 0, 0),
        }
    }
}

impl<'a> Iterator for TextLines<'a> {
    type Item = TextLineRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        while self.next >= self.current.lines.len() {
            let (slot, rest) = self.rest.split_first()?;
            self.current = LineSegment::of(slot);
            self.rest = rest;
            self.next = 0;
        }
        let segment = self.current;
        let index = self.next;
        self.next += 1;
        self.remaining -= 1;
        Some(TextLineRef::new(
            &segment.lines[index],
            segment.first_line + index,
            segment.byte,
            segment.y,
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for TextLines<'_> {}
