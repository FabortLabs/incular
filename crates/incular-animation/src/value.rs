//! Composable and implicit animation values.

use crate::{AnimationController, Curve, Tween, TweenSequence, TweenValue};
use incular_core::time::Instant;
use std::{marker::PhantomData, time::Duration};

/// A value that can be sampled from normalized animation progress.
pub trait Animatable<T> {
    #[must_use]
    fn value_at(&self, progress: f32) -> T;
}

impl<T: Tween> Animatable<T> for TweenValue<T> {
    fn value_at(&self, progress: f32) -> T {
        self.evaluate(progress)
    }
}

/// A small value graph node useful when a caller needs to compose constants,
/// tweens, and weighted sequences before attaching them to a controller.
#[derive(Clone, Debug, PartialEq)]
pub enum AnimationValue<T: Tween> {
    Constant(T),
    Tween(TweenValue<T>),
    Sequence(TweenSequence<T>),
}

impl<T: Tween> AnimationValue<T> {
    #[must_use]
    pub const fn constant(value: T) -> Self {
        Self::Constant(value)
    }

    #[must_use]
    pub const fn tween(begin: T, end: T) -> Self {
        Self::Tween(TweenValue::new(begin, end))
    }

    #[must_use]
    pub const fn from_tween(tween: TweenValue<T>) -> Self {
        Self::Tween(tween)
    }

    #[must_use]
    pub const fn sequence(sequence: TweenSequence<T>) -> Self {
        Self::Sequence(sequence)
    }

    #[must_use]
    pub fn evaluate(&self, progress: f32) -> Option<T> {
        match self {
            Self::Constant(value) => Some(*value),
            Self::Tween(tween) => Some(tween.evaluate(progress)),
            Self::Sequence(sequence) => sequence.evaluate(progress),
        }
    }
}

/// A typed animation driven by an [`AnimationController`]. Its value is
/// sampled lazily, so multiple consumers can share one controller without
/// duplicating interpolation state.
#[derive(Clone)]
pub struct Animation<T: Tween> {
    controller: AnimationController,
    tween: TweenValue<T>,
}

impl<T: Tween> Animation<T> {
    #[must_use]
    pub const fn new(controller: AnimationController, tween: TweenValue<T>) -> Self {
        Self { controller, tween }
    }

    #[must_use]
    pub fn from_to(begin: T, end: T, duration: Duration, curve: Curve) -> Self {
        Self {
            controller: AnimationController::new(duration),
            tween: TweenValue::curved(begin, end, curve),
        }
    }

    #[must_use]
    pub fn controller(&self) -> AnimationController {
        self.controller.clone()
    }

    #[must_use]
    pub const fn tween(&self) -> TweenValue<T> {
        self.tween
    }

    pub fn set_tween(&mut self, tween: TweenValue<T>) {
        self.tween = tween;
    }

    #[must_use]
    pub fn value(&self) -> T {
        self.tween.evaluate(self.controller.value())
    }

    #[must_use]
    pub fn value_at(&self, progress: f32) -> T {
        self.tween.evaluate(progress)
    }

    pub fn forward(&self, now: Instant) {
        self.controller.forward(now);
    }

    pub fn reverse(&self, now: Instant) {
        self.controller.reverse(now);
    }

    pub fn tick(&self, now: Instant) -> bool {
        self.controller.tick(now)
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.controller.is_active()
    }

    /// Maps this animation into another type without creating another timing
    /// source. The mapper runs only when the mapped value is sampled.
    #[must_use]
    pub fn map<U, F>(self, mapper: F) -> MappedAnimation<T, U, F>
    where
        F: Fn(T) -> U,
    {
        MappedAnimation {
            source: self,
            mapper,
            marker: PhantomData,
        }
    }
}

impl<T: Tween> Animatable<T> for Animation<T> {
    fn value_at(&self, progress: f32) -> T {
        self.value_at(progress)
    }
}

/// A lazily mapped animation node.
pub struct MappedAnimation<T: Tween, U, F: Fn(T) -> U> {
    source: Animation<T>,
    mapper: F,
    marker: PhantomData<fn() -> U>,
}

impl<T: Tween, U, F: Fn(T) -> U> MappedAnimation<T, U, F> {
    #[must_use]
    pub fn value(&self) -> U {
        (self.mapper)(self.source.value())
    }

    #[must_use]
    pub fn value_at(&self, progress: f32) -> U {
        (self.mapper)(self.source.value_at(progress))
    }

