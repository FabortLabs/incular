//! Physics simulations against Flutter 3.41.7 `packages/flutter/lib/src/physics`.
//!
//! Reference values come from the recorded Flutter frame outputs in
//! `harness/out/frames/flutter` (scenario names in each comment) and from the
//! Flutter formulas cited on each test.

use std::time::Duration;

use incular_animation::physics::{near_equal, near_zero};
use incular_animation::{
    BoundedFrictionSimulation, ClampedSimulation, FrictionSimulation, GravitySimulation,
    ScrollSpringSimulation, Simulation, SpringDescription, SpringSimulation, SpringType, Tolerance,
};

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

fn assert_close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual} (tolerance {tolerance})"
    );
}

// Flutter `SpringSimulation._solution` supplies analytic velocity. Recorded
// `physics__spring__damping_regimes` frame 1 (16 ms) and frame 0.
#[test]
fn spring_velocity_is_analytic_across_regimes() {
    let cases = [
        (
            SpringDescription::with_damping_ratio(1., 100., 0.5),
            SpringType::UnderDamped,
            1.653_497_7_f32,
            172.446_1_f32,
        ),
        (
            SpringDescription::with_damping_ratio(1., 100., 1.),
            SpringType::CriticallyDamped,
            1.560_349_5_f32,
            157.817_03_f32,
        ),
        (
            SpringDescription::with_damping_ratio(1., 100., 2.),
            SpringType::OverDamped,
            1.396_478_5_f32,
            133.242_48_f32,
        ),
    ];
    for (spring, spring_type, x_16ms, dx_16ms) in cases {
        assert_eq!(spring.spring_type(), spring_type);
        let simulation = SpringSimulation::new(spring, 0., 100., 30., Tolerance::DEFAULT);
        // Frame 0: Flutter reports velocity equal to the initial velocity.
        assert_close(simulation.velocity(Duration::ZERO), 30., 1e-4);
        assert_close(simulation.position(ms(16)), x_16ms, 1e-3);
        assert_close(simulation.velocity(ms(16)), dx_16ms, 1e-2);
    }
}

// The analytic derivative agrees with a central difference of the position.
#[test]
fn spring_velocity_matches_central_difference() {
    let spring = SpringDescription::with_damping_ratio(1., 100., 0.5);
    let simulation = SpringSimulation::new(spring, 0., 100., 30., Tolerance::DEFAULT);
    // f32 positions limit the difference quotient; a 1 ms step keeps the
    // rounding error well below the tolerance.
    let h = 0.001_f64;
    for millis in [50_u64, 200, 700] {
        let t = 0.001_f64 * millis as f64;
        let before = simulation.position(Duration::from_secs_f64(t - h));
        let after = simulation.position(Duration::from_secs_f64(t + h));
        let numeric = (after - before) / (2. * h as f32);
        assert_close(
            simulation.velocity(Duration::from_secs_f64(t)),
            numeric,
            0.1,
        );
    }
}

// Flutter `ScrollSpringSimulation.dx` is the analytic solution derivative; the
// recorded frame 0 velocity is 0 for a spring starting at rest.
#[test]
fn scroll_spring_starts_at_rest_with_zero_velocity() {
    let simulation = ScrollSpringSimulation::new(
        SpringDescription::with_damping_ratio(1., 100., 1.5),
        0.,
        100.,
        0.,
        Tolerance::DEFAULT,
    );
    assert_eq!(simulation.velocity(Duration::ZERO), 0.);
    assert!(!simulation.is_done(Duration::ZERO));
}

// Flutter `SpringSimulation.isDone` uses `nearZero`, which is strict:
// a displacement exactly equal to the distance tolerance is not settled.
// Recorded `physics__spring__tolerance_boundary` frame 0: done 0.
#[test]
fn spring_completion_is_strict_at_tolerance_boundary() {
    let simulation = SpringSimulation::new(
        SpringDescription::with_damping_ratio(1., 100., 1.),
        0.001,
        0.,
        0.,
        Tolerance::DEFAULT,
    );
    assert!(!simulation.is_done(Duration::ZERO));

    let just_inside = SpringSimulation::new(
        SpringDescription::with_damping_ratio(1., 100., 1.),
        0.000_99,
        0.,
        0.,
        Tolerance::DEFAULT,
    );
    assert!(just_inside.is_done(Duration::ZERO));
}

