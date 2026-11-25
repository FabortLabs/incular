//! Generic interpolation and tween composition.

use crate::Curve;
use incular_core::{Color, Offset, Rect, Size};

/// Values that can be interpolated by an animation. The trait remains named
/// `Tween` for compatibility with the original crate API.
pub trait Tween: Copy {
    fn interpolate(from: Self, to: Self, t: f32) -> Self;

    /// Interpolates without clamping `t`, so overshooting curves extrapolate.
    /// Returns exactly `from` at 0 and exactly `to` at 1, as Flutter's
    /// `Animatable.transform` does.
    #[must_use]
    fn lerp(from: Self, to: Self, t: f32) -> Self {
        if t == 0.0 {
            from
        } else if t == 1.0 {
            to
        } else {
            Self::interpolate(from, to, t)
        }
    }
}

impl Tween for f32 {
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        from + (to - from) * t
    }
}

impl Tween for f64 {
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        from + (to - from) * f64::from(t)
    }
}

impl Tween for i32 {
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        let from = f64::from(from);
        (from + (f64::from(to) - from) * f64::from(t)).round() as i32
    }
}

impl Tween for u32 {
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        let from = f64::from(from);
        (from + (f64::from(to) - from) * f64::from(t)).round() as u32
    }
}

impl Tween for Offset {
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        Offset::new(
            f32::interpolate(from.x, to.x, t),
            f32::interpolate(from.y, to.y, t),
        )
    }
}

impl Tween for Size {
    /// Extrapolates without clamping, as Flutter's `Size.lerp` does, so an
    /// overshooting curve yields a negative extent. Layout code that needs a
    /// real size clamps it at that point.
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        Size {
            width: f32::interpolate(from.width, to.width, t),
            height: f32::interpolate(from.height, to.height, t),
        }
    }
}

impl Tween for Color {
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        let mix = |left: u8, right: u8| {
            (f32::from(left) + (f32::from(right) - f32::from(left)) * t).round() as u8
        };
        Color::rgba(
            mix(from.red, to.red),
            mix(from.green, to.green),
            mix(from.blue, to.blue),
            mix(from.alpha, to.alpha),
        )
    }
}

impl Tween for Rect {
    /// Extrapolates without clamping, as Flutter's `Rect.lerp` does: the
    /// origin and the size each interpolate, so every side does.
    fn interpolate(from: Self, to: Self, t: f32) -> Self {
        Rect::from_origin_size(
            Offset::interpolate(from.origin, to.origin, t),
            Size::interpolate(from.size, to.size, t),
        )
    }
}

/// A value whose absent (null) form is defined relative to a present one,
/// for Flutter's nullable `ColorTween`, `SizeTween` and `RectTween` ends.
pub trait NullableTween: Tween {
    /// The value an absent endpoint stands for when the other endpoint is
    /// `present`: a transparent color with the same channels, a zero size,
    /// or a zero rectangle. Interpolating towards it scales `present`, as
    /// Flutter's `Color.lerp`, `Size.lerp` and `Rect.lerp` do with a null end.
    #[must_use]
    fn absent_like(present: Self) -> Self;

    /// Some values define absent-end interpolation as a distinct operation
    /// (for example, scaling each side of a border without blending it).
    #[must_use]
    fn interpolate_optional(begin: Option<Self>, end: Option<Self>, t: f32) -> Option<Self> {
        match (begin, end) {
            (None, None) => None,
            (Some(begin), Some(end)) => Some(Self::lerp(begin, end, t)),
            (Some(begin), None) => Some(Self::lerp(begin, Self::absent_like(begin), t)),
            (None, Some(end)) => Some(Self::lerp(Self::absent_like(end), end, t)),
        }
    }
}

impl NullableTween for Color {
    fn absent_like(present: Self) -> Self {
        Color::rgba(present.red, present.green, present.blue, 0)
    }
}

impl NullableTween for Size {
    fn absent_like(_present: Self) -> Self {
        Size::new(0.0, 0.0)
    }
}

impl NullableTween for Rect {
    fn absent_like(_present: Self) -> Self {
        Rect::from_origin_size(Offset::ZERO, Size::new(0.0, 0.0))
    }
}

/// A tween whose endpoints may be absent (null), as Flutter's `Tween<T?>`
/// subclasses are. Both ends absent yields no value; one absent end scales
/// the present one, as Flutter's `Color.lerp`, `Size.lerp` and `Rect.lerp`
/// do with a null argument.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptionalTween<T: NullableTween> {
    pub begin: Option<T>,
    pub end: Option<T>,
    pub curve: Curve,
}

impl<T: NullableTween> Default for OptionalTween<T> {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl<T: NullableTween> OptionalTween<T> {
    #[must_use]
    pub const fn new(begin: Option<T>, end: Option<T>) -> Self {
        Self {
            begin,
            end,
            curve: Curve::Linear,
        }
    }

    #[must_use]
    pub const fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// The value at `t` without the curve: `None` only when both ends are
    /// absent. Exactly `begin` at 0 and `end` at 1 for present ends.
    #[must_use]
    pub fn lerp(&self, t: f32) -> Option<T> {
        T::interpolate_optional(self.begin, self.end, t)
    }