    pub fn tick(&self, now: Instant) -> bool {
        self.source.tick(now)
    }

    pub fn forward(&self, now: Instant) {
        self.source.forward(now);
    }

    #[must_use]
    pub fn controller(&self) -> AnimationController {
        self.source.controller()
    }
}

/// An animation that retargets from its current value, providing the common
/// implicit-animation behavior without a widget dependency.
#[derive(Clone)]
pub struct ImplicitAnimation<T: Tween> {
    controller: AnimationController,
    begin: T,
    end: T,
    curve: Curve,
}

impl<T: Tween> ImplicitAnimation<T> {
    #[must_use]
    pub fn new(value: T, duration: Duration) -> Self {
        Self {
            controller: AnimationController::new(duration),
            begin: value,
            end: value,
            curve: Curve::Linear,
        }
    }

    #[must_use]
    pub fn with_curve(value: T, duration: Duration, curve: Curve) -> Self {
        Self {
            controller: AnimationController::new(duration),
            begin: value,
            end: value,
            curve,
        }
    }

    #[must_use]
    pub fn controller(&self) -> AnimationController {
        self.controller.clone()
    }

    #[must_use]
    pub const fn begin(&self) -> T {
        self.begin
    }

    #[must_use]
    pub const fn end(&self) -> T {
        self.end
    }

    #[must_use]
    pub const fn curve(&self) -> Curve {
        self.curve
    }

    pub fn set_target(&mut self, now: Instant, target: T) {
        self.begin = self.value();
        self.end = target;
        self.controller.set_curve(self.curve);
        self.controller.set_value(0.0);
        self.controller.forward(now);
    }

    pub fn set_curve(&mut self, curve: Curve) {
        self.curve = curve;
        self.controller.set_curve(curve);
    }

    #[must_use]
    pub fn value(&self) -> T {
        T::interpolate(
            self.begin,
            self.end,
            self.curve.apply(self.controller.value()),
        )
    }

    pub fn tick(&self, now: Instant) -> bool {
        self.controller.tick(now)
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.controller.is_active()
    }
}

/// Compatibility aliases for callers that prefer a value-oriented name.
pub type AnimatedValue<T> = Animation<T>;
pub type ImplicitAnimatedValue<T> = ImplicitAnimation<T>;

/// Flutter's `Tween<T>` for any [`Lerp`](incular_core::Lerp) value, such as
/// insets, alignments, decorations or text styles. `Tween`/[`TweenValue`]
/// remain the copyable scalar path; this one clones its endpoints.
///
/// The named Flutter tweens (`EdgeInsetsTween`, `DecorationTween`, ...) are
/// aliases of this type in the crates that own their value types.
#[derive(Clone, Debug, PartialEq)]
pub struct LerpTween<T: incular_core::Lerp + Clone> {
    pub begin: T,
    pub end: T,
    pub curve: Curve,
}

impl<T: incular_core::Lerp + Clone> LerpTween<T> {
    #[must_use]
    pub const fn new(begin: T, end: T) -> Self {
        Self {
            begin,
            end,
            curve: Curve::Linear,
        }
    }

    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// Flutter's `Tween.transform`: `begin` at 0 and `end` at 1 exactly,
    /// the endpoints' `lerp` in between. The curve is not applied.
    #[must_use]
    pub fn transform(&self, t: f32) -> T {
        if t == 0.0 {
            self.begin.clone()
        } else if t == 1.0 {
            self.end.clone()
        } else {
            self.begin.lerp(&self.end, t)
        }
    }

    /// The value at `progress` through the tween's curve.
    #[must_use]
    pub fn evaluate(&self, progress: f32) -> T {
        self.transform(self.curve.apply(progress))
    }
}

impl<T: incular_core::Lerp + Clone> Animatable<T> for LerpTween<T> {
    fn value_at(&self, progress: f32) -> T {
        self.evaluate(progress)
    }
}

/// Flutter's `ColorTween`.
pub type ColorTween = TweenValue<incular_core::Color>;
/// Flutter's `SizeTween`.
pub type SizeTween = TweenValue<incular_core::Size>;
/// Flutter's `IntTween`.
pub type IntTween = TweenValue<i32>;
/// Flutter's `RectTween`.
pub type RectTween = LerpTween<incular_core::Rect>;
/// Flutter's `Matrix4Tween`.
pub type Matrix4Tween = LerpTween<incular_core::Transform>;