// Flutter `SpringSimulation.snapToEnd`: once done, report the end exactly with
// zero velocity.
#[test]
fn spring_snap_to_end_reports_exact_end_and_zero_velocity() {
    let simulation = SpringSimulation::new(
        SpringDescription::with_damping_ratio(1., 100., 1.),
        0.,
        100.,
        0.,
        Tolerance::DEFAULT,
    )
    .with_snap_to_end(true);
    let late = Duration::from_secs(10);
    assert!(simulation.is_done(late));
    assert_eq!(simulation.position(late), 100.);
    assert_eq!(simulation.velocity(late), 0.);

    let unsnapped = SpringSimulation::new(
        SpringDescription::with_damping_ratio(1., 100., 1.),
        0.,
        100.,
        0.,
        Tolerance::DEFAULT,
    );
    assert!(unsnapped.velocity(late).abs() < 1e-3);
}

// Flutter `SpringDescription` stores its constructor values unchanged.
#[test]
fn spring_description_preserves_raw_inputs() {
    let raw = SpringDescription::new(0., 0., -1.);
    assert_eq!((raw.mass, raw.stiffness, raw.damping), (0., 0., -1.));

    let negative_ratio = SpringDescription::with_damping_ratio(1., 100., -0.25);
    assert_close(negative_ratio.damping, -5., 1e-5);
    assert_eq!(negative_ratio.spring_type(), SpringType::UnderDamped);
}

// Flutter `SpringDescription.withDurationAndBounce` derives stiffness from the
// duration truncated to whole milliseconds. Recorded
// `physics__spring_description__duration_bounce` frame 0 for 300 ms 500 us:
// stiffness 438.64908, damping 29.321531.
#[test]
fn duration_bounce_truncates_to_whole_milliseconds() {
    let spring = SpringDescription::with_duration_and_bounce(Duration::from_micros(300_500), 0.3);
    assert_close(spring.stiffness, 438.649_08, 1e-2);
    assert_close(spring.damping, 29.321_53, 1e-3);
    assert_eq!(spring.duration(), ms(300));
    assert_close(spring.bounce(), 0.3, 1e-5);
}

// Flutter `SpringDescription.duration`: sqrt(4 pi^2 mass / stiffness) seconds,
// rounded to whole milliseconds.
#[test]
fn spring_duration_and_bounce_getters_follow_flutter() {
    let spring = SpringDescription::new(1., 100., 0.);
    assert_eq!(spring.duration(), ms(628));
    // Damping ratio 0 gives bounce 1 (ratio < 1 branch).
    assert_close(spring.bounce(), 1., 1e-6);

    let critical = SpringDescription::with_damping_ratio(1., 100., 1.);
    assert_close(critical.bounce(), 0., 1e-6);

    // Ratio 2 takes the `1 / ratio - 1` branch: -0.5.
    let over = SpringDescription::with_damping_ratio(1., 100., 2.);
    assert_close(over.bounce(), -0.5, 1e-6);
}

// Flutter `SpringDescription.withDurationAndBounce`: a negative bounce is
// not clamped, so it gives an overdamped spring.
#[test]
fn negative_bounce_is_overdamped() {
    let spring = SpringDescription::with_duration_and_bounce(ms(500), -0.5);
    assert_eq!(spring.spring_type(), SpringType::OverDamped);
}

