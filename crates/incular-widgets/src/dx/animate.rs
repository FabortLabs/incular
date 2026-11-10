//! Declarative effect chains, after the `flutter_animate` package:
//! `Text::new("Hi").animate().fade_in().slide_y(0.2).then().scale(0.9)`.
//!
//! An [`Animate`] owns one animation controller spanning all its effects.
//! Each effect has a delay, a duration and a curve; by default an effect
//! starts with the previous one (they run together), and [`Animate::then`]
//! makes the following effects start after everything added so far ends,
//! as `flutter_animate`'s `then` does.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::{AnimationController, AnimationStatus, Curve};
use incular_core::{Lerp, Offset};

use crate::{
    FractionalTranslation, ImageFiltered, Opacity, Transform, Widget, tree::RetainedState,
};

/// `flutter_animate`'s `Animate.defaultDuration`.
pub const DEFAULT_EFFECT_DURATION: Duration = Duration::from_millis(300);
/// `Animate.defaultCurve`.
pub const DEFAULT_EFFECT_CURVE: Curve = Curve::Linear;

/// One animated property of an [`Animate`] chain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectKind {
    /// Opacity from `begin` to `end`.
    Fade { begin: f32, end: f32 },
    /// Translation as a fraction of the child's size (`SlideEffect`).
    Slide { begin: Offset, end: Offset },
    /// Translation in logical pixels (`MoveEffect`).
    Move { begin: Offset, end: Offset },
    /// Uniform scale about the center (`ScaleEffect`).
    Scale { begin: f32, end: f32 },
    /// Rotation in turns about the center (`RotateEffect`).
    Rotate { begin: f32, end: f32 },
    /// Gaussian blur sigma (`BlurEffect`).
    Blur { begin: f32, end: f32 },
    /// A sideways shake of `hz` oscillations peaking at `offset` pixels
    /// (`ShakeEffect`).
    Shake { hz: f32, offset: f32 },
}

/// An effect with its timing within the chain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    pub kind: EffectKind,
    pub delay: Duration,
    pub duration: Duration,
    pub curve: Curve,
}

impl Effect {
    fn end(&self) -> Duration {
        self.delay + self.duration
    }

    /// This effect's eased progress when the whole chain is at `elapsed`.
    fn progress(&self, elapsed: Duration) -> f32 {
        if elapsed <= self.delay {
            return 0.;
        }
        if self.duration.is_zero() {
            return 1.;
        }
        let t = (elapsed - self.delay).as_secs_f32() / self.duration.as_secs_f32();
        self.curve.apply(t.clamp(0., 1.))
    }
}

/// How an [`Animate`] repeats once it reaches the end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimateRepeat {
    /// Play once and hold the final frame.
    #[default]
    Once,
    /// Restart from the beginning forever.
    Loop,
    /// Play forward then backward forever.
    PingPong,
}

/// A chain of effects applied to a child, started when it is first built.
#[derive(Clone)]
pub struct Animate {
    child: Widget,
    effects: Vec<Effect>,
    /// Where the next effect starts.
    cursor: Duration,
    /// The timing the next effect inherits.
    duration: Duration,
    curve: Curve,
    delay: Duration,
    repeat: AnimateRepeat,
    on_complete: Option<Rc<dyn Fn()>>,
    target: Option<f32>,
}

