use super::fonts::{
    FontFamily, FontFeature, FontVariation, FontWeight, format_font_variations,
    lerp_font_variation_settings,
};
use super::text::{
    FontStyle, LineHeight, TextBaseline, TextDecoration, TextDecorationStyle,
    TextLeadingDistribution, TextOverflow, TextScaler, TextShadow,
};
use incular_config::WidgetDefaults;
use incular_core::{ChangeImpact, Color, Lerp};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, LazyLock};

/// Multiplies a specified text metric by `factor`, then adds `delta`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetricAdjustment {
    pub factor: f32,
    pub delta: f32,
}

impl TextMetricAdjustment {
    #[must_use]
    pub const fn new(factor: f32, delta: f32) -> Self {
        Self { factor, delta }
    }

    fn transform(self, metric: f32) -> f32 {
        metric * self.factor + self.delta
    }

    fn specified(self, metric: Option<f32>, name: &str) -> Option<f32> {
        debug_assert!(
            metric.is_some() || self == Self::default(),
            "TextStyle.apply cannot adjust unspecified {name}."
        );
        metric.map(|metric| self.transform(metric))
    }
}

impl Default for TextMetricAdjustment {
    fn default() -> Self {
        Self::new(1.0, 0.0)
    }
}

/// Replacements and relative metric changes for [TextStyle::apply_options].
/// Absent options keep the source value. An explicit empty slice replaces a
/// list rather than inheriting it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStyleApply<'a> {
    pub color: Option<Color>,
    pub background_color: Option<Color>,
    pub decoration: Option<TextDecoration>,
    pub decoration_color: Option<Color>,
    pub decoration_style: Option<TextDecorationStyle>,
    pub decoration_thickness: TextMetricAdjustment,
    pub font_family: Option<FontFamily>,
    /// Qualifies the effective primary and fallback names after replacements.
    pub font_package: Option<&'a str>,
    pub font_family_fallback: Option<&'a [FontFamily]>,
    pub font_size: TextMetricAdjustment,
    pub font_weight_delta: i32,
    pub font_style: Option<FontStyle>,
    pub letter_spacing: TextMetricAdjustment,
    pub word_spacing: TextMetricAdjustment,
    pub height: TextMetricAdjustment,
    pub text_baseline: Option<TextBaseline>,
    pub leading_distribution: Option<TextLeadingDistribution>,
    pub locale: Option<&'a str>,
    pub shadows: Option<&'a [TextShadow]>,
    pub font_features: Option<&'a [FontFeature]>,
    pub font_variations: Option<&'a [FontVariation]>,
    pub overflow: Option<TextOverflow>,
}

fn flutter_height(height: f32) -> LineHeight {
    if height == 0.0 {
        LineHeight::Normal
    } else {
        LineHeight::Multiplier(height)
    }
}

/// An immutable, Flutter-style text style description.
#[derive(Clone, Debug)]
pub struct TextStyle {
    pub inherit: bool,
    pub has_explicit_color: bool,
    pub has_explicit_size: bool,
    pub has_explicit_weight: bool,
    pub has_explicit_style: bool,
    pub has_explicit_family: bool,
    /// Distinguishes an empty override from unspecified inherited fallbacks.
    pub has_explicit_fallback_families: bool,
    /// Distinguishes an explicit zero from unspecified inherited spacing.
    pub has_explicit_letter_spacing: bool,
    /// Distinguishes an empty shadow override from an unset list.
    pub has_explicit_shadows: bool,
    pub family: FontFamily,
    pub fallback_families: Arc<[FontFamily]>,
    pub size: f32,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub color: Color,
    pub background_color: Option<Color>,
    pub line_height: Option<LineHeight>,
    pub letter_spacing: f32,
    pub word_spacing: Option<f32>,
    pub text_baseline: Option<TextBaseline>,
    pub leading_distribution: Option<TextLeadingDistribution>,
    pub locale: Option<String>,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub shadows: Arc<[TextShadow]>,
    pub font_variations: Option<Arc<str>>,
    pub font_features: Option<Arc<str>>,
    pub decoration: Option<TextDecoration>,
    pub decoration_color: Option<Color>,
    pub decoration_style: Option<TextDecorationStyle>,
    pub decoration_thickness: Option<f32>,
    /// Diagnostic provenance only, never used to compare or invalidate style.
    pub debug_label: Option<String>,
    pub overflow: Option<TextOverflow>,
}

