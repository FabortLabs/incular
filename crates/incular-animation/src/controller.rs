//! Runtime-driven animation controllers.

use crate::{Curve, Tween};
use incular_core::finite_or_zero;
use incular_core::time::Instant;
use std::{cell::RefCell, fmt, rc::Rc, time::Duration};

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
fn behavior_scale(behavior: AnimationBehavior) -> f32 {
    match behavior {
        AnimationBehavior::Normal if disable_animations() => 0.05,
        AnimationBehavior::Normal | AnimationBehavior::Preserve => 1.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RepeatState {
    reverse: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct State {
    value: f32,
    status: AnimationStatus,
    duration: Duration,
    /// Duration of the run started by the last `start`, scaled by the fraction
    /// of the range it covers.
    run_duration: Duration,
    /// Last requested direction, used to pick the status while `value` is
    /// strictly inside the bounds (Flutter's `_direction`).
    forward: bool,
    lower_bound: f32,
    upper_bound: f32,
    started_at: Option<Instant>,
    /// The run starts at the next `tick` (Flutter's ticker starts on the
    /// next frame).
    pending_start: bool,
    from: f32,
    to: f32,
    curve: Curve,
    repeat: Option<RepeatState>,
    last_elapsed: Duration,
    animation_behavior: AnimationBehavior,
}

impl Default for State {
    fn default() -> Self {
        Self {
            value: 0.0,
            status: AnimationStatus::Dismissed,
            duration: Duration::ZERO,
            run_duration: Duration::ZERO,
            forward: true,
            lower_bound: 0.0,
            upper_bound: 1.0,
            started_at: None,
            pending_start: false,
            from: 0.0,
            to: 1.0,
            curve: Curve::Linear,
            repeat: None,
            last_elapsed: Duration::ZERO,
            animation_behavior: AnimationBehavior::Normal,
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
}

type Listener = Rc<dyn Fn(f32)>;

#[derive(Default)]
struct ControllerState {
    state: State,
    listeners: Vec<(usize, Listener)>,
    next_listener: usize,
}

/// A cloneable controller advanced by timestamps supplied by the runtime.
///
/// It intentionally owns no timer or event loop. `tick` is deterministic and
/// can be driven by a frame scheduler, a test clock, or a native animation
/// callback. Values are always kept inside the configured bounds.
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
        let next = other.inner.borrow().state;
        let (changed, listeners, value) = {
            let mut controller = self.inner.borrow_mut();
            let previous = controller.state;
            controller.state = next;
            (
                previous != next,
                clone_listeners(&controller),
                controller.state.value,
            )
        };
        if changed {
            notify(listeners, Some(value));
            self.changes.notify();
        }
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
            from: lower_bound,
            to: upper_bound,
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
        let (changed, listeners) = {
            let mut controller = self.inner.borrow_mut();
            let old = controller.state.value;
            controller.state.lower_bound = lower_bound;
            controller.state.upper_bound = upper_bound;
            controller.state.value = old.clamp(lower_bound, upper_bound);
            controller.state.from = controller.state.from.clamp(lower_bound, upper_bound);
            controller.state.to = controller.state.to.clamp(lower_bound, upper_bound);
            collect_listeners(&controller, controller.state.value, old)
        };
        notify(listeners, changed);
        if changed.is_some() {
            self.changes.notify();
        }
    }

    pub fn forward(&self, now: Instant) {
        let (from, to) = {
            let state = self.inner.borrow().state;
            (state.value, state.upper_bound)
        };
        self.start(Some(now), from, to, AnimationStatus::Forward);
    }

    pub fn reverse(&self, now: Instant) {
        let (from, to) = {
            let state = self.inner.borrow().state;
            (state.value, state.lower_bound)
        };
        self.start(Some(now), from, to, AnimationStatus::Reverse);
    }

    /// [`Self::forward`] timed from the next [`Self::tick`], as Flutter's
    /// `forward()` starts its ticker on the next frame. For callers, such as
    /// gesture handlers, that have no frame timestamp.
    pub fn forward_on_next_tick(&self) {
        let (from, to) = {
            let state = self.inner.borrow().state;
            (state.value, state.upper_bound)
        };
        self.start(None, from, to, AnimationStatus::Forward);
    }

    /// [`Self::reverse`] timed from the next [`Self::tick`].
    pub fn reverse_on_next_tick(&self) {
        let (from, to) = {
            let state = self.inner.borrow().state;
            (state.value, state.lower_bound)
        };
        self.start(None, from, to, AnimationStatus::Reverse);
    }

    /// Drives to an arbitrary point in the configured bounds.
    pub fn animate_to(&self, now: Instant, target: f32) {
        let state = self.inner.borrow().state;
        let target = target.clamp(state.lower_bound, state.upper_bound);
        let direction = if target >= state.value {
            AnimationStatus::Forward
        } else {
            AnimationStatus::Reverse
        };
        let from = state.value;
        self.start(Some(now), from, target, direction);
    }

    /// Alias for `animate_to`, useful when a caller semantically reverses to a
    /// custom point rather than all the way to the lower bound.
    pub fn animate_back(&self, now: Instant, target: f32) {
        self.animate_to(now, target);
    }

    /// Drives from the current value to `target`; this is a concise alias for
    /// [`Self::animate_to`] for callers building animation graphs.
    pub fn drive(&self, now: Instant, target: f32) {
        self.animate_to(now, target);
    }

    /// Drives an explicit interval, resetting the current value to `from`.
    pub fn drive_range(&self, now: Instant, from: f32, to: f32) {
        self.set_value(from);
        self.animate_to(now, to);
    }

    /// Starts a repeating lower-bound-to-upper-bound animation. When
    /// `reverse` is true, every other cycle runs backwards.
    pub fn repeat(&self, now: Instant, reverse: bool) {
        let (lower, upper) = self.bounds();
        {
            let mut controller = self.inner.borrow_mut();
            controller.state.value = lower;
            controller.state.from = lower;
            controller.state.to = upper;
            controller.state.started_at = Some(now);
            controller.state.pending_start = false;
            controller.state.last_elapsed = Duration::ZERO;
            controller.state.status = AnimationStatus::Forward;
            controller.state.forward = true;
            controller.state.repeat = Some(RepeatState { reverse });
        }
        self.notify_current();
    }

    /// Alias documenting that `repeat` remains active until stopped.
    pub fn repeat_forever(&self, now: Instant, reverse: bool) {
        self.repeat(now, reverse);
    }

    pub fn stop_repeat(&self) {
        self.inner.borrow_mut().state.repeat = None;
    }

    /// Stops the running animation without changing its value or status, and
    /// without notifying listeners, as Flutter's `stop` does.
    pub fn stop(&self) {
        let mut controller = self.inner.borrow_mut();
        controller.state.started_at = None;
        controller.state.pending_start = false;
        controller.state.repeat = None;
    }

    pub fn reset(&self) {
        let (value, listeners) = {
            let mut controller = self.inner.borrow_mut();
            let state = &mut controller.state;
            state.value = state.lower_bound;
            state.from = state.lower_bound;
            state.to = state.upper_bound;
            state.status = AnimationStatus::Dismissed;
            state.started_at = None;
            state.pending_start = false;
            state.repeat = None;
            state.last_elapsed = Duration::ZERO;
            (state.value, clone_listeners(&controller))
        };
        notify(listeners, Some(value));
        self.changes.notify();
    }

    pub fn set_value(&self, value: f32) {
        let (changed, listeners) = {
            let mut controller = self.inner.borrow_mut();
            let state = &mut controller.state;
            let old_value = state.value;
            let next = if value.is_finite() {
                value.clamp(state.lower_bound, state.upper_bound)
            } else {
                state.lower_bound
            };
            let old_status = state.status;
            state.value = next;
            state.status = state.status_at(next);
            state.started_at = None;
            state.pending_start = false;
            state.repeat = None;
            let status_changed = old_status != state.status;
            let value_changed = next != old_value;
            (
                (value_changed || status_changed).then_some(next),
                clone_listeners(&controller),
            )
        };
        notify(listeners, changed);
        if changed.is_some() {
            self.changes.notify();
        }
    }

    /// Advances the controller and returns whether value or status changed.
    pub fn tick(&self, now: Instant) -> bool {
        let (changed, listeners, value) = {
            let mut controller = self.inner.borrow_mut();
            let (changed, value) = {
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
                if !matches!(
                    state.status,
                    AnimationStatus::Forward | AnimationStatus::Reverse
                ) {
                    return false;
                }
                let elapsed = now.saturating_duration_since(started);
                state.last_elapsed = elapsed;
                let duration = if state.repeat.is_some() {
                    state.duration
                } else {
                    state.run_duration
                };
                let old_value = state.value;
                let old_status = state.status;
                let raw_progress = if duration.is_zero() {
                    1.0
                } else {
                    elapsed.as_secs_f32() / duration.as_secs_f32()
                };

                if let Some(repeat) = state.repeat {
                    if duration.is_zero() {
                        state.value = if repeat.reverse {
                            state.lower_bound
                        } else {
                            state.upper_bound
                        };
                    } else {
                        let cycle = raw_progress.floor() as u64;
                        let mut local = raw_progress.fract();
                        let backwards = repeat.reverse && cycle % 2 == 1;
                        if backwards {
                            local = 1.0 - local;
                        }
                        let (from, to) = if backwards {
                            (state.upper_bound, state.lower_bound)
                        } else {
                            (state.lower_bound, state.upper_bound)
                        };
                        state.value = Tween::lerp(from, to, state.curve.apply(local))
                            .clamp(state.lower_bound, state.upper_bound);
                        state.status = if backwards {
                            AnimationStatus::Reverse
                        } else {
                            AnimationStatus::Forward
                        };
                    }
                } else {
                    let progress = raw_progress.min(1.0);
                    state.value = Tween::lerp(state.from, state.to, state.curve.apply(progress))
                        .clamp(state.lower_bound, state.upper_bound);
                    if raw_progress >= 1.0 {
                        state.status = if state.to <= state.lower_bound {
                            AnimationStatus::Dismissed
                        } else {
                            AnimationStatus::Completed
                        };
                        state.started_at = None;
                        state.pending_start = false;
                    }
                }
                let changed = state.value != old_value || state.status != old_status;
                (changed, state.value)
            };
            (changed, clone_listeners(&controller), value)
        };
        if changed {
            notify(listeners, Some(value));
            self.changes.notify();
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
        self.inner.borrow().state.status
    }

    /// Whether the animation is currently running, as Flutter's `isAnimating`.
    /// A stopped animation keeps its forward or reverse status.
    #[must_use]
    pub fn is_active(&self) -> bool {
        let state = self.inner.borrow().state;
        (state.started_at.is_some() || state.pending_start)
            && matches!(
                state.status,
                AnimationStatus::Forward | AnimationStatus::Reverse
            )
    }

    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.status() == AnimationStatus::Completed
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.inner.borrow().state.last_elapsed
    }

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
        let id = controller.next_listener;
        controller.next_listener = controller
            .next_listener
            .checked_add(1)
            .expect("animation listener identity space exhausted");
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

    fn start(&self, now: Option<Instant>, from: f32, to: f32, status: AnimationStatus) {
        let (from, to) = {
            let state = self.inner.borrow().state;
            (
                from.clamp(state.lower_bound, state.upper_bound),
                to.clamp(state.lower_bound, state.upper_bound),
            )
        };
        {
            let mut controller = self.inner.borrow_mut();
            let state = &mut controller.state;
            let fraction = remaining_fraction(from, to, state.lower_bound, state.upper_bound);
            state.from = from;
            state.to = to;
            state.status = status;
            state.forward = status == AnimationStatus::Forward;
            state.run_duration = state
                .duration
                .mul_f32(fraction * behavior_scale(state.animation_behavior));
            state.started_at = now;
            state.pending_start = now.is_none();
            state.last_elapsed = Duration::ZERO;
            state.repeat = None;
        }
        self.notify_current();
    }

    fn notify_current(&self) {
        let listeners = clone_listeners(&self.inner.borrow());
        let value = self.inner.borrow().state.value;
        notify(listeners, Some(value));
        self.changes.notify();
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

fn collect_listeners(
    controller: &ControllerState,
    next: f32,
    old: f32,
) -> (Option<f32>, Vec<Listener>) {
    ((next != old).then_some(next), clone_listeners(controller))
}

fn clone_listeners(controller: &ControllerState) -> Vec<Listener> {
    controller
        .listeners
        .iter()
        .map(|(_, listener)| listener.clone())
        .collect()
}

fn notify(listeners: Vec<Listener>, value: Option<f32>) {
    if let Some(value) = value {
        for listener in listeners {
            listener(value);
        }
    }
}
