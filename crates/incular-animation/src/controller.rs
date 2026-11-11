//! Runtime-driven animation controllers, modelled on Flutter's
//! `AnimationController` (`animation/animation_controller.dart`).
//!
//! Every run is a simulation, as in Flutter: `forward`, `reverse`,
//! `animate_to` and `animate_back` interpolate along a curve, `repeat` runs
//! Flutter's repeating simulation, `fling` runs a critically damped spring and
//! `animate_with` runs any [`Simulation`]. A run's clock starts either at a
//! timestamp the caller supplies or, for the `*_on_next_tick` variants, at
//! the next [`AnimationController::tick`], as Flutter's ticker starts on the
//! next frame.
//!
//! A controller made with [`AnimationController::with_vsync`] ticks itself
//! from the frame scheduler (Flutter's `vsync:`); others are ticked by the
//! widget that owns them or by the caller.

use crate::scheduler::Ticker;
use crate::{
    Curve,
    physics::{Simulation, SpringDescription, SpringSimulation, Tolerance},
};
use incular_core::finite_or_zero;
use incular_core::time::Instant;
use std::{
    cell::RefCell,
    fmt,
    rc::{Rc, Weak},
    time::Duration,
};

/// The status of an animation, as in Flutter's `AnimationStatus`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimationStatus {
    /// The animation is stopped at the beginning.
    #[default]
    Dismissed,
    /// The animation is running from beginning to end.
    Forward,
    /// The animation is running backwards, from end to beginning.
    Reverse,
    /// The animation is stopped at the end.
    Completed,
}

impl AnimationStatus {
    /// Flutter `AnimationStatus.isAnimating`: running forward or in reverse.
    #[must_use]
    pub const fn is_animating(self) -> bool {
        matches!(self, Self::Forward | Self::Reverse)
    }

    /// Flutter `AnimationStatus.isCompleted`.
    #[must_use]
    pub const fn is_completed(self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Flutter `AnimationStatus.isDismissed`.
    #[must_use]
    pub const fn is_dismissed(self) -> bool {
        matches!(self, Self::Dismissed)
    }

    /// Flutter `AnimationStatus.isForwardOrCompleted`.
    #[must_use]
    pub const fn is_forward_or_completed(self) -> bool {
        matches!(self, Self::Forward | Self::Completed)
    }
}

/// Flutter `AnimationBehavior`: how a controller reacts to the platform
/// asking for animations to be disabled ([`disable_animations`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnimationBehavior {
    /// Runs `forward`, `reverse` and `animate_to` at 5% of their duration
    /// while animations are disabled, so most finish within one frame.
    #[default]
    Normal,
    /// Ignores the setting; for animations whose timing is their meaning.
    Preserve,
}

thread_local! {
    static DISABLE_ANIMATIONS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Flutter `SemanticsBinding.disableAnimations`: whether the platform asks
/// for animations to be turned off or reduced. The retained tree keeps it in
/// step with the window environment's reduced-motion preference.
#[must_use]
pub fn disable_animations() -> bool {
    DISABLE_ANIMATIONS.with(std::cell::Cell::get)
}

/// Records the platform's request to disable animations for the controllers
/// of this thread. Read when an animation starts, as Flutter does.
pub fn set_disable_animations(disabled: bool) {
    DISABLE_ANIMATIONS.with(|value| value.set(disabled));
}

/// Flutter `_animateToInternal`: the duration scale a controller applies.
fn behavior_scale(behavior: AnimationBehavior) -> f64 {
    match behavior {
        AnimationBehavior::Normal if disable_animations() => 0.05,
        AnimationBehavior::Normal | AnimationBehavior::Preserve => 1.0,
    }
}

/// Flutter `_kFlingSpringDescription`.
fn fling_spring() -> SpringDescription {
    SpringDescription::with_damping_ratio(1.0, 500.0, 1.0)
}

/// Flutter `_kFlingTolerance`.
const FLING_TOLERANCE: Tolerance = Tolerance {
    distance: 0.01,
    time: Duration::from_millis(1),
    velocity: f32::INFINITY,
};

/// Options for [`AnimationController::repeat_with`], Flutter `repeat`'s
/// named arguments. Unset bounds default to the controller's bounds and an
/// unset period to its duration; an unset count repeats until stopped.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RepeatOptions {
    pub min: Option<f32>,
    pub max: Option<f32>,
    /// Every other cycle runs from `max` back to `min`.
    pub reverse: bool,
    pub period: Option<Duration>,
    /// The number of cycles before the animation completes.
    pub count: Option<u32>,
}

impl RepeatOptions {
    #[must_use]
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    #[must_use]
    pub fn min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }

    #[must_use]
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    #[must_use]
    pub fn period(mut self, period: Duration) -> Self {
        self.period = Some(period);
        self
    }

    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }
}

