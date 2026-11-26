//! Insets that can retain both physical and reading-direction components.

use incular_core::Lerp;
use std::{
    fmt,
    ops::{Add, Div, Mul, Neg, Rem, Sub},
};

use crate::{EdgeInsets, EdgeInsetsDirectional, TextDirection};

/// Edge insets before resolving the reading direction. A mixed value keeps
/// its physical left/right and directional start/end independently.
#[derive(Clone, Copy, Debug)]
pub enum EdgeInsetsGeometry {
    Physical(EdgeInsets),
    Directional(EdgeInsetsDirectional),
    Mixed {
        /// Physical sides, including the common top and bottom totals.
        physical: EdgeInsets,
        start: f32,
        end: f32,
    },
}

/// Direction-dependent insets cannot be resolved without a reading direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissingInsetsDirection;

impl fmt::Display for MissingInsetsDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("No TextDirection found. Cannot resolve directional insets without a Directionality ancestor.")
    }
}

impl std::error::Error for MissingInsetsDirection {}

impl EdgeInsetsGeometry {
    pub const ZERO: Self = Self::Physical(EdgeInsets::ZERO);

    #[must_use]
    pub const fn all(value: f32) -> Self {
        Self::Physical(EdgeInsets::all(value))
    }

    #[must_use]
    pub const fn only(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self::Physical(EdgeInsets::only(left, top, right, bottom))
    }

    #[must_use]
    pub const fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self::only(left, top, right, bottom)
    }

    #[must_use]
    pub const fn directional(start: f32, top: f32, end: f32, bottom: f32) -> Self {
        Self::Directional(EdgeInsetsDirectional::from_steb(start, top, end, bottom))
    }

    #[must_use]
    pub const fn from_steb(start: f32, top: f32, end: f32, bottom: f32) -> Self {
        Self::directional(start, top, end, bottom)
    }

    #[must_use]
    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self::Physical(EdgeInsets::symmetric(horizontal, vertical))
    }

    /// Converts a physical view's device-pixel padding to logical pixels.
    #[must_use]
    pub fn from_view_padding(padding: EdgeInsets, device_pixel_ratio: f32) -> Self {
        Self::Physical(EdgeInsets::from_view_padding(padding, device_pixel_ratio))
    }

    /// Resolves the physical result under an explicit reading direction.
    #[must_use]
    pub fn resolve(self, direction: TextDirection) -> EdgeInsets {
        match self {
            Self::Physical(insets) => insets,
            Self::Directional(insets) => insets.resolve(direction),
            Self::Mixed {
                physical,
                start,
                end,
            } => {
                let (left, right) = match direction {
                    TextDirection::Ltr => (start, end),
                    TextDirection::Rtl => (end, start),
                };
                EdgeInsets::only(
                    physical.left + left,
                    physical.top,
                    physical.right + right,
                    physical.bottom,
                )
            }
        }
    }

    /// Resolves physical values even when no direction is supplied.
    /// A directional or mixed value requires an explicit direction.
    pub fn try_resolve(
        self,
        direction: Option<TextDirection>,
    ) -> Result<EdgeInsets, MissingInsetsDirection> {
        match self {
            Self::Physical(value) => Ok(value),
            Self::Directional(value) => value.try_resolve(direction),
            Self::Mixed { .. } => Ok(self.resolve(direction.ok_or(MissingInsetsDirection)?)),
        }
    }

    /// Interpolates absent endpoints from zero without resolving direction.
    #[must_use]
    pub fn lerp_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        match (begin, end) {
            (None, None) => None,
            (Some(a), None) => Some(a * (1. - t)),
            (None, Some(b)) => Some(b * t),
            (Some(a), Some(b)) => Some(a.lerp(&b, t)),
        }
    }

    /// Floors each unresolved edge at zero before physical and logical
    /// components are combined, as in Flutter's AnimatedPadding.
    #[must_use]
    pub fn non_negative(self) -> Self {
        self.map(|edge| edge.max(0.0))
    }

    /// Divides all populated sides and truncates each quotient toward zero.
    ///
    /// # Panics
    /// Panics if any quotient is non-finite.
    #[must_use]
    pub fn truncating_div(self, divisor: f32) -> Self {
        self.map(|edge| super::insets::truncate_edge_quotient(edge, divisor))
    }

    fn parts(self) -> (EdgeInsets, f32, f32) {
        match self {
            Self::Physical(a) => (a, 0., 0.),
            Self::Directional(a) => (EdgeInsets::only(0., a.top, 0., a.bottom), a.start, a.end),
            Self::Mixed {
                physical,
                start,
                end,
            } => (physical, start, end),
        }
    }

    fn combine(self, other: Self, operation: impl Fn(f32, f32) -> f32) -> Self {
        let (a, a_start, a_end) = self.parts();
        let (b, b_start, b_end) = other.parts();
        let start = operation(a_start, b_start);
        let end = operation(a_end, b_end);
        let physical = EdgeInsets::only(
            operation(a.left, b.left),
            operation(a.top, b.top),
            operation(a.right, b.right),
            operation(a.bottom, b.bottom),
        );
        match (self, other) {
            (Self::Physical(_), Self::Physical(_)) => Self::Physical(physical),
            (Self::Directional(_), Self::Directional(_)) => {
                Self::directional(start, physical.top, end, physical.bottom)
            }
            _ => Self::Mixed {
                physical,
                start,
                end,
            },
        }
    }

    fn map(self, operation: impl Fn(f32) -> f32) -> Self {
        let (a, start, end) = self.parts();
        match self {
            Self::Physical(_) => Self::only(
                operation(a.left),
                operation(a.top),
                operation(a.right),
                operation(a.bottom),
            ),
            Self::Directional(_) => Self::directional(
                operation(start),
                operation(a.top),
                operation(end),
                operation(a.bottom),
            ),
            Self::Mixed { .. } => Self::Mixed {
                physical: EdgeInsets::only(
                    operation(a.left),
                    operation(a.top),
                    operation(a.right),
                    operation(a.bottom),
                ),
                start: operation(start),
                end: operation(end),
            },
        }
    }
}

