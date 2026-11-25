//! Box constraints shared by all pure layout algorithms.

use incular_core::{Lerp, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintError {
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    min_width: f32,
    max_width: f32,
    min_height: f32,
    max_height: f32,
}
impl Constraints {
    /// Constructs validated bounds.
    ///
    /// # Panics
    /// Panics for invalid bounds; use [`Self::try_new`] for fallible input.
    #[must_use]
    pub fn new(min_width: f32, max_width: f32, min_height: f32, max_height: f32) -> Self {
        Self::try_new(min_width, max_width, min_height, max_height).expect("invalid constraints")
    }

    /// Validates externally supplied bounds. Minimums must be finite and
    /// nonnegative; maximums may be infinite but must not be below minimums.
    pub fn try_new(
        min_width: f32,
        max_width: f32,
        min_height: f32,
        max_height: f32,
    ) -> Result<Self, ConstraintError> {
        if !min_width.is_finite()
            || !min_height.is_finite()
            || min_width < 0.0
            || min_height < 0.0
            || max_width.is_nan()
            || max_height.is_nan()
            || max_width < min_width
            || max_height < min_height
        {
            return Err(ConstraintError::Invalid);
        }
        Ok(Self {
            min_width,
            max_width,
            min_height,
            max_height,
        })
    }

    /// Returns the min width bound.
    #[must_use]
    pub const fn min_width(self) -> f32 {
        self.min_width
    }

    /// Returns the max width bound.
    #[must_use]
    pub const fn max_width(self) -> f32 {
        self.max_width
    }

    /// Returns the min height bound.
    #[must_use]
    pub const fn min_height(self) -> f32 {
        self.min_height
    }

    /// Returns the max height bound.
    #[must_use]
    pub const fn max_height(self) -> f32 {
        self.max_height
    }

    #[must_use]
    pub fn tight(size: Size) -> Self {
        Self::new(size.width, size.width, size.height, size.height)
    }
    /// Loose bounds from a laid-out size. For an unbounded axis, which a
    /// [`Size`] cannot hold, use [`Self::at_most`].
    #[must_use]
    pub fn loose(size: Size) -> Self {
        Self::new(0., size.width, 0., size.height)
    }
    /// Flutter's `BoxConstraints(maxWidth: w, maxHeight: h)`: no minimum,
    /// and maximums that may be `f32::INFINITY` ("at most this wide, any
    /// height").
    ///
    /// # Panics
    /// Panics for a NaN or negative maximum.
    #[must_use]
    pub fn at_most(max_width: f32, max_height: f32) -> Self {
        Self::new(0., max_width, 0., max_height)
    }
    /// These bounds with a new maximum width (Flutter's
    /// `copyWith(maxWidth:)`); the minimum shrinks to fit under it.
    #[must_use]
    pub fn with_max_width(self, max_width: f32) -> Self {
        Self::new(
            self.min_width.min(max_width),
            max_width,
            self.min_height,
            self.max_height,
        )
    }
    /// These bounds with a new maximum height; the minimum shrinks to fit.
    #[must_use]
    pub fn with_max_height(self, max_height: f32) -> Self {
        Self::new(
            self.min_width,
            self.max_width,
            self.min_height.min(max_height),
            max_height,
        )
    }
    /// These bounds with a new minimum width; the maximum grows to fit.
    #[must_use]
    pub fn with_min_width(self, min_width: f32) -> Self {
        Self::new(
            min_width,
            self.max_width.max(min_width),
            self.min_height,
            self.max_height,
        )
    }
    /// These bounds with a new minimum height; the maximum grows to fit.
    #[must_use]
    pub fn with_min_height(self, min_height: f32) -> Self {
        Self::new(
            self.min_width,
            self.max_width,
            min_height,
            self.max_height.max(min_height),
        )
    }
    #[must_use]
    pub fn unbounded() -> Self {
        Self::new(0., f32::INFINITY, 0., f32::INFINITY)
    }
    #[must_use]
    pub fn constrain(self, size: Size) -> Size {
        Size::new(
            size.width.clamp(self.min_width, self.max_width),
            size.height.clamp(self.min_height, self.max_height),
        )
    }

    /// Flutter's box constraint adjustment: preserve a nonempty size's
    /// aspect ratio as far as the minimum and maximum bounds permit.
    #[must_use]
    pub fn constrain_preserving_aspect_ratio(self, size: Size) -> Size {
        if self.min_width == self.max_width && self.min_height == self.max_height {
            return self.smallest();
        }
        if size.width <= 0.
            || size.height <= 0.
            || !size.width.is_finite()
            || !size.height.is_finite()
        {
            return self.constrain(size);
        }
        let mut width = f64::from(size.width);
        let mut height = f64::from(size.height);
        let aspect = width / height;
        let [min_width, max_width, min_height, max_height] = [
            self.min_width,
            self.max_width,
            self.min_height,
            self.max_height,
        ]
        .map(f64::from);
        if width > max_width {
            width = max_width;
            height = width / aspect;
        }
        if height > max_height {
            height = max_height;
            width = height * aspect;
        }
        if width < min_width {
            width = min_width;
            height = width / aspect;
        }
        if height < min_height {
            height = min_height;
            width = height * aspect;
        }
        self.constrain(Size::new(width as f32, height as f32))
    }
    /// Flutter's `BoxConstraints.enforce`: clamps each bound of these
    /// constraints into `other`, so the result never escapes `other`.
    #[must_use]
    pub fn enforce(self, other: Self) -> Self {
        let min_width = self.min_width.clamp(other.min_width, other.max_width);
        let max_width = self.max_width.clamp(other.min_width, other.max_width);
        let min_height = self.min_height.clamp(other.min_height, other.max_height);
        let max_height = self.max_height.clamp(other.min_height, other.max_height);
        Self::new(min_width, max_width, min_height, max_height)
    }
    #[must_use]
    pub fn loosen(self) -> Self {
        Self::new(0., self.max_width, 0., self.max_height)
    }
    #[must_use]
    pub fn deflate(self, horizontal: f32, vertical: f32) -> Self {
        Self::new(
            (self.min_width - horizontal).max(0.),
            (self.max_width - horizontal).max(0.),
            (self.min_height - vertical).max(0.),
            (self.max_height - vertical).max(0.),
        )
    }
    #[must_use]
    pub fn is_width_bounded(self) -> bool {
        self.max_width.is_finite()
    }
    #[must_use]
    pub fn is_height_bounded(self) -> bool {
        self.max_height.is_finite()
    }
    #[must_use]
    pub fn biggest(self) -> Size {
        Size::new(
            if self.max_width.is_finite() {
                self.max_width
            } else {
                self.min_width
            },
            if self.max_height.is_finite() {
                self.max_height
            } else {
                self.min_height
            },
        )
    }
    #[must_use]
    pub fn smallest(self) -> Size {
        Size::new(self.min_width, self.min_height)
    }

    /// Flutter's `BoxConstraints.tightFor`: tight on each given axis and
    /// unconstrained on the other.
    #[must_use]
    pub fn tight_for(width: Option<f32>, height: Option<f32>) -> Self {
        Self::new(
            width.unwrap_or(0.0),
            width.unwrap_or(f32::INFINITY),
            height.unwrap_or(0.0),
            height.unwrap_or(f32::INFINITY),
        )
    }

    /// Flutter's `BoxConstraints.tighten`: each given dimension, clamped into
    /// these bounds, becomes both the minimum and the maximum.
    #[must_use]
    pub fn tighten(self, width: Option<f32>, height: Option<f32>) -> Self {
        let width = width.map(|width| width.clamp(self.min_width, self.max_width));
        let height = height.map(|height| height.clamp(self.min_height, self.max_height));
        Self {
            min_width: width.unwrap_or(self.min_width),
            max_width: width.unwrap_or(self.max_width),
            min_height: height.unwrap_or(self.min_height),
            max_height: height.unwrap_or(self.max_height),
        }
    }

    /// An absent endpoint starts or ends at zero bounds. If both are absent,
    /// the result is absent. Overshoot still produces a normalized box.
    #[must_use]
    pub fn lerp_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        match (begin, end) {
            (None, None) => None,
            (None, Some(end)) => Some(end.scale_for_lerp(t)),
            (Some(begin), None) => Some(begin.scale_for_lerp(1. - t)),
            (Some(begin), Some(end)) => Some(begin.lerp(&end, t)),
        }
    }

    fn scale_for_lerp(self, factor: f32) -> Self {
        if !factor.is_finite() {
            return self;
        }
        if factor <= 0. {
            return Self::tight(Size::ZERO);
        }
        Self::normalize_interpolation(
            self.min_width * factor,
            self.max_width * factor,
            self.min_height * factor,
            self.max_height * factor,
        )
        .unwrap_or(self)
    }

    fn normalize_interpolation(
        min_width: f32,
        max_width: f32,
        min_height: f32,
        max_height: f32,
    ) -> Result<Self, ConstraintError> {
        let min_width = min_width.max(0.);
        let min_height = min_height.max(0.);
        Self::try_new(
            min_width,
            max_width.max(min_width),
            min_height,
            max_height.max(min_height),
        )
    }
}

/// Interpolates and normalizes bounds like Flutter's `BoxConstraints.lerp`.
/// Mixing bounded and unbounded maxima uses the nearer endpoint instead of
/// asserting, so an inconsistent animation cannot feed invalid bounds to layout.
impl Lerp for Constraints {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        if self == other || t == 0. {
            return *self;
        }
        if t == 1. {
            return *other;
        }
        let fallback = if t < 0.5 { *self } else { *other };
        if !t.is_finite()
            || self.max_width.is_finite() != other.max_width.is_finite()
            || self.max_height.is_finite() != other.max_height.is_finite()
        {
            return fallback;
        }
        let t = f64::from(t);
        let mix = |a: f32, b: f32| {
            if a == b {
                a
            } else {
                (f64::from(a) * (1. - t) + f64::from(b) * t) as f32
            }
        };
        Self::normalize_interpolation(
            mix(self.min_width, other.min_width),
            mix(self.max_width, other.max_width),
            mix(self.min_height, other.min_height),
            mix(self.max_height, other.max_height),
        )
        .unwrap_or(fallback)
    }
}
