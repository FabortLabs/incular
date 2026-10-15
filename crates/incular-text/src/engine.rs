//! Parley-backed text shaping, fallback, bidi ordering, and line breaking.
//!
//! Incular deliberately keeps its renderer-facing glyph protocol small. Parley
//! owns all font discovery, fallback selection and shaping; this module only
//! translates its positioned glyph runs into `incular-rendering` commands.

use super::{FontFamily, FontStyle, TextAffinity, TextAlign, TextOverflow, TextStyle};
use icu_segmenter::GraphemeClusterSegmenter;
pub use incular_assets::FontId;
use incular_assets::{FontBytes, FontHandle};
use incular_config::TextDirection;
use incular_core::{Offset, Size};
use incular_rendering::{GlyphPosition, GlyphRun};
use lru::LruCache;
use parley::{
    Alignment, AlignmentOptions, BaseDirection, FontContext, FontWeight as ParleyFontWeight,
    Layout, LayoutContext, PositionedLayoutItem, StyleProperty,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    sync::{Arc, OnceLock},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetrics {
    pub size: Size,
    pub baseline: f32,
    pub line_height: f32,
}

/// A shaped cluster's selectable source coverage, in text-area coordinates
/// with `left <= right`. Spans follow visual order, so a logical byte
/// range can cover several disjoint spans in mixed-direction text.
/// Consumers merge spans intersecting a selection instead of assuming
/// the range maps to one rectangle. Zero-advance clusters are kept because
/// they still own caret stops; selection ignores them.
#[derive(Clone, Copy, Debug)]
pub struct TextClusterSpan {
    pub start: usize,
    pub end: usize,
    pub left: f32,
    pub right: f32,
    /// Whether the cluster belongs to a right-to-left run.
    pub rtl: bool,
}

impl TextLine {
    /// Visual x-intervals covered by a half-open logical byte range on
    /// this line. Collapsed and reversed ranges yield nothing by
    /// policy; callers pass normalized ranges and never rely on
    /// rejection. Caret rendering stays separate: collapsed positions
    /// highlight nothing here and resolve through caret stops instead.
    ///
    /// Four granularities meet here and are not interchangeable: the
    /// range arrives in UTF-8 bytes (clamped to char boundaries
    /// upstream, so mid-char bytes never arrive); shaping clusters are
    /// the coverage granularity (a range splitting a cluster expands
    /// to the whole shaped cluster, since no visual edge exists
    /// inside it); visual order comes from the stored spans, never
    /// from re-sorting bytes or glyphs downstream. Touching spans
    /// merge; genuinely disjoint spans stay separate. An empty line
    /// covered by a nonempty range yields its caret point. Join controls
    /// are excluded from source coverage without adding caret stops inside
    /// the complete shaping cluster.
    #[must_use]
    pub fn selection_spans(&self, start: usize, end: usize) -> Vec<(f32, f32)> {
        if start >= end {
            return Vec::new();
        }
        let mut spans: Vec<(f32, f32)> = self
            .clusters
            .iter()
            .filter(|span| span.left < span.right && span.start < end && start < span.end)
            .map(|span| (span.left, span.right))
            .collect();
        if spans.is_empty() && self.start == self.end && start <= self.start && self.end <= end {
            spans.push((self.offset, self.offset));
        }
        spans.sort_by(|left, right| left.0.total_cmp(&right.0));
        let mut merged: Vec<(f32, f32)> = Vec::with_capacity(spans.len());
        for (left, right) in spans {
            if let Some(last) = merged.last_mut()
                && left <= last.1
            {
                last.1 = last.1.max(right);
            } else {
                merged.push((left, right));
            }
        }
        merged
    }
}

/// One visual line as shaped within its paragraph.
///
/// Byte offsets (`start`, `caret_end`, `end`, cluster spans and glyph
/// clusters) count from the start of the paragraph, and vertical positions
/// (`baseline`, glyph run origins and glyph y) from its top. A layout keeps
/// lines this way so editing one paragraph of a document shares every other
/// paragraph's lines instead of rewriting their positions. Read lines through
/// [`TextLayout::line`] or [`TextLayout::lines`], whose [`TextLineRef`] adds
/// the paragraph's position.
#[derive(Clone, Debug)]
pub struct TextLine {
    pub runs: Arc<[Arc<GlyphRun>]>,
    pub glyphs: Arc<[GlyphPosition]>,
    pub clusters: Arc<[TextClusterSpan]>,
    pub width: f32,
    /// Horizontal alignment offset in text-area coordinates. Glyph and
    /// caret x positions already include it; `width` stays the unshifted
    /// advance so extent checks keep working. Consumers mapping absolute
    /// x positions must add this to `width` instead of re-deriving the
    /// alignment.
    pub offset: f32,
    pub baseline: f32,
    pub start: usize,
    /// End of selectable/caret text on this visual line. This excludes a
    /// trailing hard line-break which has no shaped glyph of its own.
    pub caret_end: usize,
    pub end: usize,
}

#[derive(Clone, Debug)]
pub struct FontRunDebug {
    pub range: std::ops::Range<usize>,
    pub font: FontId,
    pub direction: &'static str,
}

/// One paragraph of a layout: its lines, shaped relative to the paragraph,
/// and where the paragraph sits in the layout.
#[derive(Clone, Debug)]
struct ParagraphSlot {
    /// Byte offset of the paragraph in the layout's text.
    byte: usize,
    /// Vertical offset of the paragraph's top in the layout.
    y: f32,
    /// Index of the paragraph's first line in the layout.
    first_line: usize,
    lines: Arc<[TextLine]>,
    font_runs: Arc<[FontRunDebug]>,
    /// The paragraph's own extent, summed again when a later composition
    /// carries this slot: width, height, line height, overflow. Only composed
    /// documents fill these; a one-paragraph layout's extent is its metrics.
    width: f32,
    height: f32,
    line_height: f32,
    overflowed: bool,
}

impl ParagraphSlot {
    fn leaf(lines: Arc<[TextLine]>, font_runs: Arc<[FontRunDebug]>) -> Self {
        Self {
            byte: 0,
            y: 0.,
            first_line: 0,
            lines,
            font_runs,
            width: 0.,
            height: 0.,
            line_height: 0.,
            overflowed: false,
        }
    }

    fn line(&self, index: usize) -> TextLineRef<'_> {
        TextLineRef {
            line: &self.lines[index],
            index: self.first_line + index,
            byte: self.byte,
            y: self.y,
        }
    }
}

/// A line of a [`TextLayout`] in layout coordinates: byte offsets index the
/// layout's text and vertical positions are measured from its top.
#[derive(Clone, Copy, Debug)]
pub struct TextLineRef<'a> {
    line: &'a TextLine,
    index: usize,
    byte: usize,
    y: f32,
}

impl<'a> TextLineRef<'a> {
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

    /// Where the line's glyph runs are drawn: the top of its paragraph.
    #[must_use]
    pub const fn paint_origin(&self) -> Offset {
        Offset::new(0., self.y)
    }

    /// Positioned glyphs with clusters indexing the layout's text and y in
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
        let byte = self.byte;
        self.line.clusters.iter().map(move |span| TextClusterSpan {
            start: span.start + byte,
            end: span.end + byte,
            ..*span
        })
    }

    /// Visual x-intervals covered by a half-open byte range of the layout's
    /// text; see [`TextLine::selection_spans`].
    #[must_use]
    pub fn selection_spans(&self, start: usize, end: usize) -> Vec<(f32, f32)> {
        let local = |byte: usize| byte.saturating_sub(self.byte);
        if end < self.byte {
            return Vec::new();
        }
        self.line.selection_spans(local(start), local(end))
    }

    /// The line in its paragraph's coordinates.
    #[must_use]
    pub const fn paragraph_line(&self) -> &'a TextLine {
        self.line
    }
}

/// A paragraph of a [`TextLayout`], for drawing its lines under one
/// translation.
#[derive(Clone, Copy, Debug)]
pub struct TextParagraphRef<'a> {
    slot: &'a ParagraphSlot,
}

impl<'a> TextParagraphRef<'a> {
    /// Byte offset of the paragraph in the layout's text.
    #[must_use]
    pub const fn text_offset(&self) -> usize {
        self.slot.byte
    }

    /// Translation that places the paragraph's lines in the layout.
    #[must_use]
    pub const fn paint_origin(&self) -> Offset {
        Offset::new(0., self.slot.y)
    }