/// What drives the value while the controller runs.
#[derive(Clone)]
enum Run {
    /// Flutter `_InterpolationSimulation`.
    Interpolation {
        begin: f64,
        end: f64,
        /// Seconds, already scaled by the animation behavior.
        duration: f64,
        curve: Curve,
    },
    /// Flutter `_RepeatingSimulation`.
    Repeating {
        min: f64,
        max: f64,
        reverse: bool,
        period: f64,
        initial_t: f64,
        count: Option<u32>,
        curve: Curve,
    },
    /// A caller's simulation (`animateWith`, `fling`).
    Simulation(Rc<dyn Simulation>),
}

impl Run {
    /// The position at `t` seconds, and for a repeating run the direction of
    /// the cycle it is in (Flutter's `directionSetter`).
    fn x(&self, t: f64) -> (f64, Option<bool>) {
        match self {
            Self::Interpolation {
                begin,
                end,
                duration,
                curve,
            } => {
                let progress = if *duration <= 0.0 {
                    1.0
                } else {
                    (t / duration).clamp(0.0, 1.0)
                };
                let x = if progress == 0.0 {
                    *begin
                } else if progress == 1.0 {
                    *end
                } else {
                    begin + (end - begin) * f64::from(curve.apply(progress as f32))
                };
                (x, None)
            }
            Self::Repeating {
                min,
                max,
                reverse,
                period,
                initial_t,
                curve,
                ..
            } => {
                let total = t + initial_t;
                let cycle_t = (total / period).rem_euclid(1.0);
                let odd = ((total / period).floor() as u64) % 2 == 1;
                let eased = f64::from(curve.apply(cycle_t as f32));
                if *reverse && odd {
                    (max + (min - max) * eased, Some(false))
                } else {
                    (min + (max - min) * eased, Some(true))
                }
            }
            Self::Simulation(simulation) => (
                f64::from(simulation.position(Duration::from_secs_f64(t.max(0.0)))),
                None,
            ),
        }
    }

    fn dx(&self, t: f64) -> f64 {
        match self {
            Self::Interpolation { .. } => {
                // Flutter's `dx`: a central difference over `tolerance.time`.
                const EPSILON: f64 = 1e-3;
                (self.x(t + EPSILON).0 - self.x((t - EPSILON).max(0.0)).0)
                    / (t + EPSILON - (t - EPSILON).max(0.0))
            }
            Self::Repeating {
                min, max, period, ..
            } => (max - min) / period,
            Self::Simulation(simulation) => {
                f64::from(simulation.velocity(Duration::from_secs_f64(t.max(0.0))))
            }
        }
    }

    fn is_done(&self, t: f64) -> bool {
        match self {
            // Flutter finishes once the time has passed the duration; a frame
            // landing exactly on it completes too, so a run whose last frame
            // falls on its end does not need one more frame.
            Self::Interpolation { duration, .. } => t >= *duration,
            Self::Repeating {
                period,
                initial_t,
                count,
                ..
            } => count.is_some_and(|count| t >= f64::from(count) * period - initial_t),
            Self::Simulation(simulation) => simulation.is_done(Duration::from_secs_f64(t.max(0.0))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct State {
    value: f32,
    status: AnimationStatus,
    duration: Duration,
    reverse_duration: Option<Duration>,
    /// Flutter's `_direction`.
    forward: bool,
    lower_bound: f32,
    upper_bound: f32,
    started_at: Option<Instant>,
    /// The run starts at the next `tick` (Flutter's ticker starts on the
    /// next frame).
    pending_start: bool,
    /// The curve `forward`, `reverse`, `toggle` and `repeat` ease with.
    /// Flutter's controller is always linear; this is the default.
    curve: Curve,
    last_elapsed: Duration,
    animation_behavior: AnimationBehavior,
    /// The status last reported to status listeners.
    reported_status: AnimationStatus,
    /// How many times the status has come to rest (dismissed or completed).
    rests: u64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            value: 0.0,
            status: AnimationStatus::Dismissed,
            duration: Duration::ZERO,
            reverse_duration: None,
            forward: true,
            lower_bound: 0.0,
            upper_bound: 1.0,
            started_at: None,
            pending_start: false,
            curve: Curve::Linear,
            last_elapsed: Duration::ZERO,
            animation_behavior: AnimationBehavior::Normal,
            reported_status: AnimationStatus::Dismissed,
            rests: 0,
        }
    }
}

impl State {
    /// Status implied by a value, as Flutter's `_internalSetValue` derives it.
    fn status_at(&self, value: f32) -> AnimationStatus {
        if value <= self.lower_bound {
            AnimationStatus::Dismissed
        } else if value >= self.upper_bound {
            AnimationStatus::Completed
        } else if self.forward {
            AnimationStatus::Forward
        } else {
            AnimationStatus::Reverse
        }
    }

    fn clamp(&self, value: f64) -> f32 {
        (value as f32).clamp(self.lower_bound, self.upper_bound)
    }

    fn running(&self) -> bool {
        self.started_at.is_some() || self.pending_start
    }
}

type Listener = Rc<dyn Fn(f32)>;
type StatusListener = Rc<dyn Fn(AnimationStatus)>;

#[derive(Default)]
struct ControllerState {
    state: State,
    run: Option<Run>,
    listeners: Vec<(usize, Listener)>,
    status_listeners: Vec<(usize, StatusListener)>,
    next_listener: usize,
    /// The frame-scheduler ticker of a [`AnimationController::with_vsync`]
    /// controller.
    ticker: Option<Ticker>,
}

/// A cloneable controller advanced by timestamps.
///
/// `tick` is deterministic and can be driven by a frame scheduler, a test
/// clock, or a native animation callback. Values are always kept inside the
/// configured bounds.
#[derive(Clone, Default)]
pub struct AnimationController {
    changes: incular_core::reactivity::DependencySource,
    inner: Rc<RefCell<ControllerState>>,
}

impl fmt::Debug for AnimationController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.inner.borrow().state;
        formatter
            .debug_struct("AnimationController")
            .field("value", &state.value)
            .field("status", &state.status)
            .field("duration", &state.duration)
            .field("lower_bound", &state.lower_bound)
            .field("upper_bound", &state.upper_bound)
            .finish()
    }
}

