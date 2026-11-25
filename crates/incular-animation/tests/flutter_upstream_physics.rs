//! Observable checks from Flutter 3.50.0's newton, clamped simulation,
//! near-equal, and tolerance tests. Keep Flutter's numeric bounds; the first
//! two friction cases use exact seconds and preserve precision before layout.

use incular_animation::{
    BoundedFrictionSimulation, ClampedSimulation, FrictionSimulation, GravitySimulation,
    Simulation, SpringDescription, SpringSimulation, SpringType as K, Tolerance,
};
use std::time::Duration;
fn ds(t: f64) -> Duration {
    Duration::from_secs_f64(t)
}
fn tol(d: f32, v: f32) -> Tolerance {
    Tolerance {
        distance: d,
        velocity: v,
        ..Tolerance::DEFAULT
    }
}
fn near(a: f32, b: f32) -> bool {
    (f64::from(a) - f64::from(b)).abs() <= 1e-10
}
fn near64(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-10
}
#[test]
fn flutter_friction_velocity_preserves_reference_precision() {
    let f = FrictionSimulation::new_precise(0.3, 100., 400., tol(0.001, 1.));
    assert!(!f.is_done(ds(0.)));
    assert_eq!(f.position(ds(0.)), 100.);
    assert_eq!(f.velocity(ds(0.)), 400.);
    let x1 = f.position(ds(1.));
    assert!(x1 > 330. && x1 < 335.);
    let v = [1., 2., 3., 4.].map(|t| f.dx(t));
    let x5 = f.position(ds(5.));
    assert!(
        v[0] == 120.
            && v[1] == 36.
            && near64(v[2], 10.8)
            && v[3] < 3.5
            && f.is_done(ds(5.))
            && x5 > 431.
            && x5 < 432.,
        "v={v:?} x5={x5} done5={}",
        f.is_done(ds(5.))
    );
}
#[test]
fn flutter_friction_through_matches_endpoint_and_completion() {
    let mut results = Vec::new();
    for (start, velocity) in [(10., 600.), (1000., -500.)] {
        let f = FrictionSimulation::new_precise(0.025, start, velocity, Tolerance::DEFAULT);
        let (end, vend) = (f.x(1.), f.dx(1.));
        if velocity > 0. {
            assert!(end > start && vend < velocity);
        } else {
            assert!(end < start && vend > velocity);
        }
        let f = FrictionSimulation::through(start, end, velocity, vend);
        if velocity > 0. {
            assert!(!f.is_done(ds(0.)));
            assert_eq!(f.position(ds(0.)), 10.);
            assert_eq!(f.velocity(ds(0.)), 600.);
        }
        let done = f.is_done_at_seconds(1. + 1e-10);
        let x = f.x(1.);
        let v = f.dx(1.);
        results.push((start, done, x, end, v, vend));
    }
    assert!(
        results
            .iter()
            .all(|&(_, done, x, end, v, vend)| done && near64(x, end) && near64(v, vend)),
        "{results:?}"
    );
}

fn check_friction_dedicated(
    velocity: f64,
    constant_deceleration: f64,
    expected_positions: [f64; 3],
    expected_final: f64,
    inverse_times: [f64; 3],
    unreachable: [f64; 2],
) {
    let simulation = FrictionSimulation::new_precise(0.135, 100., velocity, Tolerance::DEFAULT)
        .with_constant_deceleration(constant_deceleration);
    assert!(near64(simulation.x(0.), 100.));
    assert!(near64(simulation.dx(0.), velocity));
    for (seconds, expected) in [0.1, 0.5, 2.].into_iter().zip(expected_positions) {
        assert!((simulation.x(seconds) - expected).abs() <= 1.);
    }
    assert!((simulation.final_x_f64() - expected_final).abs() <= 1.);
    assert_eq!(simulation.time_at_x_seconds(100.), 0.);
    for seconds in inverse_times {
        let actual = simulation.time_at_x_seconds(simulation.x(seconds));
        assert!(near64(actual, seconds), "inverse at {seconds}: {actual}");
    }
    for x in unreachable {
        assert_eq!(simulation.time_at_x_seconds(x), f64::INFINITY);
    }
}

#[test]
fn flutter_friction_dedicated_positive_velocity() {
    check_friction_dedicated(
        100.,
        0.,
        [110., 131., 149.],
        149.,
        [0.1, 0.5, 2.],
        [-1., 200.],
    );
}

