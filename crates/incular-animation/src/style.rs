//! Flutter's `AnimationStyle` (`animation/animation_style.dart`).

use crate::Curve;
use std::time::Duration;

/// Flutter `AnimationStyle`: overrides for the curve and duration of a
/// widget's built-in animations, forward and in reverse. An unset field
/// keeps the widget's default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimationStyle {
    pub curve: Option<Curve>,
    pub duration: Option<Duration>,
    pub reverse_curve: Option<Curve>,
    pub reverse_duration: Option<Duration>,
}

impl AnimationStyle {
    /// Flutter `AnimationStyle.noAnimation`: zero durations both ways.
    pub const NO_ANIMATION: Self = Self {
        curve: None,
        duration: Some(Duration::ZERO),
        reverse_curve: None,
        reverse_duration: Some(Duration::ZERO),
    };

    #[must_use]
    pub const fn new() -> Self {
        Self {
            curve: None,
            duration: None,
            reverse_curve: None,
            reverse_duration: None,
        }
    }

    #[must_use]
    pub const fn curve(mut self, curve: Curve) -> Self {
        self.curve = Some(curve);
        self
    }

    #[must_use]
    pub const fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    #[must_use]
    pub const fn reverse_curve(mut self, curve: Curve) -> Self {
        self.reverse_curve = Some(curve);
        self
    }

    #[must_use]
    pub const fn reverse_duration(mut self, duration: Duration) -> Self {
        self.reverse_duration = Some(duration);
        self
    }

    /// Flutter `AnimationStyle.copyWith`: `other`'s set fields win.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            curve: other.curve.or(self.curve),
            duration: other.duration.or(self.duration),
            reverse_curve: other.reverse_curve.or(self.reverse_curve),
            reverse_duration: other.reverse_duration.or(self.reverse_duration),
        }
    }

    /// Flutter `AnimationStyle.lerp`: the fields switch from `a` to `b` at
    /// the midpoint.
    #[must_use]
    pub fn lerp(a: Option<Self>, b: Option<Self>, t: f32) -> Option<Self> {
        if a == b {
            return a;
        }
        let pick = if t < 0.5 { a } else { b };
        Some(pick.unwrap_or_default())
    }
}