    /// The paragraph's lines in its own coordinates.
    #[must_use]
    pub fn lines(&self) -> &'a [TextLine] {
        &self.slot.lines
    }
}

/// Iterator over the lines of a [`TextLayout`].
#[derive(Clone, Debug)]
pub struct TextLines<'a> {
    paragraphs: &'a [ParagraphSlot],
    line: usize,
    remaining: usize,
}

impl<'a> Iterator for TextLines<'a> {
    type Item = TextLineRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(slot) = self.paragraphs.first() {
            if self.line < slot.lines.len() {
                self.line += 1;
                self.remaining -= 1;
                return Some(slot.line(self.line - 1));
            }
            self.paragraphs = &self.paragraphs[1..];
            self.line = 0;
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for TextLines<'_> {}

/// A visual insertion point emitted by the shaping engine.
///
/// A logical byte offset can have two distinct visual positions at a bidi run
/// boundary. `affinity` keeps those positions separate, matching the concept
/// used by Parley's cursor implementation and preventing an arrow key from
/// jumping through a right-to-left run in logical UTF-8 order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextCaretPosition {
    pub offset: usize,
    pub affinity: TextAffinity,
    pub x: f32,
    pub line: usize,
}

#[derive(Clone, Debug)]
struct ClusterPosition {
    range: std::ops::Range<usize>,
    left: f32,
    right: f32,
    rtl: bool,
    ligature_start: bool,
    ligature_continuation: bool,
}

impl ClusterPosition {
    fn include(&mut self, other: &Self) {
        self.range.start = self.range.start.min(other.range.start);
        self.range.end = self.range.end.max(other.range.end);
        self.left = self.left.min(other.left);
        self.right = self.right.max(other.right);
        self.ligature_start |= other.ligature_start;
        self.ligature_continuation |= other.ligature_continuation;
    }

    fn can_join(&self, other: &Self) -> bool {
        self.rtl == other.rtl
            && if self.rtl {
                other.range.end == self.range.start
            } else {
                self.range.end == other.range.start
            }
    }
}

/// One lazily derived caret-stop list per laid-out line.
type LineCaretStops = Arc<[OnceLock<Arc<[TextCaretPosition]>>]>;

#[derive(Clone, Debug)]
pub struct TextLayout {
    /// Paragraphs in text order; one for text without hard breaks.
    paragraphs: Arc<[ParagraphSlot]>,
    line_count: usize,
    pub metrics: TextMetrics,
    /// Caret stops of every line, usually derived on first use. Layouts
    /// containing join controls retain the complete shaping boundaries
    /// before selection coverage excludes those controls.
    caret_positions: OnceLock<Arc<[TextCaretPosition]>>,
    /// Per-line caret stops derived on demand when `caret_positions` was not
    /// retained, so locating the caret in a long document derives one line
    /// instead of every stop of every line.
    line_carets: OnceLock<LineCaretStops>,
    /// Whether the source had content outside the retained layout's visible
    /// bounds. `Visible` layouts deliberately leave this false.
    pub overflowed: bool,
}

/// Paragraph-level layout controls. They deliberately describe behavior in
/// Incular terms and do not expose Parley builder types.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLayoutOptions {
    pub max_width: Option<f32>,
    pub align: TextAlign,
    /// Explicit paragraph base direction; `None` retains automatic detection.
    pub text_direction: Option<TextDirection>,
    pub soft_wrap: bool,
    pub max_lines: Option<usize>,
    pub overflow: TextOverflow,
}

impl TextLayoutOptions {
    #[must_use]
    pub const fn new(max_width: Option<f32>, align: TextAlign) -> Self {
        Self {
            max_width,
            align,
            text_direction: None,
            soft_wrap: true,
            max_lines: None,
            overflow: TextOverflow::Clip,
        }
    }

    #[must_use]
    pub const fn soft_wrap(mut self, soft_wrap: bool) -> Self {
        self.soft_wrap = soft_wrap;
        self
    }

    #[must_use]
    pub const fn text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }

    #[must_use]
    pub const fn max_lines(mut self, max_lines: Option<usize>) -> Self {
        self.max_lines = max_lines;
        self
    }

    #[must_use]
    pub const fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }
}

impl Default for TextLayoutOptions {
    fn default() -> Self {
        Self::new(None, TextAlign::Start)
    }
}

impl TextLayout {
    /// A layout of one paragraph.
    fn leaf(lines: Arc<[TextLine]>, font_runs: Arc<[FontRunDebug]>, metrics: TextMetrics) -> Self {
        Self::from_paragraphs(vec![ParagraphSlot::leaf(lines, font_runs)].into(), metrics)
    }

    fn from_paragraphs(paragraphs: Arc<[ParagraphSlot]>, metrics: TextMetrics) -> Self {
        let line_count = paragraphs.iter().map(|slot| slot.lines.len()).sum();
        Self {
            paragraphs,
            line_count,
            metrics,
            caret_positions: OnceLock::new(),
            line_carets: OnceLock::new(),
            overflowed: false,
        }
    }

    /// Lines of a one-paragraph layout, for shaping code before composition.
    fn leaf_lines(&self) -> &[TextLine] {
        debug_assert!(self.paragraphs.len() <= 1, "leaf layout");
        self.paragraphs.first().map_or(&[], |slot| &slot.lines)
    }

    fn leaf_lines_mut(&mut self) -> &mut [TextLine] {
        debug_assert!(self.paragraphs.len() == 1, "leaf layout");
        Arc::make_mut(&mut Arc::make_mut(&mut self.paragraphs)[0].lines)
    }

    #[must_use]
    pub const fn line_count(&self) -> usize {
        self.line_count
    }

