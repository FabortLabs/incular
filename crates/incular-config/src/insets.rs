use incular_core::{Lerp, Offset, Rect, Size, finite_non_negative};
use std::ops::{Add, AddAssign, Rem, Sub, SubAssign};

/// Insets on the four physical edges of a rectangle.
///
/// Values are kept as supplied so callers can use an inset as a lightweight
/// configuration value. Layout algorithms sanitize non-finite and negative
/// values before applying them; [`EdgeInsets::is_valid`] is available when a
/// caller wants to validate eagerly.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeInsets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl EdgeInsets {
    pub const ZERO: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    #[must_use]
    pub const fn zero() -> Self {
        Self::ZERO
    }

    #[must_use]
    pub const fn all(value: f32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }

    #[must_use]
    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            top: vertical,
            right: horizontal,
            bottom: vertical,
        }
    }

    #[must_use]
    pub const fn horizontal_insets(horizontal: f32) -> Self {
        Self::symmetric(horizontal, 0.0)
    }

    #[must_use]
    pub const fn vertical_insets(vertical: f32) -> Self {
        Self::symmetric(0.0, vertical)
    }

    #[must_use]
    pub const fn from_xy(horizontal: f32, vertical: f32) -> Self {
        Self::symmetric(horizontal, vertical)
    }

    #[must_use]
    pub const fn only(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    #[must_use]
    pub const fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self::only(left, top, right, bottom)
    }

    /// Physical edges are independent of the reading direction.
    #[must_use]
    pub const fn resolve(self, _direction: crate::TextDirection) -> Self {
        self
    }

    /// Converts raw device-pixel view padding to logical units.
    #[must_use]
    pub fn from_view_padding(padding: Self, device_pixel_ratio: f32) -> Self {
        padding / device_pixel_ratio
    }

    /// Copies the insets, replacing only the supplied physical edges.
    #[must_use]
    pub fn copy_with(
        self,
        left: Option<f32>,
        top: Option<f32>,
        right: Option<f32>,
        bottom: Option<f32>,
    ) -> Self {
        Self::from_ltrb(
            left.unwrap_or(self.left),
            top.unwrap_or(self.top),
            right.unwrap_or(self.right),
            bottom.unwrap_or(self.bottom),
        )
    }

    /// Interpolates missing endpoints from zero.
    #[must_use]
    pub fn lerp_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        lerp_insets(begin, end, t)
    }

    #[must_use]
    pub const fn top_left(self) -> Offset {
        Offset::new(self.left, self.top)
    }

    #[must_use]
    pub const fn top_right(self) -> Offset {
        Offset::new(-self.right, self.top)
    }

    #[must_use]
    pub const fn bottom_left(self) -> Offset {
        Offset::new(self.left, -self.bottom)
    }

    #[must_use]
    pub const fn bottom_right(self) -> Offset {
        Offset::new(-self.right, -self.bottom)
    }

    /// Total insets along the selected axis.
    #[must_use]
    pub const fn along(self, axis: crate::Axis) -> f32 {
        match axis {
            crate::Axis::Horizontal => self.horizontal(),
            crate::Axis::Vertical => self.vertical(),
        }
    }

    /// Exchanges both pairs of opposite edges.
    #[must_use]
    pub const fn flipped(self) -> Self {
        Self::from_ltrb(self.right, self.bottom, self.left, self.top)
    }

    /// Total horizontal and vertical insets as a valid render size.
    #[must_use]
    pub fn collapsed_size(self) -> Option<Size> {
        Size::maybe_new(self.horizontal(), self.vertical())
    }

    /// Adds the inset totals to a size, rejecting invalid render extents.
    #[must_use]
    pub fn inflate_size(self, size: Size) -> Option<Size> {
        Size::maybe_new(
            size.width + self.horizontal(),
            size.height + self.vertical(),
        )
    }

    /// Removes the inset totals from a size, rejecting invalid render extents.
    #[must_use]
    pub fn deflate_size(self, size: Size) -> Option<Size> {
        (-self).inflate_size(size)
    }

    /// Expands a rectangle outwards by each corresponding physical edge.
    /// A result with negative or non-finite extents is not a render rectangle.
    #[must_use]
    pub fn inflate_rect(self, rect: Rect) -> Option<Rect> {
        let origin = Offset::new(rect.origin.x - self.left, rect.origin.y - self.top);
        let end = Offset::new(
            rect.origin.x + rect.size.width + self.right,
            rect.origin.y + rect.size.height + self.bottom,
        );
        if !origin.x.is_finite() || !origin.y.is_finite() {
            return None;
        }
        let size = Size::maybe_new(end.x - origin.x, end.y - origin.y)?;
        Some(Rect::from_origin_size(origin, size))
    }

    /// Moves each physical edge inward, rejecting invalid render extents.
    #[must_use]
    pub fn deflate_rect(self, rect: Rect) -> Option<Rect> {
        (-self).inflate_rect(rect)
    }

    /// Divides each edge and truncates toward zero (Flutter's integer division).
    ///
    /// # Panics
    /// Panics if any quotient is non-finite.
    #[must_use]
    pub fn truncating_div(self, divisor: f32) -> Self {
        let div = |edge| truncate_edge_quotient(edge, divisor);
        Self::from_ltrb(
            div(self.left),
            div(self.top),
            div(self.right),
            div(self.bottom),
        )
    }

    #[must_use]
    pub const fn horizontal(self) -> f32 {
        self.left + self.right
    }

    #[must_use]
    pub const fn vertical(self) -> f32 {
        self.top + self.bottom
    }

    #[must_use]
    pub const fn dimensions(self) -> (f32, f32) {
        (self.horizontal(), self.vertical())
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.left == 0.0 && self.top == 0.0 && self.right == 0.0 && self.bottom == 0.0
    }

    #[must_use]
    pub fn is_valid(self) -> bool {
        [self.left, self.top, self.right, self.bottom]
            .into_iter()
            .all(|value| value.is_finite() && value >= 0.0)
    }

    /// Replaces invalid or negative values with zero.
    #[must_use]
    pub fn normalized(self) -> Self {
        Self::only(
            finite_non_negative(self.left),
            finite_non_negative(self.top),
            finite_non_negative(self.right),
            finite_non_negative(self.bottom),
        )
    }

    #[must_use]
    pub fn clamp(self, min: f32, max: f32) -> Self {
        let (min, max) = if min.is_finite() && max.is_finite() && min <= max {
            (min, max)
        } else {
            (0.0, f32::MAX)
        };
        Self::only(
            self.left.clamp(min, max),
            self.top.clamp(min, max),
            self.right.clamp(min, max),
            self.bottom.clamp(min, max),
        )
    }

    #[must_use]
    pub fn inset(self, amount: f32) -> Self {
        Self::only(
            (self.left - amount).max(0.0),
            (self.top - amount).max(0.0),
            (self.right - amount).max(0.0),
            (self.bottom - amount).max(0.0),
        )
    }

    #[must_use]
    pub fn outset(self, amount: f32) -> Self {
        Self::only(
            (self.left + amount).max(0.0),
            (self.top + amount).max(0.0),
            (self.right + amount).max(0.0),
            (self.bottom + amount).max(0.0),
        )
    }
}