impl PartialEq for TextStyle {
    fn eq(&self, other: &Self) -> bool {
        self.inherit == other.inherit
            && self.has_explicit_color == other.has_explicit_color
            && self.has_explicit_size == other.has_explicit_size
            && self.has_explicit_weight == other.has_explicit_weight
            && self.has_explicit_style == other.has_explicit_style
            && self.has_explicit_family == other.has_explicit_family
            && self.has_explicit_fallback_families == other.has_explicit_fallback_families
            && self.has_explicit_letter_spacing == other.has_explicit_letter_spacing
            && self.has_explicit_shadows == other.has_explicit_shadows
            && self.family == other.family
            && self.fallback_families == other.fallback_families
            && self.size == other.size
            && self.weight == other.weight
            && self.style == other.style
            && self.color == other.color
            && self.background_color == other.background_color
            && self.line_height == other.line_height
            && self.letter_spacing == other.letter_spacing
            && self.word_spacing == other.word_spacing
            && self.text_baseline == other.text_baseline
            && self.leading_distribution == other.leading_distribution
            && self.locale == other.locale
            && self.foreground == other.foreground
            && self.background == other.background
            && self.shadows == other.shadows
            && self.font_variations == other.font_variations
            && self.font_features == other.font_features
            && self.decoration == other.decoration
            && self.decoration_color == other.decoration_color
            && self.decoration_style == other.decoration_style
            && self.decoration_thickness == other.decoration_thickness
            && self.overflow == other.overflow
    }
}

impl Hash for TextStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // IEEE equality identifies signed zeros, so hash them alike. All
        // remaining floats keep their bits; NaN is not equal to any value.
        let number = |value: f32| if value == 0. { 0 } else { value.to_bits() };
        self.inherit.hash(state);
        self.has_explicit_color.hash(state);
        self.has_explicit_size.hash(state);
        self.has_explicit_weight.hash(state);
        self.has_explicit_style.hash(state);
        self.has_explicit_family.hash(state);
        self.has_explicit_fallback_families.hash(state);
        self.has_explicit_letter_spacing.hash(state);
        self.has_explicit_shadows.hash(state);
        self.family.hash(state);
        self.fallback_families.hash(state);
        number(self.size).hash(state);
        self.weight.hash(state);
        self.style.hash(state);
        self.color.hash(state);
        self.background_color.hash(state);
        self.line_height
            .map(|height| match height {
                LineHeight::Normal => (0_u8, 0),
                LineHeight::Multiplier(value) => (1, number(value)),
                LineHeight::Absolute(value) => (2, number(value)),
            })
            .hash(state);
        number(self.letter_spacing).hash(state);
        self.word_spacing.map(number).hash(state);
        self.text_baseline.hash(state);
        self.leading_distribution.hash(state);
        self.locale.hash(state);
        self.foreground.hash(state);
        self.background.hash(state);
        self.shadows.len().hash(state);
        for shadow in self.shadows.iter() {
            (
                shadow.color,
                number(shadow.offset.x),
                number(shadow.offset.y),
                number(shadow.blur_radius),
            )
                .hash(state);
        }
        self.font_variations.hash(state);
        self.font_features.hash(state);
        self.decoration.hash(state);
        self.decoration_color.hash(state);
        self.decoration_style.hash(state);
        self.decoration_thickness.map(number).hash(state);
        self.overflow.hash(state);
    }
}

impl TextStyle {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn inherit(mut self, inherit: bool) -> Self {
        self.inherit = inherit;
        self
    }

    #[must_use]
    pub fn font_family(mut self, family: impl Into<String>) -> Self {
        self.family = FontFamily::named(family);
        self.has_explicit_family = true;
        self
    }

    #[must_use]
    pub fn family(mut self, family: FontFamily) -> Self {
        self.family = family;
        self.has_explicit_family = true;
        self
    }

    #[must_use]
    pub fn fallback_families<I, S>(mut self, families: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.fallback_families = families.into_iter().map(FontFamily::named).collect();
        self.has_explicit_fallback_families = true;
        self
    }

    /// Uses the package-qualified font family as the primary candidate.
    #[must_use]
    pub fn packaged_font_family(self, package: &str, family: &str) -> Self {
        self.family(FontFamily::packaged(package, family))
    }

    /// Sets the qualifier of the specified primary and fallback names.
    /// An existing canonical package prefix is replaced, not duplicated.
    /// An omitted family or fallback list remains omitted.
    #[must_use]
    pub fn font_package(mut self, package: &str) -> Self {
        let qualify = |family: &FontFamily| {
            let canonical = family.name();
            let name = canonical
                .strip_prefix("packages/")
                .and_then(|remaining| remaining.split_once('/'))
                .map_or(canonical, |(_, name)| name);
            FontFamily::packaged(package, name)
        };
        if self.has_explicit_family {
            self.family = qualify(&self.family);
        }
        if !self.fallback_families.is_empty() {
            self.fallback_families = self.fallback_families.iter().map(qualify).collect();
        }
        self
    }