#[test]
fn flutter_friction_dedicated_negative_velocity() {
    check_friction_dedicated(-100., 0., [91., 68., 51.], 50., [0.1, 0.5, 2.], [101., 40.]);
}

#[test]
fn flutter_friction_dedicated_constant_deceleration() {
    check_friction_dedicated(
        -100.,
        100.,
        [91., 80., 80.],
        80.,
        [0.1, 0.2, 0.3],
        [101., 40.],
    );
}
#[test]
fn flutter_bounded_friction_stops_at_the_upper_bound() {
    let mut f = BoundedFrictionSimulation::new(0.3, 100., 400., 50., 150.);
    f.set_tolerance(tol(0.001, 1.));
    assert!(!f.is_done(ds(0.)));
    assert_eq!(f.position(ds(0.)), 100.);
    assert_eq!(f.velocity(ds(0.)), 400.);
    assert_eq!(f.position(ds(1.)), 150.);
    assert!(f.is_done(ds(1.)));
}
#[test]
fn flutter_gravity_reports_position_velocity_and_completion() {
    let f = GravitySimulation::new(200., 100., 600., 0.);
    assert!(!f.is_done(ds(0.)));
    assert_eq!(f.position(ds(0.)), 100.);
    assert_eq!(f.velocity(ds(0.)), 0.);
    let times = [0.25, 0.50, 0.75, 1.00, 1.25, 1.50, 1.75];
    assert_eq!(
        times.map(|t| f.position(ds(t))),
        [106.25, 125., 156.25, 200., 256.25, 325., 406.25]
    );
    assert_eq!(
        times.map(|t| f.velocity(ds(t))),
        [50., 100., 150., 200., 250., 300., 350.]
    );
    assert!(f.is_done(ds(2.5)));
    assert_eq!(f.position(ds(2.5)), 725.);
    assert_eq!(f.velocity(ds(2.5)), 500.);
}

/// physics/gravity_simulation_test.dart:9.
#[test]
fn flutter_gravity_dedicated_description_and_decimal_acceleration() {
    let gravity = GravitySimulation::new(9.81, 10., 0., 0.);
    let description = format!("{gravity:?}");
    assert!(description.starts_with("GravitySimulation") && !description.contains('\n'));
    assert!((f64::from(gravity.position(ds(10.))) - (50. * 9.81 + 10.)).abs() <= 1e-10);
}

/// physics/gravity_simulation_test.dart:14.
#[test]
fn flutter_gravity_dedicated_turnaround_and_absolute_completion() {
    let gravity = GravitySimulation::new(-10., 0., 6., 10.);
    for (t, x, dx, done_t, done) in [
        (0., 0., 10., 0., false),
        (1., 5., 0., 0.2, false),
        (2., 0., -10., 2., false),
        (3., -15., -20., 3., true),
    ] {
        assert_eq!(gravity.position(ds(t)), x);
        assert_eq!(gravity.velocity(ds(t)), dx);
        assert_eq!(gravity.is_done(ds(done_t)), done);
    }
}
#[test]
fn flutter_spring_description_classifies_damping() {
    for (ratio, kind) in [
        (1., K::CriticallyDamped),
        (1., K::CriticallyDamped),
        (0.75, K::UnderDamped),
        (1.25, K::OverDamped),
    ] {
        assert_eq!(
            SpringDescription::with_damping_ratio(1., 100., ratio).spring_type(),
            kind
        );
    }
    assert_eq!(
        SpringDescription::new(1., 100., 20.).spring_type(),
        K::CriticallyDamped
    );
}
#[test]
fn flutter_critical_spring_matches_reference_samples() {
    let d = SpringDescription::with_damping_ratio(1., 100., 1.);
    let f = SpringSimulation::new(d, 0., 500., 0., tol(0.01, 0.01));
    assert_eq!(d.spring_type(), K::CriticallyDamped);
    assert!(!f.is_done(ds(0.)));
    assert_eq!(f.position(ds(0.)), 0.);
    assert_eq!(f.velocity(ds(0.)), 0.);
    let times = [0.25, 0.50, 0.75];
    assert_eq!(times.map(|t| f.position(ds(t)).floor()), [356., 479., 497.]);
    assert_eq!(times.map(|t| f.velocity(ds(t)).floor()), [1026., 168., 20.]);
    assert!(f.position(ds(1.5)) > 499. && f.position(ds(1.5)) < 501.);
    assert!(f.velocity(ds(1.5)) < 0.1);
    assert!(f.is_done(ds(1.6)));
}
#[test]
fn flutter_overdamped_spring_matches_reference_samples() {
    let d = SpringDescription::with_damping_ratio(1., 100., 1.25);
    let f = SpringSimulation::new(d, 0., 500., 0., tol(0.01, 0.01));
    assert_eq!(d.spring_type(), K::OverDamped);
    assert!(!f.is_done(ds(0.)));
    assert_eq!(f.position(ds(0.)), 0.);
    assert!(near(f.velocity(ds(0.)), 0.));
    let times = [0.5, 1.0, 1.5];
    assert_eq!(times.map(|t| f.position(ds(t)).floor()), [445., 495., 499.]);
    assert_eq!(times.map(|t| f.velocity(ds(t)).floor()), [273., 22., 1.]);
    assert!(f.is_done(ds(3.)));
}
#[test]
fn flutter_underdamped_spring_matches_reference_samples() {
    let d = SpringDescription::with_damping_ratio(1., 100., 0.25);
    let f = SpringSimulation::new(d, 0., 300., 0., Tolerance::DEFAULT);
    assert_eq!(d.spring_type(), K::UnderDamped);
    assert!(!f.is_done(ds(0.)));
    assert!(near(f.position(ds(0.)), 0.));
    assert!(near(f.velocity(ds(0.)), 0.));
    assert_eq!(f.position(ds(1.)).floor(), 325.);
    assert_eq!(f.velocity(ds(1.)).floor(), -65.);
    assert_eq!(f.velocity(ds(6.)).floor(), 0.);
    assert_eq!(f.position(ds(6.)).floor(), 299.);
    assert!(f.is_done(ds(6.)));
}

