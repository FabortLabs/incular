//! An ordered palette of related encoded sRGB shades.

use std::{
    borrow::{Borrow, Cow},
    hash::{DefaultHasher, Hash, Hasher},
};

use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::Color;

/// A primary color with a small insertion-ordered table of named shades.
///
/// Lookup and equality use the keys and colors; insertion order is kept
/// for iteration. This is independent of any widget design system.
#[derive(Clone, Debug)]
pub struct ColorSwatch<K: Eq + Hash> {
    primary: Color,
    shades: IndexMap<K, Color>,
}

impl<K: Eq + Hash> ColorSwatch<K> {
    #[must_use]
    pub fn new(primary: Color, shades: impl IntoIterator<Item = (K, Color)>) -> Self {
        Self {
            primary,
            shades: shades.into_iter().collect(),
        }
    }

    #[must_use]
    pub const fn primary(&self) -> Color {
        self.primary
    }

    /// Returns the encoded primary color in ARGB byte order.
    #[must_use]
    pub const fn to_argb32(&self) -> u32 {
        self.primary.to_argb32()
    }

    pub fn keys(&self) -> impl ExactSizeIterator<Item = &K> {
        self.shades.keys()
    }

    #[must_use]
    pub fn get<Q: Eq + Hash + ?Sized>(&self, key: &Q) -> Option<Color>
    where
        K: Borrow<Q>,
    {
        self.shades.get(key).copied()
    }

    /// Interpolates the primary and its shades with independent byte clamps.
    ///
    /// If both swatches exist, the left-hand keys and their order are kept.
    /// A shade with no target fades its alpha without changing its RGB.
    /// If the whole left swatch is absent, the right-hand keys fade in.
    /// Two references to the same swatch return a borrow with no allocation.
    #[must_use]
    pub fn lerp<'a>(a: Option<&'a Self>, b: Option<&'a Self>, t: f64) -> Option<Cow<'a, Self>>
    where
        K: Clone,
    {
        let result = match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(Cow::Borrowed(a)),
            (Some(a), Some(b)) => Self::new(
                mix(a.primary, b.primary, t),
                a.shades.iter().map(|(key, &color)| {
                    let shade = match b.get(key) {
                        Some(target) => mix(color, target, t),
                        None => scale_alpha(color, 1.0 - t),
                    };
                    (key.clone(), shade)
                }),
            ),
            (Some(swatch), None) => swatch.fade(1.0 - t),
            (None, Some(swatch)) => swatch.fade(t),
        };
        Some(Cow::Owned(result))
    }

    fn fade(&self, factor: f64) -> Self
    where
        K: Clone,
    {
        Self::new(
            scale_alpha(self.primary, factor),
            self.shades
                .iter()
                .map(|(key, &shade)| (key.clone(), scale_alpha(shade, factor))),
        )
    }
}

impl<K: Eq + Hash> PartialEq for ColorSwatch<K> {
    fn eq(&self, other: &Self) -> bool {
        self.primary == other.primary && self.shades == other.shades
    }
}

impl<K: Eq + Hash> Eq for ColorSwatch<K> {}

impl<K: Eq + Hash> Hash for ColorSwatch<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.primary.hash(state);
        // Equality ignores insertion order. Combine sorted entry hashes
        // rather than requiring an ordering for the keys themselves.
        let mut entries = SmallVec::<[u64; 16]>::with_capacity(self.shades.len());
        for pair in &self.shades {
            let mut hasher = DefaultHasher::new();
            pair.hash(&mut hasher);
            entries.push(hasher.finish());
        }
        entries.sort_unstable();
        entries.hash(state);
    }
}

fn mix(a: Color, b: Color, t: f64) -> Color {
    let channel = |a, b| {
        let a = f64::from(a) / 255.0;
        let b = f64::from(b) / 255.0;
        encoded_byte(a * (1.0 - t) + b * t)
    };
    Color::rgba(
        channel(a.red, b.red),
        channel(a.green, b.green),
        channel(a.blue, b.blue),
        channel(a.alpha, b.alpha),
    )
}

fn scale_alpha(color: Color, factor: f64) -> Color {
    Color {
        alpha: encoded_byte(f64::from(color.alpha) / 255.0 * factor),
        ..color
    }
}

fn encoded_byte(unit: f64) -> u8 {
    if unit.is_nan() {
        u8::MAX
    } else {
        (unit.clamp(0.0, 1.0) * 255.0).round() as u8
    }
}