    /// The line at `index`.
    ///
    /// # Panics
    ///
    /// Panics when `index` is not below [`TextLayout::line_count`].
    #[must_use]
    pub fn line(&self, index: usize) -> TextLineRef<'_> {
        self.get_line(index).expect("text line index out of range")
    }

    #[must_use]
    pub fn get_line(&self, index: usize) -> Option<TextLineRef<'_>> {
        if index >= self.line_count {
            return None;
        }
        let slot = self
            .paragraphs
            .partition_point(|slot| slot.first_line <= index)
            .checked_sub(1)?;
        let slot = &self.paragraphs[slot];
        Some(slot.line(index - slot.first_line))
    }

    pub fn lines(&self) -> TextLines<'_> {
        TextLines {
            paragraphs: &self.paragraphs,
            line: 0,
            remaining: self.line_count,
        }
    }

    pub fn paragraphs(&self) -> impl ExactSizeIterator<Item = TextParagraphRef<'_>> {
        self.paragraphs.iter().map(|slot| TextParagraphRef { slot })
    }

    /// Paragraphs whose vertical span meets `top..bottom`, in layout
    /// coordinates, for painting only what a viewport shows. Pad the range
    /// for glyphs that overhang their line.
    pub fn paragraphs_between(
        &self,
        top: f32,
        bottom: f32,
    ) -> impl ExactSizeIterator<Item = TextParagraphRef<'_>> {
        let first = self
            .paragraphs
            .partition_point(|slot| slot.y <= top)
            .saturating_sub(1);
        let end = self
            .paragraphs
            .partition_point(|slot| slot.y < bottom)
            .max(first);
        self.paragraphs[first..end]
            .iter()
            .map(|slot| TextParagraphRef { slot })
    }

    /// Index of the line whose `[start, end)` holds `byte`, or the last line
    /// when none does, like Flutter's `Paragraph.getLineBoundary`.
    #[must_use]
    pub fn line_for_offset(&self, byte: usize) -> usize {
        let last = self.line_count.saturating_sub(1);
        let Some(slot) = self
            .paragraphs
            .partition_point(|slot| slot.byte <= byte)
            .checked_sub(1)
        else {
            return last;
        };
        let slot = &self.paragraphs[slot];
        let local = byte - slot.byte;
        let line = slot.lines.partition_point(|line| line.start <= local);
        match line.checked_sub(1) {
            Some(line) if local < slot.lines[line].end => slot.first_line + line,
            _ => last,
        }
    }

    /// Font runs with ranges indexing the layout's text.
    pub fn font_runs(&self) -> impl Iterator<Item = FontRunDebug> + '_ {
        self.paragraphs.iter().flat_map(|slot| {
            slot.font_runs.iter().map(|run| FontRunDebug {
                range: run.range.start + slot.byte..run.range.end + slot.byte,
                ..run.clone()
            })
        })
    }

    #[must_use]
    pub fn glyph_count(&self) -> usize {
        self.paragraphs
            .iter()
            .flat_map(|slot| slot.lines.iter())
            .map(|line| line.glyphs.len())
            .sum()
    }

    /// Returns the visual insertion points for one laid-out line in
    /// left-to-right screen order.
    #[must_use]
    pub fn line_caret_positions(&self, line: usize) -> &[TextCaretPosition] {
        if let Some(caret_positions) = self.caret_positions.get() {
            // Retained stops are grouped by ascending line.
            let start = caret_positions.partition_point(|position| position.line < line);
            let end = caret_positions.partition_point(|position| position.line <= line);
            return &caret_positions[start..end];
        }
        if line >= self.line_count {
            return &[];
        }
        let lines = self.line_carets.get_or_init(|| {
            (0..self.line_count)
                .map(|_| OnceLock::new())
                .collect::<Vec<_>>()
                .into()
        });
        lines[line].get_or_init(|| {
            let mut positions = Vec::new();
            self.push_line_caret_positions(line, &mut positions);
            positions.into()
        })
    }

    fn derive_caret_positions(&self) -> Arc<[TextCaretPosition]> {
        let mut positions = Vec::new();
        for index in 0..self.line_count {
            self.push_line_caret_positions(index, &mut positions);
        }
        positions.into()
    }

    /// Two stops per cluster (logical start and end, placed by direction),
    /// plus the line's start and caret end when no cluster supplies them,
    /// sorted into screen order.
    fn push_line_caret_positions(&self, index: usize, positions: &mut Vec<TextCaretPosition>) {
        let line = self.line(index);
        let stop = |offset, affinity, x| TextCaretPosition {
            offset,
            affinity,
            x,
            line: index,
        };
        let first = positions.len();
        for cluster in line.clusters() {
            let (left, right) = if cluster.rtl {
                (cluster.end, cluster.start)
            } else {
                (cluster.start, cluster.end)
            };
            let (left_affinity, right_affinity) = if cluster.rtl {
                (TextAffinity::Upstream, TextAffinity::Downstream)
            } else {
                (TextAffinity::Downstream, TextAffinity::Upstream)
            };
            positions.push(stop(left, left_affinity, cluster.left));
            positions.push(stop(right, right_affinity, cluster.right));
        }
        let line_stops = &positions[first..];
        if !line_stops.iter().any(|stop| stop.offset == line.start()) {
            positions.push(stop(line.start(), TextAffinity::Downstream, line.offset()));
        }
        if line.caret_end() > line.start()
            && !positions[first..]
                .iter()
                .any(|stop| stop.offset == line.caret_end())
        {
            positions.push(stop(
                line.caret_end(),
                TextAffinity::Upstream,
                line.offset() + line.width(),
            ));
        }
        positions[first..].sort_by(|left, right| {
            left.x
                .total_cmp(&right.x)
                .then_with(|| left.offset.cmp(&right.offset))
        });
    }
}

/// Layout/cache counters. Legacy field names are retained for source
/// compatibility; Parley performs fallback and coverage resolution internally.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextDiagnostics {
    pub layouts_requested: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub font_resolution_requests: u64,
    pub font_resolution_cache_hits: u64,
    pub font_resolution_cache_misses: u64,
    pub coverage_queries: u64,
    pub fallback_runs: u64,
    pub fallback_fonts_used: u64,
    pub missing_clusters: u64,
    pub shaping_runs: u64,
    // Task 15 document-granularity instrumentation.
    /// Paragraphs examined by the document path.
    pub paragraphs_considered: u64,
    /// Paragraphs that required fresh shaping (cache misses at paragraph
    /// granularity).
    pub paragraphs_reshaped: u64,
    /// Parley layouts satisfied entirely from cache.
    pub parley_layouts_reused: u64,
    /// Multi-paragraph documents assembled from retained paragraphs.
    pub documents_composed: u64,
    /// Multi-paragraph documents returned whole from the cache.
    pub documents_reused: u64,
    /// Layouts served from a single-line layout shaped at another width.
    pub width_reuses: u64,
    /// Times a layout pool doubled because its working set did not fit.
    pub cache_growths: u64,
    /// Leading paragraphs carried unchanged from an earlier composition.
    pub paragraphs_carried: u64,
    /// Wrapping paragraphs re-broken at a new width without reshaping.
    pub shapes_reused: u64,
}

#[derive(Hash, PartialEq, Eq)]
struct LayoutKey {
    text: String,
    family: FontFamily,
    fallbacks: Arc<[FontFamily]>,
    size: u32,
    weight: u16,
    style: FontStyle,
    line_height: Option<(u8, u32)>,
    letter_spacing: u32,
    font_variations: Option<Arc<str>>,
    font_features: Option<Arc<str>>,
    width: Option<u32>,
    align: TextAlign,
    text_direction: Option<TextDirection>,
    soft_wrap: bool,
    max_lines: Option<usize>,
    overflow: TextOverflow,
    generation: u64,
}

impl LayoutKey {
    fn new(text: &str, style: &TextStyle, options: TextLayoutOptions, generation: u64) -> Self {
        Self {
            text: text.into(),
            family: style.family.clone(),
            fallbacks: style.fallback_families.clone(),
            size: style.size.to_bits(),
            weight: style.weight.0,
            style: style.style,
            line_height: style.line_height.map(|lh| match lh {
                crate::LineHeight::Normal => (0, 0),
                crate::LineHeight::Multiplier(m) => (1, m.to_bits()),
                crate::LineHeight::Absolute(a) => (2, a.to_bits()),
            }),
            letter_spacing: style.letter_spacing.to_bits(),
            font_variations: style.font_variations.clone(),
            font_features: style.font_features.clone(),
            width: options.max_width.map(f32::to_bits),
            align: options.align,
            text_direction: options.text_direction,
            soft_wrap: options.soft_wrap,
            max_lines: options.max_lines,
            overflow: options.overflow,
            generation,
        }
    }
}

/// The sole owner of font discovery and app-registered fonts. Parley owns the
/// actual font collection and resolves scripts, bidi direction and fallback.
pub struct TextEngine {
    /// One `FontHandle` per distinct font blob. Handles share the font bytes
    /// through a single `Arc`, so every cached text layout references the
    /// same allocation instead of holding a private copy of the whole font
    /// file (the Task 15 root cause of ~500 KB retained per shaped string
    /// and a full-file memcpy plus full-file hash per glyph run).
    font_handles: HashMap<(usize, usize, u32), FontHandle>,
    font_context: FontContext,
    layout_context: LayoutContext<()>,
    generation: u64,
    cache: LayoutPool,
    /// Width-independent single-line layouts keyed without their width; see
    /// `shape_or_reuse`.
    unwrapped: LayoutPool,
    /// Shaped Parley layouts of paragraphs that wrapped, keyed without their
    /// width; see `shape_raw`.
    shaped: LayoutPool<Layout<()>>,
    /// Recent compositions, most recent first; see `carried_paragraphs`.
    documents: Vec<ComposedDocument>,
    diagnostics: TextDiagnostics,
}

/// Compositions retained for carrying paragraphs into the next edit: a few
/// fields or editor groups can be edited alternately.
const COMPOSED_DOCUMENTS: usize = 4;

/// A composed document, kept so composing an edited version can carry over
/// the paragraphs around its change instead of laying them out again.
struct ComposedDocument {
    /// Empty-text key carrying the style, paragraph options and generation.
    policy: LayoutKey,
    text: String,
    paragraphs: Arc<[ParagraphSlot]>,
}

/// Paragraphs of a retained composition that a new text shares: the first
/// `prefix` paragraphs unchanged in place, and paragraphs from `suffix` on
/// unchanged but moved by the edit.
struct CarriedParagraphs {
    document: usize,
    prefix: usize,
    suffix: usize,
}

/// Retained layouts kept beyond the largest composed document. A document
/// grows the pool to hold all of its paragraphs plus this many other entries,
/// so a file longer than the base pool cannot evict its own paragraphs before
/// they are reused.
const LAYOUT_CACHE_CAPACITY: NonZeroUsize = NonZeroUsize::new(2048).unwrap();

/// Upper bound a pool grows to when its working set keeps missing.
const MAX_RETAINED_LAYOUTS: usize = 1 << 16;