impl PartialEq for AnimationController {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl AnimationController {
    #[doc(hidden)]
    pub fn adopt_timeline_from(&self, other: &Self) {
        if Rc::ptr_eq(&self.inner, &other.inner) {
            return;
        }
        let (next, run) = {
            let other = other.inner.borrow();
            (other.state, other.run.clone())
        };
        let changed = {
            let mut controller = self.inner.borrow_mut();
            let previous = controller.state;
            controller.state = next;
            controller.run = run;
            previous != next
        };
        if changed {
            self.notify_value();
            self.notify_status();
        }
        self.sync_ticker();
    }

    #[must_use]
    pub fn new(duration: Duration) -> Self {
        Self::with_bounds(duration, 0.0, 1.0)
    }

    #[must_use]
    pub fn with_bounds(duration: Duration, lower_bound: f32, upper_bound: f32) -> Self {
        let (lower_bound, upper_bound) = normalize_bounds(lower_bound, upper_bound);
        let state = State {
            duration,
            lower_bound,
            upper_bound,
            value: lower_bound,
            ..State::default()
        };
        Self {
            changes: incular_core::reactivity::DependencySource::default(),
            inner: Rc::new(RefCell::new(ControllerState {
                state,
                ..ControllerState::default()
            })),
        }
    }

    /// Flutter's `AnimationController.unbounded`: no bounds, value 0, for
    /// physics simulations. Like Flutter, it preserves its timing when
    /// animations are disabled.
    #[must_use]
    pub fn unbounded() -> Self {
        let controller = Self::new(Duration::ZERO);
        {
            let mut inner = controller.inner.borrow_mut();
            let state = &mut inner.state;
            state.lower_bound = f32::NEG_INFINITY;
            state.upper_bound = f32::INFINITY;
            state.value = 0.;
            state.status = AnimationStatus::Forward;
            state.reported_status = AnimationStatus::Forward;
            state.animation_behavior = AnimationBehavior::Preserve;
        }
        controller
    }

    /// Flutter's `value:` constructor argument: starts at `value` (clamped),
    /// with the status that value implies.
    #[must_use]
    pub fn with_value(self, value: f32) -> Self {
        {
            let mut controller = self.inner.borrow_mut();
            let state = &mut controller.state;
            state.value = clamp_finite(value, state);
            state.status = state.status_at(state.value);
            state.reported_status = state.status;
        }
        self
    }

    /// Flutter's `reverseDuration`: the duration of `reverse` and of
    /// `animate_back` runs, when set.
    #[must_use]
    pub fn with_reverse_duration(self, duration: Duration) -> Self {
        self.inner.borrow_mut().state.reverse_duration = Some(duration);
        self
    }

    #[must_use]
    pub fn reverse_duration(&self) -> Option<Duration> {
        self.inner.borrow().state.reverse_duration
    }

    pub fn set_reverse_duration(&self, duration: Option<Duration>) {
        self.inner.borrow_mut().state.reverse_duration = duration;
    }

