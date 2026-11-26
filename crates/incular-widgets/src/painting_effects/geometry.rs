//! Corner-radius geometry shared by painting and decoration descriptors.

use std::{
    hash::{Hash, Hasher},
    ops::{Add, Mul, Sub},
};

use incular_config::EdgeInsets;
use incular_core::{Lerp, Offset, Rect, Size};
use incular_rendering::{CornerRadii, Path};
use typed_builder::TypedBuilder;

/// A 2D radius for rounded corners.
#[derive(Clone, Copy, Debug, Default, PartialEq, TypedBuilder)]
pub struct Radius {
    #[builder(default = 0.0)]
    pub x: f32,
    #[builder(default = 0.0)]
    pub y: f32,
}

impl Radius {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    #[must_use]
    pub const fn circular(radius: f32) -> Self {
        Self {
            x: radius,
            y: radius,
        }
    }

    #[must_use]
    pub const fn elliptical(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self::ZERO
    }

    /// Interpolates absent endpoints from zero, returning `None` only if
    /// both endpoints are absent. Values may extrapolate past zero.
    #[must_use]
    pub fn lerp_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        lerp_optional(begin, end, t)
    }
}

impl Hash for Radius {
    fn hash<H: Hasher>(&self, state: &mut H) {
        super::hash_paint_float(self.x, state);
        super::hash_paint_float(self.y, state);
    }
}

impl Lerp for Radius {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        // Match Flutter's Radius.lerp value semantics. Rounded-rectangle
        // geometry normalizes at paint time, not in the animation value.
        let from = 1.0 - t;
        Self::elliptical(self.x * from + other.x * t, self.y * from + other.y * t)
    }
}

impl Add for Radius {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::elliptical(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Radius {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::elliptical(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f32> for Radius {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self::elliptical(self.x * rhs, self.y * rhs)
    }
}

fn lerp_optional<T: Lerp + Mul<f32, Output = T>>(
    begin: Option<T>,
    end: Option<T>,
    t: f32,
) -> Option<T> {
    match (begin, end) {
        (None, None) => None,
        (Some(a), None) => Some(a * (1.0 - t)),
        (None, Some(b)) => Some(b * t),
        (Some(a), Some(b)) => Some(a.lerp(&b, t)),
    }
}

/// An immutable set of radii for each of the four corners of a box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Hash, TypedBuilder)]
pub struct BorderRadius {
    #[builder(default = Radius::ZERO)]
    pub top_left: Radius,
    #[builder(default = Radius::ZERO)]
    pub top_right: Radius,
    #[builder(default = Radius::ZERO)]
    pub bottom_right: Radius,
    #[builder(default = Radius::ZERO)]
    pub bottom_left: Radius,
}

impl BorderRadius {
    pub const ZERO: Self = Self {
        top_left: Radius::ZERO,
        top_right: Radius::ZERO,
        bottom_right: Radius::ZERO,
        bottom_left: Radius::ZERO,
    };

    #[must_use]
    pub const fn all(radius: Radius) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    #[must_use]
    pub const fn circular(radius: f32) -> Self {
        Self::all(Radius::circular(radius))
    }

    #[must_use]
    pub const fn vertical(top: Radius, bottom: Radius) -> Self {
        Self {
            top_left: top,
            top_right: top,
            bottom_right: bottom,
            bottom_left: bottom,
        }
    }

    #[must_use]
    pub const fn horizontal(left: Radius, right: Radius) -> Self {
        Self {
            top_left: left,
            top_right: right,
            bottom_right: right,
            bottom_left: left,
        }
    }

    #[must_use]
    pub const fn only(
        top_left: Radius,
        top_right: Radius,
        bottom_right: Radius,
        bottom_left: Radius,
    ) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self::ZERO
    }

    #[must_use]
    pub const fn resolve(self, _direction: incular_config::TextDirection) -> Self {
        self
    }

    /// Replaces requested corners and retains all omitted ones.
    #[must_use]
    pub fn copy_with(
        self,
        top_left: Option<Radius>,
        top_right: Option<Radius>,
        bottom_right: Option<Radius>,
        bottom_left: Option<Radius>,
    ) -> Self {
        Self::only(
            top_left.unwrap_or(self.top_left),
            top_right.unwrap_or(self.top_right),
            bottom_right.unwrap_or(self.bottom_right),
            bottom_left.unwrap_or(self.bottom_left),
        )
    }

    /// Applies the unnormalized corner geometry to a rectangle.
    #[must_use]
    pub const fn to_rounded_rect(self, rect: Rect) -> EllipticalRoundedRect {
        EllipticalRoundedRect { rect, radii: self }
    }

    #[must_use]
    pub fn to_corner_radii(self) -> CornerRadii {
        CornerRadii {
            top_left: self.top_left.x,
            top_right: self.top_right.x,
            bottom_right: self.bottom_right.x,
            bottom_left: self.bottom_left.x,
        }
    }
}

impl From<BorderRadius> for CornerRadii {
    fn from(value: BorderRadius) -> Self {
        value.to_corner_radii()
    }
}

/// A rounded rectangle with independent horizontal and vertical corner
/// radii. Unlike the renderer's circular fast path, it preserves the full
/// authored geometry and normalizes only when creating a paint path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EllipticalRoundedRect {
    pub rect: Rect,
    pub radii: BorderRadius,
}

impl EllipticalRoundedRect {
    #[must_use]
    pub const fn new(rect: Rect, radii: BorderRadius) -> Self {
        Self { rect, radii }
    }

