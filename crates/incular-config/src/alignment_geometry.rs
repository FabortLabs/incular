//! Combined physical and direction-relative alignment.

use std::{
    fmt,
    hash::{Hash, Hasher},
    ops::{Add, Div, Mul, Neg, Rem, Sub},
};

use incular_core::Lerp;

use crate::{Alignment, AlignmentDirectional, TextDirection, hash_layout_floats};

/// A physical, directional, or combined alignment. The two horizontal
/// components remain distinct until text direction resolves the geometry.
#[derive(Clone, Copy, Debug)]
pub enum AlignmentGeometry {
    Physical(Alignment),
    Directional(AlignmentDirectional),
    Mixed { x: f32, start: f32, y: f32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error(
    "No TextDirection found. Directional alignment cannot resolve without a Directionality ancestor or an explicit text direction."
)]
pub struct AlignmentResolutionError;

impl AlignmentGeometry {
    pub const TOP_LEFT: Self = Self::Physical(Alignment::TOP_LEFT);
    pub const TOP_CENTER: Self = Self::Physical(Alignment::TOP_CENTER);
    pub const TOP_RIGHT: Self = Self::Physical(Alignment::TOP_RIGHT);
    pub const TOP_START: Self = Self::Directional(AlignmentDirectional::TOP_START);
    pub const TOP_END: Self = Self::Directional(AlignmentDirectional::TOP_END);
    pub const CENTER_LEFT: Self = Self::Physical(Alignment::CENTER_LEFT);
    pub const CENTER: Self = Self::Physical(Alignment::CENTER);
    pub const CENTER_RIGHT: Self = Self::Physical(Alignment::CENTER_RIGHT);
    pub const CENTER_START: Self = Self::Directional(AlignmentDirectional::CENTER_START);
    pub const CENTER_END: Self = Self::Directional(AlignmentDirectional::CENTER_END);
    pub const BOTTOM_LEFT: Self = Self::Physical(Alignment::BOTTOM_LEFT);
    pub const BOTTOM_CENTER: Self = Self::Physical(Alignment::BOTTOM_CENTER);
    pub const BOTTOM_RIGHT: Self = Self::Physical(Alignment::BOTTOM_RIGHT);
    pub const BOTTOM_START: Self = Self::Directional(AlignmentDirectional::BOTTOM_START);
    pub const BOTTOM_END: Self = Self::Directional(AlignmentDirectional::BOTTOM_END);

    #[must_use]
    pub const fn xy(x: f32, y: f32) -> Self {
        Self::Physical(Alignment::new(x, y))
    }

    #[must_use]
    pub const fn directional(start: f32, y: f32) -> Self {
        Self::Directional(AlignmentDirectional::new(start, y))
    }

    #[must_use]
    pub const fn resolve(self, direction: TextDirection) -> Alignment {
        match self {
            Self::Physical(value) => value,
            Self::Directional(value) => value.resolve(direction),
            Self::Mixed { x, start, y } => match direction {
                TextDirection::Ltr => Alignment::new(x + start, y),
                TextDirection::Rtl => Alignment::new(x - start, y),
            },
        }
    }

    /// Physical alignment does not need ambient direction. Directional and
    /// mixed geometry require one even when their relative component is zero.
    pub fn try_resolve(
        self,
        direction: Option<TextDirection>,
    ) -> Result<Alignment, AlignmentResolutionError> {
        match (self, direction) {
            (Self::Physical(value), _) => Ok(value),
            (_, Some(direction)) => Ok(self.resolve(direction)),
            (_, None) => Err(AlignmentResolutionError),
        }
    }

