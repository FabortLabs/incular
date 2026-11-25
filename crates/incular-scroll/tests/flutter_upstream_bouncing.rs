//! Observable cases from Flutter 3.50.0 newton_test.dart:242,270,296.

use incular_scroll::{FlingSimulation, ScrollPhysics, ScrollSpring, ScrollTolerance};

fn within(actual: f32, expected: f32, epsilon: f32) {
    assert!(
        (actual - expected).abs() <= epsilon,
        "{actual} is not within {epsilon} of {expected}",
    );
}

#[test]
fn flutter_bouncing_fling_with_unbounded_forward_extent() {
    let physics = ScrollPhysics::clamping()
        .bouncing()
        .with_device_pixel_ratio(20.0);
    within(physics.tolerance().velocity, 1.0, 1e-6);
    let fling = physics
        .ballistic_simulation(100.0, 400.0, 0.0, f32::INFINITY)
        .unwrap();

    assert!(!fling.is_done(0.0));
    assert_eq!(fling.x(0.0), 100.0);
    assert_eq!(fling.dx(0.0), 400.0);
    within(fling.x(1.0), 272.0, 1.0);
    within(fling.dx(1.0), 54.0, 1.0);
    within(fling.dx(2.0), 7.0, 1.0);
    assert!(fling.dx(3.0) < 1.0);
    assert!(fling.is_done(5.0));
    within(fling.x(5.0), 300.0, 1.0);
}

#[test]
fn flutter_kinetic_scroll_hands_off_to_a_custom_spring_in_both_directions() {
    let spring = ScrollSpring::with_damping_ratio(1., 50., 0.5);
    let tolerance = ScrollTolerance {
        velocity: 0.5,
        distance: 0.1,
    };
    for velocity in [800., -800.] {
        let scroll = FlingSimulation::bouncing(spring, 100., velocity, 0., 300., 0., tolerance);
        assert!(!scroll.is_done(0.));
        assert!(!scroll.is_done(0.5));
        assert!(scroll.is_done(3.5));
    }
}

#[test]
fn flutter_over_under_scroll_spring_can_sample_times_out_of_order() {
    let spring = ScrollSpring::with_damping_ratio(1., 170., 1.1);
    let tolerance = ScrollTolerance {
        velocity: 45.,
        distance: 1.5,
    };
    let scroll = FlingSimulation::bouncing(spring, 500., -7500., 0., 1000., 0., tolerance);
    let at_start = || {
        assert!(!scroll.is_done(0.));
        within(scroll.x(0.), 500., 1e-6);
        within(scroll.dx(0.), -7500., 1e-6);
    };
    at_start();
    for (time, position, velocity) in [
        (0.065, 42., -6584.),
        (0.1, -123., -2613.),
        (0.5, -15., 124.),
    ] {
        assert!(!scroll.is_done(time));
        within(scroll.x(time), position, 1.);
        within(scroll.dx(time), velocity, 1.);
    }
    at_start();
    assert!(scroll.is_done(2.));
    assert_eq!(scroll.x(2.), 0.);
    within(scroll.dx(2.), 0., 1.);
}
