use super::fonts::{FontFamily, FontWeight};
use super::text_style::TextStyle;
use incular_rendering::Shadow;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextAlign {
    /// Aligns to the start edge of the paragraph's text direction (`left` for
    /// LTR, `right` for RTL) - Flutter's `TextAlign.start`, which is also the
    /// default of `Text`.
    #[default]
    Start,
    Center,
    /// Aligns to the end edge of the paragraph's text direction.
    End,
    Justify,
    /// Aligns to the physical left edge regardless of text direction.
    Left,
    /// Aligns to the physical right edge regardless of text direction.
    Right,
}

/// The baseline alignment policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextBaseline {
    #[default]
    Alphabetic,
    Ideographic,
}

/// The leading distribution policy between line boxes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextLeadingDistribution {
    #[default]
    Proportional,
    Even,
}

/// Strategy for measuring paragraph width bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextWidthBasis {
    #[default]
    Parent,
    LongestLine,
}

/// Controls line-box height calculations for first ascent and last descent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextHeightBehavior {
    pub apply_height_to_first_ascent: bool,
    pub apply_height_to_last_descent: bool,
    pub leading_distribution: TextLeadingDistribution,
}

/// Matches Flutter's `TextHeightBehavior()`: height modifiers apply to both
/// the first ascent and the last descent, with proportional leading.
impl Default for TextHeightBehavior {
    fn default() -> Self {
        Self {
            apply_height_to_first_ascent: true,
            apply_height_to_last_descent: true,
            leading_distribution: TextLeadingDistribution::Proportional,
        }
    }
}

impl TextHeightBehavior {
    #[must_use]
    pub const fn new(
        apply_height_to_first_ascent: bool,
        apply_height_to_last_descent: bool,
        leading_distribution: TextLeadingDistribution,
    ) -> Self {
        Self {
            apply_height_to_first_ascent,
            apply_height_to_last_descent,
            leading_distribution,
        }
    }
}

bitflags::bitflags! {
    /// Visual line decorations for rendered text.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct TextDecoration: u8 {
        const NONE = 0;
        const UNDERLINE = 1;
        const OVERLINE = 2;
        const LINE_THROUGH = 4;
    }
}

impl TextDecoration {
    #[must_use]
    pub const fn combine(decorations: &[Self]) -> Self {
        let mut bits = 0;
        let mut i = 0;
        while i < decorations.len() {
            bits |= decorations[i].bits();
            i += 1;
        }
        Self::from_bits_retain(bits)
    }
}

/// Visual stroke pattern for text decorations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextDecorationStyle {
    #[default]
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

/// Compatibility spelling for the renderer-owned canonical shadow value.
/// Text must not maintain a second color/offset/blur representation.
pub type TextShadow = Shadow;

/// Baseline strut configuration defining minimum vertical line spacing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StrutStyle {
    pub font_family: Option<FontFamily>,
    pub font_family_fallback: Arc<[FontFamily]>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub height: Option<f32>,
    /// Additional spacing as a multiple of the resolved font size.
    pub leading: Option<f32>,
    pub leading_distribution: Option<TextLeadingDistribution>,
    pub force_strut_height: Option<bool>,
}

impl StrutStyle {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Copies the explicitly supplied typographic attributes from a text
    /// style. The independent leading and force flags remain unset. An
    /// Incular absolute-pixel line height is not a strut height multiplier.
    #[must_use]
    pub fn from_text_style(style: &TextStyle) -> Self {
        Self {
            font_family: style.has_explicit_family.then(|| style.family.clone()),
            font_family_fallback: style.fallback_families.clone(),
            font_size: style.has_explicit_size.then_some(style.size),
            font_weight: style.has_explicit_weight.then_some(style.weight),
            font_style: style.has_explicit_style.then_some(style.style),
            height: match style.line_height {
                Some(LineHeight::Normal) => Some(0.),
                Some(LineHeight::Multiplier(factor)) => Some(factor),
                Some(LineHeight::Absolute(_)) | None => None,
            },
            leading_distribution: style.leading_distribution,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn font_family(mut self, family: impl Into<String>) -> Self {
        self.font_family = Some(FontFamily::named(family));
        self
    }

    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size.max(0.0));
        self
    }

    #[must_use]
    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.font_weight = Some(weight);
        self
    }

    #[must_use]
    pub fn font_style(mut self, style: FontStyle) -> Self {
        self.font_style = Some(style);
        self
    }

    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height.max(0.0));
        self
    }

    #[must_use]
    pub fn leading(mut self, factor: f32) -> Self {
        self.leading = Some(factor.max(0.0));
        self
    }

    #[must_use]
    pub fn leading_distribution(mut self, distribution: TextLeadingDistribution) -> Self {
        self.leading_distribution = Some(distribution);
        self
    }

    #[must_use]
    pub fn force_strut_height(mut self, force: bool) -> Self {
        self.force_strut_height = Some(force);
        self
    }
}

/// What to do when a paragraph cannot fit its configured line or width limit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextOverflow {
    #[default]
    Clip,
    Ellipsis,
    Visible,
    Fade,
}

/// Text scaling policy applied after a style's logical font size is chosen.
#[derive(Clone, Copy, Debug)]
pub struct TextScaler {
    kind: TextScalerKind,
    factor: f32,
    // Keep system ranges separate from the platform value. Projecting the
    // factor early makes separately configured policies compare equal.
    min: f32,
    max: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextScalerKind {
    Linear,
    NoScaling,
    /// Platform accessibility preference captured from a view.
    System,
    /// Platform preference constrained by a range.
    ClampedSystem,
}

/// Invalid lower or upper bound supplied to a scaler clamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextScaleBoundsError {
    /// The upper bound cannot be below the lower bound or NaN.
    InvalidMaximum,
    /// The lower bound must be finite.
    NonFiniteMinimum,
    /// The lower bound must be at least zero.
    NegativeMinimum,
}

