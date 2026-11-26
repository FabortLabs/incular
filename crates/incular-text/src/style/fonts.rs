use incular_core::Lerp;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

/// A logical font family. Named families are resolved by [`crate::TextEngine`];
/// their names are shared, so cloning a style never copies them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Cursive,
    SystemUi,
    Named(Arc<str>),
}

impl FontFamily {
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(name.into().into())
    }

    /// A font family supplied by a package, under Flutter's canonical key.
    #[must_use]
    pub fn packaged(package: &str, name: &str) -> Self {
        Self::named(format!("packages/{package}/{name}"))
    }

    /// The canonical string used by the shaping engine for this candidate.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::SansSerif => "sans-serif",
            Self::Serif => "serif",
            Self::Monospace => "monospace",
            Self::Cursive => "cursive",
            Self::SystemUi => "system-ui",
            Self::Named(name) => name,
        }
    }
}

/// Font weight values (1..1000).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const W100: Self = Self(100);
    pub const W200: Self = Self(200);
    pub const W300: Self = Self(300);
    pub const W400: Self = Self(400);
    pub const W500: Self = Self(500);
    pub const W600: Self = Self(600);
    pub const W700: Self = Self(700);
    pub const W800: Self = Self(800);
    pub const W900: Self = Self(900);

    pub const THIN: Self = Self::W100;
    pub const EXTRA_LIGHT: Self = Self::W200;
    pub const LIGHT: Self = Self::W300;
    pub const NORMAL: Self = Self::W400;
    pub const MEDIUM: Self = Self::W500;
    pub const SEMI_BOLD: Self = Self::W600;
    pub const BOLD: Self = Self::W700;
    pub const EXTRA_BOLD: Self = Self::W800;
    pub const BLACK: Self = Self::W900;

    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(if value < 1 {
            1
        } else if value > 1000 {
            1000
        } else {
            value
        })
    }

    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }
}

impl Lerp for FontWeight {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let left = self.0 as f32;
        let right = other.0 as f32;
        Self::new((left + (right - left) * t).round() as u16)
    }
}

/// OpenType font feature setting with a 4-byte ASCII tag and an integer value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontFeature {
    pub tag: [u8; 4],
    pub value: i32,
}

impl FontFeature {
    #[must_use]
    pub const fn new(tag: [u8; 4], value: i32) -> Self {
        Self { tag, value }
    }

    #[must_use]
    pub const fn enable(tag: [u8; 4]) -> Self {
        Self::new(tag, 1)
    }

    #[must_use]
    pub const fn disable(tag: [u8; 4]) -> Self {
        Self::new(tag, 0)
    }

    #[must_use]
    pub const fn alternative(value: i32) -> Self {
        Self::new(*b"aalt", value)
    }

    #[must_use]
    pub const fn contextual_alternates() -> Self {
        Self::enable(*b"calt")
    }

    #[must_use]
    pub const fn tabular_figures() -> Self {
        Self::enable(*b"tnum")
    }

    #[must_use]
    pub const fn proportional_figures() -> Self {
        Self::enable(*b"pnum")
    }

    #[must_use]
    pub const fn slashed_zero() -> Self {
        Self::enable(*b"zero")
    }

    #[must_use]
    pub fn to_css_setting(&self) -> String {
        let tag_str = std::str::from_utf8(&self.tag).unwrap_or("????");
        format!("\"{}\" {}", tag_str, self.value)
    }
}

/// OpenType font variation axis setting with a 4-byte ASCII tag and floating-point value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontVariation {
    pub axis: [u8; 4],
    pub value: f32,
}

impl FontVariation {
    #[must_use]
    pub const fn new(axis: [u8; 4], value: f32) -> Self {
        Self { axis, value }
    }

    #[must_use]
    pub const fn weight(value: f32) -> Self {
        Self::new(*b"wght", value)
    }

    #[must_use]
    pub const fn italic(value: f32) -> Self {
        Self::new(*b"ital", value)
    }

    #[must_use]
    pub const fn slant(value: f32) -> Self {
        Self::new(*b"slnt", value)
    }

    #[must_use]
    pub const fn width(value: f32) -> Self {
        Self::new(*b"wdth", value)
    }

    #[must_use]
    pub const fn optical_size(value: f32) -> Self {
        Self::new(*b"opsz", value)
    }

    #[must_use]
    pub fn to_css_setting(&self) -> String {
        let mut setting = String::new();
        self.write_css_setting(&mut setting);
        setting
    }

    fn write_css_setting(&self, setting: &mut String) {
        let axis = std::str::from_utf8(&self.axis).unwrap_or("????");
        write!(setting, "\"{axis}\" {:?}", self.value).expect("writing to a String cannot fail");
    }
}