    #[must_use]
    pub fn lerp_optional(a: Option<Self>, b: Option<Self>, t: f32) -> Option<Self> {
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b * t),
            (Some(a), None) => Some(a * (1. - t)),
            (Some(a), Some(b)) => Some(a.lerp(&b, t)),
        }
    }

    #[must_use]
    pub fn truncating_div(self, divisor: f32) -> Self {
        self.map(|value| truncating_quotient(value, divisor))
    }

    const fn components(self) -> [f32; 3] {
        match self {
            Self::Physical(a) => [a.x, 0., a.y],
            Self::Directional(a) => [0., a.start, a.y],
            Self::Mixed { x, start, y } => [x, start, y],
        }
    }

    fn map(self, operation: impl Fn(f32) -> f32) -> Self {
        match self {
            Self::Physical(a) => Self::xy(operation(a.x), operation(a.y)),
            Self::Directional(a) => Self::directional(operation(a.start), operation(a.y)),
            Self::Mixed { x, start, y } => Self::Mixed {
                x: operation(x),
                start: operation(start),
                y: operation(y),
            },
        }
    }

    fn zip(self, other: Self, operation: impl Fn(f32, f32) -> f32) -> Self {
        match (self, other) {
            (Self::Physical(a), Self::Physical(b)) => {
                Self::xy(operation(a.x, b.x), operation(a.y, b.y))
            }
            (Self::Directional(a), Self::Directional(b)) => {
                Self::directional(operation(a.start, b.start), operation(a.y, b.y))
            }
            (a, b) => {
                let [x, start, y] = a.components();
                let [other_x, other_start, other_y] = b.components();
                Self::Mixed {
                    x: operation(x, other_x),
                    start: operation(start, other_start),
                    y: operation(y, other_y),
                }
            }
        }
    }
}

impl From<Alignment> for AlignmentGeometry {
    fn from(value: Alignment) -> Self {
        Self::Physical(value)
    }
}

impl From<AlignmentDirectional> for AlignmentGeometry {
    fn from(value: AlignmentDirectional) -> Self {
        Self::Directional(value)
    }
}

impl PartialEq for AlignmentGeometry {
    fn eq(&self, other: &Self) -> bool {
        self.components() == other.components()
    }
}

impl Hash for AlignmentGeometry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_layout_floats(self.components(), state);
    }
}

impl fmt::Display for AlignmentGeometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [x, start, y] = match *self {
            Self::Physical(alignment) => return alignment.fmt(f),
            Self::Directional(alignment) => return alignment.fmt(f),
            Self::Mixed { x, start, y } => [x, start, y],
        };
        if start == 0. {
            Alignment::new(x, y).fmt(f)
        } else if x == 0. {
            AlignmentDirectional::new(start, y).fmt(f)
        } else {
            write!(
                f,
                "{} + {}",
                Alignment::new(x, y),
                AlignmentDirectional::new(start, 0.)
            )
        }
    }
}

impl Lerp for AlignmentGeometry {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        self.zip(*other, |a, b| mix(a, b, t))
    }
}

impl Add for AlignmentGeometry {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.zip(rhs, |a, b| a + b)
    }
}

impl Sub for AlignmentGeometry {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.zip(rhs, |a, b| a - b)
    }
}

impl Mul<f32> for AlignmentGeometry {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        self.map(|value| value * rhs)
    }
}

impl Div<f32> for AlignmentGeometry {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        self.map(|value| value / rhs)
    }
}

impl Rem<f32> for AlignmentGeometry {
    type Output = Self;
    fn rem(self, rhs: f32) -> Self {
        self.map(|value| value.rem_euclid(rhs))
    }
}

impl Neg for AlignmentGeometry {
    type Output = Self;
    fn neg(self) -> Self {
        self.map(|value| -value)
    }
}

impl Alignment {
    #[must_use]
    pub fn lerp_optional(a: Option<Self>, b: Option<Self>, t: f32) -> Option<Self> {
        if a.is_none() && b.is_none() {
            return None;
        }
        Some(
            a.unwrap_or(Self::CENTER)
                .lerp(&b.unwrap_or(Self::CENTER), t),
        )
    }

    #[must_use]
    pub fn truncating_div(self, divisor: f32) -> Self {
        Self::new(
            truncating_quotient(self.x, divisor),
            truncating_quotient(self.y, divisor),
        )
    }
}

impl AlignmentDirectional {
    #[must_use]
    pub fn lerp_optional(a: Option<Self>, b: Option<Self>, t: f32) -> Option<Self> {
        if a.is_none() && b.is_none() {
            return None;
        }
        Some(
            a.unwrap_or(Self::CENTER)
                .lerp(&b.unwrap_or(Self::CENTER), t),
        )
    }