    /// Flutter's `vsync:`: ticks this controller from the frame scheduler
    /// while it runs, so it animates without a ticking widget around it and
    /// keeps the runtime producing frames until it stops.
    #[must_use]
    pub fn with_vsync(self) -> Self {
        let weak = WeakController {
            changes: self.changes.clone(),
            inner: Rc::downgrade(&self.inner),
        };
        let ticker = Ticker::with_frame_time(move |now, _| {
            if let Some(controller) = weak.upgrade() {
                controller.tick(now);
            }
        });
        self.inner.borrow_mut().ticker = Some(ticker);
        self.sync_ticker();
        self
    }

    /// Whether the controller ticks itself from the frame scheduler.
    #[must_use]
    pub fn has_vsync(&self) -> bool {
        self.inner.borrow().ticker.is_some()
    }

    /// Flutter `TickerMode`: a muted controller keeps time but its value
    /// stops changing until it is unmuted. Only a
    /// [`with_vsync`](Self::with_vsync) controller has a ticker to mute.
    pub fn set_ticker_muted(&self, muted: bool) {
        if let Some(ticker) = &self.inner.borrow().ticker {
            ticker.set_muted(muted);
        }
    }

    /// Flutter's `animateWith`: from the next [`Self::tick`], the value
    /// follows `simulation` (clamped to the bounds) until it is done, which
    /// completes the animation.
    pub fn animate_with(&self, simulation: impl Simulation + 'static) {
        self.start_run(None, Run::Simulation(Rc::new(simulation)), true);
    }

    /// Flutter's `animateBackWith`: [`Self::animate_with`] in the reverse
    /// direction, so the animation is dismissed when the simulation is done.
    pub fn animate_back_with(&self, simulation: impl Simulation + 'static) {
        self.start_run(None, Run::Simulation(Rc::new(simulation)), false);
    }

    /// Sets how the controller reacts to [`disable_animations`] (Flutter
    /// `animationBehavior`, default [`AnimationBehavior::Normal`]).
    #[must_use]
    pub fn with_animation_behavior(self, behavior: AnimationBehavior) -> Self {
        self.inner.borrow_mut().state.animation_behavior = behavior;
        self
    }

    #[must_use]
    pub fn animation_behavior(&self) -> AnimationBehavior {
        self.inner.borrow().state.animation_behavior
    }

    #[must_use]
    pub fn duration(&self) -> Duration {
        self.inner.borrow().state.duration
    }

    pub fn set_duration(&self, duration: Duration) {
        self.inner.borrow_mut().state.duration = duration;
    }

    #[must_use]
    pub fn bounds(&self) -> (f32, f32) {
        let state = self.inner.borrow().state;
        (state.lower_bound, state.upper_bound)
    }

    pub fn set_bounds(&self, lower_bound: f32, upper_bound: f32) {
        let (lower_bound, upper_bound) = normalize_bounds(lower_bound, upper_bound);
        let changed = {
            let mut controller = self.inner.borrow_mut();
            let old = controller.state.value;
            controller.state.lower_bound = lower_bound;
            controller.state.upper_bound = upper_bound;
            controller.state.value = old.clamp(lower_bound, upper_bound);
            controller.state.value != old
        };
        if changed {
            self.notify_value();
        }
    }

    /// Flutter `forward()`, timed from `now`.
    pub fn forward(&self, now: Instant) {
        self.forward_at(Some(now));
    }

    /// Flutter `reverse()`, timed from `now`.
    pub fn reverse(&self, now: Instant) {
        self.reverse_at(Some(now));
    }

    /// [`Self::forward`] timed from the next [`Self::tick`], as Flutter's
    /// `forward()` starts its ticker on the next frame. For callers, such as
    /// gesture handlers, that have no frame timestamp.
    pub fn forward_on_next_tick(&self) {
        self.forward_at(None);
    }

    /// [`Self::reverse`] timed from the next [`Self::tick`].
    pub fn reverse_on_next_tick(&self) {
        self.reverse_at(None);
    }

    /// Flutter `toggle()`: reverses a controller that is running forward or
    /// completed, and runs any other forward.
    pub fn toggle(&self, now: Instant) {
        self.toggle_at(Some(now));
    }

    /// [`Self::toggle`] timed from the next [`Self::tick`].
    pub fn toggle_on_next_tick(&self) {
        self.toggle_at(None);
    }

    /// Flutter `animateTo(target)`: runs forward in direction to `target`
    /// (clamped to the bounds) over the share of the duration it covers, so
    /// the animation completes there, even below the current value.
    pub fn animate_to(&self, now: Instant, target: f32) {
        self.animate_to_internal(Some(now), target, None, None, true);
    }

