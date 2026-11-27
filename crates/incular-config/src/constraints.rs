//! Box constraints shared by all pure layout algorithms.

use incular_core::{Lerp, Size};
use std::{
    borrow::Cow,
    fmt,
    hash::{Hash, Hasher},
    ops::{Div, Mul, Rem},
};

/// Raw host-view size bounds, before they are admitted to the finite-minimum
/// layout model. Unspecified width and height are unconstrained.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewConstraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl ViewConstraints {
    #[must_use]
    pub const fn new(min_width: f32, max_width: f32, min_height: f32, max_height: f32) -> Self {
        Self {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }

    #[must_use]
    pub const fn tight(size: Size) -> Self {
        Self::new(size.width, size.width, size.height, size.height)
    }

    #[must_use]
    pub const fn is_tight(self) -> bool {
        self.min_width >= self.max_width && self.min_height >= self.max_height
    }

    #[must_use]
    pub fn is_satisfied_by(self, size: Size) -> bool {
        self.min_width <= size.width
            && size.width <= self.max_width
            && self.min_height <= size.height
            && size.height <= self.max_height
    }
}

impl Default for ViewConstraints {
    fn default() -> Self {
        Self::new(0., f32::INFINITY, 0., f32::INFINITY)
    }
}

impl fmt::Display for ViewConstraints {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            min_width,
            max_width,
            min_height,
            max_height,
        } = *self;
        let normalized = min_width >= 0.
            && max_width >= min_width
            && min_height >= 0.
            && max_height >= min_height;
        let annotation = if normalized { "" } else { "; NOT NORMALIZED" };
        if min_width == f32::INFINITY && min_height == f32::INFINITY {
            return write!(formatter, "BoxConstraints(biggest{annotation})");
        }
        if *self == Self::default() {
            return write!(formatter, "BoxConstraints(unconstrained)");
        }
        fn number(value: f32, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if value == f32::INFINITY {
                f.write_str("Infinity")
            } else if value == f32::NEG_INFINITY {
                f.write_str("-Infinity")
            } else {
                // Dart's fixed one-decimal diagnostics round midpoint
                // bounds away from zero, unlike Rust's ties-to-even format.
                let rounded = (f64::from(value) * 10.).round() / 10.;
                write!(f, "{rounded:.1}")
            }
        }
        fn dimension(min: f32, max: f32, axis: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if min == max {
                write!(f, "{axis}=")?;
                number(min, f)
            } else {
                number(min, f)?;
                write!(f, "<={axis}<=")?;
                number(max, f)
            }
        }
        formatter.write_str("BoxConstraints(")?;
        dimension(min_width, max_width, "w", formatter)?;
        formatter.write_str(", ")?;
        dimension(min_height, max_height, "h", formatter)?;
        write!(formatter, "{annotation})")
    }
}

impl fmt::Display for Constraints {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&ViewConstraints::from(*self), formatter)
    }
}

impl Div<f32> for ViewConstraints {
    type Output = Self;

    fn div(self, factor: f32) -> Self {
        Self::new(
            self.min_width / factor,
            self.max_width / factor,
            self.min_height / factor,
            self.max_height / factor,
        )
    }
}

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
impl Hash for Constraints {
    fn hash<H: Hasher>(&self, state: &mut H) {
        super::hash_layout_floats(
            [
                self.min_width,
                self.max_width,
                self.min_height,
                self.max_height,
            ],
            state,
        );
    }
}

impl Default for Constraints {
    fn default() -> Self {
        Self::at_most(f32::INFINITY, f32::INFINITY)
    }
}

impl TryFrom<ViewConstraints> for Constraints {
    type Error = ConstraintError;

    fn try_from(view: ViewConstraints) -> Result<Self, Self::Error> {
        Self::try_new(
            view.min_width,
            view.max_width,
            view.min_height,
            view.max_height,
        )
    }
}

impl From<Constraints> for ViewConstraints {
    fn from(value: Constraints) -> Self {
        Self::new(
            value.min_width,
            value.max_width,
            value.min_height,
            value.max_height,
        )
    }
}

