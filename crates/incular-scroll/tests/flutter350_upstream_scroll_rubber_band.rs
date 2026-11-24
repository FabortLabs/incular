//! Trajectory assertions from Flutter 3.50.0 widgets/scroll_physics_test.dart.
//! Incular's public FlingSimulation has no exact subtype query. The original
//! BouncingScrollSimulation type assertions are not covered by this port.

use incular_scroll::ScrollPhysics;

fn assert_overscroll_return(pixels: f32, expected: [f64; 4]) {
    // Flutter's viewport dimension (500) and axis direction (down) are not
    // read by this dispatch. Preserve DPR, extents, offsets and velocities.
    let physics = ScrollPhysics::BOUNCING.with_device_pixel_ratio(1.0);
    let stationary = physics
        .ballistic_simulation(pixels, 0.0, 0.0, 1000.0)
        .expect("stationary overscroll returns to its extent");
    let moving = physics
        .ballistic_simulation(pixels, -100.0, 0.0, 1000.0)
        .expect("moving overscroll returns to its extent");
    assert!(stationary.may_overscroll() && moving.may_overscroll());

    // Flutter uses precisionErrorTolerance (1e-10) for the assertion that
    // the stationary and moving results differ.
    let at_point_two = f64::from(stationary.x(0.2));
    let moving_at_point_two = f64::from(moving.x(0.2));
    assert!(
        (at_point_two - moving_at_point_two).abs() > 1e-10,
        "stationary and moving results unexpectedly agree at 0.2 s: {at_point_two} vs {moving_at_point_two}",
    );
    let times = [0.1, 0.2, 0.3, 0.4];
    let actual = times.map(|time| f64::from(stationary.x(time)));
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(x, want)| (x - want).abs() <= 0.01),
        "at {times:?} s expected {expected:?} within 0.01 px; got {actual:?}; moving x(0.2)={moving_at_point_two}",
    );
}

#[test]
fn stationary_and_moving_return_from_leading_overscroll() {
    // Upstream registration at line 376.
    assert_overscroll_return(
        -500.0,
        [
            -185.751_143_653_683_1,
            -69.006_284_667_555_06,
            -25.635_736_232_654_022,
            -9.523_639_409_892_818,
        ],
    );
}

#[test]
fn stationary_and_moving_return_from_trailing_overscroll() {
    // Upstream registration at line 407.
    assert_overscroll_return(
        1500.0,
        [
            1185.751_143_653_683_1,
            1069.006_284_667_555,
            1025.635_736_232_654,
            1009.523_639_409_892_8,
        ],
    );
}
