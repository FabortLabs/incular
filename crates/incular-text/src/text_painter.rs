//! Stateful paragraph measurement over the shared Incular shaping engine.

use std::{
    backtrace::Backtrace,
    fmt,
    sync::{Arc, OnceLock},
};

use incular_config::TextDirection;
use incular_core::{Color, Offset, Rect, Size};

use crate::{
    StrutStyle, TextAffinity, TextAlign, TextEngine, TextHeightBehavior, TextLayout,
    TextLayoutOptions, TextOverflow, TextRange, TextScaler, TextSpan, TextStrut, TextStyle,
    TextWidthBasis, WidgetSpanAlignment, word_boundary_at,
};

/// A layout request or query that cannot be satisfied yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextPainterError {
    MissingText,
    MissingTextDirection,
    InvalidMaxLines,
    InvalidWidth,
    UnboundedSize,
    InvalidPlaceholderCount,
    NeedsLayout,
    PaintNeedsLayout,
    Disposed,
}

impl fmt::Display for TextPainterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingText => "TextPainter requires text before layout",
            Self::MissingTextDirection => "TextPainter requires a text direction before layout",
            Self::InvalidMaxLines => "TextPainter max lines must be positive",
            Self::InvalidWidth => "invalid TextPainter width constraints",
            Self::UnboundedSize => "TextPainter reports an unbounded width, not a finite Size",
            Self::InvalidPlaceholderCount => "TextPainter needs one size for each inline widget span",
            Self::NeedsLayout => "TextPainter requires a current layout",
            Self::PaintNeedsLayout => "TextPainter.paint called when text geometry was not yet calculated. Call layout() before painting.",
            Self::Disposed => "TextPainter has already been disposed",
        })
    }
}
impl std::error::Error for TextPainterError {}

/// Why a point cannot be resolved into a text position.
#[derive(Debug)]
pub enum TextPainterPositionError {
    NeverLaidOut,
    LayoutInvalidated {
        first_invalidation: Option<Arc<Backtrace>>,
    },
    Disposed,
}

impl fmt::Display for TextPainterPositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NeverLaidOut => f.write_str("The TextPainter has never been laid out."),
            Self::LayoutInvalidated { first_invalidation } => {
                f.write_str("The calls that first invalidated the text layout were:")?;
                if let Some(stack) = first_invalidation {
                    write!(f, "\n{stack}")?;
                }
                Ok(())
            }
            Self::Disposed => fmt::Display::fmt(&TextPainterError::Disposed, f),
        }
    }
}

impl std::error::Error for TextPainterPositionError {}

/// A paragraph insertion point at a UTF-8 byte boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextPainterPosition {
    pub offset: usize,
    pub affinity: TextAffinity,
}

/// Whether a discretionary line break paints a soft hyphen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hyphens {
    /// Paint a hyphen when a line ends at U+00AD, not at the end of text.
    #[default]
    Manual,
    /// Take the same line breaks without drawing a hyphen.
    Hidden,
}

/// Vertical bounds of selection boxes, independent of their horizontal extent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoxHeightStyle {
    /// Each shaped run or inline box keeps its own height.
    #[default]
    Tight,
    /// All boxes on the same visual line use the line bounds.
    Max,
    /// Splits leading between the adjacent line boxes.
    IncludeLineSpacingMiddle,
    /// Puts the extra leading above the following line.
    IncludeLineSpacingTop,
    /// Puts the extra leading below the preceding line.
    IncludeLineSpacingBottom,
    /// Uses the paragraph strut, or each run's own style when it is unset.
    Strut,
}

/// Horizontal extent of selection boxes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoxWidthStyle {
    /// Only the selected content receives a box.
    #[default]
    Tight,
    /// Adds edge boxes on nonfinal lines to fill uncovered line width.
    Max,
}

/// A paragraph paint operation, in the caller's drawing coordinates.
pub enum TextPainterPaint<'a> {
    DebugBox {
        rect: Rect,
        color: Color,
        stroke_width: f32,
    },
    Paragraph {
        layout: &'a TextLayout,
        offset: Offset,
    },
}

/// A measured inline widget's geometry. Unlike a positioned text
/// placeholder, this carries no source index: the painter associates the
/// values with its inline widget spans in traversal order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceholderDimensions {
    pub size: Size,
    pub alignment: WidgetSpanAlignment,
    pub baseline_offset: Option<f32>,
}

impl PlaceholderDimensions {
    #[must_use]
    pub const fn new(size: Size, alignment: WidgetSpanAlignment) -> Self {
        Self { size, alignment, baseline_offset: None }
    }

    #[must_use]
    pub const fn baseline_offset(mut self, baseline_offset: f32) -> Self {
        self.baseline_offset = Some(baseline_offset);
        self
    }
}

/// Geometry in the painter's local coordinates, including its alignment
/// translation. Text ranges are byte offsets into the unlabelled plain text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextPainterLine {
    pub line_number: usize,
    pub start: usize,
    pub end: usize,
    pub hard_break: bool,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub unscaled_ascent: f32,
    pub height: f32,
}

/// One layout box of a selection, in painter coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextPainterBox {
    pub rect: Rect,
    pub direction: TextDirection,
}

/// Metrics shared by an already-measured standalone or retained paragraph.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct ParagraphCaretContext<'a> {
    pub source: &'a str,
    pub direction: TextDirection,
    pub content_width: f32,
    pub paint_offset: Offset,
    pub template_baseline: f32,
    pub template_height: f32,
    pub strut: Option<TextStrut>,
}

impl TextLayout {
    /// Tight visual boxes for complete clusters in the UTF-8 range.
    #[must_use]
    pub fn tight_selection_boxes(&self, range: TextRange) -> Vec<TextPainterBox> {
        self.boxes_for_selection(range, BoxHeightStyle::Tight, BoxWidthStyle::Tight)
    }