/// LRU of retained layouts that grows while its working set does not fit.
///
/// A cyclic pass over more keys than an LRU holds misses on every access,
/// because each entry is evicted just before its reuse: thousands of labels
/// relaid out at a new width, or the paragraphs of a long file. Keys evicted
/// recently are remembered by hash; missing one of them proves the working
/// set exceeded the capacity, so the pool doubles, up to
/// `MAX_RETAINED_LAYOUTS`.
struct LayoutPool<V = Arc<TextLayout>> {
    entries: LruCache<LayoutKey, V>,
    evicted: LruCache<u64, ()>,
    growths: u64,
}

impl<V> LayoutPool<V> {
    fn new() -> Self {
        Self {
            entries: LruCache::new(LAYOUT_CACHE_CAPACITY),
            evicted: LruCache::new(LAYOUT_CACHE_CAPACITY),
            growths: 0,
        }
    }

    fn get(&mut self, key: &LayoutKey) -> Option<V>
    where
        V: Clone,
    {
        if let Some(layout) = self.entries.get(key) {
            return Some(layout.clone());
        }
        self.note_miss(key);
        None
    }

    /// Removes and returns the entry, for values used mutably and put back.
    fn take(&mut self, key: &LayoutKey) -> Option<V> {
        let taken = self.entries.pop(key);
        if taken.is_none() {
            self.note_miss(key);
        }
        taken
    }

    fn note_miss(&mut self, key: &LayoutKey) {
        if self.evicted.pop(&key_hash(key)).is_some() {
            let doubled = self.entries.cap().get().saturating_mul(2);
            if self.reserve(doubled.min(MAX_RETAINED_LAYOUTS)) {
                self.growths += 1;
            }
        }
    }

    fn put(&mut self, key: LayoutKey, layout: V) {
        if let Some((evicted, _)) = self.entries.push(key, layout)
            && !self.entries.contains(&evicted)
        {
            self.evicted.put(key_hash(&evicted), ());
        }
    }

    /// Grows to at least `capacity` entries; returns whether it grew.
    fn reserve(&mut self, capacity: usize) -> bool {
        match NonZeroUsize::new(capacity) {
            Some(capacity) if capacity > self.entries.cap() => {
                self.entries.resize(capacity);
                self.evicted.resize(capacity);
                true
            }
            _ => false,
        }
    }

    fn capacity(&self) -> usize {
        self.entries.cap().get()
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.evicted.clear();
    }
}

fn key_hash(key: &LayoutKey) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

