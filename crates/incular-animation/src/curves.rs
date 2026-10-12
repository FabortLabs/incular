//! Timing curves used by explicit and implicit animations.

/// A normalized timing function. Curves clamp input to `[0, 1]` and map the
/// endpoints exactly. Overshooting curves may return values outside `[0, 1]`,
/// as Flutter's curves do.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Curve {
    #[default]
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInQuint,
    EaseOutQuint,
    EaseInOutQuint,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInCirc,
    EaseOutCirc,
    EaseInOutCirc,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
    FastOutSlowIn,
    CubicBezier {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    StepStart,
    StepEnd,
}

impl Curve {
    #[must_use]
    pub fn apply(self, value: f32) -> f32 {
        let t = if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else if value.is_sign_negative() {
            0.0
        } else {
            1.0
        };
        if t == 0.0 || t == 1.0 {
            return t;
        }
        let pi = std::f32::consts::PI;
        match self {
            Self::Linear => t,
            Self::Ease => cubic_bezier(0.25, 0.1, 0.25, 1.0, t),
            Self::EaseIn => cubic_bezier(0.42, 0.0, 1.0, 1.0, t),
            Self::EaseOut => cubic_bezier(0.0, 0.0, 0.58, 1.0, t),
            Self::EaseInOut => cubic_bezier(0.42, 0.0, 0.58, 1.0, t),
            Self::EaseInSine => cubic_bezier(0.47, 0.0, 0.745, 0.715, t),
            Self::EaseOutSine => cubic_bezier(0.39, 0.575, 0.565, 1.0, t),
            Self::EaseInOutSine => cubic_bezier(0.445, 0.05, 0.55, 0.95, t),
            Self::EaseInQuad => cubic_bezier(0.55, 0.085, 0.68, 0.53, t),
            Self::EaseOutQuad => cubic_bezier(0.25, 0.46, 0.45, 0.94, t),
            Self::EaseInOutQuad => cubic_bezier(0.455, 0.03, 0.515, 0.955, t),
            Self::EaseInCubic => cubic_bezier(0.55, 0.055, 0.675, 0.19, t),
            Self::EaseOutCubic => cubic_bezier(0.215, 0.61, 0.355, 1.0, t),
            Self::EaseInOutCubic => cubic_bezier(0.645, 0.045, 0.355, 1.0, t),
            Self::EaseInQuart => cubic_bezier(0.895, 0.03, 0.685, 0.22, t),
            Self::EaseOutQuart => cubic_bezier(0.165, 0.84, 0.44, 1.0, t),
            Self::EaseInOutQuart => cubic_bezier(0.77, 0.0, 0.175, 1.0, t),
            Self::EaseInQuint => cubic_bezier(0.755, 0.05, 0.855, 0.06, t),
            Self::EaseOutQuint => cubic_bezier(0.23, 1.0, 0.32, 1.0, t),
            Self::EaseInOutQuint => cubic_bezier(0.86, 0.0, 0.07, 1.0, t),
            Self::EaseInExpo => cubic_bezier(0.95, 0.05, 0.795, 0.035, t),
            Self::EaseOutExpo => cubic_bezier(0.19, 1.0, 0.22, 1.0, t),
            Self::EaseInOutExpo => cubic_bezier(1.0, 0.0, 0.0, 1.0, t),
            Self::EaseInCirc => cubic_bezier(0.6, 0.04, 0.98, 0.335, t),
            Self::EaseOutCirc => cubic_bezier(0.075, 0.82, 0.165, 1.0, t),
            Self::EaseInOutCirc => cubic_bezier(0.785, 0.135, 0.15, 0.86, t),
            Self::EaseInBack => cubic_bezier(0.6, -0.28, 0.735, 0.045, t),
            Self::EaseOutBack => cubic_bezier(0.175, 0.885, 0.32, 1.275, t),
            Self::EaseInOutBack => cubic_bezier(0.68, -0.55, 0.265, 1.55, t),
            // Flutter ElasticInCurve(0.4): animation/curves.dart.
            Self::EaseInElastic => {
                let s = 0.4_f32 / 4.0;
                let t = t - 1.0;
                -(2.0f32).powf(10.0 * t) * ((t - s) * (pi * 2.0) / 0.4).sin()
            }
            // Flutter ElasticOutCurve(0.4).
            Self::EaseOutElastic => {
                let s = 0.4_f32 / 4.0;
                (2.0f32).powf(-10.0 * t) * ((t - s) * (pi * 2.0) / 0.4).sin() + 1.0
            }
            // Flutter ElasticInOutCurve(0.4).
            Self::EaseInOutElastic => {
                let s = 0.4_f32 / 4.0;
                let t = 2.0 * t - 1.0;
                if t < 0.0 {
                    -0.5 * (2.0f32).powf(10.0 * t) * ((t - s) * (pi * 2.0) / 0.4).sin()
                } else {
                    (2.0f32).powf(-10.0 * t) * ((t - s) * (pi * 2.0) / 0.4).sin() * 0.5 + 1.0
                }
            }
            Self::EaseInBounce => 1.0 - bounce_out(1.0 - t),
            Self::EaseOutBounce => bounce_out(t),
            Self::EaseInOutBounce => {
                if t < 0.5 {
                    (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
                }
            }
            Self::FastOutSlowIn => cubic_bezier(0.4, 0.0, 0.2, 1.0, t),
            Self::CubicBezier { x1, y1, x2, y2 } => cubic_bezier(x1, y1, x2, y2, t),
            Self::StepStart => {
                if t <= 0.0 {
                    0.0
                } else {
                    1.0
                }
            }
            Self::StepEnd => {
                if t >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    #[must_use]
    pub const fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self::CubicBezier { x1, y1, x2, y2 }
    }

    #[must_use]
    pub const fn reverse(self) -> Self {
        match self {
            Self::Linear => Self::Linear,
            Self::Ease => Self::Ease,
            Self::EaseIn => Self::EaseOut,
            Self::EaseOut => Self::EaseIn,
            Self::EaseInOut => Self::EaseInOut,
            Self::EaseInSine => Self::EaseOutSine,
            Self::EaseOutSine => Self::EaseInSine,
            Self::EaseInOutSine => Self::EaseInOutSine,
            Self::EaseInQuad => Self::EaseOutQuad,
            Self::EaseOutQuad => Self::EaseInQuad,
            Self::EaseInOutQuad => Self::EaseInOutQuad,
            Self::EaseInCubic => Self::EaseOutCubic,
            Self::EaseOutCubic => Self::EaseInCubic,
            Self::EaseInOutCubic => Self::EaseInOutCubic,
            Self::EaseInQuart => Self::EaseOutQuart,
            Self::EaseOutQuart => Self::EaseInQuart,
            Self::EaseInOutQuart => Self::EaseInOutQuart,
            Self::EaseInQuint => Self::EaseOutQuint,
            Self::EaseOutQuint => Self::EaseInQuint,
            Self::EaseInOutQuint => Self::EaseInOutQuint,
            Self::EaseInExpo => Self::EaseOutExpo,
            Self::EaseOutExpo => Self::EaseInExpo,
            Self::EaseInOutExpo => Self::EaseInOutExpo,
            Self::EaseInCirc => Self::EaseOutCirc,
            Self::EaseOutCirc => Self::EaseInCirc,
            Self::EaseInOutCirc => Self::EaseInOutCirc,
            Self::EaseInBack => Self::EaseOutBack,
            Self::EaseOutBack => Self::EaseInBack,
            Self::EaseInOutBack => Self::EaseInOutBack,
            Self::EaseInElastic => Self::EaseOutElastic,
            Self::EaseOutElastic => Self::EaseInElastic,
            Self::EaseInOutElastic => Self::EaseInOutElastic,
            Self::EaseInBounce => Self::EaseOutBounce,
            Self::EaseOutBounce => Self::EaseInBounce,
            Self::EaseInOutBounce => Self::EaseInOutBounce,
            Self::FastOutSlowIn => Self::FastOutSlowIn,
            Self::CubicBezier { x1, y1, x2, y2 } => Self::CubicBezier {
                x1: 1.0 - x2,
                y1: 1.0 - y2,
                x2: 1.0 - x1,
                y2: 1.0 - y1,
            },
            Self::StepStart => Self::StepEnd,
            Self::StepEnd => Self::StepStart,
        }
    }

    #[must_use]
    pub const fn then(self, next: Self) -> CurveChain {
        CurveChain { first: self, next }
    }
}

fn bounce_out(mut t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        t -= 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        t -= 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        t -= 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

/// Sequential curve composition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveChain {
    first: Curve,
    next: Curve,
}

impl CurveChain {
    #[must_use]
    pub fn apply(self, value: f32) -> f32 {
        let value = if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else if value.is_sign_negative() {
            0.0
        } else {
            1.0
        };
        if value < 0.5 {
            self.first.apply(value * 2.0) * 0.5
        } else {
            0.5 + self.next.apply((value - 0.5) * 2.0) * 0.5
        }
    }
}

/// Named curve constants, mirroring Flutter's `Curves` namespace.
pub struct Curves;

impl Curves {
    pub const LINEAR: Curve = Curve::Linear;
    pub const EASE: Curve = Curve::Ease;
    pub const EASE_IN: Curve = Curve::EaseIn;
    pub const EASE_OUT: Curve = Curve::EaseOut;
    pub const EASE_IN_OUT: Curve = Curve::EaseInOut;
    pub const EASE_IN_SINE: Curve = Curve::EaseInSine;
    pub const EASE_OUT_SINE: Curve = Curve::EaseOutSine;
    pub const EASE_IN_OUT_SINE: Curve = Curve::EaseInOutSine;
    pub const EASE_IN_QUAD: Curve = Curve::EaseInQuad;
    pub const EASE_OUT_QUAD: Curve = Curve::EaseOutQuad;
    pub const EASE_IN_OUT_QUAD: Curve = Curve::EaseInOutQuad;
    pub const EASE_IN_CUBIC: Curve = Curve::EaseInCubic;
    pub const EASE_OUT_CUBIC: Curve = Curve::EaseOutCubic;
    pub const EASE_IN_OUT_CUBIC: Curve = Curve::EaseInOutCubic;
    pub const EASE_IN_QUART: Curve = Curve::EaseInQuart;
    pub const EASE_OUT_QUART: Curve = Curve::EaseOutQuart;
    pub const EASE_IN_OUT_QUART: Curve = Curve::EaseInOutQuart;
    pub const EASE_IN_QUINT: Curve = Curve::EaseInQuint;
    pub const EASE_OUT_QUINT: Curve = Curve::EaseOutQuint;
    pub const EASE_IN_OUT_QUINT: Curve = Curve::EaseInOutQuint;
    pub const EASE_IN_EXPO: Curve = Curve::EaseInExpo;
    pub const EASE_OUT_EXPO: Curve = Curve::EaseOutExpo;
    pub const EASE_IN_OUT_EXPO: Curve = Curve::EaseInOutExpo;
    pub const EASE_IN_CIRC: Curve = Curve::EaseInCirc;
    pub const EASE_OUT_CIRC: Curve = Curve::EaseOutCirc;
    pub const EASE_IN_OUT_CIRC: Curve = Curve::EaseInOutCirc;
    pub const EASE_IN_BACK: Curve = Curve::EaseInBack;
    pub const EASE_OUT_BACK: Curve = Curve::EaseOutBack;
    pub const EASE_IN_OUT_BACK: Curve = Curve::EaseInOutBack;
    pub const EASE_IN_ELASTIC: Curve = Curve::EaseInElastic;
    pub const EASE_OUT_ELASTIC: Curve = Curve::EaseOutElastic;
    pub const EASE_IN_OUT_ELASTIC: Curve = Curve::EaseInOutElastic;
    pub const EASE_IN_BOUNCE: Curve = Curve::EaseInBounce;
    pub const EASE_OUT_BOUNCE: Curve = Curve::EaseOutBounce;
    pub const EASE_IN_OUT_BOUNCE: Curve = Curve::EaseInOutBounce;
    pub const BOUNCE_OUT: Curve = Curve::EaseOutBounce;
    pub const BOUNCE_IN: Curve = Curve::EaseInBounce;
    pub const BOUNCE_IN_OUT: Curve = Curve::EaseInOutBounce;
    pub const ELASTIC_OUT: Curve = Curve::EaseOutElastic;
    pub const ELASTIC_IN: Curve = Curve::EaseInElastic;
    pub const ELASTIC_IN_OUT: Curve = Curve::EaseInOutElastic;
    pub const FAST_OUT_SLOW_IN: Curve = Curve::FastOutSlowIn;
}

/// Evaluates a cubic timing curve `Cubic(x1, y1, x2, y2)` at `target` as
/// Flutter's `Cubic.transformInternal` does: bisects the curve parameter until
/// the x coordinate is within `0.001` of `target`, then returns y.
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, target: f32) -> f32 {
    const ERROR_BOUND: f32 = 0.001;
    let mut start = 0.0_f32;
    let mut end = 1.0_f32;
    loop {
        let midpoint = (start + end) / 2.0;
        let estimate = evaluate_cubic(x1, x2, midpoint);
        if (target - estimate).abs() < ERROR_BOUND {
            return evaluate_cubic(y1, y2, midpoint);
        }
        // The interval can no longer shrink (only reachable for non-finite
        // control points), so return the current sample instead of spinning.
        if midpoint == start || midpoint == end {
            return evaluate_cubic(y1, y2, midpoint);
        }
        if estimate < target {
            start = midpoint;
        } else {
            end = midpoint;
        }
    }
}

fn evaluate_cubic(a: f32, b: f32, m: f32) -> f32 {
    3.0 * a * (1.0 - m) * (1.0 - m) * m + 3.0 * b * (1.0 - m) * m * m + m * m * m
}
