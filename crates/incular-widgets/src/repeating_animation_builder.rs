//! Flutter's `RepeatingAnimationBuilder`
//! (`widgets/repeating_animation_builder.dart`): an animation that repeats
//! for as long as the widget is built.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::{Animatable, AnimationController, Curve, Simulation};

use crate::tree::RetainedState;
use crate::{BuildContext, Widget};

/// Flutter's `RepeatMode`: how each cycle follows the last.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RepeatMode {
    /// Every cycle runs from the start to the end.
    #[default]
    Restart,
    /// Cycles alternate between forward and backward.
    Reverse,
}

/// Builds the widget from the animated value and the optional child.
pub type RepeatingAnimationWidgetBuilder<T> =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, T, Option<Widget>) -> Widget>;

/// Flutter's `RepeatingAnimationBuilder`: runs `animatable` over 0..1
/// through `curve` every `duration`, rebuilding with each value. Pausing
/// holds the current value; resuming continues from it.
pub struct RepeatingAnimationBuilder<T> {
    animatable: Rc<dyn Animatable<T>>,
    duration: Duration,
    curve: Curve,
    repeat_mode: RepeatMode,
    paused: bool,
    builder: RepeatingAnimationWidgetBuilder<T>,
    child: Option<Widget>,
}

impl<T: 'static> RepeatingAnimationBuilder<T> {
    #[must_use]
    pub fn new<W: Into<Widget>>(
        animatable: impl Animatable<T> + 'static,
        duration: Duration,
        builder: impl for<'a> Fn(&BuildContext<'a>, T, Option<Widget>) -> W + 'static,
    ) -> Self {
        Self {
            animatable: Rc::new(animatable),
            duration,
            curve: Curve::Linear,
            repeat_mode: RepeatMode::Restart,
            paused: false,
            builder: Rc::new(move |context, value, child| builder(context, value, child).into()),
            child: None,
        }
    }

    /// Defaults to linear.
    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// Defaults to [`RepeatMode::Restart`].
    #[must_use]
    pub fn repeat_mode(mut self, mode: RepeatMode) -> Self {
        self.repeat_mode = mode;
        self
    }

    /// Holds the animation at its current value. Defaults to false.
    #[must_use]
    pub fn paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// A subtree passed to the builder unchanged on every frame.
    #[must_use]
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }
}

/// Flutter's `_RepeatingSimulation`, which `AnimationController.repeat`
/// runs: cycles of `period` from the current value, without end.
struct RepeatingSimulation {
    min: f32,
    max: f32,
    reverse: bool,
    period: f64,
    /// The time into a forward cycle the start value corresponds to.
    initial_t: f64,
}

impl RepeatingSimulation {
    fn new(initial: f32, min: f32, max: f32, reverse: bool, period: Duration) -> Self {
        let period = period.as_secs_f64();
        let initial_t = if max == min {
            0.
        } else {
            f64::from((initial.clamp(min, max) - min) / (max - min)) * period
        };
        Self {
            min,
            max,
            reverse,
            period,
            initial_t,
        }
    }
}

impl Simulation for RepeatingSimulation {
    fn position(&self, time: Duration) -> f32 {
        if self.period <= 0. {
            return self.max;
        }
        let total = time.as_secs_f64() + self.initial_t;
        let t = ((total / self.period) % 1.) as f32;
        let backwards = self.reverse && (total / self.period).floor() as u64 % 2 == 1;
        if backwards {
            self.max + (self.min - self.max) * t
        } else {
            self.min + (self.max - self.min) * t
        }
    }

    fn velocity(&self, _time: Duration) -> f32 {
        if self.period <= 0. {
            0.
        } else {
            (self.max - self.min) / self.period as f32
        }
    }

    fn is_done(&self, _time: Duration) -> bool {
        false
    }
}

struct RepeatingState<T> {
    config: RefCell<Option<Rc<RepeatingAnimationBuilder<T>>>>,
    controller: AnimationController,
    listening: Cell<bool>,
    revision: RefCell<Rc<Cell<u64>>>,
}

impl<T: 'static> RetainedState for RepeatingState<T> {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl<T: 'static> RepeatingState<T> {
    /// `_controller.repeat(reverse: ...)`, continuing from the value.
    fn repeat(&self, config: &RepeatingAnimationBuilder<T>) {
        let (min, max) = self.controller.bounds();
        self.controller.animate_with(RepeatingSimulation::new(
            self.controller.value(),
            min,
            max,
            config.repeat_mode == RepeatMode::Reverse,
            config.duration,
        ));
    }

    /// `initState` and `didUpdateWidget`.
    fn update(&self, config: &Rc<RepeatingAnimationBuilder<T>>) {
        let Some(old) = self.config.replace(Some(config.clone())) else {
            if !config.paused {
                self.repeat(config);
            }
            return;
        };
        if Rc::ptr_eq(&old, config) {
            return;
        }
        if config.duration != old.duration {
            self.controller.set_duration(config.duration);
        }
        if config.paused {
            if !old.paused || self.controller.is_active() {
                self.controller.stop();
            }
            return;
        }
        let restart = old.paused
            || config.repeat_mode != old.repeat_mode
            || config.duration != old.duration
            || !self.controller.is_active();
        if restart {
            self.repeat(config);
        }
    }
}

impl<T: 'static> From<RepeatingAnimationBuilder<T>> for Widget {
    fn from(value: RepeatingAnimationBuilder<T>) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            {
                let config = config.clone();
                move |revision| RepeatingState::<T> {
                    config: RefCell::new(None),
                    controller: AnimationController::new(config.duration),
                    listening: Cell::new(false),
                    revision: RefCell::new(revision),
                }
            },
            move |state, context, _| {
                if !state.listening.replace(true) {
                    let weak = Rc::downgrade(state);
                    state.controller.add_listener(move |_| {
                        if let Some(state) = weak.upgrade() {
                            let revision = state.revision.borrow().clone();
                            revision.set(revision.get().wrapping_add(1));
                        }
                    });
                }
                state.update(&config);
                let config = state.config.borrow().clone().expect("updated above");
                let t = config.curve.apply(state.controller.value());
                let child =
                    (config.builder)(context, config.animatable.value_at(t), config.child.clone());
                Widget::animation_ticker_with_revision(
                    state.controller.clone(),
                    false,
                    state.revision.borrow().clone(),
                    child,
                )
            },
            false,
        )
    }
}
