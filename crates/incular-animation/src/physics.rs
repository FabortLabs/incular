//! Time-based UI physics simulations.
//!
//! The formulas and completion rules follow Flutter 3.41.7
//! `packages/flutter/lib/src/physics`. Intermediate arithmetic is carried in
//! `f64`, as Dart does, and narrowed to `f32` at the [`Simulation`] boundary.

use std::f64::consts::PI;
use std::time::Duration;

/// Tolerances used to decide when a physical simulation has settled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    pub distance: f32,
    pub time: Duration,
    pub velocity: f32,
}

impl Tolerance {
    pub const DEFAULT: Self = Self {
        distance: 0.001,
        time: Duration::from_millis(1),
        velocity: 0.001,
    };

    #[must_use]
    pub const fn new(distance: f32, time: Duration, velocity: f32) -> Self {
        Self {
            distance,
            time,
            velocity,
        }
    }
}

impl Default for Tolerance {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Flutter `nearEqual`: whether `a` lies strictly within `epsilon` of `b`, or equals it.
/// An absent value is equal only to another absent value.
#[must_use]
pub fn near_equal(a: impl Into<Option<f32>>, b: impl Into<Option<f32>>, epsilon: f32) -> bool {
    debug_assert!(epsilon >= 0.0, "epsilon must be nonnegative");
    match (a.into(), b.into()) {
        (Some(a), Some(b)) => near_equal_f64(f64::from(a), f64::from(b), f64::from(epsilon)),
        (None, None) => true,
        _ => false,
    }
}

/// Flutter `nearZero`: whether `a` lies strictly within `epsilon` of zero, or is zero.
#[must_use]
pub fn near_zero(a: f32, epsilon: f32) -> bool {
    near_equal(a, 0.0, epsilon)
}

fn near_equal_f64(a: f64, b: f64, epsilon: f64) -> bool {
    (a > b - epsilon && a < b + epsilon) || a == b
}

fn near_zero_f64(a: f64, epsilon: f64) -> bool {
    near_equal_f64(a, 0.0, epsilon)
}

/// Flutter `clampDouble` for the bounds this module uses; it never panics
/// on inverted bounds, unlike `f32::clamp`.
fn clamp_value(value: f32, min: f32, max: f32) -> f32 {
    clamp_value_f64(f64::from(value), f64::from(min), f64::from(max)) as f32
}

fn clamp_value_f64(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max || value.is_nan() {
        max
    } else {
        value
    }
}

/// Newton's method with a fixed iteration count, as Flutter's `_newtonsMethod`.
fn newton(
    initial_guess: f64,
    target: f64,
    iterations: u32,
    f: impl Fn(f64) -> f64,
    df: impl Fn(f64) -> f64,
) -> f64 {
    let mut guess = initial_guess;
    for _ in 0..iterations {
        guess -= (f(guess) - target) / df(guess);
    }
    guess
}

/// Damping classification of a harmonic spring.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpringType {
    CriticallyDamped,
    UnderDamped,
    OverDamped,
}

/// Physical parameters for a damped harmonic oscillator.
///
/// Values are stored as given, matching Flutter's `SpringDescription`. Invalid
/// physical values such as a nonpositive mass produce the arithmetic results
/// Flutter produces; callers are responsible for validating them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringDescription {
    pub mass: f32,
    pub stiffness: f32,
    pub damping: f32,
}

