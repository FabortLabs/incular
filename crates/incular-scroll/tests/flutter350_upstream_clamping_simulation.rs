//! Observable tests from Flutter 3.50.0 widgets/scroll_simulation_test.dart.
//! Incular exposes the simulation through unbounded clamping dispatch, with
//! f32 inputs/outputs. It has no public standalone clamping constructor.

use incular_scroll::{FlingSimulation, ScrollPhysics};

const INITIAL_CONDITIONS: [(f64, f64); 9] = [
    (51.0, 2866.915_37),
    (584.0, 2617.294_734),
    (345.0, 1982.785_934),
    (0.0, 1831.366_634),
    (-156.2, 1541.576_65),
    (4.0, 1139.250_439),
    (4534.0, 1073.553_798),
    (75.0, 614.209_3),
    (5469.0, 182.114_534),
];
const FLUTTER_EPSILON: f64 = 1e-10;

fn simulation(position: f64, velocity: f64) -> FlingSimulation {
    ScrollPhysics::CLAMPING
        .fling_simulation(position as f32, velocity as f32)
        .expect("nonzero in-range clamping fling above the dispatch tolerance")
}

#[test]
fn clamping_has_stable_initial_conditions() {
    // Upstream line 11. Preserve the reference's double literals and epsilon
    // and check preservation of each caller-representable input separately.
    let mut errors = Vec::new();
    for (position, velocity) in INITIAL_CONDITIONS {
        let drag = simulation(position, velocity);
        assert_eq!(drag.x(0.0), position as f32);
        assert_eq!(drag.dx(0.0), velocity as f32);
        let x_error = (f64::from(drag.x(0.0)) - position).abs();
        let dx_error = (f64::from(drag.dx(0.0)) - velocity).abs();
        if x_error > FLUTTER_EPSILON || dx_error > FLUTTER_EPSILON {
            errors.push((position, velocity, x_error, dx_error));
        }
    }
    assert!(
        errors.is_empty(),
        "initial-condition error (input x, input dx, absolute x error, absolute dx error): {errors:?}",
    );
}

#[test]
fn clamping_only_decelerates_and_finishes() {
    // Upstream line 29.
    let drag = simulation(0.0, 8000.0);
    let mut t = 0.0_f64;
    let mut prior = drag.dx(0.0);
    while !drag.is_done(t as f32) {
        assert!(t < 3.0, "did not stop before the upstream cutoff");
        t += 1.0 / 60.0;
        let next = drag.dx(t as f32);
        assert!(next <= prior, "speed increased at {t}: {prior} -> {next}");
        prior = next;
    }
}

#[test]
fn clamping_deceleration_remains_bounded_and_reaches_zero() {
    // Upstream line 43.
    let drag = simulation(0.0, 8000.0);
    let mut t = 0.0_f64;
    let step = 1.0 / 60.0;
    let mut prior = f64::from(drag.dx(0.0));
    loop {
        assert!(t < 3.0, "did not stop before the upstream cutoff");
        t += step;
        let next = f64::from(drag.dx(t as f32));
        assert!(
            (next - prior).abs() < step * 5130.0,
            "deceleration exceeded the upstream bound at {t}: {prior} -> {next}",
        );
        prior = next;
        if drag.is_done(t as f32) {
            break;
        }
    }
    assert!(prior.abs() <= FLUTTER_EPSILON, "terminal velocity: {prior}");
}

#[test]
fn clamping_public_fling_can_restart_from_each_position_and_speed() {
    // Upstream line 65. With an effectively zero dispatch tolerance, every
    // nonzero sample in this test can restart. The public API returns no
    // simulation at exactly zero rather than a new finished simulation.
    let physics = ScrollPhysics::CLAMPING.with_device_pixel_ratio(1.0e30);
    let original = simulation(0.0, 8000.0);
    let mut restarted = Some(original);
    let mut position = 0.0;
    let mut elapsed = 0.0_f64;
    let step = 1.0 / 90.0;
    let mut errors = Vec::new();
    loop {
        assert!(elapsed < 4.0, "did not stop before the upstream cutoff");
        elapsed += step;
        let mut velocity = 0.0;
        if let Some(active) = restarted {
            position = active.x(step as f32);
            velocity = active.dx(step as f32);
            restarted = physics.fling_simulation(position, velocity);
            if let Some(next) = restarted {
                position = next.x(0.0);
                velocity = next.dx(0.0);
            } else {
                assert_eq!(velocity, 0.0, "dispatch rejected a moving sample");
            }
        }
        let x_error = (f64::from(position) - f64::from(original.x(elapsed as f32))).abs();
        let dx_error = (f64::from(velocity) - f64::from(original.dx(elapsed as f32))).abs();
        errors.push((elapsed, x_error, dx_error));
        if restarted.is_none_or(|drag| drag.is_done(0.0)) && original.is_done(elapsed as f32) {
            break;
        }
    }
    let (_, final_x, _) = *errors.last().unwrap();
    let max_x = errors.iter().map(|(_, x, _)| *x).fold(0.0, f64::max);
    let max_dx = errors.iter().map(|(_, _, dx)| *dx).fold(0.0, f64::max);
    let first = errors
        .iter()
        .find(|(_, x, dx)| *x > FLUTTER_EPSILON || *dx > FLUTTER_EPSILON);
    assert!(
        final_x <= FLUTTER_EPSILON && first.is_none(),
        "restart at 90 Hz: terminal distance error {final_x}, max x error {max_x}, max dx error {max_dx}, first failing (seconds, x error, dx error) {first:?}",
    );
}

#[test]
fn clamping_satisfies_the_speed_based_acceleration_formula() {
    // Upstream line 101 bounds a centered finite difference with a
    // third-derivative bound at the interval's right endpoint.
    let rate = 0.78_f64.ln() / 0.9_f64.ln();
    let reference_speed = 0.015 * 9.80665 * 39.37 * 160.0 * 0.84 / 0.35;
    let reference_duration = rate * 0.35;
    let reference_deceleration = (rate - 1.0) * reference_speed / reference_duration;
    let step = 1.0 / 60.0;
    for (position, velocity) in INITIAL_CONDITIONS {
        let drag = simulation(position, velocity);
        let mut t = 0.0_f64;
        while t < 2.0 {
            let start = f64::from(drag.dx(t as f32));
            let mid = f64::from(drag.dx((t + step / 2.0) as f32));
            let end = f64::from(drag.dx((t + step) as f32));
            let acceleration = -mid.signum()
                * reference_deceleration
                * (mid.abs() / reference_speed).powf((rate - 2.0) / (rate - 1.0));
            let jerk = reference_speed / reference_duration.powi(2)
                * (rate - 1.0)
                * (rate - 2.0)
                * (end.abs() / reference_speed).powf((rate - 3.0) / (rate - 1.0));
            let error = (end - start - step * acceleration).abs();
            let allowance = jerk * step * step / 2.0;
            assert!(
                error < allowance,
                "x0={position}, v0={velocity}, t={t}: error={error}, allowance={allowance}",
            );
            t += step;
        }
    }
}