    /// Replaces the fallback list with ordered names from the package.
    /// An empty list still overrides inherited fallbacks.
    #[must_use]
    pub fn packaged_fallback_families(
        mut self,
        package: &str,
        families: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        self.fallback_families = families
            .into_iter()
            .map(|name| FontFamily::packaged(package, name.as_ref()))
            .collect();
        self.has_explicit_fallback_families = true;
        self
    }

    #[must_use]
    pub fn fallback_family(mut self, family: FontFamily) -> Self {
        let mut families = self.fallback_families.to_vec();
        families.push(family);
        self.fallback_families = families.into();
        self.has_explicit_fallback_families = true;
        self
    }

    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.size = size.max(0.0);
        self.has_explicit_size = true;
        self
    }

    #[must_use]
    pub fn size(self, size: f32) -> Self {
        self.font_size(size)
    }

    #[must_use]
    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self.has_explicit_weight = true;
        self
    }

    #[must_use]
    pub fn bold(self) -> Self {
        self.font_weight(FontWeight::BOLD)
    }

    #[must_use]
    pub fn font_style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self.has_explicit_style = true;
        self
    }

    #[must_use]
    pub fn italic(self) -> Self {
        self.font_style(FontStyle::Italic)
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self.foreground = None;
        self.has_explicit_color = true;
        self
    }

    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self.background = None;
        self
    }

    #[must_use]
    pub fn line_height(mut self, line_height: Option<f32>) -> Self {
        self.line_height = line_height.map(|value| {
            debug_assert!(!value.is_nan(), "TextStyle.height must not be NaN.");
            if value <= 4.0 {
                LineHeight::Multiplier(value.max(0.0))
            } else {
                LineHeight::Absolute(value.max(0.0))
            }
        });
        self
    }

    /// Flutter's proportional text height. Zero selects the natural font
    /// metrics; use [Self::line_height_absolute] for a logical pixel height.
    #[must_use]
    pub fn height(mut self, height: Option<f32>) -> Self {
        self.line_height = height.map(|height| {
            debug_assert!(!height.is_nan(), "TextStyle.height must not be NaN.");
            flutter_height(height.max(0.0))
        });
        self
    }

    #[must_use]
    pub fn line_height_multiplier(mut self, factor: f32) -> Self {
        self.line_height = Some(LineHeight::Multiplier(factor.max(0.0)));
        self
    }

    #[must_use]
    pub fn line_height_absolute(mut self, pixels: f32) -> Self {
        self.line_height = Some(LineHeight::Absolute(pixels.max(0.0)));
        self
    }

    #[must_use]
    pub fn line_height_mode(mut self, mode: LineHeight) -> Self {
        self.line_height = Some(mode);
        self
    }

    #[must_use]
    pub fn letter_spacing(mut self, letter_spacing: f32) -> Self {
        self.letter_spacing = letter_spacing;
        self.has_explicit_letter_spacing = true;
        self
    }

    #[must_use]
    pub fn word_spacing(mut self, word_spacing: Option<f32>) -> Self {
        self.word_spacing = word_spacing;
        self
    }

    #[must_use]
    pub fn text_baseline(mut self, baseline: TextBaseline) -> Self {
        self.text_baseline = Some(baseline);
        self
    }

    #[must_use]
    pub fn leading_distribution(mut self, distribution: TextLeadingDistribution) -> Self {
        self.leading_distribution = Some(distribution);
        self
    }

    #[must_use]
    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = Some(locale.into());
        self
    }

    #[must_use]
    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self.has_explicit_color = false;
        self
    }

    /// The solid foreground supplied to the renderer, including paint
    /// overrides. A foreground paint takes precedence over a plain color.
    #[must_use]
    pub fn effective_foreground_color(&self) -> Color {
        self.foreground.unwrap_or(self.color)
    }

    #[must_use]
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self.background_color = None;
        self
    }

    /// The solid color used to paint beneath each styled glyph run.
    #[must_use]
    pub fn effective_background_color(&self) -> Option<Color> {
        self.background.or(self.background_color)
    }

    #[must_use]
    pub fn shadows<I>(mut self, shadows: I) -> Self
    where
        I: IntoIterator<Item = TextShadow>,
    {
        self.shadows = shadows.into_iter().collect::<Vec<_>>().into();
        self.has_explicit_shadows = true;
        self
    }

    #[must_use]
    pub fn shadow(mut self, shadow: TextShadow) -> Self {
        let mut shadows = self.shadows.to_vec();
        shadows.push(shadow);
        self.shadows = shadows.into();
        self.has_explicit_shadows = true;
        self
    }

    #[must_use]
    pub fn font_variations(mut self, settings: impl Into<Arc<str>>) -> Self {
        self.font_variations = Some(settings.into());
        self
    }

    #[must_use]
    pub fn font_variation_settings(mut self, variations: &[FontVariation]) -> Self {
        self.font_variations = Some(format_font_variations(variations));
        self
    }

    #[must_use]
    pub fn font_features(mut self, settings: impl Into<Arc<str>>) -> Self {
        self.font_features = Some(settings.into());
        self
    }

    #[must_use]
    pub fn font_feature_settings(mut self, features: &[FontFeature]) -> Self {
        let formatted = features
            .iter()
            .map(FontFeature::to_css_setting)
            .collect::<Vec<_>>()
            .join(", ");
        self.font_features = Some(formatted.into());
        self
    }

    #[must_use]
    pub fn decoration(mut self, decoration: TextDecoration) -> Self {
        self.decoration = Some(decoration);
        self
    }

    #[must_use]
    pub fn decoration_color(mut self, color: Color) -> Self {
        self.decoration_color = Some(color);
        self
    }

    #[must_use]
    pub fn decoration_style(mut self, style: TextDecorationStyle) -> Self {
        self.decoration_style = Some(style);
        self
    }

    #[must_use]
    pub fn decoration_thickness(mut self, thickness: f32) -> Self {
        self.decoration_thickness = Some(thickness.max(0.0));
        self
    }

    #[must_use]
    pub fn debug_label(mut self, label: impl Into<String>) -> Self {
        self.debug_label = Some(label.into());
        self
    }

    #[must_use]
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = Some(overflow);
        self
    }

    // Immutable derivation with_* methods
    #[must_use]
    pub fn with_font_size(&self, size: f32) -> Self {
        let mut s = self.clone();
        s.size = size.max(0.0);
        s.has_explicit_size = true;
        s
    }

    #[must_use]
    pub fn with_font_weight(&self, weight: FontWeight) -> Self {
        let mut s = self.clone();
        s.weight = weight;
        s.has_explicit_weight = true;
        s
    }

    #[must_use]
    pub fn with_color(&self, color: Color) -> Self {
        let mut s = self.clone();
        if s.foreground.is_none() {
            s.color = color;
            s.has_explicit_color = true;
        }
        s
    }

    #[must_use]
    pub fn with_letter_spacing(&self, letter_spacing: f32) -> Self {
        self.clone().letter_spacing(letter_spacing)
    }

    #[must_use]
    pub fn with_line_height(&self, line_height: Option<f32>) -> Self {
        self.clone().line_height(line_height)
    }

    #[must_use]
    pub fn with_decoration(&self, decoration: Option<TextDecoration>) -> Self {
        let mut s = self.clone();
        s.decoration = decoration;
        s
    }

    /// Copies the style and applies the supplied builder updates. An explicit
    /// debug label in the update supersedes the automatically derived label.
    #[must_use]
    pub fn copy_with(&self, update: impl FnOnce(Self) -> Self) -> Self {
        let mut result = self.clone();
        result.debug_label = diagnostic_operation(&self.debug_label, "copyWith");
        update(result)
    }

    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        if !other.inherit {
            return other.clone();
        }
        let mut merged = self.clone();
        if other.has_explicit_family {
            merged.family = other.family.clone();
            merged.has_explicit_family = true;
        }
        if other.has_explicit_fallback_families || !other.fallback_families.is_empty() {
            merged.fallback_families = other.fallback_families.clone();
            merged.has_explicit_fallback_families = true;
        }
        if other.has_explicit_size {
            merged.size = other.size;
            merged.has_explicit_size = true;
        }
        if other.has_explicit_weight {
            merged.weight = other.weight;
            merged.has_explicit_weight = true;
        }
        if other.has_explicit_style {
            merged.style = other.style;
            merged.has_explicit_style = true;
        }
        if let Some(foreground) = other.foreground {
            merged.foreground = Some(foreground);
            merged.has_explicit_color = false;
        } else if merged.foreground.is_none() && other.has_explicit_color {
            merged.color = other.color;
            merged.has_explicit_color = true;
        }
        if let Some(background) = other.background {
            merged.background = Some(background);
            merged.background_color = None;
        } else if merged.background.is_none() && other.background_color.is_some() {
            merged.background_color = other.background_color;
        }
        if other.line_height.is_some() {
            merged.line_height = other.line_height;
        }
        if other.has_explicit_letter_spacing || other.letter_spacing != 0.0 {
            merged.letter_spacing = other.letter_spacing;
            merged.has_explicit_letter_spacing = true;
        }
        if other.word_spacing.is_some() {
            merged.word_spacing = other.word_spacing;
        }
        if other.text_baseline.is_some() {
            merged.text_baseline = other.text_baseline;
        }
        if other.leading_distribution.is_some() {
            merged.leading_distribution = other.leading_distribution;
        }
        if other.locale.is_some() {
            merged.locale = other.locale.clone();
        }


        if other.has_explicit_shadows || !other.shadows.is_empty() {
            merged.shadows = other.shadows.clone();
            merged.has_explicit_shadows = true;
        }
        if other.font_variations.is_some() {
            merged.font_variations = other.font_variations.clone();
        }
        if other.font_features.is_some() {
            merged.font_features = other.font_features.clone();
        }
        if other.decoration.is_some() {
            merged.decoration = other.decoration;
        }
        if other.decoration_color.is_some() {
            merged.decoration_color = other.decoration_color;
        }
        if other.decoration_style.is_some() {
            merged.decoration_style = other.decoration_style;
        }
        if other.decoration_thickness.is_some() {
            merged.decoration_thickness = other.decoration_thickness;
        }
        if other.overflow.is_some() {
            merged.overflow = other.overflow;
        }
        merged.debug_label = (cfg!(debug_assertions)
            && (self.debug_label.is_some() || other.debug_label.is_some()))
        .then(|| {
            format!(
                "({}).merge({})",
                self.debug_label.as_deref().unwrap_or("unknown"),
                other.debug_label.as_deref().unwrap_or("unknown")
            )
        });
        merged
    }

    #[must_use]
    pub fn apply(
        &self,
        color: Option<Color>,
        background_color: Option<Color>,
        font_size_factor: Option<f32>,
        font_size_delta: Option<f32>,
        letter_spacing_factor: Option<f32>,
        letter_spacing_delta: Option<f32>,
    ) -> Self {
        self.apply_options(TextStyleApply {
            color,
            background_color,
            font_size: TextMetricAdjustment::new(
                font_size_factor.unwrap_or(1.0),
                font_size_delta.unwrap_or(0.0),
            ),
            letter_spacing: TextMetricAdjustment::new(
                letter_spacing_factor.unwrap_or(1.0),
                letter_spacing_delta.unwrap_or(0.0),
            ),
            ..TextStyleApply::default()
        })
    }

    /// Applies relative adjustments before replacing the supplied descriptors.
    /// Existing foreground/background paint still wins over a plain color.
    #[must_use]
    pub fn apply_options(&self, options: TextStyleApply<'_>) -> Self {
        let mut result = self.clone();
        if let Some(color) = options.color
            && result.foreground.is_none()
        {
            result.color = color;
            result.has_explicit_color = true;
        }
        if let Some(color) = options.background_color
            && result.background.is_none()
        {
            result.background_color = Some(color);
        }
        if let Some(size) = options.font_size.specified(
            (self.has_explicit_size || self.size != WidgetDefaults::DEFAULT.text_size)
                .then_some(self.size),
            "fontSize",
        ) {
            result.size = size;
            result.has_explicit_size = true;
        }
        if let Some(spacing) = options.letter_spacing.specified(
            (self.has_explicit_letter_spacing || self.letter_spacing != 0.0)
                .then_some(self.letter_spacing),
            "letterSpacing",
        ) {
            result.letter_spacing = spacing;
            result.has_explicit_letter_spacing = true;
        }
        if self.has_explicit_weight || self.weight != FontWeight::NORMAL {
            let index = (i32::from(self.weight.value()) / 100 - 1).clamp(0, 8);
            result.weight = FontWeight::new(
                ((index.saturating_add(options.font_weight_delta).clamp(0, 8) + 1) * 100) as u16,
            );
            result.has_explicit_weight = true;
        } else {
            debug_assert!(
                options.font_weight_delta == 0,
                "TextStyle.apply cannot adjust unspecified fontWeight."
            );
        }
        result.word_spacing = options.word_spacing.specified(self.word_spacing, "wordSpacing");
        result.decoration_thickness = options
            .decoration_thickness
            .specified(self.decoration_thickness, "decorationThickness");
        result.line_height = self.line_height.map(|height| match height {
            LineHeight::Normal | LineHeight::Multiplier(0.0) => height,
            LineHeight::Multiplier(factor) => flutter_height(options.height.transform(factor)),
            LineHeight::Absolute(pixels) => LineHeight::Absolute(options.height.transform(pixels)),
        });
        if let Some(family) = options.font_family {
            result = result.family(family);
        }
        if let Some(fallback) = options.font_family_fallback {
            result.fallback_families = fallback.into();
            result.has_explicit_fallback_families = true;
        }
        if let Some(package) = options.font_package {
            result = result.font_package(package);
        }
        if let Some(style) = options.font_style {
            result = result.font_style(style);
        }
        result.decoration = options.decoration.or(self.decoration);
        result.decoration_color = options.decoration_color.or(self.decoration_color);
        result.decoration_style = options.decoration_style.or(self.decoration_style);
        result.text_baseline = options.text_baseline.or(self.text_baseline);
        result.leading_distribution = options.leading_distribution.or(self.leading_distribution);
        result.overflow = options.overflow.or(self.overflow);
        if let Some(locale) = options.locale {
            result.locale = Some(locale.into());
        }
        if let Some(shadows) = options.shadows {
            result.shadows = shadows.into();
            result.has_explicit_shadows = true;
        }
        if let Some(features) = options.font_features {
            result = result.font_feature_settings(features);
        }
        if let Some(variations) = options.font_variations {
            result = result.font_variation_settings(variations);
        }
        result.debug_label = diagnostic_operation(&self.debug_label, "apply");
        result
    }

    #[must_use]
    pub fn scaled(&self, scaler: TextScaler) -> Self {
        let mut scaled = self.clone();
        scaled.size = scaler.scale(self.size);
        scaled
    }

    #[must_use]
    pub fn change_impact(&self, other: &Self) -> ChangeImpact {
        if self == other {
            return ChangeImpact::None;
        }
        if self.family != other.family
            || self.fallback_families != other.fallback_families
            || self.size != other.size
            || self.weight != other.weight
            || self.style != other.style
            || self.line_height != other.line_height
            || self.letter_spacing != other.letter_spacing
            || self.word_spacing != other.word_spacing
            || self.text_baseline != other.text_baseline
            || self.leading_distribution != other.leading_distribution
            || self.font_variations != other.font_variations
            || self.font_features != other.font_features
            || self.overflow != other.overflow
            || self.effective_background_color().is_some()
                != other.effective_background_color().is_some()
        {
            ChangeImpact::Layout
        } else {
            ChangeImpact::Paint
        }
    }

    #[must_use]
    pub fn invalidation(&self, other: &Self) -> incular_core::Invalidation {
        if self == other {
            return incular_core::Invalidation::NONE;
        }
        if self.family != other.family
            || self.fallback_families != other.fallback_families
            || self.size != other.size
            || self.weight != other.weight
            || self.style != other.style
            || self.line_height != other.line_height
            || self.letter_spacing != other.letter_spacing
            || self.word_spacing != other.word_spacing
            || self.text_baseline != other.text_baseline
            || self.leading_distribution != other.leading_distribution
            || self.font_variations != other.font_variations
            || self.font_features != other.font_features
            || self.overflow != other.overflow
            || self.effective_background_color().is_some()
                != other.effective_background_color().is_some()
        {
            incular_core::Invalidation::LAYOUT | incular_core::Invalidation::PAINT
        } else {
            incular_core::Invalidation::PAINT
        }
    }
}