// Flutter `FrictionSimulation.isDone` is strict: `dx.abs() < tolerance.velocity`.
// Recorded `physics__friction__tolerance_boundary` frame 0: done 0.
#[test]
fn friction_completion_is_strict_at_velocity_tolerance() {
    let simulation = FrictionSimulation::new(0.135, 0., 0.001, Tolerance::new(0.001, ms(1), 0.001));
    assert!(!simulation.is_done(Duration::ZERO));
}

// Flutter `BoundedFrictionSimulation.x` clamps the position and `dx` keeps the
// unbounded friction velocity. Recorded `physics__bounded_friction__position_and_velocity`
// frame 3 (48 ms): x = 4, dx = 90.8356.
#[test]
fn bounded_friction_keeps_velocity_at_the_bound() {
    let simulation = BoundedFrictionSimulation::new(0.135, 0., 100., -2., 4.);
    assert_close(simulation.position(ms(48)), 4., 1e-6);
    assert_close(simulation.velocity(ms(48)), 90.835_6, 1e-2);
    assert!(simulation.is_done(ms(48)));
}

// Flutter `BoundedFrictionSimulation.isDone` completes when the clamped
// position is strictly within the distance tolerance of a bound.
// Recorded `physics__bounded_friction__distance_tolerance` frame 0: done 1.
#[test]
fn bounded_friction_completes_within_distance_tolerance_of_bound() {
    let simulation = BoundedFrictionSimulation::new(0.135, 3.9995, 100., -2., 4.);
    assert!(simulation.is_done(Duration::ZERO));

    let far = BoundedFrictionSimulation::new(0.135, 3.5, 100., -2., 4.);
    assert!(!far.is_done(Duration::ZERO));
}

// Flutter `GravitySimulation.isDone` is `x(time).abs() >= endDistance` whatever the
// sign of the acceleration. Recorded `physics__gravity__negative_acceleration_threshold`
// frame 0: done 0.
#[test]
fn gravity_negative_acceleration_completes_by_absolute_distance() {
    let simulation = GravitySimulation::new(-10., 0., 100., 0.);
    assert!(!simulation.is_done(Duration::ZERO));
    // x = -5 t^2 reaches |x| = 100 at t = sqrt(20) ~= 4.4721 s.
    assert!(!simulation.is_done(Duration::from_secs_f64(4.4)));
    assert!(simulation.is_done(Duration::from_secs_f64(4.5)));
}

// Flutter `ClampedSimulation` clamps velocity to `[dxMin, dxMax]`. Recorded
// `physics__clamped_simulation__position_and_velocity` uses dxMin -50 and dxMax 50.
#[test]
fn clamped_simulation_clamps_velocity_to_bounds() {
    let simulation = ClampedSimulation::new(GravitySimulation::new(100., 0., 50., 200.), 0., 20.)
        .with_velocity_bounds(-50., 50.);
    assert_eq!(simulation.velocity(Duration::ZERO), 50.);
    assert_eq!(simulation.velocity(ms(100)), 50.);
    assert_eq!(simulation.position(Duration::ZERO), 0.);

    let unbounded = ClampedSimulation::new(GravitySimulation::new(100., 0., 50., 200.), 0., 20.);
    assert_eq!(unbounded.velocity(Duration::ZERO), 200.);
}

// Flutter `FrictionSimulation.finalX` for zero constant deceleration is
// `x - v / ln(drag)`: 10 - 120 / ln(0.135) = 10 + 59.9258 = 69.9258.
#[test]
fn friction_final_x_is_the_asymptote_without_deceleration() {
    let simulation = FrictionSimulation::new(0.135, 10., 120., Tolerance::DEFAULT);
    assert_close(simulation.final_x(), 69.925_8, 1e-3);
    assert_close(
        simulation.position(Duration::from_secs(10)),
        simulation.final_x(),
        1e-3,
    );
}