    /// Flutter `animateTo(target, duration:, curve:)`.
    pub fn animate_to_curved(
        &self,
        now: Instant,
        target: f32,
        duration: Option<Duration>,
        curve: Curve,
    ) {
        self.animate_to_internal(Some(now), target, duration, Some(curve), true);
    }

    /// [`Self::animate_to`] timed from the next [`Self::tick`].
    pub fn animate_to_on_next_tick(&self, target: f32) {
        self.animate_to_internal(None, target, None, None, true);
    }

    /// Flutter `animateBack(target)`: [`Self::animate_to`] in the reverse
    /// direction, ending dismissed and timed by the reverse duration.
    pub fn animate_back(&self, now: Instant, target: f32) {
        self.animate_to_internal(Some(now), target, None, None, false);
    }

    /// Flutter `animateBack(target, duration:, curve:)`.
    pub fn animate_back_curved(
        &self,
        now: Instant,
        target: f32,
        duration: Option<Duration>,
        curve: Curve,
    ) {
        self.animate_to_internal(Some(now), target, duration, Some(curve), false);
    }

    /// [`Self::animate_back`] timed from the next [`Self::tick`].
    pub fn animate_back_on_next_tick(&self, target: f32) {
        self.animate_to_internal(None, target, None, None, false);
    }

    /// Drives from the current value to `target`; an alias for
    /// [`Self::animate_to`] for callers building animation graphs.
    pub fn drive(&self, now: Instant, target: f32) {
        self.animate_to(now, target);
    }

    /// Drives an explicit interval, resetting the current value to `from`.
    pub fn drive_range(&self, now: Instant, from: f32, to: f32) {
        self.set_value(from);
        self.animate_to(now, to);
    }

    /// Flutter `repeat(reverse:)`: cycles between the bounds, starting from
    /// the current value, until stopped. When `reverse` is true, every other
    /// cycle runs backwards.
    pub fn repeat(&self, now: Instant, reverse: bool) {
        self.repeat_with(now, RepeatOptions::default().reverse(reverse));
    }

    /// Flutter `repeat(min:, max:, reverse:, period:, count:)`.
    pub fn repeat_with(&self, now: Instant, options: RepeatOptions) {
        self.repeat_at(Some(now), options);
    }

    /// [`Self::repeat_with`] timed from the next [`Self::tick`].
    pub fn repeat_on_next_tick(&self, options: RepeatOptions) {
        self.repeat_at(None, options);
    }

    /// Alias documenting that `repeat` remains active until stopped.
    pub fn repeat_forever(&self, now: Instant, reverse: bool) {
        self.repeat(now, reverse);
    }

    /// Lets a repeating animation finish the cycle it is in and complete
    /// there, instead of stopping mid-cycle.
    pub fn stop_repeat(&self) {
        let mut controller = self.inner.borrow_mut();
        let elapsed = controller.state.last_elapsed.as_secs_f64();
        if let Some(Run::Repeating {
            period,
            initial_t,
            count,
            ..
        }) = &mut controller.run
        {
            let cycles = ((elapsed + *initial_t) / *period).floor() as u32 + 1;
            *count = Some(count.map_or(cycles, |count| count.min(cycles)));
        }
    }

    /// Flutter `fling(velocity:)`: drives towards the upper bound (or the
    /// lower one for a negative velocity) on a critically damped spring,
    /// starting at `velocity` units per second.
    pub fn fling(&self, now: Instant, velocity: f32) {
        self.fling_with(now, velocity, None, None);
    }

    /// Flutter `fling(velocity:, springDescription:, animationBehavior:)`.
    pub fn fling_with(
        &self,
        now: Instant,
        velocity: f32,
        spring: Option<SpringDescription>,
        behavior: Option<AnimationBehavior>,
    ) {
        self.fling_at(Some(now), velocity, spring, behavior);
    }

    /// [`Self::fling`] timed from the next [`Self::tick`].
    pub fn fling_on_next_tick(&self, velocity: f32) {
        self.fling_at(None, velocity, None, None);
    }

    /// Stops the running animation without changing its value or status, and
    /// without notifying listeners, as Flutter's `stop` does.
    pub fn stop(&self) {
        {
            let mut controller = self.inner.borrow_mut();
            controller.run = None;
            controller.state.started_at = None;
            controller.state.pending_start = false;
        }
        self.sync_ticker();
    }

    /// Flutter `reset()`: sets the value to the lower bound.
    pub fn reset(&self) {
        let lower = self.inner.borrow().state.lower_bound;
        self.set_value(lower);
    }