impl Default for TextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            font_handles: HashMap::new(),
            font_context: FontContext::new(),
            layout_context: LayoutContext::new(),
            generation: 1,
            cache: LayoutPool::new(),
            unwrapped: LayoutPool::new(),
            shaped: LayoutPool::new(),
            documents: Vec::new(),
            diagnostics: TextDiagnostics::default(),
        }
    }

    /// Runs `f` with this thread's shared measuring engine.
    ///
    /// Application code that measures text while building or inside a
    /// `LayoutBuilder` should use this instead of `TextEngine::new()`: a new
    /// engine starts with an empty cache and reshapes every string on every
    /// rebuild, while the shared engine keeps layouts across frames. A nested
    /// call from inside `f` gets a temporary engine rather than panicking.
    pub fn with_shared<R>(f: impl FnOnce(&mut Self) -> R) -> R {
        thread_local! {
            static SHARED: std::cell::RefCell<TextEngine> =
                std::cell::RefCell::new(TextEngine::new());
        }
        SHARED.with(|shared| match shared.try_borrow_mut() {
            Ok(mut engine) => f(&mut engine),
            Err(_) => f(&mut Self::new()),
        })
    }

    #[must_use]
    pub const fn diagnostics(&self) -> TextDiagnostics {
        let mut diagnostics = self.diagnostics;
        diagnostics.cache_growths =
            self.cache.growths + self.unwrapped.growths + self.shaped.growths;
        diagnostics
    }

    #[must_use]
    pub const fn font_database_generation(&self) -> u64 {
        self.generation
    }

    /// Registers all faces in a TrueType/OpenType byte buffer with Parley's
    /// Fontique collection. Invalid buffers are harmless: Fontique ignores
    /// them, while the generation still invalidates stale cached layouts.
    pub fn register_font_bytes(&mut self, bytes: impl Into<Vec<u8>>) {
        self.font_context
            .collection
            .register_fonts(parley::fontique::Blob::from(bytes.into()), None);
        self.generation = self
            .generation
            .checked_add(1)
            .expect("font database generation exhausted");
        self.cache.clear();
        self.unwrapped.clear();
        self.shaped.clear();
        self.documents.clear();
    }

    pub fn layout(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
        align: TextAlign,
    ) -> Arc<TextLayout> {
        self.layout_with_options(text, style, TextLayoutOptions::new(max_width, align))
    }

    /// Shapes a paragraph with retained wrapping and overflow policy.
    pub fn layout_with_options(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> Arc<TextLayout> {
        self.diagnostics.layouts_requested += 1;
        // Documents with hard line breaks compose from retained per-paragraph
        // layouts so editing one paragraph never reshapes the rest. Overflow
        // policies keep the exact monolithic semantics and stay on the
        // whole-string path.
        // With soft wrapping, `Clip` can only trigger on single-line
        // horizontal overflow, which a hard-break document never has; so this
        // route preserves monolithic semantics exactly. Without soft wrapping,
        // `Clip`/`Fade` against a fixed width only cut horizontal overflow to
        // the first line, which `layout_document` applies to the composition.
        let document = text.contains('\n')
            && options.max_lines.is_none()
            && (options.soft_wrap
                || options.overflow == TextOverflow::Visible
                || (options.max_width.is_some()
                    && matches!(options.overflow, TextOverflow::Clip | TextOverflow::Fade)));
        // The route is a pure function of the key, and paragraph keys never
        // contain a hard break, so composed documents share the cache safely.
        // An unchanged document is then one lookup instead of a re-assembly
        // that copies every glyph.
        let key = LayoutKey::new(text, style, options, self.generation);
        if let Some(layout) = self.cache.get(&key) {
            self.diagnostics.cache_hits += 1;
            if document {
                self.diagnostics.documents_reused += 1;
            }
            return layout;
        }
        let layout = if document {
            Arc::new(self.layout_document(text, style, options))
        } else {
            self.diagnostics.cache_misses += 1;
            self.shape_or_reuse(text, style, options)
        };
        self.cache.put(key, layout.clone());
        layout
    }

    /// Shapes `text`, or returns a layout shaped for the same text at another
    /// width when that layout is provably identical at this one.
    ///
    /// A single unclipped line whose alignment offset is zero is independent
    /// of the width it was shaped against, as long as it still fits: nothing
    /// wraps and start alignment in a left-to-right paragraph never moves it.
    /// Window resizes and flex measurement give labels new widths every
    /// frame; without this each such change reshaped every label.
    fn shape_or_reuse(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> Arc<TextLayout> {
        let key = LayoutKey::new(
            text,
            style,
            TextLayoutOptions {
                max_width: None,
                ..options
            },
            self.generation,
        );
        if let Some(layout) = self.unwrapped.get(&key)
            && options
                .max_width
                .is_none_or(|width| layout.metrics.size.width <= width)
        {
            self.diagnostics.width_reuses += 1;
            return layout;
        }
        let layout = Arc::new(self.shape_and_wrap(text, style, options));
        if width_independent(&layout, options) {
            self.unwrapped.put(key, layout.clone());
        }
        layout
    }

    /// Grows the retained pool so `paragraphs` fit beside the base pool. A
    /// cyclic scan longer than an LRU's capacity evicts every entry before its
    /// reuse, which made each layout of a long file reshape all of it.
    fn reserve_document(&mut self, paragraphs: usize) {
        let required = paragraphs.saturating_add(LAYOUT_CACHE_CAPACITY.get());
        if self.cache.capacity() < required {
            let capacity = required.next_power_of_two();
            self.cache.reserve(capacity);
            self.unwrapped.reserve(capacity);
        }
    }

    /// Composes a multi-paragraph document from per-paragraph retained
    /// layouts. Shaping reuse is exact (paragraph slice == cache key), and
    /// paragraphs keep their own lines: composing only records where each
    /// paragraph starts.
    fn layout_document(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> TextLayout {
        self.diagnostics.documents_composed += 1;
        let paragraph_count = text.bytes().filter(|byte| *byte == b'\n').count() + 1;
        self.reserve_document(paragraph_count);
        let paragraph_options = TextLayoutOptions {
            max_lines: None,
            overflow: TextOverflow::Visible,
            ..options
        };
        let policy = LayoutKey::new("", style, paragraph_options, self.generation);
        let mut slots: Vec<ParagraphSlot> = Vec::with_capacity(paragraph_count);
        let mut byte_start = 0usize;
        let mut height = 0f32;
        let mut line_count = 0usize;
        let mut document_carets = has_join_controls(text).then(Vec::new);
        // Paragraphs before the first change keep their position; those after
        // the last change keep their lines and move by the edit's size.
        let carried = if document_carets.is_none() {
            self.carried_paragraphs(&policy, text)
        } else {
            None
        };
        let mut suffix = None;
        if let Some(carried) = carried {
            let previous = &self.documents[carried.document];
            let paragraphs = previous.paragraphs.clone();
            let previous_len = previous.text.len();
            slots.extend_from_slice(&paragraphs[..carried.prefix]);
            let next = &paragraphs[carried.prefix];
            (byte_start, height, line_count) = (next.byte, next.y, next.first_line);
            self.diagnostics.paragraphs_carried += carried.prefix as u64;
            if carried.suffix < paragraphs.len() {
                let start = paragraphs[carried.suffix].byte + text.len() - previous_len;
                suffix = Some((paragraphs, carried.suffix, start));
            }
        }
        loop {
            if let Some((previous, first, start)) = &suffix
                && *start == byte_start
            {
                let anchor = &previous[*first];
                for slot in &previous[*first..] {
                    slots.push(ParagraphSlot {
                        byte: slot.byte - anchor.byte + byte_start,
                        y: slot.y - anchor.y + height,
                        first_line: slot.first_line - anchor.first_line + line_count,
                        ..slot.clone()
                    });
                }
                self.diagnostics.paragraphs_carried += (previous.len() - first) as u64;
                break;
            }
            let paragraph_end = text[byte_start..]
                .find('\n')
                .map_or(text.len(), |end| byte_start + end);
            let paragraph = &text[byte_start..paragraph_end];
            self.diagnostics.paragraphs_considered += 1;
            let key = LayoutKey::new(paragraph, style, paragraph_options, self.generation);
            let paragraph_layout = if let Some(existing) = self.cache.get(&key) {
                self.diagnostics.parley_layouts_reused += 1;
                self.diagnostics.cache_hits += 1;
                existing
            } else {
                self.diagnostics.paragraphs_reshaped += 1;
                self.diagnostics.cache_misses += 1;
                let built = self.shape_or_reuse(paragraph, style, paragraph_options);
                self.cache.put(key, built.clone());
                built
            };
            if let Some(carets) = &mut document_carets {
                let paragraph_carets = paragraph_layout
                    .caret_positions
                    .get_or_init(|| paragraph_layout.derive_caret_positions());
                carets.extend(paragraph_carets.iter().map(|position| TextCaretPosition {
                    offset: position.offset + byte_start,
                    line: position.line + line_count,
                    ..*position
                }));
            }
            // Paragraph keys never contain a hard break, so the layout is a
            // single paragraph whose lines are shared as they are.
            let lines = paragraph_layout.leaf_lines();
            slots.push(ParagraphSlot {
                byte: byte_start,
                y: height,
                first_line: line_count,
                lines: paragraph_layout.paragraphs[0].lines.clone(),
                font_runs: paragraph_layout.paragraphs[0].font_runs.clone(),
                width: paragraph_layout.metrics.size.width,
                height: paragraph_layout.metrics.size.height,
                line_height: paragraph_layout.metrics.line_height,
                overflowed: paragraph_layout.overflowed,
            });
            height += paragraph_layout.metrics.size.height;
            line_count += lines.len();
            if paragraph_end == text.len() {
                break;
            }
            byte_start = paragraph_end + 1; // include the hard break
        }
        let first = &slots[0];
        let last = slots.last().expect("a document has paragraphs");
        let metrics = TextMetrics {
            size: Size::new(
                slots.iter().fold(0f32, |width, slot| width.max(slot.width)),
                last.y + last.height,
            ),
            baseline: first.y + first.lines.first().map_or(0., |line| line.baseline),
            line_height: last.line_height,
        };
        let overflowed = slots.iter().any(|slot| slot.overflowed);
        let paragraphs: Arc<[ParagraphSlot]> = slots.into();
        self.documents.truncate(COMPOSED_DOCUMENTS - 1);
        self.documents.insert(
            0,
            ComposedDocument {
                policy,
                text: text.to_owned(),
                paragraphs: paragraphs.clone(),
            },
        );
        let mut layout = TextLayout::from_paragraphs(paragraphs, metrics);
        layout.overflowed = overflowed;
        if let Some(carets) = document_carets {
            layout.caret_positions = OnceLock::from(Arc::from(carets));
        }
        // Same horizontal-overflow policy as `shape_and_wrap` for unwrapped
        // text; the router only sends clipping overflows here.
        let horizontal_overflow = !options.soft_wrap
            && options.overflow != TextOverflow::Visible
            && options
                .max_width
                .is_some_and(|max| metrics.size.width > max);
        if horizontal_overflow {
            clipped_layout(layout, 1)
        } else {
            layout
        }
    }

    /// The retained composition sharing `policy` that carries the most
    /// paragraphs into `text`; see [`carried_paragraphs`].
    fn carried_paragraphs(&self, policy: &LayoutKey, text: &str) -> Option<CarriedParagraphs> {
        self.documents
            .iter()
            .enumerate()
            .filter(|(_, document)| document.policy == *policy)
            .map(|(index, document)| {
                let (prefix, suffix) =
                    carried_paragraphs(&document.paragraphs, &document.text, text);
                let carried = prefix + document.paragraphs.len() - suffix;
                (
                    carried,
                    CarriedParagraphs {
                        document: index,
                        prefix,
                        suffix,
                    },
                )
            })
            .filter(|(carried, _)| *carried > 0)
            .max_by_key(|(carried, _)| *carried)
            .map(|(_, carried)| carried)
    }

    fn font_stack(style: &TextStyle) -> parley::FontFamily<'static> {
        let mut names = vec![family_name(&style.family)];
        names.extend(style.fallback_families.iter().map(family_name));
        parley::FontFamily::Source(names.join(", ").into())
    }

    fn shape_and_wrap(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
    ) -> TextLayout {
        let wrap_width = options.soft_wrap.then_some(options.max_width).flatten();
        let layout = self.shape_raw(
            text,
            style,
            wrap_width,
            options.max_width,
            options.align,
            options.text_direction,
        );
        let horizontal_overflow = !options.soft_wrap
            && options
                .max_width
                .is_some_and(|width| layout.metrics.size.width > width);
        let line_limit = options
            .max_lines
            .or_else(|| horizontal_overflow.then_some(1));
        let line_overflow = line_limit.is_some_and(|limit| layout.line_count() > limit);
        if (!horizontal_overflow && !line_overflow) || options.overflow == TextOverflow::Visible {
            return layout;
        }

        let limit = line_limit.unwrap_or(layout.line_count());
        match options.overflow {
            TextOverflow::Visible => layout,
            TextOverflow::Clip | TextOverflow::Fade => clipped_layout(layout, limit),
            TextOverflow::Ellipsis => self.ellipsized_layout(text, style, options, limit),
        }
    }

    fn ellipsized_layout(
        &mut self,
        text: &str,
        style: &TextStyle,
        options: TextLayoutOptions,
        limit: usize,
    ) -> TextLayout {
        if limit == 0 {
            return clipped_layout(
                self.shape_raw(
                    "",
                    style,
                    None,
                    options.max_width,
                    options.align,
                    options.text_direction,
                ),
                0,
            );
        }
        let wrap_width = options.soft_wrap.then_some(options.max_width).flatten();
        let initial = self.shape_raw(
            text,
            style,
            wrap_width,
            options.max_width,
            options.align,
            options.text_direction,
        );
        let end = initial
            .get_line(limit.saturating_sub(1))
            .map_or(0, |line| line.caret_end().min(text.len()));
        let mut end = grapheme_boundary_at_or_before(text, end);

        loop {
            let mut candidate = text[..end].to_owned();
            candidate.push('\u{2026}');
            // The ellipsis always enters Parley as source text and is shaped
            // with the selected/fallback font stack, never appended as a raw
            // renderer glyph or a byte-truncated source suffix.
            let shaped = self.shape_raw(
                &candidate,
                style,
                wrap_width,
                options.max_width,
                options.align,
                options.text_direction,
            );
            let fits_width = options.max_width.is_none_or(|width| {
                shaped
                    .lines()
                    .all(|line| line.width() <= width + f32::EPSILON)
            });
            if shaped.line_count() <= limit && fits_width {
                return TextLayout {
                    overflowed: true,
                    ..shaped
                };
            }
            let previous = previous_grapheme_boundary(text, end);
            if previous == end {
                return TextLayout {
                    overflowed: true,
                    ..shaped
                };
            }
            end = previous;
        }
    }

    /// Shapes `text` into an unbroken Parley layout.
    fn shape_paragraph(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
        text_direction: Option<TextDirection>,
    ) -> Layout<()> {
        let mut builder =
            self.layout_context
                .ranged_builder(&mut self.font_context, text, 1.0, true);
        builder.set_base_direction(match text_direction {
            Some(TextDirection::Ltr) => BaseDirection::Ltr,
            Some(TextDirection::Rtl) => BaseDirection::Rtl,
            None => BaseDirection::Auto,
        });
        // Ranged builders retain the source text; this restores the previous
        // hanging-space policy without collapsing any source characters.
        builder.push_default(StyleProperty::WhiteSpaceCollapse(
            parley::WhiteSpaceCollapse::Collapse,
        ));
        if max_width.is_none() {
            builder.push_default(StyleProperty::TextWrapMode(parley::TextWrapMode::NoWrap));
        }
        builder.push_default(StyleProperty::FontFamily(Self::font_stack(style)));
        builder.push_default(StyleProperty::FontSize(style.size.max(0.0)));
        builder.push_default(StyleProperty::FontWeight(ParleyFontWeight::new(
            style.weight.0 as f32,
        )));
        builder.push_default(StyleProperty::FontStyle(match style.style {
            FontStyle::Normal => parley::FontStyle::Normal,
            FontStyle::Italic => parley::FontStyle::Italic,
        }));
        builder.push_default(StyleProperty::LetterSpacing(style.letter_spacing));
        if let Some(settings) = &style.font_variations {
            builder.push_default(StyleProperty::FontVariations(
                parley::FontVariations::Source(Cow::Owned(settings.to_string())),
            ));
        }
        if let Some(settings) = &style.font_features {
            builder.push_default(StyleProperty::FontFeatures(parley::FontFeatures::Source(
                Cow::Owned(settings.to_string()),
            )));
        }
        if let Some(line_height) = style.line_height
            && let Some(parley_lh) = line_height.to_parley(style.size)
        {
            builder.push_default(StyleProperty::LineHeight(parley_lh));
        }

        builder.build(text)
    }

    fn shape_raw(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
        align_width: Option<f32>,
        align: TextAlign,
        text_direction: Option<TextDirection>,
    ) -> TextLayout {
        // Shaping does not depend on the wrap width; only line breaking and
        // alignment do. A wrapping paragraph keeps its shaped layout so a new
        // width re-breaks it instead of shaping it again.
        let shaping_key = LayoutKey::new(
            text,
            style,
            TextLayoutOptions {
                max_width: max_width.map(|_| 1.),
                align: TextAlign::Start,
                text_direction,
                soft_wrap: true,
                max_lines: None,
                overflow: TextOverflow::Visible,
            },
            self.generation,
        );
        let mut layout = match self.shaped.take(&shaping_key) {
            Some(layout) => {
                self.diagnostics.shapes_reused += 1;
                layout
            }
            None => self.shape_paragraph(text, style, max_width, text_direction),
        };
        layout.break_all_lines(align_width.or(max_width));
        // Alignment resolves against the measured width even when the text
        // does not wrap: single-line content otherwise ignores alignment
        // because the wrap width is empty.
        layout.align(
            match align {
                TextAlign::Start => Alignment::Start,
                TextAlign::Center => Alignment::Center,
                TextAlign::End => Alignment::End,
                TextAlign::Justify => Alignment::Justify,
                TextAlign::Left => Alignment::Left,
                TextAlign::Right => Alignment::Right,
            },
            AlignmentOptions::default(),
        );

        let mut translated = self.translate_layout(&layout, text);
        if text.is_empty() {
            // Parley uses an internal space to obtain empty-paragraph font
            // metrics. That space must not become a visible advance or move
            // the editor's caret away from the empty aligned paragraph edge.
            let width = align_width.unwrap_or(0.0).max(0.0);
            let offset = match (align, layout.is_rtl()) {
                (TextAlign::Right, _)
                | (TextAlign::Start | TextAlign::Justify, true)
                | (TextAlign::End, false) => width,
                (TextAlign::Center, _) => width * 0.5,
                _ => 0.0,
            };
            for line in translated.leaf_lines_mut() {
                line.runs = Arc::new([]);
                line.glyphs = Arc::new([]);
                line.clusters = Arc::new([]);
                line.width = 0.0;
                line.offset = offset;
            }
            translated.metrics.size.width = 0.0;
        }
        if layout.len() > 1 {
            self.shaped.put(shaping_key, layout);
        }
        translated
    }

    fn font_handle_for(&mut self, font: &parley::FontData) -> FontHandle {
        font_handle(self, font)
    }

    fn translate_layout(&mut self, layout: &Layout<()>, text: &str) -> TextLayout {
        let mut lines = Vec::new();
        let mut font_runs = Vec::new();
        let mut used_fonts = HashSet::new();
        // First glyph run decides the primary font; resolve its handle before
        // the mutable translation loop so borrowck stays trivial.
        let primary_handle = layout
            .lines()
            .flat_map(|line| line.runs())
            .next()
            .map(|run| self.font_handle_for(run.font()));
        let primary = primary_handle.as_ref().map(FontHandle::id);

        for line in layout.lines() {
            let metrics = line.metrics();
            let range = line.text_range();
            let mut runs: Vec<Arc<GlyphRun>> = Vec::new();
            let mut glyphs = Vec::new();
            let mut caret_end = range.start;
            // Walk Parley's visual cluster advances once for the whole line.
            // This is the same cell geometry Parley uses for hit testing and
            // accounts for per-cluster spacing and justified advances. A
            // source cluster can be split across font/style glyph runs, so
            // merge duplicate byte ranges into one logical span and caret
            // pair here rather than deriving cells separately per glyph run.
            let mut cluster_positions: Vec<ClusterPosition> = Vec::new();
            let mut cluster_position_index: HashMap<(usize, usize), usize> = HashMap::new();
            let mut cluster_x = metrics.offset;
            for run in line.runs() {
                let rtl = run.is_rtl();
                for cluster in run.visual_clusters() {
                    let cluster_range = cluster.text_range();
                    // Invisible continuation clusters still own source bytes.
                    // Keep the logical line endpoint through their ranges so
                    // a final ligature component does not lose its caret stop.
                    caret_end = caret_end.max(cluster_range.end);
                    let edge = cluster_x + cluster.advance();
                    let ligature_start = cluster.is_ligature_start();
                    let ligature_continuation = cluster.is_ligature_continuation();
                    let mut cluster_glyphs = cluster.glyphs();
                    let Some(first_glyph) = cluster_glyphs.next() else {
                        // A shaping cluster can carry advance for an
                        // invisible continuation. Ligature components remain
                        // part of the start cluster's selectable range, while
                        // standalone join controls have no visible selection
                        // geometry of their own.
                        let is_join_control =
                            text.get(cluster_range.clone()).is_some_and(|component| {
                                !component.is_empty()
                                    && component.chars().all(|character| {
                                        matches!(character, '\u{200C}' | '\u{200D}')
                                    })
                            });
                        if ligature_continuation && !is_join_control {
                            let position = ClusterPosition {
                                range: cluster_range.clone(),
                                left: cluster_x.min(edge),
                                right: cluster_x.max(edge),
                                rtl,
                                ligature_start,
                                ligature_continuation,
                            };
                            let key = (cluster_range.start, cluster_range.end);
                            if let Some(index) = cluster_position_index.get(&key).copied() {
                                cluster_positions[index].include(&position);
                            } else {
                                cluster_position_index.insert(key, cluster_positions.len());
                                cluster_positions.push(position);
                            }
                        }
                        cluster_x = edge;
                        continue;
                    };
                    let glyph_advance = std::iter::once(first_glyph)
                        .chain(cluster_glyphs)
                        .map(|glyph| glyph.advance)
                        .sum::<f32>();
                    // Ligature components can share a larger glyph advance
                    // than their divided cluster advances; retain the glyph's
                    // full cell so no caret or selection edge lands inside
                    // that ligature. Justification and letter spacing can
                    // also grow the cluster cell beyond its raw glyphs.
                    let visible_edge = cluster_x + cluster.advance().max(glyph_advance);
                    let left = cluster_x.min(visible_edge);
                    let right = cluster_x.max(visible_edge);
                    let position = ClusterPosition {
                        range: cluster_range.clone(),
                        left,
                        right,
                        rtl,
                        ligature_start,
                        ligature_continuation,
                    };
                    let key = (cluster_range.start, cluster_range.end);
                    if let Some(index) = cluster_position_index.get(&key).copied() {
                        cluster_positions[index].include(&position);
                    } else {
                        cluster_position_index.insert(key, cluster_positions.len());
                        cluster_positions.push(position);
                    }
                    cluster_x = edge;
                }
            }
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(parley_run) = item else {
                    continue;
                };
                let run = parley_run.run();
                let font = self.font_handle_for(run.font());
                let font_id = font.id();
                if Some(font_id) != primary {
                    self.diagnostics.fallback_runs += 1;
                }
                used_fonts.insert(font_id);
                self.diagnostics.shaping_runs += 1;

                // `PositionedLayoutItem::GlyphRun` is a line-local slice of
                // the underlying shaping run. Parley's renderer-facing Glyph
                // intentionally has no byte index, so recover each glyph's
                // cluster from the authoritative visual-cluster sequence
                // before translating it. This keeps caret movement and
                // selection on shaped cluster boundaries instead of assigning
                // every glyph to the run's first byte.
                //
                // Order and cardinality correspond structurally, not by
                // length coincidence: `glyphs()` and `positioned_glyphs()`
                // both iterate `Run::visual_clusters().flat_map(glyphs)`,
                // the latter only adding running x (which already carries
                // the line offset). The window below re-locates this
                // style-uniform slice within that same sequence, so the
                // zip pairs each positioned glyph with its own cluster
                // for RTL runs, ligatures, and multi-glyph clusters alike.
                let visible_glyphs = parley_run.glyphs().collect::<Vec<_>>();
                let all_clustered_glyphs = run
                    .visual_clusters()
                    .flat_map(|cluster| {
                        let cluster_range = cluster.text_range();
                        cluster
                            .glyphs()
                            .map(move |glyph| (glyph, cluster_range.clone()))
                    })
                    .collect::<Vec<_>>();
                let first_glyph = all_clustered_glyphs
                    .windows(visible_glyphs.len())
                    .position(|candidate| {
                        candidate
                            .iter()
                            .zip(&visible_glyphs)
                            .all(|((glyph, _), visible)| glyph == visible)
                    })
                    .unwrap_or(0);
                let clusters = all_clustered_glyphs
                    .iter()
                    .skip(first_glyph)
                    .map(|(_, range)| range.clone())
                    .take(visible_glyphs.len())
                    .collect::<Vec<_>>();
                let positions: Vec<_> = parley_run
                    .positioned_glyphs()
                    .zip(
                        clusters
                            .iter()
                            .map(|range| range.start as u32)
                            .chain(std::iter::repeat(run.text_range().start as u32)),
                    )
                    .map(|(glyph, cluster)| GlyphPosition {
                        id: glyph.id as u16,
                        // Incular's renderer uses `origin.y - offset.y`; Parley
                        // exposes glyph y from its top-left layout coordinate.
                        // Parley seeds positioned glyph x with the line
                        // alignment offset, so adding it again would translate
                        // aligned text twice.
                        offset: Offset::new(glyph.x, -glyph.y),
                        advance: glyph.advance,
                        cluster,
                    })
                    .collect();
                if runs.len() == 1 {
                    glyphs.extend(runs[0].glyphs.iter().copied());
                }
                if !runs.is_empty() {
                    glyphs.extend(positions.iter().copied());
                }
                runs.push(Arc::new(GlyphRun {
                    font,
                    font_size: run.font_size(),
                    origin: Offset::ZERO,
                    glyphs: positions.into(),
                }));
                font_runs.push(FontRunDebug {
                    range: run.text_range(),
                    font: font_id,
                    direction: if run.is_rtl() { "rtl" } else { "ltr" },
                });
            }
            // Every merged cluster, including zero-advance ones that only
            // carry caret stops; the layout derives its stops from these.
            let cluster_spans: Vec<_> = merge_ligature_components(cluster_positions)
                .iter()
                .map(|position| TextClusterSpan {
                    start: position.range.start,
                    end: position.range.end,
                    left: position.left,
                    right: position.right,
                    rtl: position.rtl,
                })
                .collect();
            // The common one-run line has exactly the same positioned glyphs
            // for caret handling and painting. Share the immutable allocation
            // instead of retaining a second copy in TextLine.
            let glyphs = if runs.len() == 1 {
                runs[0].glyphs.clone()
            } else {
                glyphs.into()
            };
            lines.push(TextLine {
                runs: runs.into(),
                glyphs,
                clusters: cluster_spans.into(),
                width: metrics.advance,
                offset: metrics.offset,
                baseline: metrics.baseline,
                start: range.start,
                caret_end,
                end: range.end,
            });
        }

        if lines.is_empty() {
            lines.push(TextLine {
                runs: Arc::new([]),
                glyphs: Arc::new([]),
                clusters: Arc::new([]),
                width: 0.0,
                offset: 0.0,
                baseline: 0.0,
                start: 0,
                caret_end: 0,
                end: 0,
            });
        }
        self.diagnostics.fallback_fonts_used += used_fonts.len().saturating_sub(1) as u64;
        let baseline = lines[0].baseline;
        let line_height = layout
            .lines()
            .next()
            .map_or(0.0, |line| line.metrics().line_height);
        let mut translated = TextLayout::leaf(
            lines.into(),
            font_runs.into(),
            TextMetrics {
                size: Size::new(layout.width(), layout.height()),
                baseline,
                line_height,
            },
        );
        if has_join_controls(text) {
            // Carets keep Parley's complete grapheme boundaries. Selection
            // coverage separately excludes invisible format controls, even
            // when Parley attaches them to a visible grapheme.
            translated
                .caret_positions
                .get_or_init(|| translated.derive_caret_positions());
            for line in translated.leaf_lines_mut() {
                let mut visible = Vec::with_capacity(line.clusters.len());
                for span in line.clusters.iter() {
                    let Some(component) = text.get(span.start..span.end) else {
                        visible.push(*span);
                        continue;
                    };
                    let mut start = span.start;
                    for (offset, character) in component.char_indices() {
                        if matches!(character, '\u{200C}' | '\u{200D}') {
                            let end = span.start + offset;
                            if start < end {
                                visible.push(TextClusterSpan {
                                    start,
                                    end,
                                    ..*span
                                });
                            }
                            start = end + character.len_utf8();
                        }
                    }
                    if start < span.end {
                        visible.push(TextClusterSpan { start, ..*span });
                    }
                }
                line.clusters = visible.into();
            }
        }
        translated
    }
}

