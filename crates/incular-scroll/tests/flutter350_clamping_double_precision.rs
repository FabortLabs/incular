use incular_scroll::ClampingScrollSimulation;

fn nearly(lhs: f64, rhs: f64, description: impl std::fmt::Display) {
    assert!(
        (lhs - rhs).abs() < 1e-10,
        "{description}: {lhs:.14} vs {rhs:.14}"
    );
}

#[test]
fn the_nine_pinned_clamping_initial_conditions_are_stable_in_double_precision() {
    for (position, velocity) in [
        (51., 2866.91537),
        (584., 2617.294734),
        (345., 1982.785934),
        (0., 1831.366634),
        (-156.2, 1541.57665),
        (4., 1139.250439),
        (4534., 1073.553798),
        (75., 614.2093),
        (5469., 182.114534),
    ] {
        let simulation = ClampingScrollSimulation::new(position, velocity);
        assert_eq!(simulation.position(), position);
        assert_eq!(simulation.velocity(), velocity);
        nearly(simulation.x(0.), position, "position at t=0");
        nearly(simulation.dx(0.), velocity, "velocity at t=0");
    }
}

#[test]
fn repeated_restarts_preserve_the_whole_pinned_ballistic_trace() {
    let step = 1. / 90.;
    let undisturbed = ClampingScrollSimulation::new(0., 8000.);
    let mut restarted = undisturbed.clone();
    let mut time = 0.;
    let mut frames = Vec::new();
    loop {
        assert!(time < 4.);
        time += step;
        restarted = ClampingScrollSimulation::new(restarted.x(step), restarted.dx(step));
        frames.push((
            restarted.x(0.),
            undisturbed.x(time),
            restarted.dx(0.),
            undisturbed.dx(time),
        ));
        if restarted.is_done(0.) && undisturbed.is_done(time) {
            break;
        }
    }
    let &(last, expected, _, _) = frames.last().unwrap();
    nearly(last, expected, "final distance");
    for (frame, (x, expected_x, dx, expected_dx)) in frames.into_iter().enumerate() {
        nearly(x, expected_x, format_args!("position at frame {frame}"));
        nearly(dx, expected_dx, format_args!("velocity at frame {frame}"));
    }
}

#[test]
fn stationary_trajectory_is_finite_and_friction_reduces_travel() {
    let idle = ClampingScrollSimulation::new(11.25, 0.);
    assert!(idle.is_done(0.));
    assert_eq!(idle.x(0.), 11.25);
    assert_eq!(idle.dx(0.), 0.);
    let baseline = ClampingScrollSimulation::new(0., 8000.);
    assert_eq!(baseline.friction(), 0.015);
    let friction = baseline.clone().with_friction(0.03);
    assert_eq!(friction.friction(), 0.03);
    assert!(friction.x(f64::INFINITY) < baseline.x(f64::INFINITY));
}