    /// Flutter's `value` setter: stops the animation and sets the value
    /// (clamped), with the status that value implies.
    pub fn set_value(&self, value: f32) {
        self.stop();
        let changed = {
            let mut controller = self.inner.borrow_mut();
            let state = &mut controller.state;
            let old = (state.value, state.status);
            state.value = clamp_finite(value, state);
            state.status = state.status_at(state.value);
            (state.value, state.status) != old
        };
        if changed {
            self.notify_value();
        }
        self.notify_status();
    }

    /// Advances the controller and returns whether value or status changed.
    pub fn tick(&self, now: Instant) -> bool {
        let (changed, finished) = {
            let mut controller = self.inner.borrow_mut();
            let Some(run) = controller.run.clone() else {
                return false;
            };
            let state = &mut controller.state;
            let started = match state.started_at {
                Some(started) => started,
                None if state.pending_start => {
                    state.pending_start = false;
                    state.started_at = Some(now);
                    now
                }
                None => return false,
            };
            let elapsed = now.saturating_duration_since(started);
            state.last_elapsed = elapsed;
            let t = elapsed.as_secs_f64();
            let old = (state.value, state.status);
            let (x, direction) = run.x(t);
            if let Some(forward) = direction {
                state.forward = forward;
                state.status = if forward {
                    AnimationStatus::Forward
                } else {
                    AnimationStatus::Reverse
                };
            }
            state.value = state.clamp(x);
            let finished = run.is_done(t);
            if finished {
                state.status = if state.forward {
                    AnimationStatus::Completed
                } else {
                    AnimationStatus::Dismissed
                };
                state.started_at = None;
                controller.run = None;
            }
            let state = &controller.state;
            ((state.value, state.status) != old, finished)
        };
        if changed {
            self.notify_value();
        }
        self.notify_status();
        if finished {
            self.sync_ticker();
        }
        changed
    }

    #[must_use]
    pub fn value(&self) -> f32 {
        self.changes.track();
        self.inner.borrow().state.value
    }

    #[must_use]
    pub fn status(&self) -> AnimationStatus {
        self.changes.track();
        self.inner.borrow().state.status
    }

    /// Whether the animation is currently running, as Flutter's `isAnimating`.
    /// A stopped animation keeps its forward or reverse status.
    #[must_use]
    pub fn is_active(&self) -> bool {
        let controller = self.inner.borrow();
        controller.run.is_some() && controller.state.running()
    }