    /// Moves each edge outward and extends the two axes of each adjacent
    /// corner, flooring a radius at zero. An invalid render extent is rejected.
    #[must_use]
    pub fn inflate(self, insets: EdgeInsets) -> Option<Self> {
        let rect = insets.inflate_rect(self.rect)?;
        Some(self.with_adjusted_corners(rect, insets))
    }

    fn with_adjusted_corners(self, rect: Rect, insets: EdgeInsets) -> Self {
        let adjust = |corner: Radius, dx: f32, dy: f32| {
            Radius::elliptical((corner.x + dx).max(0.), (corner.y + dy).max(0.))
        };
        let corners = self.radii;
        Self::new(
            rect,
            BorderRadius::only(
                adjust(corners.top_left, insets.left, insets.top),
                adjust(corners.top_right, insets.right, insets.top),
                adjust(corners.bottom_right, insets.right, insets.bottom),
                adjust(corners.bottom_left, insets.left, insets.bottom),
            ),
        )
    }

    /// Flutter's native RRect conversion orders inverted bounds at paint
    /// time. Do not drop a reversed inner rectangle and fill its hole.
    pub(super) fn inflated_paint_path(self, insets: EdgeInsets) -> Path {
        let value = self.inflate(insets).or_else(|| {
            let left = self.rect.origin.x - insets.left;
            let top = self.rect.origin.y - insets.top;
            let right = self.rect.origin.x + self.rect.size.width + insets.right;
            let bottom = self.rect.origin.y + self.rect.size.height + insets.bottom;
            if ![left, top, right, bottom].into_iter().all(f32::is_finite) {
                return None;
            }
            let origin = Offset::new(left.min(right), top.min(bottom));
            let size = Size::maybe_new((right - left).abs(), (bottom - top).abs())?;
            Some(self.with_adjusted_corners(Rect::from_origin_size(origin, size), insets))
        });
        value.map_or_else(|| Path::builder().build(), Self::to_path)
    }

    /// Moves each edge inward and shrinks its adjacent radii independently.
    #[must_use]
    pub fn deflate(self, insets: EdgeInsets) -> Option<Self> {
        self.inflate(-insets)
    }

    /// Creates a paint path using the same normalized quarter conics as
    /// rounded decoration borders.
    #[must_use]
    pub fn to_path(self) -> Path {
        if self.rect.size.width <= 0. || self.rect.size.height <= 0. {
            return Path::builder().build();
        }
        super::box_painter::rrect_path(self.rect, self.radii)
    }
}

/// An immutable set of directional radii for the four corners of a box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Hash, TypedBuilder)]
pub struct BorderRadiusDirectional {
    #[builder(default = Radius::ZERO)]
    pub top_start: Radius,
    #[builder(default = Radius::ZERO)]
    pub top_end: Radius,
    #[builder(default = Radius::ZERO)]
    pub bottom_end: Radius,
    #[builder(default = Radius::ZERO)]
    pub bottom_start: Radius,
}

impl BorderRadiusDirectional {
    pub const ZERO: Self = Self {
        top_start: Radius::ZERO,
        top_end: Radius::ZERO,
        bottom_end: Radius::ZERO,
        bottom_start: Radius::ZERO,
    };

    #[must_use]
    pub const fn all(radius: Radius) -> Self {
        Self {
            top_start: radius,
            top_end: radius,
            bottom_end: radius,
            bottom_start: radius,
        }
    }

    #[must_use]
    pub const fn circular(radius: f32) -> Self {
        Self::all(Radius::circular(radius))
    }

    #[must_use]
    pub const fn vertical(top: Radius, bottom: Radius) -> Self {
        Self {
            top_start: top,
            top_end: top,
            bottom_end: bottom,
            bottom_start: bottom,
        }
    }

    #[must_use]
    pub const fn horizontal(start: Radius, end: Radius) -> Self {
        Self {
            top_start: start,
            top_end: end,
            bottom_end: end,
            bottom_start: start,
        }
    }

    #[must_use]
    pub const fn only(
        top_start: Radius,
        top_end: Radius,
        bottom_end: Radius,
        bottom_start: Radius,
    ) -> Self {
        Self {
            top_start,
            top_end,
            bottom_end,
            bottom_start,
        }
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self::ZERO
    }