    /// Visual boxes in this layout's coordinates. The range selects whole
    /// source clusters; synthetic ellipsis and soft-hyphen glyphs are excluded.
    #[must_use]
    pub fn boxes_for_selection(
        &self,
        range: TextRange,
        height_style: BoxHeightStyle,
        width_style: BoxWidthStyle,
    ) -> Vec<TextPainterBox> {
        let end = range.end.min(self.retained_source_end());
        if range.start >= end {
            return Vec::new();
        }
        let range = TextRange::new(range.start, end);
        let mut result: Vec<TextPainterBox> = Vec::new();
        let lines = &self.lines;
        let widest = (width_style == BoxWidthStyle::Max)
            .then(|| lines.iter().map(|line| line.width).fold(0., f32::max));
        let first = lines.partition_point(|line| line.end <= range.start);
        let mut placeholders = self.placeholders.as_ref();
        placeholders = &placeholders[placeholders.partition_point(|child| child.line < first)..];
        for (index, line) in lines.iter().enumerate().skip(first).take_while(|(_, line)| {
            line.start < range.end
        }) {
            let line_bounds = self.selection_line_bounds(index, height_style);
            let run_heights = self.selection_run_heights.as_deref().unwrap_or(&[]);
            let first_box = result.len();
            let count = placeholders.partition_point(|child| child.line == index);
            let (children, tail) = placeholders.split_at(count);
            placeholders = tail;
            let mut previous_run = (None, None);
            let mut previous_was_inline = false;
            for span in line.clusters.iter() {
                let source = span.range();
                if source.start < range.start || source.end > range.end {
                    continue;
                }
                let left = span.left;
                let right = span.right;
                let at_left = children.partition_point(|child| child.rect.origin.x < left);
                let inline = children[at_left..]
                    .iter()
                    .take_while(|child| child.rect.origin.x == left)
                    .find(|child| child.index == source.start);
                if left >= right && !inline.is_some_and(|child| child.rect.size.height > 0.) {
                    continue;
                }
                let run = self.font_run_at(source.start);
                let (tight_top, tight_height) = if let Some(inline) = inline {
                    (inline.rect.origin.y, inline.rect.size.height)
                } else {
                    run.map_or((line.top, line.height), |run| {
                        (line.baseline - run.caret_ascent, run.caret_height)
                    })
                };
                let height_recipe = run_heights
                    .get(run_heights.partition_point(|height| height.range.end <= source.start))
                    .filter(|height| height.range.contains(&source.start));
                let (top, height) = line_bounds.unwrap_or_else(|| {
                    if height_style == BoxHeightStyle::Strut
                        && inline.is_none()
                        && let Some(recipe) = height_recipe
                        && let Some(run) = run
                    {
                        let ascent = match recipe.leading {
                            crate::TextLeadingDistribution::Even => {
                                run.caret_ascent + (recipe.height - run.caret_height) / 2.
                            }
                            crate::TextLeadingDistribution::Proportional => {
                                if run.caret_height == 0. { 0. }
                                else { run.caret_ascent / run.caret_height * recipe.height }
                            }
                        };
                        (line.baseline - ascent, recipe.height)
                    } else {
                        (tight_top, tight_height)
                    }
                });
                let direction = if span.rtl { TextDirection::Rtl } else { TextDirection::Ltr };
                let run_key = (
                    run.map(|run| (run.font, run.caret_ascent, run.caret_height)),
                    height_recipe.map(|height| (height.height, height.leading)),
                );
                if result.len() > first_box
                    && inline.is_none()
                    && !previous_was_inline
                    && run_key == previous_run
                    && let Some(previous) = result.last_mut()
                    && previous.direction == direction
                    && previous.rect.origin.x + previous.rect.size.width == left
                    && previous.rect.origin.y == top
                    && previous.rect.size.height == height
                {
                    previous.rect.size.width = right - previous.rect.origin.x;
                } else {
                    result.push(TextPainterBox {
                        rect: Rect::from_origin_size(
                            Offset::new(left, top),
                            Size::new(right - left, height),
                        ),
                        direction,
                    });
                }
                previous_run = run_key;
                previous_was_inline = inline.is_some();
            }
            if let Some(widest) = widest
                && index + 1 < lines.len()
                && let Some(left) = result.get(first_box).copied()
            {
                let right = *result.last().expect("line supplied a selection box");
                if left.direction == TextDirection::Rtl
                    && left.rect.origin.x > line.offset
                    && left.rect.origin.x > 0.
                {
                    let mut extension = left;
                    extension.rect.origin.x = 0.;
                    extension.rect.size.width = left.rect.origin.x;
                    result.insert(first_box + 1, extension);
                }
                let right_edge = right.rect.origin.x + right.rect.size.width;
                if right.direction == TextDirection::Ltr
                    && right_edge >= line.offset + line.width - line.hanging_advance
                    && right_edge < widest
                {
                    let mut extension = right;
                    extension.rect.origin.x = right_edge;
                    extension.rect.size.width = widest - right_edge;
                    result.push(extension);
                }
            }
        }
        result
    }

    fn selection_line_bounds(&self, index: usize, style: BoxHeightStyle) -> Option<(f32, f32)> {
        let line = &self.lines[index];
        if style == BoxHeightStyle::Tight {
            return None;
        }
        if style == BoxHeightStyle::Strut {
            return self.selection_strut.map(|strut| {
                (line.baseline - strut.box_ascent, strut.box_ascent + strut.box_descent)
            });
        }
        // The selection starts at the rounded line's metric delta, which
        // can be a fraction of a point away from its layout origin.
        let mut top = line.top + (line.height - (line.ascent + line.descent));
        let mut bottom = line.top + line.height;
        if style == BoxHeightStyle::Max {
            return Some((top, bottom - top));
        }
        let spacing = match self.selection_strut {
            Some(strut) if strut.force => strut.box_ascent - strut.raw_ascent,
            Some(strut) => line.ascent - line.unscaled_ascent.max(strut.raw_ascent),
            None => line.ascent - line.unscaled_ascent,
        };
        let first = index == 0;
        let last = index + 1 == self.lines.len();
        match style {
            BoxHeightStyle::IncludeLineSpacingTop => {
                if first { top += spacing; }
            }
            BoxHeightStyle::IncludeLineSpacingMiddle => {
                top += spacing * if first { 1. } else { 0.5 };
                if !last { bottom += spacing * 0.5; }
            }
            BoxHeightStyle::IncludeLineSpacingBottom => {
                top += spacing;
                if !last { bottom += spacing; }
            }
            _ => unreachable!("height style was handled above"),
        }
        Some((top, bottom - top))
    }