// Flutter `FrictionSimulation.timeAtX` inverts `x`.
#[test]
fn friction_time_at_x_inverts_position() {
    let simulation = FrictionSimulation::new(0.135, 10., 120., Tolerance::DEFAULT);
    let t = simulation.time_at_x(simulation.position(ms(300)));
    let t = t.expect("reachable position");
    assert!((t.as_secs_f64() - 0.3).abs() < 1e-3, "got {t:?}");
    assert_eq!(simulation.time_at_x(10.), Some(Duration::ZERO));
    // Behind the start and in the wrong direction is unreachable.
    assert_eq!(simulation.time_at_x(5.), None);
}

// Flutter `FrictionSimulation.through`: the drag passes through both the start
// and end velocities, and the completion tolerance is the end speed.
#[test]
fn friction_through_passes_through_start_and_end_velocity() {
    let simulation = FrictionSimulation::through(0., 100., 50., 10.);
    assert_eq!(simulation.tolerance().velocity, 10.);
    let t = simulation
        .time_at_x(100.)
        .expect("end position is reachable");
    assert_close(simulation.velocity(t), 10., 1e-2);
    assert_close(simulation.velocity(Duration::ZERO), 50., 1e-4);
}

// Flutter `FrictionSimulation` with `constantDeceleration` stops at its final time,
// where velocity is zero and position is `finalX`.
#[test]
fn friction_with_constant_deceleration_stops_at_final_time() {
    let simulation = FrictionSimulation::new(0.135, 0., 100., Tolerance::DEFAULT)
        .with_constant_deceleration(50.);
    assert_eq!(simulation.velocity(Duration::from_secs(60)), 0.);
    assert_close(
        simulation.position(Duration::from_secs(60)),
        simulation.final_x(),
        1e-3,
    );
    assert!(simulation.is_done(Duration::from_secs(60)));
    // Constant deceleration reduces the distance travelled.
    let plain = FrictionSimulation::new(0.135, 0., 100., Tolerance::DEFAULT);
    assert!(simulation.final_x() < plain.final_x());
}

// Flutter `nearEqual` / `nearZero` (`physics/utils.dart`): strict within epsilon, or equal.
#[test]
fn near_helpers_are_strict_within_epsilon() {
    assert!(near_zero(0., 0.));
    assert!(!near_zero(0.001, 0.001));
    assert!(near_zero(0.000_9, 0.001));
    assert!(near_zero(-0.000_9, 0.001));
    assert!(near_equal(1., 1.000_5, 0.001));
    assert!(!near_equal(1., 1.01, 0.001));
    assert!(near_equal(2., 2., 0.));
    assert!(!near_equal(f32::NAN, 0., 1.));
}

// Flutter `Simulation.tolerance` is readable through the base type.
#[test]
fn tolerance_is_readable_through_trait_object() {
    let custom = Tolerance::new(0.5, ms(2), 0.25);
    let simulations: Vec<Box<dyn Simulation>> = vec![
        Box::new(SpringSimulation::new(
            SpringDescription::with_damping_ratio(1., 100., 1.),
            0.,
            1.,
            0.,
            custom,
        )),
        Box::new(FrictionSimulation::new(0.135, 0., 1., custom)),
        Box::new(ScrollSpringSimulation::new(
            SpringDescription::with_damping_ratio(1., 100., 1.),
            0.,
            1.,
            0.,
            custom,
        )),
    ];
    for simulation in &simulations {
        assert_eq!(simulation.tolerance(), custom);
    }
    let gravity = GravitySimulation::new(1., 0., 1., 0.);
    assert_eq!(gravity.tolerance(), Tolerance::DEFAULT);
}

// Updating tolerance on a concrete simulation changes its completion decision.
#[test]
fn set_tolerance_changes_completion() {
    let mut simulation = SpringSimulation::new(
        SpringDescription::with_damping_ratio(1., 100., 1.),
        0.01,
        0.,
        0.,
        Tolerance::DEFAULT,
    );
    assert!(!simulation.is_done(Duration::ZERO));
    simulation.set_tolerance(Tolerance::new(0.05, ms(1), 0.001));
    assert!(simulation.is_done(Duration::ZERO));
    assert_eq!(simulation.tolerance().distance, 0.05);
}
