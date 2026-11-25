//! Retained motion primitives: perceptual springs, transitions and
//! velocity-preserving [`MotionValue`]s.
//!
//! A [`MotionValue`] is a retained scalar that animates toward targets with a
//! [`Transition`]. Retargeting mid-flight starts from the current value and,
//! for springs, the current velocity, so interrupted motion never jumps. The
//! value is advanced by an owner (normally a retained widget node) calling
//! [`MotionValue::tick`] with the tree's animation clock; the first tick after
//! a retarget fixes the start time, so a slow build never eats into an
//! animation.

use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
    time::Duration,
};

use incular_core::time::Instant;

use crate::{Curve, Simulation, SpringDescription, SpringSimulation, Tolerance};

/// `Duration::from_millis`, for terse motion code: `Transition::ease(ms(200))`.
#[must_use]
pub const fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Fractional seconds as a [`Duration`]; negative or non-finite input is
/// zero, and input too large for a `Duration` saturates to [`Duration::MAX`].
#[must_use]
pub fn secs(seconds: f32) -> Duration {
    if seconds.is_finite() && seconds > 0. {
        Duration::try_from_secs_f32(seconds).unwrap_or(Duration::MAX)
    } else {
        Duration::ZERO
    }
}

/// Scales `duration` by a finite, non-negative `factor`, saturating instead
/// of panicking when the product does not fit.
fn scale_duration(duration: Duration, factor: f32) -> Duration {
    // `Duration::mul_f32` is `from_secs_f32(as_secs_f32() * factor)`, which
    // panics on overflow; this is the same product, saturated.
    Duration::try_from_secs_f32(duration.as_secs_f32() * factor).unwrap_or(Duration::MAX)
}

/// A spring described by how long it takes to visually settle and how much it
/// bounces, as designers reason about motion. [`Spring::physical`] is the
/// escape hatch to mass, stiffness and damping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    description: SpringDescription,
}

impl Spring {
    /// A spring that settles in about `duration` with `bounce` (0 is no
    /// overshoot, 0.3 is playful, negative values are overdamped).
    #[must_use]
    pub fn new(duration: Duration, bounce: f32) -> Self {
        let bounce = if bounce.is_finite() {
            bounce.clamp(-0.99, 0.99)
        } else {
            0.
        };
        Self {
            description: SpringDescription::with_duration_and_bounce(duration, bounce),
        }
    }

    /// A spring from raw physical parameters.
    #[must_use]
    pub fn physical(mass: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            description: SpringDescription::new(mass, stiffness, damping),
        }
    }

    /// Quick and decisive, no bounce (about 250 ms).
    #[must_use]
    pub fn snappy() -> Self {
        Self::new(ms(250), 0.)
    }

    /// The default: calm, no bounce (about 400 ms).
    #[must_use]
    pub fn smooth() -> Self {
        Self::new(ms(400), 0.)
    }

    /// Playful with a visible overshoot (about 450 ms, bounce 0.3).
    #[must_use]
    pub fn bouncy() -> Self {
        Self::new(ms(450), 0.3)
    }

    #[must_use]
    pub const fn description(&self) -> SpringDescription {
        self.description
    }
}

impl Default for Spring {
    fn default() -> Self {
        Self::smooth()
    }
}

/// How a value travels to a new target.
#[derive(Clone, Debug, PartialEq)]
pub enum TransitionKind {
    /// Jump to the target.
    Instant,
    /// A fixed-duration curve.
    Ease { duration: Duration, curve: Curve },
    /// A physical spring that keeps velocity across retargets.
    Spring(Spring),
}

/// The kind of value a transition is resolved for. Opacity and colors never
/// spring: under a spring transition they use a short ease unless a
/// per-property override says otherwise, so a bouncy fade cannot overshoot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Channel {
    /// Positions, scales, rotations, sizes.
    Geometry,
    /// Alpha.
    Opacity,
    /// Color and filter amounts.
    Color,
}

/// The default ease for opacity and colors under a spring transition.
pub const FADE_UNDER_SPRING: Duration = ms(150);

/// How values move to new targets: an instant jump, a curve or a spring, with
/// an optional delay and per-channel overrides.
#[derive(Clone, PartialEq)]
pub struct Transition {
    kind: TransitionKind,
    delay: Duration,
    opacity: Option<Rc<Transition>>,
    color: Option<Rc<Transition>>,
}

impl fmt::Debug for Transition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Transition")
            .field("kind", &self.kind)
            .field("delay", &self.delay)
            .field("opacity", &self.opacity)
            .field("color", &self.color)
            .finish()
    }
}