    /// Flutter `isAnimating`.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.is_active()
    }

    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.status() == AnimationStatus::Completed
    }

    #[must_use]
    pub fn is_dismissed(&self) -> bool {
        self.status() == AnimationStatus::Dismissed
    }

    /// Flutter `isForwardOrCompleted`.
    #[must_use]
    pub fn is_forward_or_completed(&self) -> bool {
        self.status().is_forward_or_completed()
    }

    /// How many times the status has come to rest. Animation graph nodes
    /// compare it to see that a run ended between two reads.
    #[doc(hidden)]
    #[must_use]
    pub fn rest_count(&self) -> u64 {
        self.inner.borrow().state.rests
    }

    /// Flutter `velocity`: units per second of the running animation, 0
    /// when stopped.
    #[must_use]
    pub fn velocity(&self) -> f32 {
        let controller = self.inner.borrow();
        match &controller.run {
            Some(run) if controller.state.running() => {
                run.dx(controller.state.last_elapsed.as_secs_f64()) as f32
            }
            _ => 0.0,
        }
    }

    /// Flutter `lastElapsedDuration`: time into the current run at its last
    /// tick.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.inner.borrow().state.last_elapsed
    }

    /// Sets the curve `forward`, `reverse`, `toggle` and `repeat` ease
    /// with. Flutter's controller is linear; ease it with a
    /// `CurvedAnimation` to keep that model.
    pub fn set_curve(&self, curve: Curve) {
        self.inner.borrow_mut().state.curve = curve;
    }

    #[must_use]
    pub fn curve(&self) -> Curve {
        self.inner.borrow().state.curve
    }

    /// Observes value changes until the returned token is dropped.
    pub fn observe(
        &self,
        listener: impl Fn(f32) + 'static,
    ) -> incular_core::reactivity::Subscription {
        let inner = Rc::downgrade(&self.inner);
        self.changes.subscribe((), move || {
            if let Some(inner) = inner.upgrade() {
                let value = inner.borrow().state.value;
                listener(value);
            }
        })
    }

    /// Low-level manual registration. Prefer `observe` for owned listeners.
    pub fn add_listener(&self, listener: impl Fn(f32) + 'static) -> usize {
        let mut controller = self.inner.borrow_mut();
        let id = next_listener_id(&mut controller);
        controller.listeners.push((id, Rc::new(listener)));
        id
    }

    pub fn remove_listener(&self, id: usize) -> bool {
        let mut controller = self.inner.borrow_mut();
        let before = controller.listeners.len();
        controller
            .listeners
            .retain(|(listener_id, _)| *listener_id != id);
        before != controller.listeners.len()
    }

    /// Flutter `addStatusListener`: called whenever the status changes.
    pub fn add_status_listener(&self, listener: impl Fn(AnimationStatus) + 'static) -> usize {
        let mut controller = self.inner.borrow_mut();
        let id = next_listener_id(&mut controller);
        controller.status_listeners.push((id, Rc::new(listener)));
        id
    }

    /// Flutter `removeStatusListener`.
    pub fn remove_status_listener(&self, id: usize) -> bool {
        let mut controller = self.inner.borrow_mut();
        let before = controller.status_listeners.len();
        controller
            .status_listeners
            .retain(|(listener_id, _)| *listener_id != id);
        before != controller.status_listeners.len()
    }

    fn forward_at(&self, now: Option<Instant>) {
        let upper = self.inner.borrow().state.upper_bound;
        self.animate_to_internal(now, upper, None, None, true);
    }

    fn reverse_at(&self, now: Option<Instant>) {
        let lower = self.inner.borrow().state.lower_bound;
        self.animate_to_internal(now, lower, None, None, false);
    }

    fn toggle_at(&self, now: Option<Instant>) {
        if self.inner.borrow().state.status.is_forward_or_completed() {
            self.reverse_at(now);
        } else {
            self.forward_at(now);
        }
    }

    /// Flutter `_animateToInternal`.
    fn animate_to_internal(
        &self,
        now: Option<Instant>,
        target: f32,
        duration: Option<Duration>,
        curve: Option<Curve>,
        forward: bool,
    ) {
        let state = self.inner.borrow().state;
        let scale = behavior_scale(state.animation_behavior);
        let target = clamp_finite(target, &state);
        let duration = match duration {
            Some(_) if target == state.value => Duration::ZERO,
            Some(duration) => duration,
            None => {
                let base = if forward {
                    state.duration
                } else {
                    state.reverse_duration.unwrap_or(state.duration)
                };
                base.mul_f64(f64::from(remaining_fraction(
                    state.value,
                    target,
                    state.lower_bound,
                    state.upper_bound,
                )))
            }
        };
        let curve = curve.unwrap_or(state.curve);
        if duration.is_zero() {
            // Flutter: a zero-duration run lands on the target at once.
            self.stop();
            let changed = {
                let mut controller = self.inner.borrow_mut();
                let state = &mut controller.state;
                let old = (state.value, state.status);
                state.forward = forward;
                state.value = target;
                state.status = if forward {
                    AnimationStatus::Completed
                } else {
                    AnimationStatus::Dismissed
                };
                (state.value, state.status) != old
            };
            if changed {
                self.notify_value();
            }
            self.notify_status();
            return;
        }
        self.start_run(
            now,
            Run::Interpolation {
                begin: f64::from(state.value),
                end: f64::from(target),
                duration: duration.as_secs_f64() * scale,
                curve,
            },
            forward,
        );
    }

    fn repeat_at(&self, now: Option<Instant>, options: RepeatOptions) {
        let state = self.inner.borrow().state;
        let min = options.min.unwrap_or(state.lower_bound);
        let max = options.max.unwrap_or(state.upper_bound);
        let period = options.period.unwrap_or(state.duration).as_secs_f64();
        if period <= 0.0 {
            // Flutter asserts a positive period; without one the controller
            // has no cycle to run, so it rests at the cycle's end.
            self.set_value(max);
            return;
        }
        let (min, max) = (f64::from(min), f64::from(max));
        let initial_t = if max == min {
            0.0
        } else {
            (f64::from(state.value).clamp(min.min(max), min.max(max)) - min) / (max - min) * period
        };
        self.start_run(
            now,
            Run::Repeating {
                min,
                max,
                reverse: options.reverse,
                period,
                initial_t,
                count: options.count.filter(|count| *count > 0),
                curve: state.curve,
            },
            true,
        );
    }

    fn fling_at(
        &self,
        now: Option<Instant>,
        velocity: f32,
        spring: Option<SpringDescription>,
        behavior: Option<AnimationBehavior>,
    ) {
        let state = self.inner.borrow().state;
        let forward = velocity >= 0.0;
        let target = if forward {
            state.upper_bound + FLING_TOLERANCE.distance
        } else {
            state.lower_bound - FLING_TOLERANCE.distance
        };
        let scale = match behavior.unwrap_or(state.animation_behavior) {
            AnimationBehavior::Normal if disable_animations() => 200.0,
            AnimationBehavior::Normal | AnimationBehavior::Preserve => 1.0,
        };
        let simulation = SpringSimulation::new(
            spring.unwrap_or_else(fling_spring),
            state.value,
            target,
            velocity * scale,
            FLING_TOLERANCE,
        );
        self.start_run(now, Run::Simulation(Rc::new(simulation)), forward);
    }

    /// Flutter `_startSimulation`: the value jumps to the run's start and the
    /// status to the direction of travel.
    fn start_run(&self, now: Option<Instant>, run: Run, forward: bool) {
        let changed = {
            let mut controller = self.inner.borrow_mut();
            let start = run.x(0.0);
            let vsync = controller.ticker.is_some();
            let state = &mut controller.state;
            let old = (state.value, state.status);
            state.forward = start.1.unwrap_or(forward);
            state.value = state.clamp(start.0);
            state.status = if state.forward {
                AnimationStatus::Forward
            } else {
                AnimationStatus::Reverse
            };
            // Flutter `Ticker.start`: a vsync run started while a frame is being
            // produced is timed from that frame; otherwise from the next tick.
            // A widget-ticked controller starts at its first tick, which the
            // tree gives it later in the same frame.
            let now = now.or_else(|| vsync.then(crate::scheduler::current_frame_time).flatten());
            state.started_at = now;
            state.pending_start = now.is_none();
            state.last_elapsed = Duration::ZERO;
            controller.run = Some(run);
            (controller.state.value, controller.state.status) != old
        };
        // Builders that read the value rebuild, and the runtime learns that
        // an animation started.
        if changed {
            self.notify_value();
        } else {
            self.changes.notify();
        }
        self.notify_status();
        self.sync_ticker();
    }

    /// Starts or stops the vsync ticker to match whether the controller runs.
    fn sync_ticker(&self) {
        let controller = self.inner.borrow();
        if let Some(ticker) = &controller.ticker {
            let running = controller.run.is_some() && controller.state.running();
            if running && !ticker.is_active() {
                ticker.start();
            } else if !running && ticker.is_active() {
                ticker.stop();
            }
        }
    }

    fn notify_value(&self) {
        let (listeners, value) = {
            let controller = self.inner.borrow();
            (
                controller
                    .listeners
                    .iter()
                    .map(|(_, listener)| listener.clone())
                    .collect::<Vec<_>>(),
                controller.state.value,
            )
        };
        for listener in listeners {
            listener(value);
        }
        self.changes.notify();
    }

    /// Flutter `_checkStatusChanged`: reports a status that differs from the
    /// last reported one.
    fn notify_status(&self) {
        let (listeners, status) = {
            let mut controller = self.inner.borrow_mut();
            let status = controller.state.status;
            if controller.state.reported_status == status {
                return;
            }
            controller.state.reported_status = status;
            if !status.is_animating() {
                controller.state.rests += 1;
            }
            (
                controller
                    .status_listeners
                    .iter()
                    .map(|(_, listener)| listener.clone())
                    .collect::<Vec<_>>(),
                status,
            )
        };
        for listener in listeners {
            listener(status);
        }
    }
}