    /// Closest insertion point and affinity in this layout's coordinates.
    #[must_use]
    pub fn closest_text_position(&self, point: Offset) -> TextPainterPosition {
        let lines = &self.lines;
        let index = lines
            .partition_point(|line| line.top <= point.y)
            .saturating_sub(1);
        let x = point.x;
        let stops = self.line_caret_positions(index);
        let index_at_x = stops.partition_point(|stop| stop.x < x);
        let left = index_at_x.checked_sub(1).and_then(|index| stops.get(index));
        let right = stops.get(index_at_x);
        let closest = match (left, right) {
            (_, Some(right)) if right.x == x => {
                let same_x = &stops[index_at_x..];
                let last = &same_x[same_x.partition_point(|stop| stop.x == x) - 1];
                // At an ordinary shared edge, choose the leading edge of
                // the next visual glyph. Coincident bidi stops with different
                // logical positions are not interchangeable.
                Some(if last.offset == right.offset { last } else { right })
            }
            (Some(left), Some(right)) if x - left.x <= right.x - x => Some(left),
            (_, Some(right)) => Some(right),
            (left, None) => left,
        };
        closest.map_or(
            TextPainterPosition {
                offset: lines.get(index).map_or(0, |line| line.start),
                affinity: TextAffinity::Downstream,
            },
            |stop| TextPainterPosition {
                offset: stop.offset,
                affinity: stop.affinity,
            },
        )
    }

    #[doc(hidden)]
    #[must_use]
    pub fn caret_offset_in_paragraph(
        &self,
        byte: usize,
        affinity: TextAffinity,
        caret_width: f32,
        context: ParagraphCaretContext<'_>,
    ) -> Offset {
        let source = context.source;
        let lines = &self.lines;
        if lines.is_empty() {
            return Offset::ZERO;
        }
        let line_at = |byte| {
            lines
                .partition_point(|line| line.start <= byte)
                .saturating_sub(1)
        };
        let mut byte = byte.min(source.len());
        let mut index = line_at(byte);
        if byte < lines[index].caret_end
            && !self
                .line_caret_positions(index)
                .iter()
                .any(|stop| stop.offset == byte)
        {
            // A rare inside-cluster query may not have a shaped stop.
            // Search only its visual line; ordinary caret moves do not
            // scan preceding text or allocate a segmentation vector.
            let start = lines[index].start;
            let end = lines[index].caret_end.min(source.len());
            if let Some(line_text) = source.get(start..end) {
                let relative = byte - start;
                if let Some(next) = icu_segmenter::GraphemeClusterSegmenter::new()
                    .segment_str(line_text)
                    .find(|boundary| *boundary >= relative)
                {
                    byte = start + next;
                    index = line_at(byte);
                }
            }
        }

        let after_break = byte
            .checked_sub(1)
            .is_some_and(|prev| matches!(source.as_bytes().get(prev), Some(b'\n' | b'\r')));
        let use_leading = byte == 0 || after_break || affinity == TextAffinity::Downstream;
        if self.overflowed {
            let last = &lines[lines.len() - 1];
            let anchor = if use_leading {
                byte
            } else {
                byte.saturating_sub(1)
            };
            if anchor >= last.end {
                let rtl = context.direction == TextDirection::Rtl;
                let x = if rtl {
                    last.offset + last.hanging_advance - caret_width.max(0.)
                } else {
                    last.offset + last.width - last.hanging_advance
                };
                return Offset::new(
                    (x + context.paint_offset.x).clamp(0., context.content_width),
                    last.baseline - context.template_baseline,
                );
            }
        }
        if !use_leading && index > 0 && lines[index].start == byte {
            index -= 1;
        }
        let line = &lines[index];
        let stops = self.line_caret_positions(index);
        let paragraph_rtl = context.direction == TextDirection::Rtl;
        let (x, rtl) = if byte == source.len() && use_leading && !line.clusters.is_empty() {
            // For downstream end-of-text, anchor to the paragraph's visual
            // trailing edge, even if its last glyph has the other direction.
            let edge = if paragraph_rtl {
                stops.first()
            } else {
                stops.last()
            };
            (edge.map_or(line.offset, |stop| stop.x), paragraph_rtl)
        } else {
            let wanted = if use_leading {
                TextAffinity::Downstream
            } else {
                TextAffinity::Upstream
            };
            let stop = stops
                .iter()
                .find(|stop| stop.offset == byte && stop.affinity == wanted)
                .or_else(|| stops.iter().find(|stop| stop.offset == byte));
            let direction = line
                .clusters
                .iter()
                .find(|cluster| {
                    if use_leading {
                        cluster.range().start == byte
                    } else {
                        cluster.range().end == byte
                    }
                })
                .map_or(paragraph_rtl && !line.clusters.is_empty(), |cluster| {
                    cluster.rtl
                });
            (
                stop.map_or_else(
                    || {
                        if byte >= line.caret_end && !paragraph_rtl {
                            line.offset + line.width
                        } else {
                            line.offset
                        }
                    },
                    |stop| stop.x,
                ),
                direction,
            )
        };
        let advance = if rtl && caret_width.is_finite() {
            caret_width.max(0.)
        } else {
            0.
        };
        Offset::new(
            (x - advance + context.paint_offset.x).clamp(0., context.content_width),
            line.top,
        )
    }