impl Default for Transition {
    /// [`Spring::smooth`].
    fn default() -> Self {
        Self::spring(Spring::smooth())
    }
}

impl Transition {
    const fn from_kind(kind: TransitionKind) -> Self {
        Self {
            kind,
            delay: Duration::ZERO,
            opacity: None,
            color: None,
        }
    }

    /// An ease-out curve over `duration`; change it with [`Self::curve`].
    #[must_use]
    pub const fn ease(duration: Duration) -> Self {
        Self::from_kind(TransitionKind::Ease {
            duration,
            curve: Curve::EaseOut,
        })
    }

    /// A linear tween over `duration`.
    #[must_use]
    pub const fn linear(duration: Duration) -> Self {
        Self::from_kind(TransitionKind::Ease {
            duration,
            curve: Curve::Linear,
        })
    }

    #[must_use]
    pub const fn spring(spring: Spring) -> Self {
        Self::from_kind(TransitionKind::Spring(spring))
    }

    /// No animation: values jump to their targets.
    #[must_use]
    pub const fn instant() -> Self {
        Self::from_kind(TransitionKind::Instant)
    }

    /// Replaces the curve of an ease transition. Springs and instant
    /// transitions have no curve and are returned unchanged.
    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        if let TransitionKind::Ease { curve: current, .. } = &mut self.kind {
            *current = curve;
        }
        self
    }

    /// Waits `delay` before moving.
    #[must_use]
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// The transition used for opacity.
    #[must_use]
    pub fn opacity(mut self, transition: impl Into<Transition>) -> Self {
        self.opacity = Some(Rc::new(transition.into()));
        self
    }

    /// The transition used for colors and filter amounts.
    #[must_use]
    pub fn color(mut self, transition: impl Into<Transition>) -> Self {
        self.color = Some(Rc::new(transition.into()));
        self
    }

    #[must_use]
    pub fn kind(&self) -> TransitionKind {
        self.kind.clone()
    }

    #[must_use]
    pub const fn delay_duration(&self) -> Duration {
        self.delay
    }

    #[must_use]
    pub const fn is_instant(&self) -> bool {
        matches!(self.kind, TransitionKind::Instant)
    }

    /// The transition a value of `channel` actually uses: its override, or
    /// for opacity and colors under a spring, a short ease with this delay.
    #[must_use]
    pub fn for_channel(&self, channel: Channel) -> Transition {
        let override_ = match channel {
            Channel::Geometry => None,
            Channel::Opacity => self.opacity.as_deref(),
            Channel::Color => self.color.as_deref(),
        };
        if let Some(transition) = override_ {
            return transition.clone();
        }
        match (channel, &self.kind) {
            (Channel::Opacity | Channel::Color, TransitionKind::Spring(_)) => {
                Transition::ease(FADE_UNDER_SPRING).delay(self.delay)
            }
            _ => Transition::from_kind(self.kind.clone()).delay(self.delay),
        }
    }

    /// The same transition with every duration and delay scaled by `factor`
    /// (springs keep their physics; only delays scale).
    #[must_use]
    pub fn scaled(&self, factor: f32) -> Transition {
        let factor = if factor.is_finite() {
            factor.max(0.)
        } else {
            1.
        };
        let kind = match &self.kind {
            TransitionKind::Ease { duration, curve } => TransitionKind::Ease {
                duration: scale_duration(*duration, factor),
                curve: curve.clone(),
            },
            kind => kind.clone(),
        };
        Transition {
            kind,
            delay: scale_duration(self.delay, factor),
            opacity: self.opacity.as_ref().map(|t| Rc::new(t.scaled(factor))),
            color: self.color.as_ref().map(|t| Rc::new(t.scaled(factor))),
        }
    }
}

impl From<Spring> for Transition {
    fn from(spring: Spring) -> Self {
        Self::spring(spring)
    }
}

impl From<Duration> for Transition {
    /// An ease-out over the duration.
    fn from(duration: Duration) -> Self {
        Self::ease(duration)
    }
}

impl From<&Transition> for Transition {
    fn from(transition: &Transition) -> Self {
        transition.clone()
    }
}

thread_local! {
    static TRANSACTION: RefCell<Option<Transition>> = const { RefCell::new(None) };
}