impl From<EdgeInsets> for EdgeInsetsGeometry {
    fn from(value: EdgeInsets) -> Self {
        Self::Physical(value)
    }
}

impl From<EdgeInsetsDirectional> for EdgeInsetsGeometry {
    fn from(value: EdgeInsetsDirectional) -> Self {
        Self::Directional(value)
    }
}

impl PartialEq for EdgeInsetsGeometry {
    fn eq(&self, other: &Self) -> bool {
        self.parts() == other.parts()
    }
}

impl Lerp for EdgeInsetsGeometry {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if self == other && std::mem::discriminant(self) == std::mem::discriminant(other) {
            return *self;
        }
        self.combine(*other, |a, b| a + (b - a) * t)
    }
}

impl Add for EdgeInsetsGeometry {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        self.combine(other, |a, b| a + b)
    }
}

impl Sub for EdgeInsetsGeometry {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        self.combine(other, |a, b| a - b)
    }
}

impl Neg for EdgeInsetsGeometry {
    type Output = Self;
    fn neg(self) -> Self {
        self.map(|value| -value)
    }
}

impl Mul<f32> for EdgeInsetsGeometry {
    type Output = Self;
    fn mul(self, factor: f32) -> Self {
        self.map(|value| value * factor)
    }
}

impl Div<f32> for EdgeInsetsGeometry {
    type Output = Self;
    fn div(self, divisor: f32) -> Self {
        self.map(|value| value / divisor)
    }
}

impl Rem<f32> for EdgeInsetsGeometry {
    type Output = Self;
    fn rem(self, divisor: f32) -> Self {
        self.map(|value| value.rem_euclid(divisor))
    }
}

impl fmt::Display for EdgeInsetsGeometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (p, start, end) = self.parts();
        let [left, top, right, bottom] = [p.left, p.top, p.right, p.bottom];
        if start == 0. && end == 0. {
            return if p.is_zero() {
                f.write_str("EdgeInsets.zero")
            } else if left == top && top == right && right == bottom {
                write!(f, "EdgeInsets.all({left:.1})")
            } else {
                write!(
                    f,
                    "EdgeInsets({left:.1}, {top:.1}, {right:.1}, {bottom:.1})"
                )
            };
        }
        if left == 0. && right == 0. {
            return write!(
                f,
                "EdgeInsetsDirectional({start:.1}, {top:.1}, {end:.1}, {bottom:.1})"
            );
        }
        write!(
            f,
            "EdgeInsets({left:.1}, {top:.1}, {right:.1}, {bottom:.1}) + EdgeInsetsDirectional({start:.1}, 0.0, {end:.1}, 0.0)"
        )
    }
}

impl fmt::Display for EdgeInsets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&EdgeInsetsGeometry::Physical(*self), f)
    }
}

impl fmt::Display for EdgeInsetsDirectional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&EdgeInsetsGeometry::Directional(*self), f)
    }
}

impl EdgeInsetsDirectional {
    /// Reports a missing text direction rather than applying physical LTR.
    pub fn try_resolve(
        self,
        direction: Option<TextDirection>,
    ) -> Result<EdgeInsets, MissingInsetsDirection> {
        Ok(self.resolve(direction.ok_or(MissingInsetsDirection)?))
    }
}