    #[doc(hidden)]
    #[must_use]
    pub fn caret_height_in_paragraph(
        &self,
        byte: usize,
        affinity: TextAffinity,
        context: ParagraphCaretContext<'_>,
    ) -> f32 {
        if let Some(strut) = context.strut {
            let height = strut.ascent + strut.descent;
            return if height > 0. {
                height
            } else {
                context.template_height
            };
        }
        let source = context.source.as_bytes();
        let byte = byte.min(source.len());
        let anchor = if byte > 0
            && affinity == TextAffinity::Upstream
            && !matches!(source[byte - 1], b'\n' | b'\r')
        {
            byte - 1
        } else {
            byte
        };
        if self.overflowed
            && let Some(line) = self.lines.last()
            && anchor >= line.end
        {
            return line.height;
        }
        self.font_run_at(anchor)
            .map_or(context.template_height, |run| run.caret_height)
    }
}

struct MeasuredText {
    min: f32,
    max: f32,
    engine_id: u64,
    generation: u64,
    shaped: Arc<TextLayout>,
    content_width: f32,
    max_intrinsic_width: f32,
    preferred_line_height: f32,
    preferred_baseline: f32,
    strut: Option<TextStrut>,
    paint_offset: Offset,
    lines: OnceLock<Arc<[TextPainterLine]>>,
}

impl MeasuredText {
    fn caret_context<'a>(&self, source: &'a str, direction: TextDirection) -> ParagraphCaretContext<'a> {
        ParagraphCaretContext {
            source,
            direction,
            content_width: self.content_width,
            paint_offset: self.paint_offset,
            template_baseline: self.preferred_baseline,
            template_height: self.preferred_line_height,
            strut: self.strut,
        }
    }
}

/// Measures and positions a text span without building widgets or owning
/// a renderer. Configuration changes invalidate measurements. Drawing
/// consumers use the retained layout together with `paint_offset`.
pub struct TextPainter {
    text: Option<TextSpan>,
    plain: String,
    direction: Option<TextDirection>,
    alignment: TextAlign,
    width_basis: TextWidthBasis,
    max_lines: Option<usize>,
    default_style: TextStyle,
    scaler: TextScaler,
    strut_style: Option<StrutStyle>,
    height_behavior: Option<TextHeightBehavior>,
    hyphens: Hyphens,
    has_soft_hyphen: bool,
    placeholder_dimensions: Vec<PlaceholderDimensions>,
    measured: Option<MeasuredText>,
    layout_invalidated: bool,
    #[cfg(debug_assertions)]
    first_invalidation: Option<Arc<Backtrace>>,
    debug_paint_text_layout_boxes: bool,
    disposed: bool,
    #[cfg(debug_assertions)]
    allocation: Option<incular_core::memory::allocations::ObjectLifetime>,
}

impl Default for TextPainter {
    fn default() -> Self {
        Self {
            text: None,
            plain: String::new(),
            direction: None,
            alignment: TextAlign::default(),
            width_basis: TextWidthBasis::default(),
            max_lines: None,
            default_style: TextStyle::default(),
            scaler: TextScaler::default(),
            strut_style: None,
            height_behavior: None,
            hyphens: Hyphens::default(),
            has_soft_hyphen: false,
            placeholder_dimensions: Vec::new(),
            measured: None,
            layout_invalidated: false,
            #[cfg(debug_assertions)]
            first_invalidation: None,
            debug_paint_text_layout_boxes: false,
            disposed: false,
            #[cfg(debug_assertions)]
            allocation: Some(incular_core::memory::allocations::ObjectLifetime::new(
                "painting", "TextPainter",
            )),
        }
    }
}