impl SpringDescription {
    #[must_use]
    pub const fn new(mass: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            mass,
            stiffness,
            damping,
        }
    }

    /// Flutter `SpringDescription.withDampingRatio`: `damping = ratio * 2 * sqrt(mass * stiffness)`.
    #[must_use]
    pub fn with_damping_ratio(mass: f32, stiffness: f32, ratio: f32) -> Self {
        let damping = f64::from(ratio) * 2.0 * (f64::from(mass) * f64::from(stiffness)).sqrt();
        Self {
            mass,
            stiffness,
            damping: damping as f32,
        }
    }

    /// Flutter `SpringDescription.withDurationAndBounce`.
    ///
    /// Flutter truncates the duration to whole milliseconds and asserts that it
    /// is positive. Release builds retain a one-millisecond fallback for
    /// invalid durations. The bounce is not clamped, so a negative bounce
    /// gives an overdamped spring.
    #[must_use]
    pub fn with_duration_and_bounce(duration: Duration, bounce: f32) -> Self {
        debug_assert!(
            duration.as_millis() > 0,
            "spring duration must be at least one millisecond"
        );
        let millis = duration.as_millis().max(1) as f64;
        let seconds = millis / 1000.0;
        let mass = 1.0_f64;
        let stiffness = 4.0 * PI * PI * mass / (seconds * seconds);
        let bounce = f64::from(bounce);
        let ratio = if bounce > 0.0 {
            1.0 - bounce
        } else {
            1.0 / (bounce + 1.0)
        };
        // A zero bounce is critical in the stored precision, not merely
        // critical before the two coefficients are separately rounded.
        let damping = if ratio == 1. {
            2.0 * (mass * f64::from(stiffness as f32)).sqrt()
        } else {
            ratio * 2.0 * (mass * stiffness).sqrt()
        };
        Self {
            mass: mass as f32,
            stiffness: stiffness as f32,
            damping: damping as f32,
        }
    }

    /// Flutter `SpringDescription.duration`: the period implied by the mass and
    /// stiffness, rounded to whole milliseconds.
    #[must_use]
    pub fn duration(&self) -> Duration {
        let seconds = (4.0 * PI * PI * f64::from(self.mass) / f64::from(self.stiffness)).sqrt();
        let millis = (seconds * 1000.0).round();
        Duration::from_millis(millis as u64)
    }

    /// Flutter `SpringDescription.bounce`: the bounce implied by the damping ratio.
    #[must_use]
    pub fn bounce(&self) -> f32 {
        let critical = 2.0 * (f64::from(self.mass) * f64::from(self.stiffness)).sqrt();
        if self.damping > 0. && self.damping.is_finite() && self.damping == critical as f32 {
            return 0.;
        }
        let ratio = f64::from(self.damping) / critical;
        let bounce = if ratio < 1.0 {
            1.0 - ratio
        } else {
            1.0 / ratio - 1.0
        };
        bounce as f32
    }

    #[must_use]
    pub fn spring_type(&self) -> SpringType {
        let discriminant = self.discriminant();
        if discriminant > 0.0 {
            SpringType::OverDamped
        } else if discriminant < 0.0 {
            SpringType::UnderDamped
        } else {
            SpringType::CriticallyDamped
        }
    }

    fn discriminant(&self) -> f64 {
        let squared_critical = 4.0 * f64::from(self.mass) * f64::from(self.stiffness);
        let damping = f64::from(self.damping);
        // No interval of near-critical damping is treated as equal: only
        // the single f32 value that represents critical damping can match.
        if self.damping.is_finite() && self.damping.abs() == squared_critical.sqrt() as f32 {
            0.
        } else {
            damping * damping - squared_critical
        }
    }
}

/// Abstract time-based scalar simulation.
pub trait Simulation {
    /// Computes position at elapsed time `t`.
    fn position(&self, t: Duration) -> f32;
    /// Computes velocity at elapsed time `t`.
    fn velocity(&self, t: Duration) -> f32;
    /// Checks whether the simulation has settled at elapsed time `t`.
    fn is_done(&self, t: Duration) -> bool;
    /// The tolerances this simulation uses to decide completion (Flutter
    /// `Simulation.tolerance`).
    ///
    /// Implementors that do not store a tolerance report [`Tolerance::DEFAULT`].
    /// The concrete simulations in this module store theirs and expose
    /// `set_tolerance` for updates; a trait object cannot be mutated through
    /// this trait without a breaking change.
    fn tolerance(&self) -> Tolerance {
        Tolerance::DEFAULT
    }
}