impl Lerp for TextStyle {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if std::ptr::eq(self, other) {
            return self.clone();
        }
        // Use the original progress in the provenance even when the
        // presentation side of interpolation clamps its value.
        let debug_label = cfg!(debug_assertions).then(|| {
            format!(
                "lerp({} ⎯{t:.1}→ {})",
                self.debug_label.as_deref().unwrap_or("unknown"),
                other.debug_label.as_deref().unwrap_or("unknown")
            )
        });
        let font_variations = lerp_font_variation_settings(
            self.font_variations.as_ref(),
            other.font_variations.as_ref(),
            t,
        );
        let mix = |left: f32, right: f32| left.lerp(&right, t);
        let mix_optional = |left: Option<f32>, right: Option<f32>| match (left, right) {
            (Some(left), Some(right)) => Some(mix(left, right)),
            (left, right) => left.or(right),
        };
        let t = t.clamp(0.0, 1.0);
        let endpoint = if t < 0.5 { self } else { other };
        let size = |style: &Self| {
            // Direct struct initializers are also supported by Incular.
            (style.has_explicit_size || style.size != WidgetDefaults::DEFAULT.text_size)
                .then_some(style.size)
        };
        let spacing = |style: &Self| {
            (style.has_explicit_letter_spacing || style.letter_spacing != 0.0)
                .then_some(style.letter_spacing)
        };
        let plain_color = |style: &Self| {
            (style.has_explicit_color || style.color != Color::WHITE).then_some(style.color)
        };
        // Flutter switches Paint objects at the midpoint. When mixing paint
        // and simple color, the simple endpoint is promoted to solid paint.
        let foreground = (self.foreground.is_some() || other.foreground.is_some())
            .then(|| endpoint.effective_foreground_color());
        let uses_background_paint = self.background.is_some() || other.background.is_some();
        let background = uses_background_paint
            .then(|| endpoint.effective_background_color())
            .flatten();
        let fade = |color: Color, amount: f32| Color {
            alpha: (f32::from(color.alpha) * amount).round() as u8,
            ..color
        };
        let mix_color = |left: Option<Color>, right: Option<Color>| match (left, right) {
            (Some(left), Some(right)) => Some(left.lerp(&right, t)),
            (Some(left), None) => Some(fade(left, 1.0 - t)),
            (None, Some(right)) => Some(fade(right, t)),
            (None, None) => None,
        };
        let color = foreground
            .is_none()
            .then(|| mix_color(plain_color(self), plain_color(other)))
            .flatten();
        let size = mix_optional(size(self), size(other));
        let letter_spacing = mix_optional(spacing(self), spacing(other));
        #[cfg(debug_assertions)]
        if self.inherit != other.inherit {
            let fields = [
                ("color", foreground.is_none() && color.is_none()),
                (
                    "backgroundColor",
                    !uses_background_paint
                        && self.background_color.is_none()
                        && other.background_color.is_none(),
                ),
                ("fontSize", size.is_none()),
                ("letterSpacing", letter_spacing.is_none()),
                (
                    "wordSpacing",
                    self.word_spacing.is_none() && other.word_spacing.is_none(),
                ),
                (
                    "height",
                    self.line_height.is_none() && other.line_height.is_none(),
                ),
                (
                    "decorationColor",
                    self.decoration_color.is_none() && other.decoration_color.is_none(),
                ),
                (
                    "decorationThickness",
                    self.decoration_thickness.is_none() && other.decoration_thickness.is_none(),
                ),
            ];
            if fields.iter().any(|&(_, missing)| missing) {
                let unspecified: Vec<_> = fields
                    .into_iter()
                    .filter_map(|(name, missing)| missing.then_some(name))
                    .collect();
                panic!(
                    "Failed to interpolate TextStyles with different inherit values. \
                     Unspecified in both TextStyles: {}.",
                    unspecified.join(", "),
                );
            }
        }
        Self {
            inherit: endpoint.inherit,
            has_explicit_color: foreground.is_none() && color.is_some(),
            has_explicit_size: size.is_some(),
            has_explicit_weight: self.has_explicit_weight || other.has_explicit_weight,
            has_explicit_style: endpoint.has_explicit_style,
            has_explicit_family: endpoint.has_explicit_family,
            has_explicit_fallback_families: endpoint.has_explicit_fallback_families
                || !endpoint.fallback_families.is_empty(),
            has_explicit_letter_spacing: letter_spacing.is_some(),
            has_explicit_shadows: self.has_explicit_shadows
                || other.has_explicit_shadows
                || !self.shadows.is_empty()
                || !other.shadows.is_empty(),
            family: endpoint.family.clone(),
            fallback_families: endpoint.fallback_families.clone(),
            size: size.unwrap_or(WidgetDefaults::DEFAULT.text_size),
            weight: self.weight.lerp(&other.weight, t),
            style: endpoint.style,
            color: foreground.or(color).unwrap_or(Color::WHITE),
            background_color: if uses_background_paint {
                None
            } else {
                mix_color(self.background_color, other.background_color)
            },
            line_height: match (self.line_height, other.line_height) {
                (Some(LineHeight::Multiplier(a)), Some(LineHeight::Multiplier(b))) => {
                    Some(flutter_height(mix(a, b)))
                }
                (Some(LineHeight::Normal), Some(LineHeight::Multiplier(b))) => {
                    Some(flutter_height(mix(0.0, b)))
                }
                (Some(LineHeight::Multiplier(a)), Some(LineHeight::Normal)) => {
                    Some(flutter_height(mix(a, 0.0)))
                }
                (Some(LineHeight::Absolute(a)), Some(LineHeight::Absolute(b))) => {
                    Some(LineHeight::Absolute(mix(a, b)))
                }
                (Some(a), Some(b)) => {
                    if t < 0.5 {
                        Some(a)
                    } else {
                        Some(b)
                    }
                }
                (a, b) => a.or(b),
            },
            letter_spacing: letter_spacing.unwrap_or(0.0),
            word_spacing: mix_optional(self.word_spacing, other.word_spacing),
            text_baseline: if t < 0.5 {
                self.text_baseline
            } else {
                other.text_baseline
            },
            leading_distribution: if t < 0.5 {
                self.leading_distribution
            } else {
                other.leading_distribution
            },
            locale: if t < 0.5 {
                self.locale.clone()
            } else {
                other.locale.clone()
            },
            foreground,
            background,
            shadows: if self.shadows == other.shadows {
                self.shadows.clone()
            } else {
                TextShadow::lerp_list(&self.shadows, &other.shadows, t).into()
            },
            font_variations,
            font_features: if t < 0.5 {
                self.font_features.clone()
            } else {
                other.font_features.clone()
            },
            decoration: if t < 0.5 {
                self.decoration
            } else {
                other.decoration
            },
            decoration_color: mix_color(self.decoration_color, other.decoration_color),
            decoration_style: if t < 0.5 {
                self.decoration_style
            } else {
                other.decoration_style
            },
            decoration_thickness: mix_optional(
                self.decoration_thickness,
                other.decoration_thickness,
            ),
            debug_label,
            overflow: if t < 0.5 {
                self.overflow
            } else {
                other.overflow
            },
        }
    }
}

