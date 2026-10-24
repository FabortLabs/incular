//! Ballistic scroll simulations ported from Flutter's `physics` library and
//! `widgets/scroll_simulation.dart`.
//!
//! Every simulation is a closed-form function of the elapsed time since it
//! started, so a driver evaluates [`FlingSimulation::x`] at the time since
//! the simulation began and the travel never depends on how frames partition
//! it.

/// Flutter `Tolerance` reduced to the two fields scrolling consults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollTolerance {
    /// Logical pixels per second under which a motion counts as stopped.
    pub velocity: f32,
    /// Logical pixels under which a position counts as arrived.
    pub distance: f32,
}

impl ScrollTolerance {
    /// Flutter `ScrollPhysics.toleranceFor`: `velocity = 1 / (0.050 * dpr)`
    /// and `distance = 1 / dpr`.
    #[must_use]
    pub fn for_device_pixel_ratio(device_pixel_ratio: f32) -> Self {
        let ratio = if device_pixel_ratio.is_finite() && device_pixel_ratio > 0. {
            device_pixel_ratio
        } else {
            1.
        };
        Self {
            velocity: 1. / (0.050 * ratio),
            distance: 1. / ratio,
        }
    }
}

/// Flutter `SpringDescription`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollSpring {
    /// Mass of the particle.
    pub mass: f32,
    /// Stiffness of the spring.
    pub stiffness: f32,
    /// Viscous damping coefficient.
    pub damping: f32,
}

impl ScrollSpring {
    /// Flutter `SpringDescription.withDampingRatio`.
    #[must_use]
    pub fn with_damping_ratio(mass: f32, stiffness: f32, ratio: f32) -> Self {
        Self {
            mass,
            stiffness,
            damping: ratio * 2. * (mass * stiffness).sqrt(),
        }
    }
}

/// Flutter `nearZero`.
fn near_zero(value: f64, epsilon: f64) -> bool {
    value == 0. || (value > -epsilon && value < epsilon)
}

/// Flutter's `_SpringSolution`: the closed-form motion of a damped harmonic
/// oscillator relative to its rest position.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SpringSolution {
    Critical { r: f64, c1: f64, c2: f64 },
    Over { r1: f64, r2: f64, c1: f64, c2: f64 },
    Under { w: f64, r: f64, c1: f64, c2: f64 },
}

impl SpringSolution {
    fn new(spring: ScrollSpring, distance: f64, velocity: f64) -> Self {
        let mass = f64::from(spring.mass);
        let stiffness = f64::from(spring.stiffness);
        let damping = f64::from(spring.damping);
        let discriminant = damping * damping - 4. * mass * stiffness;
        if discriminant > 0. {
            let root = discriminant.sqrt();
            let r1 = (-damping - root) / (2. * mass);
            let r2 = (-damping + root) / (2. * mass);
            let c2 = (velocity - r1 * distance) / (r2 - r1);
            Self::Over {
                r1,
                r2,
                c1: distance - c2,
                c2,
            }
        } else if discriminant < 0. {
            let w = (4. * mass * stiffness - damping * damping).sqrt() / (2. * mass);
            let r = -(damping / 2. / mass);
            Self::Under {
                w,
                r,
                c1: distance,
                c2: (velocity - r * distance) / w,
            }
        } else {
            let r = -damping / (2. * mass);
            Self::Critical {
                r,
                c1: distance,
                c2: velocity - r * distance,
            }
        }
    }

    fn x(self, time: f64) -> f64 {
        match self {
            Self::Critical { r, c1, c2 } => (c1 + c2 * time) * (r * time).exp(),
            Self::Over { r1, r2, c1, c2 } => c1 * (r1 * time).exp() + c2 * (r2 * time).exp(),
            Self::Under { w, r, c1, c2 } => {
                (r * time).exp() * (c1 * (w * time).cos() + c2 * (w * time).sin())
            }
        }
    }