/// Position and derivative of the harmonic solution, evaluated in `f64`.
///
/// Mirrors Flutter's `_SpringSolution` family, which chooses the form from the
/// sign of the discriminant. Velocity is the analytic derivative of each form.
fn spring_solution(
    spring: SpringDescription,
    discriminant: f64,
    initial_displacement: f64,
    initial_velocity: f64,
    t: Duration,
) -> (f64, f64) {
    let sec = t.as_secs_f64();
    let m = f64::from(spring.mass);
    let c = f64::from(spring.damping);
    let x0 = initial_displacement;
    let v0 = initial_velocity;

    if discriminant > 0.0 {
        // Overdamped, Flutter `_OverdampedSolution`.
        let root = discriminant.sqrt();
        let r1 = (-c - root) / (2.0 * m);
        let r2 = (-c + root) / (2.0 * m);
        let c2 = (v0 - r1 * x0) / (r2 - r1);
        let c1 = x0 - c2;
        let e1 = (r1 * sec).exp();
        let e2 = (r2 * sec).exp();
        (c1 * e1 + c2 * e2, c1 * r1 * e1 + c2 * r2 * e2)
    } else if discriminant < 0.0 {
        // Underdamped, Flutter `_UnderdampedSolution`.
        let w = (-discriminant).sqrt() / (2.0 * m);
        let r = -c / (2.0 * m);
        let c1 = x0;
        let c2 = (v0 - r * x0) / w;
        let power = (r * sec).exp();
        let cosine = (w * sec).cos();
        let sine = (w * sec).sin();
        let x = power * (c1 * cosine + c2 * sine);
        let dx = power * (c2 * w * cosine - c1 * w * sine) + r * power * (c2 * sine + c1 * cosine);
        (x, dx)
    } else {
        // Critically damped, Flutter `_CriticalSolution`.
        let r = -c / (2.0 * m);
        let c1 = x0;
        let c2 = v0 - r * x0;
        let power = (r * sec).exp();
        let shape = c1 + c2 * sec;
        (shape * power, r * shape * power + c2 * power)
    }
}

/// A damped harmonic spring simulation.
#[derive(Clone, Debug, PartialEq)]
pub struct SpringSimulation {
    spring: SpringDescription,
    discriminant: f64,
    start: f32,
    end: f32,
    initial_velocity: f32,
    tolerance: Tolerance,
    snap_to_end: bool,
}

impl SpringSimulation {
    #[must_use]
    pub fn new(
        spring: SpringDescription,
        start: f32,
        end: f32,
        initial_velocity: f32,
        tolerance: Tolerance,
    ) -> Self {
        Self {
            spring,
            discriminant: spring.discriminant(),
            start,
            end,
            initial_velocity,
            tolerance,
            snap_to_end: false,
        }
    }

    /// Flutter `snapToEnd`: once the simulation is done, report the end
    /// position and zero velocity exactly.
    #[must_use]
    pub fn with_snap_to_end(mut self, snap_to_end: bool) -> Self {
        self.snap_to_end = snap_to_end;
        self
    }

    /// Replaces the completion tolerances.
    pub fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
    }

    /// Displacement from the end point and its derivative at `t`.
    fn solution(&self, t: Duration) -> (f64, f64) {
        spring_solution(
            self.spring,
            self.discriminant,
            f64::from(self.start) - f64::from(self.end),
            f64::from(self.initial_velocity),
            t,
        )
    }
}

impl Simulation for SpringSimulation {
    fn position(&self, t: Duration) -> f32 {
        if self.snap_to_end && self.is_done(t) {
            return self.end;
        }
        (f64::from(self.end) + self.solution(t).0) as f32
    }

    fn velocity(&self, t: Duration) -> f32 {
        if self.snap_to_end && self.is_done(t) {
            return 0.0;
        }
        self.solution(t).1 as f32
    }

    /// Flutter `SpringSimulation.isDone`: strictly within both tolerances.
    fn is_done(&self, t: Duration) -> bool {
        let (displacement, velocity) = self.solution(t);
        near_zero_f64(displacement, f64::from(self.tolerance.distance))
            && near_zero_f64(velocity, f64::from(self.tolerance.velocity))
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

/// Spring simulation tailored for scroll boundaries and overscroll recovery.
///
/// Reports the end position once done, as Flutter's `ScrollSpringSimulation`.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollSpringSimulation {
    inner: SpringSimulation,
}

impl ScrollSpringSimulation {
    #[must_use]
    pub fn new(
        spring: SpringDescription,
        start: f32,
        end: f32,
        initial_velocity: f32,
        tolerance: Tolerance,
    ) -> Self {
        Self {
            inner: SpringSimulation::new(spring, start, end, initial_velocity, tolerance),
        }
    }

    /// Replaces the completion tolerances.
    pub fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.inner.set_tolerance(tolerance);
    }
}

impl Simulation for ScrollSpringSimulation {
    fn position(&self, t: Duration) -> f32 {
        if self.inner.is_done(t) {
            self.inner.end
        } else {
            self.inner.position(t)
        }
    }

    fn velocity(&self, t: Duration) -> f32 {
        self.inner.velocity(t)
    }