impl Animate {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            effects: Vec::new(),
            cursor: Duration::ZERO,
            duration: DEFAULT_EFFECT_DURATION,
            curve: DEFAULT_EFFECT_CURVE,
            delay: Duration::ZERO,
            repeat: AnimateRepeat::Once,
            on_complete: None,
            target: None,
        }
    }

    /// The duration of effects added after this call.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// The duration in milliseconds of effects added after this call.
    #[must_use]
    pub fn ms(self, millis: u64) -> Self {
        self.duration(Duration::from_millis(millis))
    }

    /// The curve of effects added after this call.
    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// The whole chain waits `delay` before starting (`Animate.delay`).
    #[must_use]
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Adds an effect with the current timing, starting with the previous
    /// effect.
    #[must_use]
    pub fn effect(mut self, kind: EffectKind) -> Self {
        // An effect starts with the previous one, unless `then` moved the
        // cursor past it.
        let start = self
            .effects
            .last()
            .map_or(self.cursor, |last| last.delay.max(self.cursor));
        self.effects.push(Effect {
            kind,
            delay: start,
            duration: self.duration,
            curve: self.curve,
        });
        self
    }

    /// Adds an effect with explicit timing relative to the chain start.
    #[must_use]
    pub fn effect_at(
        mut self,
        kind: EffectKind,
        delay: Duration,
        duration: Duration,
        curve: Curve,
    ) -> Self {
        self.effects.push(Effect {
            kind,
            delay,
            duration,
            curve,
        });
        self
    }

    /// `flutter_animate`'s `then`: effects added next start after every
    /// effect so far has ended.
    #[must_use]
    pub fn then(mut self) -> Self {
        self.cursor = self.total_duration();
        self
    }

    #[must_use]
    pub fn fade_in(self) -> Self {
        self.effect(EffectKind::Fade { begin: 0., end: 1. })
    }

    #[must_use]
    pub fn fade_out(self) -> Self {
        self.effect(EffectKind::Fade { begin: 1., end: 0. })
    }

    #[must_use]
    pub fn fade(self, begin: f32, end: f32) -> Self {
        self.effect(EffectKind::Fade { begin, end })
    }

    /// Slides in horizontally from `begin` times the child's width.
    #[must_use]
    pub fn slide_x(self, begin: f32) -> Self {
        self.effect(EffectKind::Slide {
            begin: Offset::new(begin, 0.),
            end: Offset::ZERO,
        })
    }

    /// Slides in vertically from `begin` times the child's height.
    #[must_use]
    pub fn slide_y(self, begin: f32) -> Self {
        self.effect(EffectKind::Slide {
            begin: Offset::new(0., begin),
            end: Offset::ZERO,
        })
    }

    #[must_use]
    pub fn slide(self, begin: Offset, end: Offset) -> Self {
        self.effect(EffectKind::Slide { begin, end })
    }

    /// Moves from `begin` pixels to rest.
    #[must_use]
    pub fn move_from(self, begin: Offset) -> Self {
        self.effect(EffectKind::Move {
            begin,
            end: Offset::ZERO,
        })
    }

    /// Scales from `begin` to full size.
    #[must_use]
    pub fn scale_in(self, begin: f32) -> Self {
        self.effect(EffectKind::Scale { begin, end: 1. })
    }

    #[must_use]
    pub fn scale(self, begin: f32, end: f32) -> Self {
        self.effect(EffectKind::Scale { begin, end })
    }

    /// Rotates from `begin` turns to rest.
    #[must_use]
    pub fn rotate_in(self, begin: f32) -> Self {
        self.effect(EffectKind::Rotate { begin, end: 0. })
    }

    #[must_use]
    pub fn rotate(self, begin: f32, end: f32) -> Self {
        self.effect(EffectKind::Rotate { begin, end })
    }

    /// Unblurs from `sigma` to sharp.
    #[must_use]
    pub fn blur_in(self, sigma: f32) -> Self {
        self.effect(EffectKind::Blur {
            begin: sigma,
            end: 0.,
        })
    }

    /// `ShakeEffect` with `flutter_animate`'s defaults: 8 Hz, 5 pixels.
    #[must_use]
    pub fn shake(self) -> Self {
        self.effect(EffectKind::Shake { hz: 8., offset: 5. })
    }

    #[must_use]
    pub fn repeat(mut self, repeat: AnimateRepeat) -> Self {
        self.repeat = repeat;
        self
    }

    /// Runs when a non-repeating chain reaches its end.
    #[must_use]
    pub fn on_complete(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_complete = Some(Rc::new(callback));
        self
    }

    /// `Animate.target`: drives the chain to this progress (0 to 1) instead
    /// of playing it, animating whenever the target changes.
    #[must_use]
    pub fn target(mut self, target: f32) -> Self {
        self.target = Some(target.clamp(0., 1.));
        self
    }

    /// The effects with their final timing.
    #[must_use]
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    /// When the last effect ends.
    #[must_use]
    pub fn total_duration(&self) -> Duration {
        self.effects
            .iter()
            .map(Effect::end)
            .max()
            .unwrap_or(Duration::ZERO)
    }

    /// The child as it looks with the chain `elapsed` in.
    #[must_use]
    pub fn frame(&self, elapsed: Duration) -> Widget {
        let mut child = self.child.clone();
        for effect in &self.effects {
            let t = effect.progress(elapsed);
            child = match effect.kind {
                EffectKind::Fade { begin, end } => {
                    Opacity::new(begin.lerp(&end, t).clamp(0., 1.), child).into()
                }
                EffectKind::Slide { begin, end } => {
                    FractionalTranslation::new(begin.lerp(&end, t), child).into()
                }
                EffectKind::Move { begin, end } => {
                    Transform::translation(begin.lerp(&end, t), child).into()
                }
                EffectKind::Scale { begin, end } => {
                    Transform::scale(begin.lerp(&end, t), child).into()
                }
                EffectKind::Rotate { begin, end } => {
                    Transform::rotation(begin.lerp(&end, t) * std::f32::consts::TAU, child).into()
                }
                EffectKind::Blur { begin, end } => {
                    let sigma = begin.lerp(&end, t).max(0.);
                    if sigma == 0. {
                        child
                    } else {
                        ImageFiltered::blur(sigma, child).into()
                    }
                }
                EffectKind::Shake { hz, offset } => {
                    // `ShakeEffect`: sin(t * 2π * hz * seconds), fading out.
                    let phase = t * effect.duration.as_secs_f32() * hz * std::f32::consts::TAU;
                    let dx = phase.sin() * offset * (1. - t);
                    Transform::translation(Offset::new(dx, 0.), child).into()
                }
            };
        }
        child
    }
}