/// Runs `update` so that every animation it causes, `.transition()`,
/// `layout()`, enter and exit, `shared()`, uses `transition` instead of its
/// local default. The override holds until the frame that consumes the
/// update ends ([`end_transaction`]).
///
/// ```ignore
/// with_transition(Spring::bouncy(), || sort_by.set(SortBy::Date));
/// ```
pub fn with_transition<R>(transition: impl Into<Transition>, update: impl FnOnce() -> R) -> R {
    let transition = transition.into();
    TRANSACTION.with(|current| *current.borrow_mut() = Some(transition));
    let result = update();
    crate::schedule_frame();
    result
}

/// Runs `update` so that everything it changes jumps to its new value.
pub fn without_transition<R>(update: impl FnOnce() -> R) -> R {
    with_transition(Transition::instant(), update)
}

/// The transition of the update being consumed this frame, if any.
#[must_use]
pub fn transaction() -> Option<Transition> {
    TRANSACTION
        .try_with(|current| current.borrow().clone())
        .ok()
        .flatten()
}

/// Ends the current transaction; frame drivers call this once a frame's
/// build, layout and compositor passes are done.
pub fn end_transaction() {
    let _ = TRANSACTION.try_with(|current| current.borrow_mut().take());
}

/// Values a keyframe track visits, evenly spaced unless `times` are given.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframes {
    values: Vec<f32>,
    times: Option<Vec<f32>>,
    duration: Duration,
    curve: Curve,
}

impl Keyframes {
    /// A track through `values` over `duration`. Each segment uses `curve`
    /// (default ease-in-out).
    #[must_use]
    pub fn new(values: impl IntoIterator<Item = f32>, duration: Duration) -> Self {
        Self {
            values: values.into_iter().collect(),
            times: None,
            duration,
            curve: Curve::EaseInOut,
        }
    }

    /// Explicit offsets in `[0, 1]`, one per value. Mismatched lengths fall
    /// back to even spacing.
    #[must_use]
    pub fn times(mut self, times: impl IntoIterator<Item = f32>) -> Self {
        self.times = Some(times.into_iter().map(|t| t.clamp(0., 1.)).collect());
        self
    }

    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    #[must_use]
    pub const fn duration(&self) -> Duration {
        self.duration
    }

    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    fn time_at(&self, index: usize) -> f32 {
        match &self.times {
            Some(times) if times.len() == self.values.len() => times[index],
            _ if self.values.len() <= 1 => 0.,
            _ => index as f32 / (self.values.len() - 1) as f32,
        }
    }

    /// The track's value at normalized progress `t`.
    #[must_use]
    pub fn sample(&self, t: f32) -> f32 {
        let Some(&first) = self.values.first() else {
            return 0.;
        };
        let t = t.clamp(0., 1.);
        if self.values.len() == 1 || t <= self.time_at(0) {
            return first;
        }
        for index in 1..self.values.len() {
            let end = self.time_at(index);
            if t <= end {
                let start = self.time_at(index - 1);
                let span = end - start;
                let local = if span <= 0. { 1. } else { (t - start) / span };
                let eased = self.curve.apply(local);
                let a = self.values[index - 1];
                let b = self.values[index];
                return a + (b - a) * eased;
            }
        }
        self.values[self.values.len() - 1]
    }
}

#[derive(Clone, Debug)]
enum Active {
    Ease {
        from: f32,
        to: f32,
        duration: Duration,
        curve: Curve,
        delay: Duration,
        start: Option<Instant>,
    },
    Spring {
        simulation: SpringSimulation,
        to: f32,
        delay: Duration,
        start: Option<Instant>,
    },
    Keys {
        track: Keyframes,
        delay: Duration,
        start: Option<Instant>,
    },
}

struct Source {
    value: MotionValue,
    map: Rc<dyn Fn(f32) -> f32>,
}

struct Inner {
    value: Cell<f32>,
    velocity: Cell<f32>,
    target: Cell<f32>,
    last_tick: Cell<Option<Instant>>,
    active: RefCell<Option<Active>>,
    source: Option<Source>,
    revision: Cell<u64>,
}

/// A retained, velocity-aware animated scalar.
///
/// Cloning shares the value. A value made by [`MotionValue::map`] is derived:
/// it reads its source on every access and cannot be set.
#[derive(Clone)]
pub struct MotionValue(Rc<Inner>);

impl fmt::Debug for MotionValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MotionValue")
            .field("value", &self.get())
            .field("velocity", &self.velocity())
            .field("animating", &self.is_animating())
            .finish()
    }
}

impl PartialEq for MotionValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Default for MotionValue {
    fn default() -> Self {
        Self::new(0.)
    }
}

