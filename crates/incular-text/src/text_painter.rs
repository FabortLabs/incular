//! Stateful paragraph measurement over the shared Incular shaping engine.

use std::{
    fmt,
    sync::{Arc, OnceLock},
};

use incular_config::TextDirection;
use incular_core::{Offset, Size};

use crate::{
    TextAlign, TextEngine, TextLayout, TextLayoutOptions, TextOverflow, TextRange, TextSpan,
    TextStyle, TextWidthBasis, word_boundary_at,
};

/// A layout request or query that cannot be satisfied yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextPainterError {
    MissingText,
    MissingTextDirection,
    InvalidWidth,
    NeedsLayout,
    Disposed,
}

impl fmt::Display for TextPainterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingText => "TextPainter requires text before layout",
            Self::MissingTextDirection => "TextPainter requires a text direction before layout",
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
    pub width: f32,
    pub baseline: f32,
}

struct MeasuredText {
    min: f32,
    max: f32,
    engine_id: u64,
    generation: u64,
    shaped: Arc<TextLayout>,
    size: Size,
    max_intrinsic_width: f32,
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
    measured: Option<MeasuredText>,
    disposed: bool,
}

impl TextPainter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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
        let runs = text.flatten(&TextStyle::default());
        let placeholders = text.placeholders();
        let measure = |engine: &mut TextEngine, width| {
            engine.layout_rich_with_options(
                &self.plain,
                &runs,
                &placeholders,
                TextLayoutOptions::new(width, self.alignment)
                    .text_direction(direction)
                    .overflow(TextOverflow::Visible),
            )
        };
        let (intrinsic, shaped, shaping_width) = if max_width.is_finite() {
            let intrinsic = reusable.map_or_else(
                || longest(&measure(engine, None)),
                |last| last.max_intrinsic_width,
            );
            (intrinsic, measure(engine, Some(max_width)), max_width)
        } else {
            let shaped = measure(engine, None);
            let intrinsic = longest(&shaped);
            (intrinsic, shaped, intrinsic)
        };
        let measured_width = match self.width_basis {
            TextWidthBasis::Parent => intrinsic,
            TextWidthBasis::LongestLine => longest(&shaped),
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

    pub fn paint_offset(&self) -> Result<Offset, TextPainterError> {
        Ok(self.measured()?.paint_offset)
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
                    hard_break: line.caret_end < line.end || i + 1 == lines.len(),
                    left: line.offset + measured.paint_offset.x,
                    width: line.width,
                    baseline: line.baseline,
                })
                .collect()
        }))
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