fn has_join_controls(text: &str) -> bool {
    text.contains('\u{200C}') || text.contains('\u{200D}')
}

/// Coalesces Parley's glyphless ligature components with their rendered
/// cluster. Visual cluster iteration is forward for LTR runs and reversed for
/// RTL runs, so the component sequence starts at opposite ends of a ligature
/// in each direction.
fn merge_ligature_components(positions: Vec<ClusterPosition>) -> Vec<ClusterPosition> {
    let mut merged = Vec::with_capacity(positions.len());
    let mut pending: Option<ClusterPosition> = None;
    let mut pending_has_start = false;

    for position in positions {
        let is_start = position.ligature_start;
        let is_continuation = position.ligature_continuation;
        if !is_start && !is_continuation {
            if let Some(pending) = pending.take() {
                merged.push(pending);
            }
            pending_has_start = false;
            merged.push(position);
            continue;
        }
        // A single source cluster can already cover the whole ligature.
        if is_start && is_continuation {
            if let Some(pending) = pending.take() {
                merged.push(pending);
            }
            pending_has_start = false;
            merged.push(position);
            continue;
        }

        if position.rtl {
            if is_continuation {
                // In visual RTL order, continuations precede their start. A
                // continuation after a completed group begins the next one.
                if pending_has_start {
                    if let Some(pending) = pending.take() {
                        merged.push(pending);
                    }
                    pending_has_start = false;
                }
                let can_extend = pending
                    .as_ref()
                    .is_some_and(|pending| pending.can_join(&position));
                if can_extend {
                    pending
                        .as_mut()
                        .expect("checked pending ligature group")
                        .include(&position);
                } else {
                    if let Some(pending) = pending.take() {
                        merged.push(pending);
                    }
                    pending_has_start = false;
                    pending = Some(position);
                }
            } else {
                // The start completes the pending continuation group.
                match pending.take() {
                    Some(mut pending_group)
                        if !pending_has_start && pending_group.can_join(&position) =>
                    {
                        pending_group.include(&position);
                        merged.push(pending_group);
                    }
                    Some(pending_group) => {
                        merged.push(pending_group);
                        merged.push(position);
                    }
                    None => merged.push(position),
                }
                pending_has_start = false;
            }
        } else if is_start {
            if let Some(pending) = pending.take() {
                merged.push(pending);
            }
            pending = Some(position);
            pending_has_start = true;
        } else if pending.is_some() {
            // LTR starts precede their continuations in visual order. The
            // byte check prevents an excluded join control from being
            // bridged by a later glyphless continuation.
            if pending
                .as_ref()
                .is_some_and(|pending_group| pending_group.can_join(&position))
            {
                pending
                    .as_mut()
                    .expect("checked pending ligature group")
                    .include(&position);
            } else {
                let pending_group = pending.take().expect("pending ligature group");
                merged.push(pending_group);
                pending = Some(position);
                pending_has_start = false;
            }
        } else {
            // A run can begin in the middle of a shaped ligature if a prior
            // fallback/style run owns its start.
            pending = Some(position);
            pending_has_start = false;
        }
    }

    if let Some(pending) = pending {
        merged.push(pending);
    }
    merged
}