/// Distance and velocity at which a spring is considered settled.
const SPRING_TOLERANCE: Tolerance = Tolerance {
    distance: 1e-3,
    time: Duration::from_millis(1),
    velocity: 1e-2,
};

fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

impl MotionValue {
    #[must_use]
    pub fn new(value: f32) -> Self {
        let value = finite(value, 0.);
        Self(Rc::new(Inner {
            value: Cell::new(value),
            velocity: Cell::new(0.),
            target: Cell::new(value),
            last_tick: Cell::new(None),
            active: RefCell::new(None),
            source: None,
            revision: Cell::new(0),
        }))
    }

    /// A derived value: `map(source)` on every read. Derived values follow
    /// their source without being ticked or rebuilt.
    #[must_use]
    pub fn map(&self, map: impl Fn(f32) -> f32 + 'static) -> Self {
        Self(Rc::new(Inner {
            value: Cell::new(0.),
            velocity: Cell::new(0.),
            target: Cell::new(0.),
            last_tick: Cell::new(None),
            active: RefCell::new(None),
            source: Some(Source {
                value: self.clone(),
                map: Rc::new(map),
            }),
            revision: Cell::new(0),
        }))
    }

    #[must_use]
    pub fn is_derived(&self) -> bool {
        self.0.source.is_some()
    }

    /// The current value.
    #[must_use]
    pub fn get(&self) -> f32 {
        match &self.0.source {
            Some(source) => finite((source.map)(source.value.get()), 0.),
            None => self.0.value.get(),
        }
    }

    /// The current velocity in units per second.
    #[must_use]
    pub fn velocity(&self) -> f32 {
        match &self.0.source {
            Some(source) => {
                // Finite difference through the mapping.
                let x = source.value.get();
                let v = source.value.velocity();
                let h = 1e-3;
                let slope = ((source.map)(x + h) - (source.map)(x - h)) / (2. * h);
                finite(slope * v, 0.)
            }
            None => self.0.velocity.get(),
        }
    }

    /// Where the value is heading (the current value when at rest).
    #[must_use]
    pub fn target(&self) -> f32 {
        match &self.0.source {
            Some(source) => finite((source.map)(source.value.target()), 0.),
            None => self.0.target.get(),
        }
    }

    #[must_use]
    pub fn is_animating(&self) -> bool {
        match &self.0.source {
            Some(source) => source.value.is_animating(),
            None => self.0.active.borrow().is_some(),
        }
    }

    /// Bumped whenever the value changes; cheap change detection for owners.
    #[must_use]
    pub fn revision(&self) -> u64 {
        match &self.0.source {
            Some(source) => source.value.revision(),
            None => self.0.revision.get(),
        }
    }

    fn write(&self, value: f32) -> bool {
        let value = finite(value, self.0.value.get());
        if self.0.value.get() == value {
            return false;
        }
        self.0.value.set(value);
        self.0.revision.set(self.0.revision.get().wrapping_add(1));
        true
    }

    /// Jumps to `value`, stopping any animation. Derived values ignore it.
    pub fn set(&self, value: f32) -> bool {
        if self.is_derived() {
            return false;
        }
        *self.0.active.borrow_mut() = None;
        self.0.velocity.set(0.);
        self.0.target.set(finite(value, self.0.target.get()));
        self.write(value)
    }

    /// Replaces the velocity used by the next spring retarget (for example
    /// a drag's release velocity). Derived values ignore it.
    pub fn set_velocity(&self, velocity: f32) {
        if !self.is_derived() {
            self.0.velocity.set(finite(velocity, 0.));
        }
    }

    /// Stops where it is, keeping the current value.
    pub fn stop(&self) {
        if self.is_derived() {
            return;
        }
        *self.0.active.borrow_mut() = None;
        self.0.velocity.set(0.);
        self.0.target.set(self.0.value.get());
    }

