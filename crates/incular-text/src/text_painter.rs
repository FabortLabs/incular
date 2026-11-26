//! Stateful paragraph measurement over the shared Incular shaping engine.

use std::{
    fmt,
    sync::{Arc, OnceLock},
};

use incular_config::TextDirection;
use incular_core::{Offset, Size};

use crate::{
    StrutStyle, TextAffinity, TextAlign, TextEngine, TextHeightBehavior, TextLayout,
    TextLayoutOptions, TextOverflow, TextRange, TextScaler, TextSpan, TextStrut, TextStyle,
    TextWidthBasis, word_boundary_at,
};

/// A layout request or query that cannot be satisfied yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextPainterError {
    MissingText,
    MissingTextDirection,
    InvalidMaxLines,
    InvalidWidth,
    NeedsLayout,
    Disposed,
}

impl fmt::Display for TextPainterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingText => "TextPainter requires text before layout",
            Self::MissingTextDirection => "TextPainter requires a text direction before layout",
            Self::InvalidMaxLines => "TextPainter max lines must be positive",
            Self::InvalidWidth => "invalid TextPainter width constraints",
            Self::NeedsLayout => "TextPainter requires a current layout",
            Self::Disposed => "TextPainter has already been disposed",
        })
    }
}
impl std::error::Error for TextPainterError {}

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

#[derive(Clone, Copy)]
struct RunHeight {
    start: usize,
    end: usize,
    height: f32,
}

struct MeasuredText {
    min: f32,
    max: f32,
    engine_id: u64,
    generation: u64,
    shaped: Arc<TextLayout>,
    size: Size,
    max_intrinsic_width: f32,
    preferred_line_height: f32,
    preferred_baseline: f32,
    strut: Option<TextStrut>,
    run_heights: OnceLock<Vec<RunHeight>>,
    paint_offset: Offset,
    lines: OnceLock<Arc<[TextPainterLine]>>,
}

