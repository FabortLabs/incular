//! Observable result of Flutter 3.50.0 newton_test.dart:270.
//! No spring is reached with the forward extent unbounded; the public Rust
//! bouncing policy supplies the same 1 px/s velocity tolerance at DPR 20.

use incular_scroll::ScrollPhysics;

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
