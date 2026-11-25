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

    /// Interpolates durations in microseconds, treating a missing duration
    /// as zero only between the endpoints. Curve overrides currently choose
    /// their endpoint at the midpoint; their samples do not blend yet.
    #[must_use]
    pub fn lerp(a: Option<Self>, b: Option<Self>, t: f32) -> Option<Self> {
        if a == b {
            return a;
        }
        let start = a.unwrap_or_default();
        let end = b.unwrap_or_default();
        let curves = if t < 0.5 { start } else { end };
        Some(Self {
            duration: lerp_duration(start.duration, end.duration, t),
            reverse_duration: lerp_duration(start.reverse_duration, end.reverse_duration, t),
            ..curves
        })
    }
}

fn lerp_duration(a: Option<Duration>, b: Option<Duration>, t: f32) -> Option<Duration> {
    if a == b || t == 0. {
        return a;
    }
    if t == 1. {
        return b;
    }
    let micros = |duration: Option<Duration>| duration.map_or(0, |value| value.as_micros()) as f64;
    let t = f64::from(t);
    let value = (micros(a) * (1. - t) + micros(b) * t).round();
    // Flutter durations are signed; Rust's Duration saturates at zero if
    // extrapolation crosses there. Use u128 so its seconds range survives
    // conversion to microseconds.
    let value = (value as u128).min(Duration::MAX.as_micros());
    let seconds = (value / 1_000_000) as u64;
    let nanos = (value % 1_000_000) as u32 * 1_000;
    Some(Duration::new(seconds, nanos))
}