fn at(ms: u64) -> Duration {
    Duration::from_millis(ms)
}
#[test]
fn flutter_clamped_gravity_respects_position_and_velocity_bounds() {
    let s = ClampedSimulation::new(GravitySimulation::new(9.81, 10., 0., 0.), 20., 100.)
        .with_velocity_bounds(7., 11.);
    assert_eq!([s.position(at(0)), s.velocity(at(0))], [20., 7.]);
    assert_eq!(
        [s.position(at(100_000)), s.velocity(at(100_000))],
        [100., 11.]
    );
}
#[test]
fn flutter_clamped_gravity_delegates_completion() {
    let s = ClampedSimulation::new(GravitySimulation::new(-10., 0., 6., 10.), 0., 2.5)
        .with_velocity_bounds(-1., 1.);
    assert_eq!([s.position(at(0)), s.velocity(at(0))], [0., 1.]);
    assert!(!s.is_done(at(0)));
    assert_eq!([s.position(at(1000)), s.velocity(at(1000))], [2.5, 0.]);
    assert!(!s.is_done(at(200)));
    assert_eq!([s.position(at(2000)), s.velocity(at(2000))], [0., -1.]);
    assert!(!s.is_done(at(2000)));
    assert_eq!([s.position(at(3000)), s.velocity(at(3000))], [0., -1.]);
    assert!(s.is_done(at(3000)));
}

use incular_animation::physics::near_equal;
#[test]
fn flutter_near_equal_is_symmetric_at_finite_distances() {
    assert!(near_equal(5., 6., 2.));
    assert!(near_equal(6., 5., 2.));
    assert!(!near_equal(5., 6., 0.5));
    assert!(!near_equal(6., 5., 0.5));
}

/// physics/near_equal_test.dart:16.
#[test]
fn flutter_near_equal_distinguishes_one_absent_operand_from_two() {
    assert!(!near_equal(5., None, 2.));
    assert!(!near_equal(None, 5., 2.));
    assert!(near_equal(None, None, 2.));
}
#[test]
fn flutter_near_equal_handles_infinities_and_finite_equality() {
    assert!(near_equal(f32::INFINITY, f32::INFINITY, 0.1));
    assert!(near_equal(f32::NEG_INFINITY, f32::NEG_INFINITY, 0.1));
    assert!(!near_equal(f32::INFINITY, f32::NEG_INFINITY, 0.1));
    assert!(!near_equal(0.1, 0.11, 0.001));
    assert!(near_equal(0.1, 0.11, 0.1));
    assert!(near_equal(0.1, 0.1, 0.0000001));
}
#[test]
fn flutter_tolerance_has_a_compact_diagnostic() {
    let s = format!("{:?}", Tolerance::default());
    assert!(!s.contains(['\r', '\n']));
    assert!(!s.is_empty());
    assert!(!s.contains("Instance of "));
    assert_eq!(s.trim(), s);
}