    fn is_done(&self, t: Duration) -> bool {
        self.inner.is_done(t)
    }

    fn tolerance(&self) -> Tolerance {
        self.inner.tolerance
    }
}

/// Kinetic friction decay simulation, with optional constant deceleration.
///
/// Mirrors Flutter's `FrictionSimulation`, including the Newton-solved final
/// time after which position and velocity stop evolving.
#[derive(Clone, Debug, PartialEq)]
pub struct FrictionSimulation {
    drag: f64,
    drag_log: f64,
    start: f64,
    initial_velocity: f64,
    /// Already signed by the initial velocity, as Flutter stores it.
    constant_deceleration: f64,
    final_time: f64,
    tolerance: Tolerance,
    velocity_threshold: f64,
}

impl FrictionSimulation {
    #[must_use]
    pub fn new(drag: f32, start: f32, initial_velocity: f32, tolerance: Tolerance) -> Self {
        Self::new_precise(
            f64::from(drag),
            f64::from(start),
            f64::from(initial_velocity),
            tolerance,
        )
    }

    /// Creates a simulation without narrowing the input coefficients.
    /// Use the precise sampling methods when the result must not become f32.
    #[must_use]
    pub fn new_precise(drag: f64, start: f64, initial_velocity: f64, tolerance: Tolerance) -> Self {
        let drag = if drag > 0.0 { drag } else { 0.135 };
        Self::build(drag, start, initial_velocity, 0.0, tolerance)
    }

    /// Flutter `FrictionSimulation(..., constantDeceleration:)`: a deceleration
    /// of constant magnitude opposing the motion, applied on top of the
    /// exponential drag (desktop friction).
    #[must_use]
    pub fn with_constant_deceleration(self, deceleration: impl Into<f64>) -> Self {
        let signed = deceleration.into() * sign(self.initial_velocity);
        Self {
            velocity_threshold: self.velocity_threshold,
            ..Self::build(
                self.drag,
                self.start,
                self.initial_velocity,
                signed,
                self.tolerance,
            )
        }
    }

    /// Flutter `FrictionSimulation.through`: the drag that passes through the
    /// given start and end position and velocity. The completion velocity
    /// tolerance is `|end_velocity|`.
    ///
    /// Flutter asserts that the velocities share a direction, that the start
    /// speed is at least the end speed, and that the position moves in the
    /// velocity's direction; those inputs are not checked here.
    #[must_use]
    pub fn through(
        start_position: impl Into<f64>,
        end_position: impl Into<f64>,
        start_velocity: impl Into<f64>,
        end_velocity: impl Into<f64>,
    ) -> Self {
        let start_position = start_position.into();
        let end_position = end_position.into();
        let start_velocity = start_velocity.into();
        let end_velocity = end_velocity.into();
        let drag = ((start_velocity - end_velocity) / (start_position - end_position)).exp();
        let tolerance = Tolerance {
            velocity: end_velocity.abs() as f32,
            ..Tolerance::DEFAULT
        };
        Self {
            velocity_threshold: end_velocity.abs(),
            ..Self::build(drag, start_position, start_velocity, 0.0, tolerance)
        }
    }