    #[must_use]
    pub const fn resolve(self, direction: incular_config::TextDirection) -> BorderRadius {
        match direction {
            incular_config::TextDirection::Ltr => BorderRadius {
                top_left: self.top_start,
                top_right: self.top_end,
                bottom_right: self.bottom_end,
                bottom_left: self.bottom_start,
            },
            incular_config::TextDirection::Rtl => BorderRadius {
                top_left: self.top_end,
                top_right: self.top_start,
                bottom_right: self.bottom_start,
                bottom_left: self.bottom_end,
            },
        }
    }
}

macro_rules! impl_corner_arithmetic {
    ($type:ty, $($corner:ident),+ $(,)?) => {
        impl $type {
            /// Interpolates absent endpoints from zero, returning `None` only
            /// if both endpoints are absent.
            #[must_use]
            pub fn lerp_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
                lerp_optional(begin, end, t)
            }
        }

        impl Lerp for $type {
            fn lerp(&self, other: &Self, t: f32) -> Self {
                if std::ptr::eq(self, other) {
                    return *self;
                }
                Self { $($corner: self.$corner.lerp(&other.$corner, t)),+ }
            }
        }

        impl Add for $type {
            type Output = Self;

            fn add(self, rhs: Self) -> Self {
                Self { $($corner: self.$corner + rhs.$corner),+ }
            }
        }

        impl Sub for $type {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self {
                Self { $($corner: self.$corner - rhs.$corner),+ }
            }
        }

        impl Mul<f32> for $type {
            type Output = Self;

            fn mul(self, rhs: f32) -> Self {
                Self { $($corner: self.$corner * rhs),+ }
            }
        }
    };
}

impl_corner_arithmetic!(BorderRadius, top_left, top_right, bottom_right, bottom_left);
impl_corner_arithmetic!(
    BorderRadiusDirectional,
    top_start,
    top_end,
    bottom_end,
    bottom_start
);

/// Flutter's direction-aware corner geometry. Mixed values keep physical and
/// logical corners separate until a text direction is known.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub enum BorderRadiusGeometry {
    /// All corners are physical.
    Physical(BorderRadius),
    /// All corners follow the text direction.
    Directional(BorderRadiusDirectional),
    /// The resolved radii are the sum of both parts.
    Mixed {
        physical: BorderRadius,
        directional: BorderRadiusDirectional,
    },
}

impl BorderRadiusGeometry {
    pub const ZERO: Self = Self::Physical(BorderRadius::ZERO);

    #[must_use]
    pub const fn all(radius: Radius) -> Self {
        Self::Physical(BorderRadius::all(radius))
    }

    #[must_use]
    pub const fn circular(radius: f32) -> Self {
        Self::all(Radius::circular(radius))
    }

    #[must_use]
    pub fn resolve(self, direction: incular_config::TextDirection) -> BorderRadius {
        match self {
            Self::Physical(value) => value,
            Self::Directional(value) => value.resolve(direction),
            Self::Mixed {
                physical,
                directional,
            } => physical + directional.resolve(direction),
        }
    }

    /// Flutter's nullable interpolation, using physical zero when just one
    /// endpoint is omitted.
    #[must_use]
    pub fn lerp_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        match (begin, end) {
            (None, None) => None,
            (begin, end) => {
                let begin = begin.unwrap_or(Self::ZERO);
                let end = end.unwrap_or(Self::ZERO);
                Some(begin.lerp(&end, t))
            }
        }
    }

    fn parts(self) -> (BorderRadius, BorderRadiusDirectional) {
        match self {
            Self::Physical(value) => (value, BorderRadiusDirectional::ZERO),
            Self::Directional(value) => (BorderRadius::ZERO, value),
            Self::Mixed {
                physical,
                directional,
            } => (physical, directional),
        }
    }

    fn combine(
        self,
        other: Self,
        physical: impl FnOnce(BorderRadius, BorderRadius) -> BorderRadius,
        directional: impl FnOnce(
            BorderRadiusDirectional,
            BorderRadiusDirectional,
        ) -> BorderRadiusDirectional,
    ) -> Self {
        match (self, other) {
            (Self::Physical(a), Self::Physical(b)) => Self::Physical(physical(a, b)),
            (Self::Directional(a), Self::Directional(b)) => Self::Directional(directional(a, b)),
            _ => {
                let (ap, ad) = self.parts();
                let (bp, bd) = other.parts();
                Self::Mixed {
                    physical: physical(ap, bp),
                    directional: directional(ad, bd),
                }
            }
        }
    }
}

impl Default for BorderRadiusGeometry {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<BorderRadius> for BorderRadiusGeometry {
    fn from(value: BorderRadius) -> Self {
        Self::Physical(value)
    }
}

impl From<BorderRadiusDirectional> for BorderRadiusGeometry {
    fn from(value: BorderRadiusDirectional) -> Self {
        Self::Directional(value)
    }
}

impl Add for BorderRadiusGeometry {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        self.combine(rhs, Add::add, Add::add)
    }
}

impl Sub for BorderRadiusGeometry {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        self.combine(rhs, Sub::sub, Sub::sub)
    }
}

impl Mul<f32> for BorderRadiusGeometry {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        match self {
            Self::Physical(value) => Self::Physical(value * rhs),
            Self::Directional(value) => Self::Directional(value * rhs),
            Self::Mixed {
                physical,
                directional,
            } => Self::Mixed {
                physical: physical * rhs,
                directional: directional * rhs,
            },
        }
    }
}

impl Lerp for BorderRadiusGeometry {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if std::ptr::eq(self, other) {
            return *self;
        }
        *self + (*other - *self) * t
    }
}