impl std::fmt::Display for TextScaleBoundsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidMaximum => "maxScaleFactor >= minScaleFactor",
            Self::NonFiniteMinimum => "minScaleFactor.isFinite",
            Self::NegativeMinimum => "minScaleFactor >= 0",
        })
    }
}

impl std::error::Error for TextScaleBoundsError {}

impl PartialEq for TextScaler {
    fn eq(&self, other: &Self) -> bool {
        use TextScalerKind::{ClampedSystem, System};
        match (self.kind, other.kind) {
            (ClampedSystem, ClampedSystem) => {
                self.factor == other.factor && self.min == other.min && self.max == other.max
            }
            (ClampedSystem, _) | (_, ClampedSystem) => false,
            (System, System) => self.factor == other.factor,
            (System, _) | (_, System) => self.factor == 1.0 && other.factor == 1.0,
            // Flutter's noScaling is the same policy as linear(1).
            _ => self.factor == other.factor,
        }
    }
}

impl TextScaler {
    pub const NO_SCALING: Self = Self {
        kind: TextScalerKind::NoScaling,
        factor: 1.0,
        min: 0.0,
        max: f32::INFINITY,
    };

    #[must_use]
    pub const fn linear(factor: f32) -> Self {
        Self {
            kind: TextScalerKind::Linear,
            factor,
            ..Self::NO_SCALING
        }
    }

    /// Snapshots the platform's accessibility scale. Incular hosts currently
    /// supply a proportional factor; clamps retain their system provenance
    /// and bounds instead of reducing to an unrelated explicit linear policy.
    #[must_use]
    pub const fn system(factor: f32) -> Self {
        Self {
            kind: TextScalerKind::System,
            factor,
            ..Self::NO_SCALING
        }
    }

    #[must_use]
    pub const fn no_scaling() -> Self {
        Self::NO_SCALING
    }

    #[must_use]
    pub const fn factor(self) -> f32 {
        if !matches!(self.kind, TextScalerKind::ClampedSystem) {
            self.factor
        } else if self.factor < self.min {
            self.min
        } else if self.factor > self.max {
            self.max
        } else {
            self.factor
        }
    }

    #[must_use]
    pub const fn kind(self) -> TextScalerKind {
        self.kind
    }

    fn effective_factor(self) -> f32 {
        let factor = if self.factor.is_finite() && self.factor >= 0.0 {
            self.factor
        } else {
            1.0
        };
        match self.kind {
            TextScalerKind::ClampedSystem => factor.clamp(self.min, self.max),
            _ => factor,
        }
    }

    #[must_use]
    pub fn scale(self, font_size: f32) -> f32 {
        (font_size.max(0.0) * self.effective_factor()).max(0.0)
    }

    /// Restricts scaled size to between these two multiples of the input.
    /// A minimum of zero and maximum of infinity leave a valid policy
    /// unchanged. Successive linear clamps act on the current factor; a
    /// system clamp intersects the policy ranges and collapses disjoint bounds.
    pub fn try_clamp(self, min: f32, max: f32) -> Result<Self, TextScaleBoundsError> {
        if min.is_nan() || max.is_nan() || max < min {
            return Err(TextScaleBoundsError::InvalidMaximum);
        }
        if !min.is_finite() {
            return Err(TextScaleBoundsError::NonFiniteMinimum);
        }
        if min < 0.0 {
            return Err(TextScaleBoundsError::NegativeMinimum);
        }
        use TextScalerKind::{ClampedSystem, System};
        Ok(match self.kind {
            System | ClampedSystem if min == 0.0 && max == f32::INFINITY => self,
            System | ClampedSystem => {
                let lower = self.min.max(min);
                let upper = self.max.min(max);
                if upper <= lower {
                    // With no overlap, use the nearest boundary of the new range.
                    Self::linear(self.min.clamp(min, max))
                } else {
                    Self {
                        kind: ClampedSystem,
                        min: lower,
                        max: upper,
                        ..self
                    }
                }
            }
            _ => {
                let factor = self.effective_factor().clamp(min, max);
                if factor == self.factor {
                    self
                } else {
                    Self::linear(factor)
                }
            }
        })
    }

    #[must_use]
    pub fn is_no_scaling(self) -> bool {
        self == Self::NO_SCALING
    }
}

impl Default for TextScaler {
    fn default() -> Self {
        Self::NO_SCALING
    }
}

/// Explicit text line height representation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    /// Natural line height derived from font metrics.
    Normal,
    /// Line height as a proportional multiplier of font size (e.g. `1.4` = 1.4 * font_size).
    Multiplier(f32),
    /// Line height in absolute logical pixels.
    Absolute(f32),
}

impl LineHeight {
    #[must_use]
    pub const fn multiplier(factor: f32) -> Self {
        Self::Multiplier(factor)
    }

    #[must_use]
    pub const fn absolute(pixels: f32) -> Self {
        Self::Absolute(pixels)
    }

    #[must_use]
    pub fn to_parley(self, font_size: f32) -> Option<parley::style::LineHeight> {
        match self {
            Self::Normal => None,
            Self::Multiplier(factor) => Some(parley::style::LineHeight::Absolute(
                (factor * font_size).max(0.0),
            )),
            Self::Absolute(pixels) => Some(parley::style::LineHeight::Absolute(pixels.max(0.0))),
        }
    }
}