impl Constraints {
    /// Flutter's BoxConstraints.fromViewConstraints, with layout-bound
    /// validation. Use the fallible conversion for untrusted host values.
    ///
    /// # Panics
    /// Panics if the host bounds cannot be used for layout.
    #[must_use]
    pub fn from_view_constraints(view: ViewConstraints) -> Self {
        Self::try_from(view).expect("invalid view constraints")
    }

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
    /// Copies the provided bounds, leaving unspecified fields unchanged.
    ///
    /// # Panics
    /// Panics if the resulting measures are not valid layout bounds.
    #[must_use]
    pub fn copy_with(
        self,
        min_width: Option<f32>,
        max_width: Option<f32>,
        min_height: Option<f32>,
        max_height: Option<f32>,
    ) -> Self {
        Self::new(
            min_width.unwrap_or(self.min_width),
            max_width.unwrap_or(self.max_width),
            min_height.unwrap_or(self.min_height),
            max_height.unwrap_or(self.max_height),
        )
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

    /// Divides each bound and truncates toward zero, Flutter's BoxConstraints
    /// truncating division.
    ///
    /// # Panics
    /// Panics if the result is not valid finite-minimum layout bounds.
    #[must_use]
    pub fn truncating_div(self, factor: f32) -> Self {
        self.map_bounds(|value| {
            let divided = value / factor;
            assert!(
                divided.is_finite(),
                "truncating division requires finite results"
            );
            divided.trunc()
        })
    }

    fn map_bounds(self, map: impl Fn(f32) -> f32) -> Self {
        Self::new(
            map(self.min_width),
            map(self.max_width),
            map(self.min_height),
            map(self.max_height),
        )
    }

    /// Checked interpolation rejects mixed finite/infinite maxima before
    /// a tween can enter layout. Use trait Lerp for an animation that must
    /// continue with a valid endpoint instead of reporting an error.
    pub fn try_lerp(self, other: Self, t: f32) -> Result<Self, ConstraintError> {
        if !t.is_finite()
            || self.max_width.is_finite() != other.max_width.is_finite()
            || self.max_height.is_finite() != other.max_height.is_finite()
        {
            return Err(ConstraintError::Invalid);
        }
        let t = f64::from(t);
        let mix = |a: f32, b: f32| {
            if a == b {
                a
            } else {
                (f64::from(a) * (1. - t) + f64::from(b) * t) as f32
            }
        };
        Self::try_normalized(
            mix(self.min_width, other.min_width),
            mix(self.max_width, other.max_width),
            mix(self.min_height, other.min_height),
            mix(self.max_height, other.max_height),
        )
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

    /// Preserves the exact borrowed endpoint when both arguments refer
    /// to the same constraints, matching Flutter's identical fast path.
    #[must_use]
    pub fn lerp_ref<'a>(
        begin: Option<&'a Self>,
        end: Option<&'a Self>,
        t: f32,
    ) -> Option<Cow<'a, Self>> {
        if let (Some(a), Some(b)) = (begin, end)
            && std::ptr::eq(a, b)
        {
            return Some(Cow::Borrowed(a));
        }
        Self::lerp_optional(begin.copied(), end.copied(), t).map(Cow::Owned)
    }

    fn scale_for_lerp(self, factor: f32) -> Self {
        if !factor.is_finite() {
            return self;
        }
        if factor <= 0. {
            return Self::tight(Size::ZERO);
        }
        Self::try_normalized(
            self.min_width * factor,
            self.max_width * factor,
            self.min_height * factor,
            self.max_height * factor,
        )
        .unwrap_or(self)
    }

    /// Normalizes input bounds like Flutter's `BoxConstraints.normalize`:
    /// each minimum is at least zero and each maximum is raised to that
    /// minimum if necessary. As with `try_new`, NaN and positive-infinite
    /// minimums are not layout constraints.
    pub fn try_normalized(
        min_width: f32,
        max_width: f32,
        min_height: f32,
        max_height: f32,
    ) -> Result<Self, ConstraintError> {
        if [min_width, max_width, min_height, max_height]
            .into_iter()
            .any(f32::is_nan)
        {
            return Err(ConstraintError::Invalid);
        }
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

/// Scales every bound. Arithmetic is checked at the layout boundary:
/// NaN, infinite minima, inverted results, and other invalid constraints
/// panic rather than entering the renderer.
impl Mul<f32> for Constraints {
    type Output = Self;

    fn mul(self, factor: f32) -> Self {
        self.map_bounds(|value| value * factor)
    }
}

/// Divides every bound, rejecting a result that cannot be laid out.
impl Div<f32> for Constraints {
    type Output = Self;

    fn div(self, factor: f32) -> Self {
        self.map_bounds(|value| value / factor)
    }
}

/// Applies Dart's Euclidean remainder to every bound; an inverted result
/// remains invalid for Incular layout.
impl Rem<f32> for Constraints {
    type Output = Self;

    fn rem(self, rhs: f32) -> Self {
        self.map_bounds(|value| value.rem_euclid(rhs))
    }
}

/// Interpolates and normalizes bounds like Flutter's BoxConstraints.lerp.
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
        self.try_lerp(*other, t)
            .unwrap_or_else(|_| if t < 0.5 { *self } else { *other })
    }
}