    /// Flutter's `Tween.transform` without applying the curve. Exact
    /// endpoints retain their original absence.
    #[must_use]
    pub fn transform(&self, t: f32) -> Option<T> {
        if t == 0. {
            self.begin
        } else if t == 1. {
            self.end
        } else {
            self.lerp(t)
        }
    }

    /// The value at `progress` through the tween's curve.
    #[must_use]
    pub fn evaluate(&self, progress: f32) -> Option<T> {
        self.transform(self.curve.apply(progress))
    }
}

impl<T: NullableTween> crate::Animatable<Option<T>> for OptionalTween<T> {
    fn value_at(&self, progress: f32) -> Option<T> {
        self.evaluate(progress)
    }
}

/// Flutter's nullable `ColorTween`.
pub type OptionalColorTween = OptionalTween<Color>;
/// Flutter's nullable `SizeTween`.
pub type OptionalSizeTween = OptionalTween<Size>;
/// Flutter's nullable `RectTween`.
pub type OptionalRectTween = OptionalTween<Rect>;

/// A typed interval with an optional timing curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TweenValue<T: Tween> {
    pub begin: T,
    pub end: T,
    pub curve: Curve,
}

impl<T: Tween> TweenValue<T> {
    #[must_use]
    pub const fn new(begin: T, end: T) -> Self {
        Self {
            begin,
            end,
            curve: Curve::Linear,
        }
    }

    #[must_use]
    pub const fn curved(begin: T, end: T, curve: Curve) -> Self {
        Self { begin, end, curve }
    }

    #[must_use]
    pub const fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    #[must_use]
    pub fn evaluate(self, progress: f32) -> T {
        T::lerp(self.begin, self.end, self.curve.apply(progress))
    }

    #[must_use]
    pub fn lerp(self, progress: f32) -> T {
        T::lerp(self.begin, self.end, progress)
    }

    /// Plays the tween backwards in time: swaps the endpoints and mirrors the
    /// curve. This matches Flutter's reversed `CurvedAnimation` with no
    /// `reverseCurve`, where the value is `curve(1 - elapsed)`.
    #[must_use]
    pub fn reversed(self) -> Self {
        Self {
            begin: self.end,
            end: self.begin,
            curve: self.curve.reverse(),
        }
    }

    #[must_use]
    pub fn then(self, next: Self) -> TweenSequence<T> {
        TweenSequence::new().push(self, 1.0).push(next, 1.0)
    }
}

/// A normalized weighted sequence of typed tweens.
#[derive(Clone, Debug, PartialEq)]
pub struct TweenSequence<T: Tween> {
    segments: Vec<TweenSegment<T>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TweenSegment<T: Tween> {
    pub tween: TweenValue<T>,
    pub weight: f32,
}

impl<T: Tween> TweenSequence<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    #[must_use]
    pub fn push(mut self, tween: TweenValue<T>, weight: f32) -> Self {
        if weight.is_finite() && weight > 0.0 {
            self.segments.push(TweenSegment { tween, weight });
        }
        self
    }

    #[must_use]
    pub fn with_segment(&mut self, tween: TweenValue<T>, weight: f32) -> &mut Self {
        if weight.is_finite() && weight > 0.0 {
            self.segments.push(TweenSegment { tween, weight });
        }
        self
    }

    #[must_use]
    pub fn evaluate(&self, progress: f32) -> Option<T> {
        let first = self.segments.first()?;
        let total = self
            .segments
            .iter()
            .map(|segment| segment.weight)
            .sum::<f32>();
        if total <= 0.0 || !total.is_finite() {
            return Some(first.tween.evaluate(0.0));
        }
        let target = progress.clamp(0.0, 1.0) * total;
        let mut cursor = 0.0;
        for (index, segment) in self.segments.iter().enumerate() {
            let end = cursor + segment.weight;
            // Flutter's intervals are half-open: a boundary belongs to the
            // item that follows it. The last item also takes progress 1.
            if target < end || index + 1 == self.segments.len() {
                let local = if segment.weight == 0.0 {
                    0.0
                } else {
                    ((target - cursor) / segment.weight).clamp(0.0, 1.0)
                };
                return Some(segment.tween.evaluate(local));
            }
            cursor = end;
        }
        Some(first.tween.evaluate(0.0))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    #[must_use]
    pub fn segments(&self) -> &[TweenSegment<T>] {
        &self.segments
    }
}

impl TweenSequence<f32> {
    /// Flutter `FlippedTweenSequence`: this sequence played backwards and
    /// inverted, `1 - sequence(1 - t)`.
    #[must_use]
    pub fn flipped(self) -> FlippedTweenSequence {
        FlippedTweenSequence(self)
    }
}

/// Flutter `FlippedTweenSequence`: `1 - sequence(1 - t)`. It is an
/// [`Animatable`](crate::Animatable) of `f32`.
#[derive(Clone, Debug, PartialEq)]
pub struct FlippedTweenSequence(pub TweenSequence<f32>);

impl FlippedTweenSequence {
    /// The flipped value at `progress`, or `None` for an empty sequence.
    #[must_use]
    pub fn evaluate(&self, progress: f32) -> Option<f32> {
        self.0
            .evaluate(1.0 - progress.clamp(0.0, 1.0))
            .map(|value| 1.0 - value)
    }
}

impl<T: Tween> Default for TweenSequence<T> {
    fn default() -> Self {
        Self::new()
    }
}