impl TextPainter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a painter with its paragraph source and direction set.
    #[must_use]
    pub fn with_span(text: TextSpan, direction: TextDirection) -> Self {
        let mut painter = Self::new();
        painter.set_text(Some(text));
        painter.set_text_direction(Some(direction));
        painter
    }

    /// Measures content width with the default scaling and width basis.
    pub fn compute_width(
        text: TextSpan,
        direction: TextDirection,
        min_width: f32,
        max_width: f32,
    ) -> Result<f32, TextPainterError> {
        let mut painter = Self::with_span(text, direction);
        painter.layout(min_width, max_width)?;
        painter.width()
    }

    /// Measures natural width independently of the resulting paint width.
    pub fn compute_max_intrinsic_width(
        text: TextSpan,
        direction: TextDirection,
        min_width: f32,
        max_width: f32,
    ) -> Result<f32, TextPainterError> {
        let mut painter = Self::with_span(text, direction);
        painter.layout(min_width, max_width)?;
        painter.max_intrinsic_width()
    }

    /// Drops current geometry. In debug builds, the first invalidation
    /// after a successful layout retains its backtrace until relayout.
    pub fn mark_needs_layout(&mut self) {
        if self.measured.take().is_some() {
            self.layout_invalidated = true;
            #[cfg(debug_assertions)]
            {
                self.first_invalidation = Some(Arc::new(Backtrace::force_capture()));
            }
        }
    }

    /// Replaces the span and invalidates its measurement.
    pub fn set_text(&mut self, text: Option<TextSpan>) {
        if self.text != text {
            self.plain = text.as_ref().map_or_else(String::new, TextSpan::plain_text);
            self.has_soft_hyphen = self.plain.contains('\u{00ad}');
            if !self.placeholder_dimensions.is_empty()
                && text.as_ref().map_or(0, |span| span.placeholders().len()) != self.placeholder_dimensions.len()
            {
                self.placeholder_dimensions.clear();
            }
            self.text = text;
            self.mark_needs_layout();
        }
    }

    /// Sets the measured sizes of inline widgets in source order. An
    /// absent, empty or identical slice leaves the existing layout alone.
    pub fn set_placeholder_dimensions(
        &mut self,
        dimensions: Option<&[PlaceholderDimensions]>,
    ) -> Result<(), TextPainterError> {
        if self.disposed {
            return Err(TextPainterError::Disposed);
        }
        let Some(dimensions) = dimensions.filter(|dimensions| !dimensions.is_empty()) else {
            return Ok(());
        };
        if self.placeholder_dimensions == dimensions {
            return Ok(());
        }
        let count = self.text.as_ref().map_or(0, |span| span.placeholders().len());
        if count != dimensions.len() {
            return Err(TextPainterError::InvalidPlaceholderCount);
        }
        self.placeholder_dimensions = dimensions.to_vec();
        self.mark_needs_layout();
        Ok(())
    }

    pub fn set_text_direction(&mut self, direction: Option<TextDirection>) {
        if self.direction != direction {
            self.direction = direction;
            self.mark_needs_layout();
        }
    }

    pub fn set_text_align(&mut self, alignment: TextAlign) {
        if self.alignment != alignment {
            self.alignment = alignment;
            self.mark_needs_layout();
        }
    }

    pub fn set_text_width_basis(&mut self, basis: TextWidthBasis) {
        if self.width_basis != basis {
            self.width_basis = basis;
            self.mark_needs_layout();
        }
    }

    /// Caps the number of laid-out lines without changing the source text.
    pub fn set_max_lines(&mut self, max_lines: Option<usize>) -> Result<(), TextPainterError> {
        if self.disposed {
            return Err(TextPainterError::Disposed);
        }
        if max_lines == Some(0) {
            return Err(TextPainterError::InvalidMaxLines);
        }
        if self.max_lines != max_lines {
            self.max_lines = max_lines;
            self.mark_needs_layout();
        }
        Ok(())
    }

    /// Sets the inherited style for spans that do not specify their own.
    pub fn set_default_style(&mut self, style: TextStyle) {
        if self.default_style != style {
            self.default_style = style;
            self.mark_needs_layout();
        }
    }

    /// Changes font scaling without changing the source span's logical sizes.
    pub fn set_text_scaler(&mut self, scaler: TextScaler) {
        if self.scaler != scaler {
            self.scaler = scaler;
            self.mark_needs_layout();
        }
    }

    /// Sets the paragraph's minimum (or forced) line metrics.
    pub fn set_strut_style(&mut self, strut: Option<StrutStyle>) {
        if self.strut_style != strut {
            self.strut_style = strut;
            self.mark_needs_layout();
        }
    }

    #[must_use]
    pub const fn hyphens(&self) -> Hyphens {
        self.hyphens
    }

    pub fn set_hyphens(&mut self, hyphens: Hyphens) {
        if self.hyphens != hyphens {
            self.hyphens = hyphens;
            self.mark_needs_layout();
        }
    }

    /// Overrides how leading applies at the first and final line edges.
    pub fn set_text_height_behavior(&mut self, behavior: Option<TextHeightBehavior>) {
        if self.height_behavior != behavior {
            self.height_behavior = behavior;
            self.mark_needs_layout();
        }
    }

    pub fn text(&self) -> Option<&TextSpan> {
        self.text.as_ref()
    }

    /// Text as laid out, not as announced. Inline widgets contribute U+FFFC.
    #[must_use]
    pub fn plain_text(&self) -> &str {
        &self.plain
    }

    /// Measures the text using the thread's shared engine. An infinite
    /// maximum is unbounded; the minimum must be a valid finite width.
    pub fn layout(&mut self, min_width: f32, max_width: f32) -> Result<(), TextPainterError> {
        TextEngine::with_shared(|engine| self.layout_with_engine(engine, min_width, max_width))
    }

    /// Measures with a caller's font collection and retained shape cache.
    pub fn layout_with_engine(
        &mut self,
        engine: &mut TextEngine,
        min_width: f32,
        max_width: f32,
    ) -> Result<(), TextPainterError> {
        if self.disposed {
            return Err(TextPainterError::Disposed);
        }
        let text = self.text.as_ref().ok_or(TextPainterError::MissingText)?;
        let direction = self
            .direction
            .ok_or(TextPainterError::MissingTextDirection)?;
        if min_width.is_nan() || min_width < 0. || max_width.is_nan() || max_width < min_width {
            return Err(TextPainterError::InvalidWidth);
        }
        engine.sync_application_fonts();
        let (engine_id, generation) = (engine.instance_id(), engine.font_database_generation());
        let reusable = self
            .measured
            .as_ref()
            .filter(|last| last.engine_id == engine_id && last.generation == generation);
        if reusable.is_some_and(|last| last.min == min_width && last.max == max_width) {
            return Ok(());
        }
        let inherited = &self.default_style;
        let base = text
            .style
            .as_ref()
            .map_or_else(|| inherited.clone(), |style| inherited.merge(style))
            .scaled(self.scaler);
        let mut runs = text.flatten(inherited);
        for run in &mut runs {
            run.style = run.style.scaled(self.scaler);
        }
        let mut placeholders = text.placeholders();
        for (placeholder, dimensions) in placeholders.iter_mut().zip(&self.placeholder_dimensions) {
            placeholder.size = dimensions.size;
            placeholder.alignment = dimensions.alignment;
            placeholder.baseline = dimensions.baseline_offset;
        }
        let strut = reusable.map_or_else(
            || {
                let mut style = self.strut_style.clone()?;
                style.font_size = style.font_size.map(|size| self.scaler.scale(size));
                if style.leading_distribution.is_none() {
                    style.leading_distribution = self
                        .height_behavior
                        .map(|behavior| behavior.leading_distribution);
                }
                (style.font_size != Some(0.)).then(|| engine.resolve_strut(&style, &base))
            },
            |last| last.strut,
        );
        let (preferred_line_height, preferred_baseline) = reusable.map_or_else(
            || {
                let template = engine.layout_with_options(
                    " ",
                    &base,
                    TextLayoutOptions::new(None, TextAlign::Start)
                        .strut(strut)
                        .height_behavior(self.height_behavior.unwrap_or_default()),
                );
                (template.metrics.size.height, template.metrics.baseline)
            },
            |last| (last.preferred_line_height, last.preferred_baseline),
        );
        let measure = |engine: &mut TextEngine, width, max_lines: Option<usize>| {
            let options = TextLayoutOptions::new(width, self.alignment)
                .text_direction(direction)
                .max_lines(max_lines)
                .overflow(if max_lines.is_some() {
                    TextOverflow::Clip
                } else {
                    TextOverflow::Visible
                })
                .strut(strut)
                .height_behavior(self.height_behavior.unwrap_or_default());
            if runs.is_empty() && placeholders.is_empty() {
                engine.layout_with_options(&self.plain, &base, options)
            } else {
                engine.layout_rich_with_options(&self.plain, &runs, &placeholders, options)
            }
        };
        let fraction = match (self.alignment, direction) {
            (TextAlign::Left, _)
            | (TextAlign::Start | TextAlign::Justify, TextDirection::Ltr)
            | (TextAlign::End, TextDirection::Rtl) => 0.,
            (TextAlign::Center, _) => 0.5,
            _ => 1.,
        };
        let (intrinsic, mut shaped, shaping_width) = if max_width.is_finite() {
            let intrinsic = reusable.map_or_else(
                || longest(&measure(engine, None, None)),
                |last| last.max_intrinsic_width,
            );
            (
                intrinsic,
                measure(engine, Some(max_width), self.max_lines),
                max_width,
            )
        } else {
            let unbounded = measure(engine, None, None);
            let intrinsic = longest(&unbounded);
            let shaped = if fraction != 0. && min_width.is_finite() && intrinsic > 0. {
                // An unbounded measurement has no finite alignment target.
                // Use its natural width for the actual paragraph positions,
                // including the alignment gap beside hanging whitespace.
                measure(engine, Some(intrinsic), self.max_lines)
            } else if self
                .max_lines
                .is_some_and(|limit| unbounded.lines.len() > limit)
            {
                measure(engine, None, self.max_lines)
            } else {
                unbounded
            };
            (intrinsic, shaped, intrinsic)
        };
        let hyphenated_width = (self.hyphens == Hyphens::Manual
            && self.has_soft_hyphen
            && shaped.lines.len() > 1)
            .then(|| paint_discretionary_hyphens(engine, &self.plain, &base, &runs, &mut shaped));
        let measured_width = match self.width_basis {
            TextWidthBasis::Parent => intrinsic,
            TextWidthBasis::LongestLine => hyphenated_width.unwrap_or_else(|| visual_width(&shaped)),
        }
        .clamp(min_width, max_width);
        self.layout_invalidated = false;
        #[cfg(debug_assertions)]
        {
            self.first_invalidation = None;
        }
        self.measured = Some(MeasuredText {
            min: min_width,
            max: max_width,
            engine_id,
            generation,
            content_width: measured_width,
            shaped,
            max_intrinsic_width: intrinsic,
            preferred_line_height,
            preferred_baseline,
            strut,
            paint_offset: Offset::new(
                if fraction == 0. { 0. } else { fraction * (measured_width - shaping_width) },
                0.,
            ),
            lines: OnceLock::new(),
        });
        Ok(())
    }

    fn measured(&self) -> Result<&MeasuredText, TextPainterError> {
        if self.disposed {
            return Err(TextPainterError::Disposed);
        }
        self.measured.as_ref().ok_or(TextPainterError::NeedsLayout)
    }

    /// The measured paragraph width, which may be infinite under an
    /// explicitly infinite minimum. It is not necessarily a layout Size.
    pub fn width(&self) -> Result<f32, TextPainterError> {
        Ok(self.measured()?.content_width)
    }

    pub fn height(&self) -> Result<f32, TextPainterError> {
        Ok(self.measured()?.shaped.metrics.size.height)
    }

    /// Returns a finite size suitable for a retained layout box.
    pub fn size(&self) -> Result<Size, TextPainterError> {
        Size::maybe_new(self.width()?, self.height()?).ok_or(TextPainterError::UnboundedSize)
    }

    pub fn max_intrinsic_width(&self) -> Result<f32, TextPainterError> {
        Ok(self.measured()?.max_intrinsic_width)
    }

    /// Distance from the top to the first line's alphabetic baseline.
    pub fn alphabetic_baseline(&self) -> Result<f32, TextPainterError> {
        Ok(self.measured()?.shaped.metrics.baseline)
    }

    pub fn paint_offset(&self) -> Result<Offset, TextPainterError> {
        Ok(self.measured()?.paint_offset)
    }

    /// Toggles drawing tight layout boxes before the paragraph in debug builds.
    /// This does not invalidate the retained paragraph.
    pub fn set_debug_paint_text_layout_boxes(&mut self, enabled: bool) {
        self.debug_paint_text_layout_boxes = enabled;
    }

    /// Sends the measured paragraph to a renderer without diagnostic overlays.
    pub fn paint_with(
        &self,
        origin: Offset,
        draw: impl FnOnce(&TextLayout, Offset),
    ) -> Result<(), TextPainterError> {
        if let Some((layout, offset)) = self.paint_geometry(origin)? {
            draw(layout, offset);
        }
        Ok(())
    }

    /// Visits diagnostic box outlines, when enabled, and then the paragraph.
    /// Nonfinite paint positions yield no operations.
    pub fn paint_operations(
        &self,
        origin: Offset,
        mut draw: impl FnMut(TextPainterPaint<'_>),
    ) -> Result<(), TextPainterError> {
        let Some((layout, offset)) = self.paint_geometry(origin)? else {
            return Ok(());
        };
        if cfg!(debug_assertions) && self.debug_paint_text_layout_boxes {
            for item in self.tight_boxes_for_selection(TextRange::new(0, self.plain.len()))? {
                draw(TextPainterPaint::DebugBox {
                    rect: Rect::from_origin_size(item.rect.origin + origin, item.rect.size),
                    color: Color::rgba(0, 255, 255, 255),
                    stroke_width: 1.,
                });
            }
        }
        draw(TextPainterPaint::Paragraph { layout, offset });
        Ok(())
    }

    fn paint_geometry(
        &self,
        origin: Offset,
    ) -> Result<Option<(&TextLayout, Offset)>, TextPainterError> {
        let measured = self.measured().map_err(|error| match error {
            TextPainterError::NeedsLayout => TextPainterError::PaintNeedsLayout,
            other => other,
        })?;
        let offset = origin + measured.paint_offset;
        Ok((offset.x.is_finite() && offset.y.is_finite())
            .then_some((measured.shaped.as_ref(), offset)))
    }

    /// Whether the line limit hid source characters. Suppressing only the
    /// synthetic empty line after a final newline does not count as overflow.
    pub fn did_exceed_max_lines(&self) -> Result<bool, TextPainterError> {
        let layout = &self.measured()?.shaped;
        Ok(layout.overflowed
            && layout
                .lines
                .last()
                .is_some_and(|line| line.end < self.plain.len()))
    }

    pub fn shaped_layout(&self) -> Result<&Arc<TextLayout>, TextPainterError> {
        Ok(&self.measured()?.shaped)
    }

    /// Cached line geometry. An unchanged calculation returns the same
    /// allocation even when alignment shifts the paragraph during paint.
    pub fn line_metrics(&self) -> Result<&Arc<[TextPainterLine]>, TextPainterError> {
        let measured = self.measured()?;
        Ok(measured.lines.get_or_init(|| {
            let lines = &measured.shaped.lines;
            lines
                .iter()
                .enumerate()
                .map(|(i, line)| TextPainterLine {
                    line_number: i,
                    start: line.start,
                    end: line.end,
                    hard_break: line.caret_end < line.end
                        || matches!(
                            self.plain.as_bytes().get(line.caret_end),
                            Some(b'\n' | b'\r')
                        )
                        || line.end.checked_sub(1).is_some_and(|byte| {
                            matches!(self.plain.as_bytes().get(byte), Some(b'\n' | b'\r'))
                        })
                        || (i + 1 == lines.len() && !measured.shaped.overflowed),
                    left: line.offset
                        + if self.direction == Some(TextDirection::Rtl) {
                            line.hanging_advance
                        } else {
                            0.
                        }
                        + measured.paint_offset.x,
                    top: line.top,
                    width: (line.width - line.hanging_advance).max(0.),
                    baseline: line.baseline,
                    ascent: line.ascent,
                    descent: line.descent,
                    unscaled_ascent: line.unscaled_ascent,
                    height: line.height,
                })
                .collect()
        }))
    }

    /// Tight selection boxes in painter coordinates.
    pub fn tight_boxes_for_selection(
        &self,
        range: TextRange,
    ) -> Result<Vec<TextPainterBox>, TextPainterError> {
        self.boxes_for_selection(range, BoxHeightStyle::Tight, BoxWidthStyle::Tight)
    }

    /// Selection boxes in paint coordinates with caller-chosen bounds.
    pub fn boxes_for_selection(
        &self,
        range: TextRange,
        height_style: BoxHeightStyle,
        width_style: BoxWidthStyle,
    ) -> Result<Vec<TextPainterBox>, TextPainterError> {
        let measured = self.measured()?;
        let mut boxes = measured.shaped.boxes_for_selection(range, height_style, width_style);
        for selection in &mut boxes {
            selection.rect.origin = selection.rect.origin + measured.paint_offset;
        }
        Ok(boxes)
    }

    /// Locates a caret at a UTF-8 byte position. Affinity chooses the
    /// trailing or leading edge on a soft-wrap or bidi boundary. At an
    /// explicit newline the source position takes precedence.
    pub fn caret_offset(
        &self,
        byte: usize,
        affinity: TextAffinity,
        caret_width: f32,
    ) -> Result<Offset, TextPainterError> {
        let measured = self.measured()?;
        Ok(measured.shaped.caret_offset_in_paragraph(
            byte,
            affinity,
            caret_width,
            measured.caret_context(&self.plain, self.direction.unwrap_or(TextDirection::Ltr)),
        ))
    }

    /// Height of a line using the root style as its text template.
    pub fn preferred_line_height(&self) -> Result<f32, TextPainterError> {
        Ok(self.measured()?.preferred_line_height)
    }

    /// Height at a UTF-8 caret position. Without a strut, affinity
    /// chooses the adjacent shaped run's tight font height. The downstream
    /// text end and degenerate layouts use the root line template.
    pub fn full_height_for_caret(
        &self,
        byte: usize,
        affinity: TextAffinity,
    ) -> Result<f32, TextPainterError> {
        let measured = self.measured()?;
        Ok(measured.shaped.caret_height_in_paragraph(
            byte,
            affinity,
            measured.caret_context(&self.plain, self.direction.unwrap_or(TextDirection::Ltr)),
        ))
    }

    /// Asks for the same caret using a Flutter/IME UTF-16 code-unit offset.
    /// A position inside a surrogate pair resolves at the containing
    /// grapheme boundary, not a split UTF-8 scalar.
    pub fn caret_offset_utf16(
        &self,
        position: usize,
        affinity: TextAffinity,
        caret_width: f32,
    ) -> Result<Offset, TextPainterError> {
        let mut units = 0;
        for (byte, character) in self.plain.char_indices() {
            if units == position {
                return self.caret_offset(byte, affinity, caret_width);
            }
            units += character.len_utf16();
            if units > position {
                return self.caret_offset(byte + character.len_utf8(), affinity, caret_width);
            }
        }
        self.caret_offset(self.plain.len(), affinity, caret_width)
    }

    /// Returns the closest retained insertion point in local paint
    /// coordinates. The result uses UTF-8 bytes and preserves wrap affinity.
    pub fn position_for_offset(
        &self,
        point: Offset,
    ) -> Result<TextPainterPosition, TextPainterPositionError> {
        if self.disposed {
            return Err(TextPainterPositionError::Disposed);
        }
        let Some(measured) = self.measured.as_ref() else {
            return Err(if self.layout_invalidated {
                TextPainterPositionError::LayoutInvalidated {
                    #[cfg(debug_assertions)]
                    first_invalidation: self.first_invalidation.clone(),
                    #[cfg(not(debug_assertions))]
                    first_invalidation: None,
                }
            } else {
                TextPainterPositionError::NeverLaidOut
            });
        };
        Ok(measured.shaped.closest_text_position(point - measured.paint_offset))
    }

    /// Looks up the word segment containing the given UTF-8 byte position.
    pub fn word_boundary(&self, position: usize) -> Result<TextRange, TextPainterError> {
        self.measured()?;
        let (start, end) = word_boundary_at(&self.plain, position);
        Ok(TextRange::new(start, end))
    }

    #[must_use]
    pub fn debug_disposed(&self) -> bool {
        self.disposed
    }

    /// Releases the paragraph and marks it permanently unavailable.
    pub fn dispose(&mut self) -> Result<(), TextPainterError> {
        if self.disposed {
            return Err(TextPainterError::Disposed);
        }
        self.disposed = true;
        #[cfg(debug_assertions)]
        drop(self.allocation.take());
        self.measured = None;
        #[cfg(debug_assertions)]
        {
            self.first_invalidation = None;
        }
        self.placeholder_dimensions.clear();
        self.text = None;
        self.plain = String::new();
        Ok(())
    }
}