    pub fn try_resolve(
        self,
        direction: Option<TextDirection>,
    ) -> Result<Alignment, AlignmentResolutionError> {
        direction
            .map(|direction| self.resolve(direction))
            .ok_or(AlignmentResolutionError)
    }

    #[must_use]
    pub fn truncating_div(self, divisor: f32) -> Self {
        Self::new(
            truncating_quotient(self.start, divisor),
            truncating_quotient(self.y, divisor),
        )
    }
}

impl Hash for Alignment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_layout_floats([self.x, 0., self.y], state);
    }
}

impl Hash for AlignmentDirectional {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_layout_floats([0., self.start, self.y], state);
    }
}

impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_coordinates(
            f,
            self.x,
            self.y,
            "Alignment",
            &[
                "topLeft",
                "topCenter",
                "topRight",
                "centerLeft",
                "center",
                "centerRight",
                "bottomLeft",
                "bottomCenter",
                "bottomRight",
            ],
        )
    }
}

impl fmt::Display for AlignmentDirectional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_coordinates(
            f,
            self.start,
            self.y,
            "AlignmentDirectional",
            &[
                "topStart",
                "topCenter",
                "topEnd",
                "centerStart",
                "center",
                "centerEnd",
                "bottomStart",
                "bottomCenter",
                "bottomEnd",
            ],
        )
    }
}

fn format_coordinates(
    f: &mut fmt::Formatter<'_>,
    x: f32,
    y: f32,
    ty: &str,
    names: &[&str; 9],
) -> fmt::Result {
    let axis_index = |value: f32| [-1., 0., 1.].iter().position(|&entry| entry == value);
    if let (Some(row), Some(column)) = (axis_index(y), axis_index(x)) {
        write!(f, "{ty}.{}", names[row * 3 + column])
    } else {
        write!(f, "{ty}({}, {})", FixedCoordinate(x), FixedCoordinate(y))
    }
}

// Dart's fixed decimal diagnostics round exact half-way coordinates
// away from zero, rather than Rust's default half-to-even formatting.
struct FixedCoordinate(f32);

impl fmt::Display for FixedCoordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = f64::from(self.0);
        if value.is_infinite() {
            return f.write_str(if value.is_sign_negative() {
                "-Infinity"
            } else {
                "Infinity"
            });
        }
        if value.abs() >= 1e21 {
            let exponent = format!("{value:e}");
            if let Some((mantissa, power)) = exponent.split_once('e') {
                return write!(f, "{mantissa}e+{power}");
            }
        }
        let nearest = (value * 10.).round() / 10.;
        write!(f, "{nearest:.1}")
    }
}

fn truncating_quotient(value: f32, divisor: f32) -> f32 {
    let quotient = value / divisor;
    assert!(
        quotient.is_finite(),
        "alignment truncating division requires a finite quotient"
    );
    quotient.trunc()
}

pub(super) fn mix(a: f32, b: f32, t: f32) -> f32 {
    if a == b || a.is_nan() && b.is_nan() {
        a
    } else {
        a * (1. - t) + b * t
    }
}

impl Rem<f32> for Alignment {
    type Output = Self;
    fn rem(self, rhs: f32) -> Self {
        Self::new(self.x.rem_euclid(rhs), self.y.rem_euclid(rhs))
    }
}

impl Add for AlignmentDirectional {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.start + rhs.start, self.y + rhs.y)
    }
}

impl Sub for AlignmentDirectional {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.start - rhs.start, self.y - rhs.y)
    }
}

impl Mul<f32> for AlignmentDirectional {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.start * rhs, self.y * rhs)
    }
}

impl Div<f32> for AlignmentDirectional {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self::new(self.start / rhs, self.y / rhs)
    }
}

impl Rem<f32> for AlignmentDirectional {
    type Output = Self;
    fn rem(self, rhs: f32) -> Self {
        Self::new(self.start.rem_euclid(rhs), self.y.rem_euclid(rhs))
    }
}

impl Neg for AlignmentDirectional {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.start, -self.y)
    }
}