    fn dx(self, time: f64) -> f64 {
        match self {
            Self::Critical { r, c1, c2 } => {
                let power = (r * time).exp();
                r * (c1 + c2 * time) * power + c2 * power
            }
            Self::Over { r1, r2, c1, c2 } => {
                c1 * r1 * (r1 * time).exp() + c2 * r2 * (r2 * time).exp()
            }
            Self::Under { w, r, c1, c2 } => {
                let power = (r * time).exp();
                let (sine, cosine) = (w * time).sin_cos();
                power * (c2 * w * cosine - c1 * w * sine) + r * power * (c2 * sine + c1 * cosine)
            }
        }
    }
}

/// Flutter `SpringSimulation` / `ScrollSpringSimulation`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SpringMotion {
    end: f64,
    solution: SpringSolution,
    distance_tolerance: f64,
    velocity_tolerance: f64,
    /// `ScrollSpringSimulation.x` reports the end exactly once it is done.
    snap_when_done: bool,
}

impl SpringMotion {
    fn new(
        spring: ScrollSpring,
        start: f64,
        end: f64,
        velocity: f64,
        tolerance: ScrollTolerance,
        snap_when_done: bool,
    ) -> Self {
        Self {
            end,
            solution: SpringSolution::new(spring, start - end, velocity),
            distance_tolerance: f64::from(tolerance.distance),
            velocity_tolerance: f64::from(tolerance.velocity),
            snap_when_done,
        }
    }

    fn is_done(&self, time: f64) -> bool {
        near_zero(self.solution.x(time), self.distance_tolerance)
            && near_zero(self.solution.dx(time), self.velocity_tolerance)
    }

    fn x(&self, time: f64) -> f64 {
        if self.snap_when_done && self.is_done(time) {
            self.end
        } else {
            self.end + self.solution.x(time)
        }
    }

    fn dx(&self, time: f64) -> f64 {
        self.solution.dx(time)
    }
}

/// `FrictionSimulation`'s drag for `BouncingScrollSimulation`, `0.998^1000`.
const BOUNCING_DRAG: f64 = 0.135;
/// `BouncingScrollSimulation.maxSpringTransferVelocity`.
const MAX_SPRING_TRANSFER_VELOCITY: f64 = 5000.;

/// Flutter `FrictionSimulation`, including its constant deceleration.
#[derive(Clone, Copy, Debug, PartialEq)]
struct FrictionMotion {
    drag: f64,
    drag_log: f64,
    position: f64,
    velocity: f64,
    constant_deceleration: f64,
    final_time: f64,
    velocity_tolerance: f64,
}

impl FrictionMotion {
    fn new(
        drag: f64,
        position: f64,
        velocity: f64,
        constant_deceleration: f64,
        tolerance: ScrollTolerance,
    ) -> Self {
        let mut motion = Self {
            drag,
            drag_log: drag.ln(),
            position,
            velocity,
            constant_deceleration: constant_deceleration * velocity.signum(),
            final_time: f64::INFINITY,
            velocity_tolerance: f64::from(tolerance.velocity),
        };
        // `_newtonsMethod(initialGuess: 0, target: 0, f: dx, df: ..., 10)`.
        motion.final_time = if velocity == 0. {
            0.
        } else {
            let mut guess = 0.;
            for _ in 0..10 {
                let slope = motion.velocity * motion.drag.powf(guess) * motion.drag_log
                    - motion.constant_deceleration;
                guess -= (motion.raw_dx(guess) - 0.) / slope;
            }
            guess
        };
        motion
    }

    fn raw_dx(&self, time: f64) -> f64 {
        self.velocity * self.drag.powf(time) - self.constant_deceleration * time
    }

    fn x(&self, time: f64) -> f64 {
        if time > self.final_time {
            return self.final_x();
        }
        self.position + self.velocity * self.drag.powf(time) / self.drag_log
            - self.velocity / self.drag_log
            - (self.constant_deceleration / 2.) * time * time
    }