    /// Animates to `target`. Retargeting while moving starts from the
    /// current value and, for springs, the current velocity. An ease that
    /// reverses toward where it came from is shortened by the distance it
    /// had covered, as CSS transitions do. Returns false when nothing
    /// changes (already at, or already heading to, `target`).
    pub fn animate_to(&self, target: f32, transition: &Transition) -> bool {
        if self.is_derived() {
            return false;
        }
        let target = finite(target, self.0.target.get());
        let current = self.0.value.get();
        if self.0.target.get() == target && (self.is_animating() || current == target) {
            return false;
        }
        let delay = transition.delay_duration();
        let previous = self.0.active.borrow_mut().take();
        self.0.target.set(target);
        match transition.kind() {
            TransitionKind::Instant => {
                if delay.is_zero() {
                    self.0.velocity.set(0.);
                    return self.write(target);
                }
                *self.0.active.borrow_mut() = Some(Active::Ease {
                    from: current,
                    to: target,
                    duration: Duration::ZERO,
                    curve: Curve::Linear,
                    delay,
                    start: None,
                });
            }
            TransitionKind::Ease { duration, curve } => {
                let mut duration = duration;
                if let Some(Active::Ease {
                    from, to, start, ..
                }) = previous
                    && start.is_some()
                    && from == target
                    && to != from
                {
                    // Reversal shortening: return in the fraction of the
                    // duration the value had travelled.
                    let covered = ((current - from) / (to - from)).abs().clamp(0., 1.);
                    duration = duration.mul_f32(covered);
                }
                if duration.is_zero() && delay.is_zero() {
                    self.0.velocity.set(0.);
                    return self.write(target);
                }
                *self.0.active.borrow_mut() = Some(Active::Ease {
                    from: current,
                    to: target,
                    duration,
                    curve,
                    delay,
                    start: None,
                });
            }
            TransitionKind::Spring(spring) => {
                let simulation = SpringSimulation::new(
                    spring.description(),
                    current,
                    target,
                    self.0.velocity.get(),
                    SPRING_TOLERANCE,
                )
                .with_snap_to_end(true);
                *self.0.active.borrow_mut() = Some(Active::Spring {
                    simulation,
                    to: target,
                    delay,
                    start: None,
                });
            }
        }
        true
    }

    /// Plays a keyframe track from its first value, ending at its last.
    pub fn animate_keys(&self, track: Keyframes, delay: Duration) -> bool {
        if self.is_derived() || track.values().is_empty() {
            return false;
        }
        self.0
            .target
            .set(*track.values().last().expect("checked non-empty"));
        *self.0.active.borrow_mut() = Some(Active::Keys {
            track,
            delay,
            start: None,
        });
        true
    }

    /// Advances the animation to `now`. The first tick after a retarget
    /// fixes the start time. Returns whether the value changed.
    pub fn tick(&self, now: Instant) -> bool {
        if self.is_derived() {
            return false;
        }
        let previous_tick = self.0.last_tick.replace(Some(now));
        let mut active = self.0.active.borrow_mut();
        let Some(state) = active.as_mut() else {
            return false;
        };
        let before = self.0.value.get();
        let (value, velocity, done) = match state {
            Active::Ease {
                from,
                to,
                duration,
                curve,
                delay,
                start,
            } => {
                let start = *start.get_or_insert(now);
                let elapsed = now.saturating_duration_since(start);
                if elapsed < *delay {
                    (*from, 0., false)
                } else {
                    let elapsed = elapsed - *delay;
                    let t = if duration.is_zero() {
                        1.
                    } else {
                        (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.)
                    };
                    let value = *from + (*to - *from) * curve.apply(t);
                    let velocity = match previous_tick {
                        Some(last) if now > last => {
                            (value - before) / now.saturating_duration_since(last).as_secs_f32()
                        }
                        _ => 0.,
                    };
                    if t >= 1. {
                        (*to, 0., true)
                    } else {
                        (value, velocity, false)
                    }
                }
            }
            Active::Spring {
                simulation,
                to,
                delay,
                start,
            } => {
                let start = *start.get_or_insert(now);
                let elapsed = now.saturating_duration_since(start);
                if elapsed < *delay {
                    (before, self.0.velocity.get(), false)
                } else {
                    let t = elapsed - *delay;
                    if simulation.is_done(t) {
                        (*to, 0., true)
                    } else {
                        (simulation.position(t), simulation.velocity(t), false)
                    }
                }
            }
            Active::Keys {
                track,
                delay,
                start,
            } => {
                let start = *start.get_or_insert(now);
                let elapsed = now.saturating_duration_since(start);
                if elapsed < *delay {
                    (track.sample(0.), 0., false)
                } else {
                    let elapsed = elapsed - *delay;
                    let duration = track.duration();
                    let t = if duration.is_zero() {
                        1.
                    } else {
                        (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.)
                    };
                    (track.sample(t), 0., t >= 1.)
                }
            }
        };
        if done {
            *active = None;
        }
        drop(active);
        self.0.velocity.set(finite(velocity, 0.));
        self.write(value)
    }
}