/// Paragraphs of `previous` that `text` shares, as the number carried
/// unchanged from the start and the index of the first one carried from the
/// end (the paragraph count when none is).
///
/// Paragraph `k` and everything before it are unchanged when the common
/// byte prefix reaches the start of paragraph `k`, so paragraphs before the
/// one holding the first change are carried. Paragraph `j` and everything
/// after it are unchanged when the common byte suffix covers the hard break
/// before it, or when the whole previous text follows a new hard break. The
/// suffix starts at or after the first recomposed paragraph in both texts,
/// so no paragraph is carried twice.
fn carried_paragraphs(paragraphs: &[ParagraphSlot], previous: &str, text: &str) -> (usize, usize) {
    let prefix = common_prefix_len(previous, text);
    let carried = paragraphs
        .partition_point(|slot| slot.byte <= prefix)
        .saturating_sub(1);
    let floor = paragraphs[carried].byte;
    let shared = common_suffix_len(previous, text);
    if carried == 0
        && shared == previous.len()
        && text.len() > previous.len()
        && text.as_bytes()[text.len() - previous.len() - 1] == b'\n'
    {
        return (0, 0);
    }
    // Lowest start whose preceding hard break is shared and which lands at
    // or after `floor` in the new text.
    let threshold = floor
        .max((floor + previous.len()).saturating_sub(text.len()))
        .max(previous.len() - shared + 1);
    let suffix = paragraphs
        .partition_point(|slot| slot.byte < threshold)
        .max(carried);
    (carried, suffix)
}