fn visual_width(layout: &TextLayout) -> f32 {
    layout
        .lines
        .iter()
        .map(|line| line.width - line.hanging_advance)
        .fold(0., f32::max)
}

/// A discretionary mark is painted by the line without adding a text
/// cluster. Flutter exposes its advance in the overall painted width,
/// not in line metrics, selection geometry, or caret positions.
fn paint_discretionary_hyphens(
    engine: &mut TextEngine,
    source: &str,
    base: &TextStyle,
    styles: &[crate::StyledTextRun],
    shaped: &mut Arc<TextLayout>,
) -> f32 {
    use incular_rendering::GlyphRun;
    let mut width = visual_width(shaped);
    let mut marks = Vec::new();
    for (index, lines) in shaped.lines.windows(2).enumerate() {
        let line = &lines[0];
        let next = &lines[1];
        let Some(prefix) = source.get(..next.start) else {
            continue;
        };
        let line_break = |ch| matches!(ch, '\r' | '\n' | '\u{2028}' | '\u{2029}');
        let Some(prefix) = prefix.trim_end_matches(line_break).strip_suffix('\u{00ad}') else {
            continue;
        };
        let byte = prefix.len();
        if byte < line.start
            || (line.end < next.start
                && !source
                    .get(line.end..next.start)
                    .is_some_and(|gap| gap.chars().all(line_break)))
        {
            continue;
        }
        let style = styles
            .get(styles.partition_point(|run| run.range.end <= byte))
            .filter(|run| run.range.contains(&byte))
            .map_or(base, |run| &run.style);
        let edge = line
            .clusters
            .iter()
            .rev()
            .find(|cluster| cluster.range().start <= byte && cluster.range().end >= byte);
        let rtl = edge.is_some_and(|cluster| cluster.rtl);
        let hyphen = engine.layout_with_options(
            "\u{2010}",
            style,
            TextLayoutOptions::new(None, TextAlign::Left),
        );
        let Some(template) = hyphen.lines.first() else {
            continue;
        };
        let advance = template.width;
        let x = edge.map_or(line.offset + line.width - line.hanging_advance, |cluster| {
            if rtl {
                cluster.left - advance
            } else {
                cluster.right
            }
        });
        let snapped_baseline =
            |line: &crate::TextLine| line.top + (line.baseline - line.top).round();
        let dy = snapped_baseline(template) - snapped_baseline(line);
        width = width.max(line.width - line.hanging_advance + advance);
        marks.push((
            index,
            template
                .runs
                .iter()
                .map(|run| {
                    Arc::new(GlyphRun {
                        glyphs: run
                            .glyphs
                            .iter()
                            .map(|glyph| {
                                let mut marked = *glyph;
                                marked.offset.x += x;
                                marked.offset.y += dy;
                                marked.cluster = byte as u32;
                                marked
                            })
                            .collect(),
                        ..run.as_ref().clone()
                    })
                })
                .collect::<Vec<_>>(),
        ));
    }
    if !marks.is_empty() {
        let lines = Arc::make_mut(&mut Arc::make_mut(shaped).lines);
        for (index, marks) in marks {
            let line = &mut lines[index];
            line.glyphs = line
                .glyphs
                .iter()
                .copied()
                .chain(marks.iter().flat_map(|mark| mark.glyphs.iter().copied()))
                .collect();
            line.runs = line.runs.iter().cloned().chain(marks).collect();
        }
    }
    width
}

impl TextEngine {
    /// Adds marks at taken soft-hyphen breaks without changing caret boxes.
    #[doc(hidden)]
    pub fn apply_discretionary_hyphens(
        &mut self,
        layout: &mut Arc<TextLayout>,
        source: &str,
        style: &TextStyle,
        runs: &[crate::StyledTextRun],
        mode: Hyphens,
        max_width: Option<f32>,
    ) {
        const SOFT_HYPHEN: char = 0x00ad_u8 as char;
        if mode == Hyphens::Manual && layout.lines.len() > 1 && source.contains(SOFT_HYPHEN) {
            let advance = paint_discretionary_hyphens(self, source, style, runs, layout);
            let advance = max_width.map_or(advance, |limit| advance.min(limit));
            if advance > layout.metrics.size.width {
                Arc::make_mut(layout).metrics.size.width = advance;
            }
        }
    }
}

fn longest(layout: &TextLayout) -> f32 {
    layout
        .lines
        .iter()
        .map(|line| line.width)
        .fold(0., f32::max)
}