/// A controller handle that does not keep the controller alive, for its own
/// vsync ticker.
struct WeakController {
    changes: incular_core::reactivity::DependencySource,
    inner: Weak<RefCell<ControllerState>>,
}

impl WeakController {
    fn upgrade(&self) -> Option<AnimationController> {
        Some(AnimationController {
            changes: self.changes.clone(),
            inner: self.inner.upgrade()?,
        })
    }
}

fn next_listener_id(controller: &mut ControllerState) -> usize {
    let id = controller.next_listener;
    controller.next_listener = controller
        .next_listener
        .checked_add(1)
        .expect("animation listener identity space exhausted");
    id
}

/// Clamps a requested value to the bounds; a non-finite one goes to the
/// lower bound.
fn clamp_finite(value: f32, state: &State) -> f32 {
    if value.is_finite() {
        value.clamp(state.lower_bound, state.upper_bound)
    } else {
        state.lower_bound
    }
}

/// Fraction of the bounded range covered by a run from `from` to `to`, as
/// Flutter's `remainingFraction`. An infinite range counts as the whole span.
/// An empty range covers no distance, so the run takes no time.
fn remaining_fraction(from: f32, to: f32, lower: f32, upper: f32) -> f32 {
    let range = upper - lower;
    if !range.is_finite() {
        1.0
    } else if range == 0.0 {
        0.0
    } else {
        (to - from).abs() / range
    }
}

fn normalize_bounds(lower: f32, upper: f32) -> (f32, f32) {
    let lower = finite_or_zero(lower);
    let upper = if upper.is_finite() { upper } else { 1.0 };
    if lower <= upper {
        (lower, upper)
    } else {
        (upper, lower)
    }
}