/// Measures and positions a text span without building widgets or owning
/// a renderer. Configuration changes invalidate measurements. Drawing
/// consumers use the retained layout together with `paint_offset`.
#[derive(Default)]
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
    measured: Option<MeasuredText>,
    disposed: bool,
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
        Ok(painter.size()?.width)
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

    /// Replaces the span and invalidates its measurement.
    pub fn set_text(&mut self, text: Option<TextSpan>) {
        if self.text != text {
            self.plain = text.as_ref().map_or_else(String::new, TextSpan::plain_text);
            self.text = text;
            self.measured = None;
        }
    }

    pub fn set_text_direction(&mut self, direction: Option<TextDirection>) {
        if self.direction != direction {
            self.direction = direction;
            self.measured = None;
        }
    }

    pub fn set_text_align(&mut self, alignment: TextAlign) {
        if self.alignment != alignment {
            self.alignment = alignment;
            self.measured = None;
        }
    }

    pub fn set_text_width_basis(&mut self, basis: TextWidthBasis) {
        if self.width_basis != basis {
            self.width_basis = basis;
            self.measured = None;
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
            self.measured = None;
        }
        Ok(())
    }

    /// Sets the inherited style for spans that do not specify their own.
    pub fn set_default_style(&mut self, style: TextStyle) {
        if self.default_style != style {
            self.default_style = style;
            self.measured = None;
        }
    }

    /// Changes font scaling without changing the source span's logical sizes.
    pub fn set_text_scaler(&mut self, scaler: TextScaler) {
        if self.scaler != scaler {
            self.scaler = scaler;
            self.measured = None;
        }
    }

    /// Sets the paragraph's minimum (or forced) line metrics.
    pub fn set_strut_style(&mut self, strut: Option<StrutStyle>) {
        if self.strut_style != strut {
            self.strut_style = strut;
            self.measured = None;
        }
    }

    /// Overrides how leading applies at the first and final line edges.
    pub fn set_text_height_behavior(&mut self, behavior: Option<TextHeightBehavior>) {
        if self.height_behavior != behavior {
            self.height_behavior = behavior;
            self.measured = None;
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
        if !min_width.is_finite() || min_width < 0. || max_width.is_nan() || max_width < min_width {
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
        let placeholders = text.placeholders();
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
        let (intrinsic, shaped, shaping_width) = if max_width.is_finite() {
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
            let shaped = if self
                .max_lines
                .is_some_and(|limit| unbounded.lines.len() > limit)
            {
                measure(engine, None, self.max_lines)
            } else {
                unbounded
            };
            (intrinsic, shaped, intrinsic)
        };
        let measured_width = match self.width_basis {
            TextWidthBasis::Parent => intrinsic,
            TextWidthBasis::LongestLine => shaped
                .lines
                .iter()
                .map(|line| line.width - line.hanging_advance)
                .fold(0., f32::max),
        }
        .clamp(min_width, max_width);
        let fraction = match (self.alignment, direction) {
            (TextAlign::Left, _)
            | (TextAlign::Start | TextAlign::Justify, TextDirection::Ltr)
            | (TextAlign::End, TextDirection::Rtl) => 0.,
            (TextAlign::Center, _) => 0.5,
            _ => 1.,
        };
        self.measured = Some(MeasuredText {
            min: min_width,
            max: max_width,
            engine_id,
            generation,
            size: Size::new(measured_width, shaped.metrics.size.height),
            shaped,
            max_intrinsic_width: intrinsic,
            preferred_line_height,
            preferred_baseline,
            strut,
            run_heights: OnceLock::new(),
            paint_offset: Offset::new(fraction * (measured_width - shaping_width), 0.),
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

    pub fn size(&self) -> Result<Size, TextPainterError> {
        Ok(self.measured()?.size)
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
        let source = &self.plain;
        let lines = &measured.shaped.lines;
        if lines.is_empty() {
            return Ok(Offset::ZERO);
        }
        let line_at = |byte| {
            lines
                .partition_point(|line| line.start <= byte)
                .saturating_sub(1)
        };
        let mut byte = byte.min(source.len());
        let mut index = line_at(byte);
        if byte < lines[index].caret_end
            && !measured
                .shaped
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
        if measured.shaped.overflowed {
            let last = &lines[lines.len() - 1];
            let anchor = if use_leading {
                byte
            } else {
                byte.saturating_sub(1)
            };
            if anchor >= last.end {
                let rtl = self.direction == Some(TextDirection::Rtl);
                let x = if rtl {
                    last.offset + last.hanging_advance - caret_width.max(0.)
                } else {
                    last.offset + last.width - last.hanging_advance
                };
                return Ok(Offset::new(
                    (x + measured.paint_offset.x).clamp(0., measured.size.width),
                    last.baseline - measured.preferred_baseline,
                ));
            }
        }
        if !use_leading && index > 0 && lines[index].start == byte {
            index -= 1;
        }
        let line = &lines[index];
        let stops = measured.shaped.line_caret_positions(index);
        let paragraph_rtl = self.direction == Some(TextDirection::Rtl);
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
        Ok(Offset::new(
            (x - advance + measured.paint_offset.x).clamp(0., measured.size.width),
            line.top,
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
        if let Some(strut) = measured.strut {
            let height = strut.ascent + strut.descent;
            return Ok(if height > 0. {
                height
            } else {
                measured.preferred_line_height
            });
        }
        let source = self.plain.as_bytes();
        let byte = byte.min(source.len());
        let anchor = if byte > 0
            && affinity == TextAffinity::Upstream
            && !matches!(source[byte - 1], b'\n' | b'\r')
        {
            byte - 1
        } else {
            byte
        };
        if measured.shaped.overflowed
            && let Some(line) = measured.shaped.lines.last()
            && anchor >= line.end
        {
            return Ok(line.height);
        }
        let runs = measured.run_heights.get_or_init(|| {
            let mut runs: Vec<_> = measured
                .shaped
                .font_runs
                .iter()
                .map(|run| RunHeight {
                    start: run.range.start,
                    end: run.range.end,
                    height: run.caret_height,
                })
                .collect();
            runs.sort_unstable_by_key(|run| run.start);
            runs
        });
        Ok(runs
            .partition_point(|run| run.start <= anchor)
            .checked_sub(1)
            .and_then(|index| runs.get(index))
            .filter(|run| anchor < run.end)
            .map_or(measured.preferred_line_height, |run| run.height))
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
        self.measured = None;
        self.text = None;
        self.plain = String::new();
        Ok(())
    }
}

fn longest(layout: &TextLayout) -> f32 {
    layout
        .lines
        .iter()
        .map(|line| line.width)
        .fold(0., f32::max)
}