    fn dx(&self, time: f64) -> f64 {
        if time > self.final_time {
            0.
        } else {
            self.raw_dx(time)
        }
    }

    fn final_x(&self) -> f64 {
        if self.constant_deceleration == 0. {
            self.position - self.velocity / self.drag_log
        } else {
            self.x(self.final_time)
        }
    }

    /// Flutter `FrictionSimulation.timeAtX`.
    fn time_at_x(&self, x: f64) -> f64 {
        if x == self.position {
            return 0.;
        }
        let final_x = self.final_x();
        let unreachable = if self.velocity > 0. {
            x < self.position || x > final_x
        } else {
            x > self.position || x < final_x
        };
        if self.velocity == 0. || unreachable {
            return f64::INFINITY;
        }
        let mut guess = 0.;
        for _ in 0..10 {
            guess -= (self.x(guess) - x) / self.dx(guess);
        }
        guess
    }

    fn is_done(&self, time: f64) -> bool {
        self.dx(time).abs() < self.velocity_tolerance
    }
}

/// Flutter `BouncingScrollSimulation`: friction inside the extents with a
/// spring hand-off at the one it crosses.
#[derive(Clone, Copy, Debug, PartialEq)]
struct BouncingMotion {
    friction: Option<FrictionMotion>,
    spring: SpringMotion,
    spring_time: f64,
}

impl BouncingMotion {
    #[allow(clippy::too_many_arguments)]
    fn new(
        spring: ScrollSpring,
        position: f64,
        velocity: f64,
        leading: f64,
        trailing: f64,
        constant_deceleration: f64,
        tolerance: ScrollTolerance,
    ) -> Self {
        let underscroll =
            |x: f64, dx: f64| SpringMotion::new(spring, x, leading, dx, tolerance, true);
        let overscroll =
            |x: f64, dx: f64| SpringMotion::new(spring, x, trailing, dx, tolerance, true);
        if position < leading {
            return Self {
                friction: None,
                spring: underscroll(position, velocity),
                spring_time: f64::NEG_INFINITY,
            };
        }
        if position > trailing {
            return Self {
                friction: None,
                spring: overscroll(position, velocity),
                spring_time: f64::NEG_INFINITY,
            };
        }
        let friction = FrictionMotion::new(
            BOUNCING_DRAG,
            position,
            velocity,
            constant_deceleration,
            tolerance,
        );
        let final_x = friction.final_x();
        if velocity > 0. && final_x > trailing {
            let spring_time = friction.time_at_x(trailing);
            let transfer = friction.dx(spring_time).min(MAX_SPRING_TRANSFER_VELOCITY);
            Self {
                friction: Some(friction),
                spring: overscroll(trailing, transfer),
                spring_time,
            }
        } else if velocity < 0. && final_x < leading {
            let spring_time = friction.time_at_x(leading);
            let transfer = friction.dx(spring_time).min(MAX_SPRING_TRANSFER_VELOCITY);
            Self {
                friction: Some(friction),
                spring: underscroll(leading, transfer),
                spring_time,
            }
        } else {
            Self {
                friction: Some(friction),
                spring: underscroll(position, velocity),
                spring_time: f64::INFINITY,
            }
        }
    }

    /// The simulation active at `time` and the time offset it runs on.
    fn phase(&self, time: f64) -> (Option<&FrictionMotion>, f64) {
        match &self.friction {
            Some(friction) if time <= self.spring_time => (Some(friction), 0.),
            _ => (
                None,
                if self.spring_time.is_finite() {
                    self.spring_time
                } else {
                    0.
                },
            ),
        }
    }

    fn x(&self, time: f64) -> f64 {
        match self.phase(time) {
            (Some(friction), _) => friction.x(time),
            (None, offset) => self.spring.x(time - offset),
        }
    }