struct AnimateState {
    controller: AnimationController,
    listening: Cell<bool>,
    started: Cell<bool>,
    target: Cell<Option<f32>>,
    config: RefCell<Option<Rc<Animate>>>,
    revision: RefCell<Rc<Cell<u64>>>,
    forward: Cell<bool>,
}

impl RetainedState for AnimateState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl From<Animate> for Widget {
    fn from(animate: Animate) -> Self {
        let animate = Rc::new(animate);
        let total = animate.total_duration() + animate.delay;
        Widget::retained_builder(
            move |revision| AnimateState {
                controller: AnimationController::new(total),
                listening: Cell::new(false),
                started: Cell::new(false),
                target: Cell::new(None),
                config: RefCell::new(None),
                revision: RefCell::new(revision),
                forward: Cell::new(true),
            },
            move |state, _, _| {
                *state.config.borrow_mut() = Some(animate.clone());
                if !state.listening.replace(true) {
                    let weak = Rc::downgrade(state);
                    state.controller.add_listener(move |_| {
                        let Some(state) = weak.upgrade() else {
                            return;
                        };
                        let revision = state.revision.borrow().clone();
                        revision.set(revision.get().wrapping_add(1));
                        let config = state.config.borrow().clone();
                        let Some(config) = config else {
                            return;
                        };
                        if config.target.is_some() {
                            return;
                        }
                        match (state.controller.status(), config.repeat) {
                            (AnimationStatus::Completed, AnimateRepeat::Once) => {
                                if let Some(callback) = &config.on_complete {
                                    callback();
                                }
                            }
                            (AnimationStatus::Completed, AnimateRepeat::Loop) => {
                                state.controller.set_value(0.);
                                state.controller.forward_on_next_tick();
                            }
                            (AnimationStatus::Completed, AnimateRepeat::PingPong) => {
                                state.forward.set(false);
                                state.controller.reverse_on_next_tick();
                            }
                            (AnimationStatus::Dismissed, AnimateRepeat::PingPong)
                                if !state.forward.get() =>
                            {
                                state.forward.set(true);
                                state.controller.forward_on_next_tick();
                            }
                            _ => {}
                        }
                    });
                }
                match animate.target {
                    Some(target) => {
                        if state.target.replace(Some(target)) != Some(target) {
                            if state.started.replace(true) {
                                // `Animate.target` changes animate to the
                                // new value.
                                state
                                    .controller
                                    .animate_to(incular_core::time::Instant::now(), target);
                            } else {
                                state.controller.set_value(target);
                            }
                        }
                    }
                    None => {
                        if !state.started.replace(true) && !total.is_zero() {
                            state.controller.forward_on_next_tick();
                        }
                    }
                }
                let elapsed = if total.is_zero() {
                    Duration::ZERO
                } else {
                    total.mul_f32(state.controller.value().clamp(0., 1.))
                };
                let elapsed = elapsed.saturating_sub(animate.delay);
                Widget::animation_ticker_with_revision(
                    state.controller.clone(),
                    false,
                    state.revision.borrow().clone(),
                    animate.frame(elapsed),
                )
            },
            false,
        )
    }
}

/// `.animate()` for anything that becomes a [`Widget`].
pub trait AnimateExt: Into<Widget> + Sized {
    /// Starts an effect chain on this widget.
    #[must_use]
    fn animate(self) -> Animate {
        Animate::new(self)
    }
}

impl<T: Into<Widget>> AnimateExt for T {}