impl Lerp for FontVariation {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if self.axis != other.axis {
            return if t < 0.5 { *self } else { *other };
        }
        // Flutter clamps to signed 16.16. Its positive maximum rounds
        // upward in f32, so use the greatest supported f32 below 32768.
        const MAX: f32 = f32::from_bits(32_768.0_f32.to_bits() - 1);
        let t = f64::from(t);
        let value = (f64::from(self.value) * (1.0 - t) + f64::from(other.value) * t) as f32;
        Self::new(self.axis, value.clamp(-32_768.0, MAX))
    }
}

/// Interpolates nullable OpenType axis lists, keeping the original lists at
/// the endpoints. Axes found on only one side switch at the midpoint.
#[must_use]
pub fn lerp_font_variations<'a>(
    a: Option<&'a [FontVariation]>,
    b: Option<&'a [FontVariation]>,
    t: f32,
) -> Option<Cow<'a, [FontVariation]>> {
    let endpoint = if t < 0.5 { a } else { b };
    let (Some(a), Some(b)) = (a, b) else {
        return endpoint.map(Cow::Borrowed);
    };
    if t == 0.0 || t == 1.0 || a.is_empty() || b.is_empty() {
        return endpoint.map(Cow::Borrowed);
    }

    let mut result = Vec::with_capacity(a.len().max(b.len()));
    for (left, right) in a.iter().zip(b) {
        if left.axis != right.axis {
            break;
        }
        result.push(left.lerp(right, t));
    }

    // Preserve matching positions, including duplicate tags in the prefix.
    // Within the unmatched tail, the last occurrence per side wins.
    let start = result.len();
    if start < a.len().max(b.len()) {
        let mut unmatched = BTreeMap::<_, (Option<_>, Option<_>)>::new();
        for &axis in &a[start..] {
            unmatched.entry(axis.axis).or_default().0 = Some(axis);
        }
        for &axis in &b[start..] {
            unmatched.entry(axis.axis).or_default().1 = Some(axis);
        }
        result.extend(
            unmatched
                .into_values()
                .filter_map(|(left, right)| match (left, right) {
                    (Some(left), Some(right)) => Some(left.lerp(&right, t)),
                    _ if t < 0.5 => left,
                    _ => right,
                }),
        );
    }
    Some(Cow::Owned(result))
}

/// Read ordinary quoted CSS axis settings without changing the public raw
/// settings representation. Unsupported strings retain discrete tweening.
fn parse_font_variations(mut source: &str) -> Option<Vec<FontVariation>> {
    let mut result = Vec::new();
    source = source.trim();
    if source.is_empty() {
        return Some(result);
    }
    loop {
        let bytes = source.as_bytes();
        let quote = *bytes.first()?;
        if !matches!(quote, b'\'' | b'"') || bytes.get(5) != Some(&quote) {
            return None;
        }
        let axis: [u8; 4] = bytes[1..5].try_into().ok()?;
        if axis
            .iter()
            .any(|&b| !(b' '..=b'~').contains(&b) || matches!(b, b'\'' | b'"' | b'\\'))
        {
            return None;
        }
        let remaining = &source[6..];
        let (value, next) = remaining
            .split_once(',')
            .map_or((remaining, None), |(value, next)| (value, Some(next)));
        let value: f32 = value.trim().parse().ok()?;
        if !value.is_finite() {
            return None;
        }
        result.push(FontVariation::new(axis, value));
        match next {
            Some(next) => source = next.trim_start(),
            None => return Some(result),
        }
    }
}

pub(super) fn format_font_variations(variations: &[FontVariation]) -> Arc<str> {
    let mut settings = String::new();
    for (index, variation) in variations.iter().enumerate() {
        if index != 0 {
            settings.push_str(", ");
        }
        variation.write_css_setting(&mut settings);
    }
    settings.into()
}

pub(super) fn lerp_font_variation_settings(
    a: Option<&Arc<str>>,
    b: Option<&Arc<str>>,
    t: f32,
) -> Option<Arc<str>> {
    let endpoint = if t < 0.5 { a } else { b };
    let (Some(a), Some(b)) = (a, b) else {
        return endpoint.cloned();
    };
    if a == b || t == 0.0 || t == 1.0 || a.is_empty() || b.is_empty() {
        return endpoint.cloned();
    }
    let (Some(left), Some(right)) = (parse_font_variations(a), parse_font_variations(b)) else {
        return endpoint.cloned();
    };
    if left.is_empty() || right.is_empty() {
        return endpoint.cloned();
    }
    lerp_font_variations(Some(&left), Some(&right), t).map(|axes| format_font_variations(&axes))
}
