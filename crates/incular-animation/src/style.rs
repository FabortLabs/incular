//! Flutter's `AnimationStyle` (`animation/animation_style.dart`).

use crate::Curve;
use std::{borrow::Cow, fmt, time::Duration};

/// Flutter `AnimationStyle`: overrides for the curve and duration of a
/// widget's built-in animations, forward and in reverse. An unset field
/// keeps the widget's default.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct AnimationStyle {
    pub curve: Option<Curve>,
    pub duration: Option<Duration>,
    pub reverse_curve: Option<Curve>,
    pub reverse_duration: Option<Duration>,
}

/// One explicitly set AnimationStyle override, in diagnostic order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationStyleProperty<'a> {
    Curve(&'a Curve),
    Duration(Duration),
    ReverseCurve(&'a Curve),
    ReverseDuration(Duration),
}

impl fmt::Display for AnimationStyleProperty<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (name, duration) = match self {
            Self::Curve(curve) => return write!(f, "curve: {curve}"),
            Self::ReverseCurve(curve) => return write!(f, "reverseCurve: {curve}"),
            Self::Duration(duration) => ("duration", duration),
            Self::ReverseDuration(duration) => ("reverseDuration", duration),
        };
        let seconds = duration.as_secs();
        let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
        write!(
            f,
            "{name}: {hours}:{minutes:02}:{seconds:02}.{:06}",
            duration.subsec_micros()
        )
    }
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
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = Some(curve);
        self
    }

    #[must_use]
    pub const fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    #[must_use]
    pub fn reverse_curve(mut self, curve: Curve) -> Self {
        self.reverse_curve = Some(curve);
        self
    }

    #[must_use]
    pub const fn reverse_duration(mut self, duration: Duration) -> Self {
        self.reverse_duration = Some(duration);
        self
    }

    /// Diagnostic properties at the normal information level. Unset
    /// overrides are omitted; an explicit zero duration remains visible.
    pub fn diagnostic_properties(&self) -> impl Iterator<Item = AnimationStyleProperty<'_>> {
        [
            self.curve.as_ref().map(AnimationStyleProperty::Curve),
            self.duration.map(AnimationStyleProperty::Duration),
            self.reverse_curve.as_ref().map(AnimationStyleProperty::ReverseCurve),
            self.reverse_duration.map(AnimationStyleProperty::ReverseDuration),
        ]
        .into_iter()
        .flatten()
    }

    /// Copies this style, taking only the explicitly supplied overrides.
    #[must_use]
    pub fn copy_with(&self, overrides: Self) -> Self {
        self.clone().merge(overrides)
    }

    /// Merges two styles: `other`'s set fields win.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            curve: other.curve.or(self.curve),
            duration: other.duration.or(self.duration),
            reverse_curve: other.reverse_curve.or(self.reverse_curve),
            reverse_duration: other.reverse_duration.or(self.reverse_duration),
        }
    }

    /// Interpolates borrowed styles. A shared input style can pass through
    /// without cloning either curve or constructing a new override value.
    #[must_use]
    pub fn lerp_ref<'a>(a: Option<&'a Self>, b: Option<&'a Self>, t: f32) -> Option<Cow<'a, Self>> {
        if let (Some(left), Some(right)) = (a, b)
            && std::ptr::eq(left, right)
        {
            return Some(Cow::Borrowed(left));
        }
        Self::lerp(a.cloned(), b.cloned(), t).map(Cow::Owned)
    }

    /// Interpolates durations in microseconds, treating a missing duration
    /// as zero only between the endpoints. Blends evaluated curve outputs,
    /// using linear timing for a missing interior curve.
    #[must_use]
    pub fn lerp(a: Option<Self>, b: Option<Self>, t: f32) -> Option<Self> {
        if a == b {
            return a;
        }
        let start = a.unwrap_or_default();
        let end = b.unwrap_or_default();
        Some(Self {
            curve: lerp_curve(start.curve, end.curve, t),
            duration: lerp_duration(start.duration, end.duration, t),
            reverse_curve: lerp_curve(start.reverse_curve, end.reverse_curve, t),
            reverse_duration: lerp_duration(start.reverse_duration, end.reverse_duration, t),
        })
    }
}

fn lerp_curve(a: Option<Curve>, b: Option<Curve>, t: f32) -> Option<Curve> {
    if a == b || t == 0. {
        return a;
    }
    if t == 1. {
        return b;
    }
    Some(Curve::blend(
        a.unwrap_or_default(),
        b.unwrap_or_default(),
        t,
    ))
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