/// Length in bytes of the longest common suffix of `left` and `right`.
fn common_suffix_len(left: &str, right: &str) -> usize {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    let mut suffix = 0;
    for (a, b) in left.rchunks(64).zip(right.rchunks(64)) {
        if a == b {
            suffix += a.len();
        } else {
            return suffix
                + a.iter()
                    .rev()
                    .zip(b.iter().rev())
                    .take_while(|(x, y)| x == y)
                    .count();
        }
    }
    suffix
}

/// Length in bytes of the longest common prefix of `left` and `right`.
fn common_prefix_len(left: &str, right: &str) -> usize {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    let mut prefix = 0;
    for (a, b) in left.chunks(64).zip(right.chunks(64)) {
        if a == b {
            prefix += a.len();
        } else {
            return prefix + a.iter().zip(b).take_while(|(x, y)| x == y).count();
        }
    }
    prefix
}

/// Whether `layout` lays out identically at every width it fits: one
/// unclipped line, start-aligned in a left-to-right paragraph, unshifted.
fn width_independent(layout: &TextLayout, options: TextLayoutOptions) -> bool {
    matches!(
        options.align,
        TextAlign::Start | TextAlign::Left | TextAlign::Justify
    ) && options.text_direction != Some(TextDirection::Rtl)
        && !layout.overflowed
        && layout.line_count() == 1
        && layout.leaf_lines()[0].offset == 0.0
        && layout.font_runs().all(|run| run.direction != "rtl")
}

/// Keeps the first `limit` lines of `layout`.
fn clipped_layout(mut layout: TextLayout, limit: usize) -> TextLayout {
    let mut slots = Vec::new();
    for slot in layout.paragraphs.iter() {
        let kept = limit.saturating_sub(slot.first_line).min(slot.lines.len());
        if kept == 0 {
            break;
        }
        let mut slot = slot.clone();
        if kept < slot.lines.len() {
            slot.lines = slot.lines[..kept].into();
        }
        slots.push(slot);
    }
    let line_count: usize = slots.iter().map(|slot| slot.lines.len()).sum();
    if let Some(carets) = layout.caret_positions.get_mut() {
        let retained = carets.partition_point(|position| position.line < line_count);
        if retained < carets.len() {
            *carets = Arc::from(&carets[..retained]);
        }
    }
    let width = slots
        .iter()
        .flat_map(|slot| slot.lines.iter())
        .fold(0.0_f32, |width, line| width.max(line.width));
    layout.metrics.size = Size::new(width, layout.metrics.line_height * line_count as f32);
    layout.metrics.baseline = slots
        .first()
        .and_then(|slot| Some(slot.y + slot.lines.first()?.baseline))
        .unwrap_or(0.0);
    layout.paragraphs = slots.into();
    layout.line_count = line_count;
    layout.line_carets = OnceLock::new();
    layout.overflowed = true;
    layout
}

fn grapheme_boundary_at_or_before(text: &str, offset: usize) -> usize {
    GraphemeClusterSegmenter::new()
        .segment_str(text)
        .take_while(|boundary| *boundary <= offset)
        .last()
        .unwrap_or(0)
}

fn previous_grapheme_boundary(text: &str, offset: usize) -> usize {
    GraphemeClusterSegmenter::new()
        .segment_str(text)
        .take_while(|boundary| *boundary < offset)
        .last()
        .unwrap_or(0)
}

fn family_name(family: &FontFamily) -> String {
    match family {
        FontFamily::SansSerif => "sans-serif".into(),
        FontFamily::Serif => "serif".into(),
        FontFamily::Monospace => "monospace".into(),
        FontFamily::Cursive => "cursive".into(),
        FontFamily::SystemUi => "system-ui".into(),
        FontFamily::Named(name) => format!("'{name}'"),
    }
}

/// Identity for one font blob sighting. Pointer identity is stable because
/// `Blob` clones share the same backing allocation, so repeated sightings of
/// the same face never re-copy or re-hash the bytes.
type FontBlobKey = (usize, usize, u32);

fn font_blob_key(font: &parley::FontData) -> FontBlobKey {
    (
        font.data.as_ref().as_ptr() as usize,
        font.data.as_ref().len(),
        font.index,
    )
}

/// Returns the shared handle for this font blob, building it (one full-file
/// hash, no copy) on first sight only.
fn font_handle(engine: &mut TextEngine, font: &parley::FontData) -> FontHandle {
    let key = font_blob_key(font);
    if let Some(handle) = engine.font_handles.get(&key) {
        return handle.clone();
    }
    let mut hasher = DefaultHasher::new();
    font.data.as_ref().hash(&mut hasher);
    hasher.write_u64(u64::from(font.index));
    let id = FontId(hasher.finish());
    // Share discovery's (typically memory-mapped) source instead of copying
    // the whole font file into a private heap allocation.
    let handle = FontHandle::with_face_index(
        id,
        FontBytes::from_shared(font.data.clone().into_raw_parts().0),
        font.index,
    );
    engine.font_handles.insert(key, handle.clone());
    handle
}