fn diagnostic_operation(label: &Option<String>, operation: &str) -> Option<String> {
    label
        .as_deref()
        .filter(|_| cfg!(debug_assertions))
        .map(|label| format!("({label}).{operation}"))
}

impl fmt::Display for TextStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = Vec::new();
        if self.has_explicit_color {
            properties.push(format!("color: {:?}", self.color));
        }
        if let Some(color) = self.background_color {
            properties.push(format!("backgroundColor: {color:?}"));
        }
        if self.has_explicit_family {
            properties.push(format!("family: {}", self.family.name()));
        }
        if self.has_explicit_fallback_families || !self.fallback_families.is_empty() {
            let names: Vec<_> = self
                .fallback_families
                .iter()
                .map(FontFamily::name)
                .collect();
            properties.push(format!("familyFallback: {}", names.join(", ")));
        }
        if self.has_explicit_size {
            properties.push(format!("size: {:?}", self.size));
        }
        if self.has_explicit_weight {
            properties.push(format!("weight: {}", self.weight.0));
        }
        if self.has_explicit_style {
            properties.push(format!("style: {:?}", self.style));
        }
        if self.has_explicit_letter_spacing {
            properties.push(format!("letterSpacing: {:?}", self.letter_spacing));
        }
        if let Some(spacing) = self.word_spacing {
            properties.push(format!("wordSpacing: {spacing:?}"));
        }
        if let Some(baseline) = self.text_baseline {
            properties.push(format!("baseline: {baseline:?}"));
        }
        if let Some(height) = self.line_height {
            properties.push(format!("height: {height:?}"));
        }
        if let Some(distribution) = self.leading_distribution {
            properties.push(format!("leadingDistribution: {distribution:?}"));
        }
        if let Some(locale) = &self.locale {
            properties.push(format!("locale: {locale}"));
        }
        if let Some(foreground) = self.foreground {
            properties.push(format!("foreground: {foreground:?}"));
        }
        if let Some(background) = self.background {
            properties.push(format!("background: {background:?}"));
        }
        if let Some(decoration) = self.decoration {
            properties.push(format!("decoration: {decoration:?}"));
        }

        write!(f, "TextStyle(")?;
        if let Some(label) = &self.debug_label {
            write!(f, "debugLabel: {label}, ")?;
        }
        if !properties.is_empty() || !self.inherit {
            write!(f, "inherit: {}, ", self.inherit)?;
        }
        if properties.is_empty() {
            f.write_str(if self.inherit {
                "<all styles inherited>"
            } else {
                "<no style specified>"
            })?;
        } else {
            f.write_str(&properties.join(", "))?;
        }
        f.write_str(")")
    }
}