    fn dx(&self, time: f64) -> f64 {
        match self.phase(time) {
            (Some(friction), _) => friction.dx(time),
            (None, offset) => self.spring.dx(time - offset),
        }
    }

    fn is_done(&self, time: f64) -> bool {
        match self.phase(time) {
            (Some(friction), _) => friction.is_done(time),
            (None, offset) => self.spring.is_done(time - offset),
        }
    }

    fn final_x(&self) -> f64 {
        match &self.friction {
            Some(friction) if self.spring_time.is_infinite() && self.spring_time > 0. => {
                friction.final_x()
            }
            _ => self.spring.end,
        }
    }
}

/// `ClampingScrollSimulation.friction` default.
const CLAMPING_FRICTION: f64 = 0.015;
/// `ClampingScrollSimulation._kInflexion`.
const CLAMPING_INFLEXION: f64 = 0.35;
/// `ClampingScrollSimulation._physicalCoeff`: 0.84 times Earth gravity in
/// logical pixels per second squared (160 logical pixels per inch).
const CLAMPING_PHYSICAL_COEFF: f64 = 9.80665 * 39.37 * 160.0 * 0.84;

/// Flutter `ClampingScrollSimulation`: a spline fit to Android's fling that
/// ends exactly at its duration.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ClampingMotion {
    position: f64,
    velocity: f64,
    duration: f64,
    distance: f64,
}

impl ClampingMotion {
    fn new(position: f64, velocity: f64) -> Self {
        let rate = Self::deceleration_rate();
        let reference = CLAMPING_FRICTION * CLAMPING_PHYSICAL_COEFF / CLAMPING_INFLEXION;
        let android = (velocity.abs() / reference).powf(1.0 / (rate - 1.0));
        let duration = rate * CLAMPING_INFLEXION * android;
        Self {
            position,
            velocity,
            duration,
            distance: velocity * duration / rate,
        }
    }

    /// `_kDecelerationRate = log(0.78) / log(0.9)`.
    fn deceleration_rate() -> f64 {
        0.78f64.ln() / 0.9f64.ln()
    }

    fn x(&self, time: f64) -> f64 {
        let t = (time / self.duration).clamp(0., 1.);
        self.position + self.distance * (1.0 - (1.0 - t).powf(Self::deceleration_rate()))
    }

    fn dx(&self, time: f64) -> f64 {
        let t = (time / self.duration).clamp(0., 1.);
        self.velocity * (1.0 - t).powf(Self::deceleration_rate() - 1.0)
    }
}

/// Flutter's ballistic `Simulation` for a released drag, as created by
/// [`ScrollPhysics::ballistic_simulation`](crate::ScrollPhysics::ballistic_simulation).
///
/// Clamping physics use `ClampingScrollSimulation`, bouncing physics use
/// `BouncingScrollSimulation` (friction inside the extents, a spring at the
/// extent it crosses or starts beyond), and a clamping position that is
/// already beyond an extent runs a `ScrollSpringSimulation` back to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlingSimulation {
    model: FlingModel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FlingModel {
    Clamping(ClampingMotion),
    Bouncing(BouncingMotion),
    Spring(SpringMotion),
    Friction(FrictionMotion),
}

impl FlingSimulation {
    pub(crate) fn clamping(position: f32, velocity: f32) -> Self {
        Self {
            model: FlingModel::Clamping(ClampingMotion::new(
                f64::from(position),
                f64::from(velocity),
            )),
        }
    }

    pub(crate) fn bouncing(
        spring: ScrollSpring,
        position: f32,
        velocity: f32,
        leading: f32,
        trailing: f32,
        constant_deceleration: f32,
        tolerance: ScrollTolerance,
    ) -> Self {
        Self {
            model: FlingModel::Bouncing(BouncingMotion::new(
                spring,
                f64::from(position),
                f64::from(velocity),
                f64::from(leading),
                f64::from(trailing),
                f64::from(constant_deceleration),
                tolerance,
            )),
        }
    }