    /// Replaces the completion tolerances.
    pub fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.velocity_threshold = f64::from(tolerance.velocity);
        self.tolerance = tolerance;
    }

    /// Flutter `FrictionSimulation.finalX`: the position the motion settles at.
    #[must_use]
    pub fn final_x(&self) -> f32 {
        self.final_x_value() as f32
    }

    /// Flutter's position at a time in seconds, without Duration or f32 rounding.
    #[must_use]
    pub fn x(&self, seconds: f64) -> f64 {
        self.x_at(seconds)
    }

    /// Flutter's velocity at a time in seconds, without Duration or f32 rounding.
    #[must_use]
    pub fn dx(&self, seconds: f64) -> f64 {
        self.dx_at(seconds)
    }

    /// Evaluates the strict completion threshold at a precise time.
    #[must_use]
    pub fn is_done_at_seconds(&self, seconds: f64) -> bool {
        self.dx(seconds).abs() < self.velocity_threshold
    }

    /// The final position without narrowing to a layout coordinate.
    #[must_use]
    pub fn final_x_f64(&self) -> f64 {
        self.final_x_value()
    }

    /// Flutter `FrictionSimulation.timeAtX`: the time at which the motion reaches
    /// `x`, or `None` when it never does (Flutter returns infinity).
    #[must_use]
    pub fn time_at_x(&self, x: f32) -> Option<Duration> {
        Duration::try_from_secs_f64(self.time_at_x_seconds(f64::from(x))).ok()
    }

    /// The precise inverse of [`Self::x`]. An unreachable target returns infinity.
    #[must_use]
    pub fn time_at_x_seconds(&self, x: f64) -> f64 {
        if x == self.start {
            return 0.0;
        }
        let final_x = self.final_x_value();
        let unreachable = self.initial_velocity == 0.0
            || if self.initial_velocity > 0.0 {
                x < self.start || x > final_x
            } else {
                x > self.start || x < final_x
            };
        if unreachable {
            return f64::INFINITY;
        }
        newton(0.0, x, 10, |t| self.x(t), |t| self.dx(t))
    }

    fn build(
        drag: f64,
        start: f64,
        initial_velocity: f64,
        constant_deceleration: f64,
        tolerance: Tolerance,
    ) -> Self {
        let mut simulation = Self {
            drag,
            drag_log: drag.ln(),
            start,
            initial_velocity,
            constant_deceleration,
            final_time: f64::INFINITY,
            tolerance,
            velocity_threshold: f64::from(tolerance.velocity),
        };
        // Flutter solves for the time the velocity reaches zero, starting from
        // time 0 with ten Newton iterations. The cutoff is infinite during this
        // solve, so the raw velocity is used.
        let final_time = newton(
            0.0,
            0.0,
            10,
            |t| simulation.raw_dx(t),
            |t| simulation.raw_ddx(t),
        );
        simulation.final_time = final_time;
        simulation
    }

    fn raw_x(&self, t: f64) -> f64 {
        self.start + self.initial_velocity * self.drag.powf(t) / self.drag_log
            - self.initial_velocity / self.drag_log
            - (self.constant_deceleration / 2.0) * t * t
    }

    fn raw_dx(&self, t: f64) -> f64 {
        self.initial_velocity * self.drag.powf(t) - self.constant_deceleration * t
    }

    fn raw_ddx(&self, t: f64) -> f64 {
        self.initial_velocity * self.drag.powf(t) * self.drag_log - self.constant_deceleration
    }

    fn x_at(&self, t: f64) -> f64 {
        if t > self.final_time {
            self.final_x_value()
        } else {
            self.raw_x(t)
        }
    }

    fn dx_at(&self, t: f64) -> f64 {
        if t > self.final_time {
            0.0
        } else {
            self.raw_dx(t)
        }
    }

    fn final_x_value(&self) -> f64 {
        if self.constant_deceleration == 0.0 {
            self.start - self.initial_velocity / self.drag_log
        } else {
            self.x_at(self.final_time)
        }
    }
}

impl Simulation for FrictionSimulation {
    fn position(&self, t: Duration) -> f32 {
        self.x_at(t.as_secs_f64()) as f32
    }

    fn velocity(&self, t: Duration) -> f32 {
        self.dx_at(t.as_secs_f64()) as f32
    }

    /// Flutter `FrictionSimulation.isDone`: the velocity is strictly below the tolerance.
    fn is_done(&self, t: Duration) -> bool {
        self.is_done_at_seconds(t.as_secs_f64())
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Kinetic friction simulation clamped within `[min, max]`.
///
/// Mirrors Flutter's `BoundedFrictionSimulation`: the position is clamped, the
/// velocity is the friction velocity, and completion also occurs when the
/// clamped position is within the distance tolerance of either bound.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundedFrictionSimulation {
    friction: FrictionSimulation,
    min: f32,
    max: f32,
}

impl BoundedFrictionSimulation {
    #[must_use]
    pub fn new(drag: f32, start: f32, initial_velocity: f32, min: f32, max: f32) -> Self {
        Self {
            friction: FrictionSimulation::new(drag, start, initial_velocity, Tolerance::DEFAULT),
            min,
            max,
        }
    }

    /// Replaces the completion tolerances.
    pub fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.friction.set_tolerance(tolerance);
    }

    fn clamped_position(&self, t: Duration) -> f64 {
        clamp_value_f64(
            self.friction.x_at(t.as_secs_f64()),
            f64::from(self.min),
            f64::from(self.max),
        )
    }
}