/// Shared empty lists so default style construction allocates nothing.
static EMPTY_FALLBACK_FAMILIES: LazyLock<Arc<[FontFamily]>> = LazyLock::new(|| Arc::new([]));
static EMPTY_SHADOWS: LazyLock<Arc<[TextShadow]>> = LazyLock::new(|| Arc::new([]));

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            inherit: true,
            has_explicit_color: false,
            has_explicit_size: false,
            has_explicit_weight: false,
            has_explicit_style: false,
            has_explicit_family: false,
            has_explicit_fallback_families: false,
            has_explicit_letter_spacing: false,
            has_explicit_shadows: false,
            family: FontFamily::SystemUi,
            fallback_families: EMPTY_FALLBACK_FAMILIES.clone(),
            size: WidgetDefaults::DEFAULT.text_size,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
            color: Color::WHITE,
            background_color: None,
            line_height: None,
            letter_spacing: 0.0,
            word_spacing: None,
            text_baseline: None,
            leading_distribution: None,
            locale: None,
            foreground: None,
            background: None,
            shadows: EMPTY_SHADOWS.clone(),
            font_variations: None,
            font_features: None,
            decoration: None,
            decoration_color: None,
            decoration_style: None,
            decoration_thickness: None,
            debug_label: None,
            overflow: None,
        }
    }
}