    /// Flutter `ScrollSpringSimulation`.
    pub(crate) fn scroll_spring(
        spring: ScrollSpring,
        start: f32,
        end: f32,
        velocity: f32,
        tolerance: ScrollTolerance,
    ) -> Self {
        Self {
            model: FlingModel::Spring(SpringMotion::new(
                spring,
                f64::from(start),
                f64::from(end),
                f64::from(velocity),
                tolerance,
                true,
            )),
        }
    }

    /// Flutter `SpringSimulation` (which does not snap to its end).
    pub(crate) fn spring(
        spring: ScrollSpring,
        start: f32,
        end: f32,
        velocity: f32,
        tolerance: ScrollTolerance,
    ) -> Self {
        Self {
            model: FlingModel::Spring(SpringMotion::new(
                spring,
                f64::from(start),
                f64::from(end),
                f64::from(velocity),
                tolerance,
                false,
            )),
        }
    }

    /// Flutter `FrictionSimulation.through`: a friction whose drag is tuned
    /// to end at `end_position`.
    pub(crate) fn friction_through(
        start_position: f32,
        end_position: f32,
        start_velocity: f32,
        end_velocity: f32,
    ) -> Self {
        let drag =
            f64::from(start_velocity - end_velocity) / f64::from(start_position - end_position);
        Self {
            model: FlingModel::Friction(FrictionMotion::new(
                drag.exp(),
                f64::from(start_position),
                f64::from(start_velocity),
                0.,
                ScrollTolerance {
                    velocity: end_velocity.abs(),
                    distance: 1e-3,
                },
            )),
        }
    }

    /// The position `time` seconds into the simulation, in logical pixels.
    #[must_use]
    pub fn x(&self, time: f32) -> f32 {
        let time = f64::from(time.max(0.));
        (match &self.model {
            FlingModel::Clamping(motion) => motion.x(time),
            FlingModel::Bouncing(motion) => motion.x(time),
            FlingModel::Spring(motion) => motion.x(time),
            FlingModel::Friction(motion) => motion.x(time),
        }) as f32
    }

    /// The velocity `time` seconds into the simulation, in logical px/sec.
    #[must_use]
    pub fn dx(&self, time: f32) -> f32 {
        let time = f64::from(time.max(0.));
        (match &self.model {
            FlingModel::Clamping(motion) => motion.dx(time),
            FlingModel::Bouncing(motion) => motion.dx(time),
            FlingModel::Spring(motion) => motion.dx(time),
            FlingModel::Friction(motion) => motion.dx(time),
        }) as f32
    }

    /// Whether the simulation has finished at `time` seconds.
    #[must_use]
    pub fn is_done(&self, time: f32) -> bool {
        let time = f64::from(time.max(0.));
        match &self.model {
            FlingModel::Clamping(motion) => time >= motion.duration,
            FlingModel::Bouncing(motion) => motion.is_done(time),
            FlingModel::Spring(motion) => motion.is_done(time),
            FlingModel::Friction(motion) => motion.is_done(time),
        }
    }

    /// Where the simulation comes to rest: Flutter's `x(double.infinity)`.
    #[must_use]
    pub fn final_x(&self) -> f32 {
        (match &self.model {
            FlingModel::Clamping(motion) => motion.position + motion.distance,
            FlingModel::Bouncing(motion) => motion.final_x(),
            FlingModel::Spring(motion) => motion.end,
            FlingModel::Friction(motion) => motion.final_x(),
        }) as f32
    }

    /// Whether this simulation is allowed to carry the position beyond the
    /// scroll extents (bouncing physics and the springs back from them).
    #[must_use]
    pub fn may_overscroll(&self) -> bool {
        matches!(self.model, FlingModel::Bouncing(_) | FlingModel::Spring(_))
    }
}