impl Add for EdgeInsets {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::only(
            self.left + rhs.left,
            self.top + rhs.top,
            self.right + rhs.right,
            self.bottom + rhs.bottom,
        )
    }
}

impl AddAssign for EdgeInsets {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for EdgeInsets {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::only(
            self.left - rhs.left,
            self.top - rhs.top,
            self.right - rhs.right,
            self.bottom - rhs.bottom,
        )
    }
}

impl SubAssign for EdgeInsets {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::Mul<f32> for EdgeInsets {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::only(
            self.left * rhs,
            self.top * rhs,
            self.right * rhs,
            self.bottom * rhs,
        )
    }
}

impl std::ops::Div<f32> for EdgeInsets {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::only(
            self.left / rhs,
            self.top / rhs,
            self.right / rhs,
            self.bottom / rhs,
        )
    }
}

impl Rem<f32> for EdgeInsets {
    type Output = Self;

    fn rem(self, rhs: f32) -> Self {
        Self::from_ltrb(
            self.left.rem_euclid(rhs),
            self.top.rem_euclid(rhs),
            self.right.rem_euclid(rhs),
            self.bottom.rem_euclid(rhs),
        )
    }
}

impl std::ops::Neg for EdgeInsets {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::only(-self.left, -self.top, -self.right, -self.bottom)
    }
}

/// Flutter `EdgeInsets.lerp`: `t` is not clamped, so an overshooting curve
/// extrapolates past the endpoints.
impl incular_core::Lerp for EdgeInsets {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Self::only(
            mix(self.left, other.left),
            mix(self.top, other.top),
            mix(self.right, other.right),
            mix(self.bottom, other.bottom),
        )
    }
}

/// An immutable set of directional offsets in logical pixels.
/// Resolves against a [`crate::TextDirection`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeInsetsDirectional {
    pub start: f32,
    pub top: f32,
    pub end: f32,
    pub bottom: f32,
}

impl EdgeInsetsDirectional {
    pub const ZERO: Self = Self {
        start: 0.0,
        top: 0.0,
        end: 0.0,
        bottom: 0.0,
    };

    #[must_use]
    pub const fn zero() -> Self {
        Self::ZERO
    }

    #[must_use]
    pub const fn all(value: f32) -> Self {
        Self {
            start: value,
            top: value,
            end: value,
            bottom: value,
        }
    }