impl Simulation for BoundedFrictionSimulation {
    fn position(&self, t: Duration) -> f32 {
        self.clamped_position(t) as f32
    }

    fn velocity(&self, t: Duration) -> f32 {
        self.friction.velocity(t)
    }

    fn is_done(&self, t: Duration) -> bool {
        if self.friction.is_done(t) {
            return true;
        }
        let position = self.clamped_position(t);
        let distance = f64::from(self.friction.tolerance.distance);
        (position - f64::from(self.min)).abs() < distance
            || (position - f64::from(self.max)).abs() < distance
    }

    fn tolerance(&self) -> Tolerance {
        self.friction.tolerance
    }
}

/// Constant gravitational acceleration simulation.
#[derive(Clone, Debug, PartialEq)]
pub struct GravitySimulation {
    acceleration: f64,
    start: f32,
    end: f32,
    initial_velocity: f32,
    tolerance: Tolerance,
}

impl GravitySimulation {
    #[must_use]
    pub fn new(acceleration: impl Into<f64>, start: f32, end: f32, initial_velocity: f32) -> Self {
        Self {
            // Keep the input coefficient precise before multiplying by t².
            acceleration: acceleration.into(),
            start,
            end,
            initial_velocity,
            tolerance: Tolerance::DEFAULT,
        }
    }

    /// Replaces the tolerances. Gravity completion uses only the end distance,
    /// so this is stored for the [`Simulation::tolerance`] query.
    pub fn set_tolerance(&mut self, tolerance: Tolerance) {
        self.tolerance = tolerance;
    }

    fn position_f64(&self, t: f64) -> f64 {
        f64::from(self.start)
            + f64::from(self.initial_velocity) * t
            + 0.5 * self.acceleration * t * t
    }
}

impl Simulation for GravitySimulation {
    fn position(&self, t: Duration) -> f32 {
        self.position_f64(t.as_secs_f64()) as f32
    }

    fn velocity(&self, t: Duration) -> f32 {
        (f64::from(self.initial_velocity) + self.acceleration * t.as_secs_f64()) as f32
    }

    /// Flutter `GravitySimulation.isDone`: the distance from the origin reaches
    /// the end distance, whichever way the acceleration points.
    fn is_done(&self, t: Duration) -> bool {
        self.position_f64(t.as_secs_f64()).abs() >= f64::from(self.end)
    }

    fn tolerance(&self) -> Tolerance {
        self.tolerance
    }
}

/// Clamped simulation decorator bounding positions within `[min, max]` and
/// velocities within `[dx_min, dx_max]`.
///
/// Completion forwards to the wrapped simulation, as Flutter's
/// `ClampedSimulation.isDone` does. Tolerances also forward, so set them on the
/// wrapped simulation before wrapping it.
#[derive(Clone, Debug, PartialEq)]
pub struct ClampedSimulation<S: Simulation> {
    inner: S,
    min: f32,
    max: f32,
    dx_min: f32,
    dx_max: f32,
}

impl<S: Simulation> ClampedSimulation<S> {
    /// Clamps positions to `[min, max]`; velocities are unbounded.
    #[must_use]
    pub fn new(inner: S, min: f32, max: f32) -> Self {
        debug_assert!(min <= max, "inverted position bounds");
        Self {
            inner,
            min,
            max,
            dx_min: f32::NEG_INFINITY,
            dx_max: f32::INFINITY,
        }
    }

    /// Flutter `dxMin` and `dxMax`: clamps the reported velocity to `[dx_min, dx_max]`.
    #[must_use]
    pub fn with_velocity_bounds(mut self, dx_min: f32, dx_max: f32) -> Self {
        debug_assert!(dx_min <= dx_max, "inverted velocity bounds");
        self.dx_min = dx_min;
        self.dx_max = dx_max;
        self
    }
}

impl<S: Simulation> Simulation for ClampedSimulation<S> {
    fn position(&self, t: Duration) -> f32 {
        clamp_value(self.inner.position(t), self.min, self.max)
    }

    fn velocity(&self, t: Duration) -> f32 {
        clamp_value(self.inner.velocity(t), self.dx_min, self.dx_max)
    }

    fn is_done(&self, t: Duration) -> bool {
        self.inner.is_done(t)
    }

    fn tolerance(&self) -> Tolerance {
        self.inner.tolerance()
    }
}