    #[must_use]
    pub const fn only(start: f32, top: f32, end: f32, bottom: f32) -> Self {
        Self {
            start,
            top,
            end,
            bottom,
        }
    }

    #[must_use]
    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            start: horizontal,
            top: vertical,
            end: horizontal,
            bottom: vertical,
        }
    }

    #[must_use]
    pub const fn from_ste_b(start: f32, top: f32, end: f32, bottom: f32) -> Self {
        Self::only(start, top, end, bottom)
    }

    /// Creates insets in start, top, end, bottom order.
    #[must_use]
    pub const fn from_steb(start: f32, top: f32, end: f32, bottom: f32) -> Self {
        Self::only(start, top, end, bottom)
    }

    /// Copies the insets, replacing only the supplied directional edges.
    #[must_use]
    pub fn copy_with(
        self,
        start: Option<f32>,
        top: Option<f32>,
        end: Option<f32>,
        bottom: Option<f32>,
    ) -> Self {
        Self::from_steb(
            start.unwrap_or(self.start),
            top.unwrap_or(self.top),
            end.unwrap_or(self.end),
            bottom.unwrap_or(self.bottom),
        )
    }

    /// Divides each edge and truncates toward zero (Flutter's integer division).
    ///
    /// # Panics
    /// Panics if any quotient is non-finite.
    #[must_use]
    pub fn truncating_div(self, divisor: f32) -> Self {
        let div = |edge| truncate_edge_quotient(edge, divisor);
        Self::from_steb(
            div(self.start),
            div(self.top),
            div(self.end),
            div(self.bottom),
        )
    }

    /// Interpolates missing endpoints from zero.
    #[must_use]
    pub fn lerp_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        lerp_insets(begin, end, t)
    }

    #[must_use]
    pub const fn resolve(self, direction: crate::TextDirection) -> EdgeInsets {
        match direction {
            crate::TextDirection::Ltr => {
                EdgeInsets::only(self.start, self.top, self.end, self.bottom)
            }
            crate::TextDirection::Rtl => {
                EdgeInsets::only(self.end, self.top, self.start, self.bottom)
            }
        }
    }

    #[must_use]
    pub const fn horizontal(self) -> f32 {
        self.start + self.end
    }

    #[must_use]
    pub const fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

impl Add for EdgeInsetsDirectional {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::only(
            self.start + rhs.start,
            self.top + rhs.top,
            self.end + rhs.end,
            self.bottom + rhs.bottom,
        )
    }
}

impl Sub for EdgeInsetsDirectional {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::only(
            self.start - rhs.start,
            self.top - rhs.top,
            self.end - rhs.end,
            self.bottom - rhs.bottom,
        )
    }
}

impl std::ops::Mul<f32> for EdgeInsetsDirectional {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::only(
            self.start * rhs,
            self.top * rhs,
            self.end * rhs,
            self.bottom * rhs,
        )
    }
}

impl std::ops::Div<f32> for EdgeInsetsDirectional {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::only(
            self.start / rhs,
            self.top / rhs,
            self.end / rhs,
            self.bottom / rhs,
        )
    }
}

impl Rem<f32> for EdgeInsetsDirectional {
    type Output = Self;

    fn rem(self, rhs: f32) -> Self {
        Self::from_steb(
            self.start.rem_euclid(rhs),
            self.top.rem_euclid(rhs),
            self.end.rem_euclid(rhs),
            self.bottom.rem_euclid(rhs),
        )
    }
}

impl std::ops::Neg for EdgeInsetsDirectional {
    type Output = Self;

    fn neg(self) -> Self {
        Self::from_steb(-self.start, -self.top, -self.end, -self.bottom)
    }
}

/// Flutter `EdgeInsetsDirectional.lerp`: unclamped like [`EdgeInsets`].
impl incular_core::Lerp for EdgeInsetsDirectional {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Self::only(
            mix(self.start, other.start),
            mix(self.top, other.top),
            mix(self.end, other.end),
            mix(self.bottom, other.bottom),
        )
    }
}

fn lerp_insets<T: Lerp + PartialEq + std::ops::Mul<f32, Output = T>>(
    begin: Option<T>,
    end: Option<T>,
    t: f32,
) -> Option<T> {
    match (begin, end) {
        (None, None) => None,
        (Some(a), None) => Some(a * (1.0 - t)),
        (None, Some(b)) => Some(b * t),
        (Some(a), Some(b)) if a == b => Some(a),
        (Some(a), Some(b)) => Some(a.lerp(&b, t)),
    }
}

pub(super) fn truncate_edge_quotient(edge: f32, divisor: f32) -> f32 {
    let quotient = edge / divisor;
    assert!(quotient.is_finite(), "non-finite inset division");
    quotient.trunc()
}
