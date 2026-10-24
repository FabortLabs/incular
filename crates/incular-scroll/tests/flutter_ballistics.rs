//! Flutter 3.41.7 ballistic scrolling: `ScrollPhysics.toleranceFor`,
//! `BouncingScrollPhysics` rates and momentum, `BouncingScrollSimulation`,
//! `ClampingScrollPhysics` spring-back and `FixedExtentScrollPhysics`.
//! Expected numbers are derived by hand from the Dart source formulas.

use incular_scroll::{ScrollDecelerationRate, ScrollPhysics};

fn approx(left: f32, right: f32, tolerance: f32) -> bool {
    (left - right).abs() <= tolerance
}

#[test]
fn tolerance_follows_the_device_pixel_ratio() {
    // toleranceFor: velocity 1 / (0.050 * dpr), distance 1 / dpr.
    let base = ScrollPhysics::clamping().tolerance();
    assert!(approx(base.velocity, 20., 1.0e-4));
    assert!(approx(base.distance, 1., 1.0e-6));
    let retina = ScrollPhysics::clamping()
        .with_device_pixel_ratio(2.)
        .tolerance();
    assert!(approx(retina.velocity, 10., 1.0e-4));
    assert!(approx(retina.distance, 0.5, 1.0e-6));
    // Invalid ratios keep the previous value.
    for invalid in [0., -1., f32::NAN, f32::INFINITY] {
        let physics = ScrollPhysics::clamping()
            .with_device_pixel_ratio(2.)
            .with_device_pixel_ratio(invalid);
        assert!(approx(physics.tolerance().velocity, 10., 1.0e-4));
    }
}

#[test]
fn the_tolerance_velocity_decides_whether_a_clamping_fling_runs() {
    // 15 px/s is below the 20 px/s tolerance at DPR 1 but above the 10 px/s
    // tolerance at DPR 2.
    let physics = ScrollPhysics::clamping();
    assert!(physics.ballistic_simulation(50., 15., 0., 100.).is_none());
    assert!(
        physics
            .with_device_pixel_ratio(2.)
            .ballistic_simulation(50., 15., 0., 100.)
            .is_some()
    );
}

#[test]
fn deceleration_rates_set_the_fling_limits_and_springs() {
    let normal = ScrollPhysics::clamping().bouncing();
    let fast = ScrollPhysics::clamping().bouncing_with(ScrollDecelerationRate::Fast);
    assert_eq!(normal.deceleration_rate(), ScrollDecelerationRate::Normal);
    assert_eq!(fast.deceleration_rate(), ScrollDecelerationRate::Fast);
    // minFlingVelocity is doubled for any bouncing physics; maxFlingVelocity
    // is eight times kMaxFlingVelocity for the fast rate.
    assert_eq!(ScrollPhysics::clamping().min_fling_velocity(), 50.);
    assert_eq!(normal.min_fling_velocity(), 100.);
    assert_eq!(fast.min_fling_velocity(), 100.);
    assert_eq!(normal.max_fling_velocity(), 8000.);
    assert_eq!(fast.max_fling_velocity(), 64_000.);
    // withDampingRatio(mass: 0.3, stiffness: 75, ratio: 1.3): damping is
    // 1.3 * 2 * sqrt(0.3 * 75) = 12.3321.
    let spring = fast.spring();
    assert_eq!((spring.mass, spring.stiffness), (0.3, 75.));
    assert!(approx(spring.damping, 12.3321, 1.0e-3));
}

#[test]
fn carried_momentum_matches_the_ios_build_up_curve() {
    // sign * min(0.000816 * |v|^1.967, 40000).
    let bouncing = ScrollPhysics::clamping().bouncing();
    assert!(approx(bouncing.carried_momentum(1000.), 649.666, 0.05));
    assert!(approx(bouncing.carried_momentum(-1000.), -649.666, 0.05));
    assert!(approx(bouncing.carried_momentum(2000.), 2539.897, 0.2));
    assert_eq!(bouncing.carried_momentum(50_000.), 40_000.);
    assert_eq!(bouncing.carried_momentum(0.), 0.);
    // Clamping physics carry nothing.
    assert_eq!(ScrollPhysics::clamping().carried_momentum(1000.), 0.);
}

#[test]
fn clamping_fling_follows_the_android_spline() {
    // ClampingScrollSimulation from 70 at 500 px/s: duration 0.27507 s and
    // distance velocity * duration / rate, ending at 128.3223.
    let simulation = ScrollPhysics::clamping()
        .ballistic_simulation(70., 500., 0., 320.)
        .expect("fast enough");
    assert!(approx(simulation.x(0.), 70., 1.0e-4));
    assert!(approx(simulation.dx(0.), 500., 1.0e-2));
    assert!(approx(simulation.final_x(), 128.3223, 1.0e-2));
    assert!(!simulation.is_done(0.27));
    assert!(simulation.is_done(0.28));
    assert!(approx(simulation.x(1.), 128.3223, 1.0e-2));
    assert!(!simulation.may_overscroll());
}

#[test]
fn clamping_idles_against_the_edge_it_pushes_into() {
    let physics = ScrollPhysics::clamping();
    assert!(physics.ballistic_simulation(100., 500., 0., 100.).is_none());
    assert!(physics.ballistic_simulation(0., -500., 0., 100.).is_none());
    assert!(
        physics
            .ballistic_simulation(100., -500., 0., 100.)
            .is_some()
    );
    assert!(physics.ballistic_simulation(0., 500., 0., 100.).is_some());
}

#[test]
fn clamping_beyond_an_extent_springs_back_to_it() {
    // createBallisticSimulation: out of range -> ScrollSpringSimulation to the
    // extent with min(0, velocity). The overdamped default spring never
    // overshoots, ends exactly on the extent and may overscroll meanwhile.
    let physics = ScrollPhysics::clamping();
    let simulation = physics
        .ballistic_simulation(120., 500., 0., 100.)
        .expect("beyond the extent");
    assert!(simulation.may_overscroll());
    assert!(approx(simulation.x(0.), 120., 1.0e-3));
    assert!(
        approx(simulation.dx(0.), 0., 1.0e-3),
        "velocity is min(0, v)"
    );
    assert_eq!(simulation.final_x(), 100.);
    let mut previous = 120.;
    for step in 1..=600 {
        let x = simulation.x(step as f32 / 120.);
        assert!(x <= previous + 1.0e-4 && x >= 100. - 1.0e-4, "{x}");
        previous = x;
    }
    assert_eq!(simulation.x(5.), 100.);
    assert!(simulation.is_done(5.));
    // Below the leading extent, an outward velocity is kept as min(0, v).
    let below = physics
        .ballistic_simulation(-30., -200., 0., 100.)
        .expect("beyond the extent");
    assert!(approx(below.dx(0.), -200., 1.0e-2));
    assert_eq!(below.final_x(), 0.);
}

#[test]
fn bouncing_friction_decays_inside_the_extents() {
    // FrictionSimulation(0.135): x(inf) = x0 - v / ln(0.135), here
    // 50 + 1000 / 2.00248 = 549.3977, well inside 0..1000.
    let physics = ScrollPhysics::clamping().bouncing();
    let simulation = physics
        .ballistic_simulation(50., 1000., 0., 1000.)
        .expect("fast enough");
    assert!(approx(simulation.final_x(), 549.3977, 0.05));
    assert!(approx(simulation.dx(0.), 1000., 1.0e-2));
    // dx(t) = v * 0.135^t.
    assert!(approx(simulation.dx(1.), 135., 0.05));
    // Done once the speed falls under the 20 px/s tolerance.
    assert!(!simulation.is_done(1.5));
    assert!(simulation.is_done(2.5));
    // A slow release inside the extents does not run.
    assert!(physics.ballistic_simulation(50., 10., 0., 1000.).is_none());
}

#[test]
fn bouncing_hands_off_to_a_spring_at_the_extent_it_crosses() {
    // From 90 at 3000 px/s the friction would stop near 90 + 1498, far past
    // the trailing extent at 100: it crosses, a spring takes the (capped)
    // crossing velocity, overshoots and settles on 100.
    let physics = ScrollPhysics::clamping().bouncing();
    let simulation = physics
        .ballistic_simulation(90., 3000., 0., 100.)
        .expect("fast enough");
    assert!(simulation.may_overscroll());
    let mut highest = f32::MIN;
    for step in 0..=1200 {
        highest = highest.max(simulation.x(step as f32 / 240.));
    }
    assert!(
        highest > 100.,
        "the spring overshoots the extent: {highest}"
    );
    assert_eq!(simulation.final_x(), 100.);
    assert_eq!(simulation.x(10.), 100.);
    assert!(simulation.is_done(10.));
    // The same release toward the leading extent mirrors it.
    let back = physics
        .ballistic_simulation(10., -3000., 0., 100.)
        .expect("fast enough");
    let mut lowest = f32::MAX;
    for step in 0..=1200 {
        lowest = lowest.min(back.x(step as f32 / 240.));
    }
    assert!(lowest < 0., "{lowest}");
    assert_eq!(back.x(10.), 0.);
}

#[test]
fn bouncing_beyond_an_extent_springs_back_even_at_rest() {
    let physics = ScrollPhysics::clamping().bouncing();
    let simulation = physics
        .ballistic_simulation(-80., 0., 0., 100.)
        .expect("beyond the extent runs at zero velocity");
    assert!(approx(simulation.x(0.), -80., 1.0e-3));
    assert_eq!(simulation.final_x(), 0.);
    assert_eq!(simulation.x(10.), 0.);
    let past_end = physics
        .ballistic_simulation(180., 0., 0., 100.)
        .expect("beyond the extent runs at zero velocity");
    assert_eq!(past_end.x(10.), 100.);
}

#[test]
fn fast_deceleration_stops_sooner_than_normal() {
    // The fast rate adds a constant deceleration of 1400 px/s^2 to the
    // friction, so the same release travels less far.
    let normal = ScrollPhysics::clamping()
        .bouncing()
        .ballistic_simulation(50., 1000., 0., 10_000.)
        .expect("fast enough");
    let fast = ScrollPhysics::clamping()
        .bouncing_with(ScrollDecelerationRate::Fast)
        .ballistic_simulation(50., 1000., 0., 10_000.)
        .expect("fast enough");
    assert!(fast.final_x() > 50.);
    assert!(fast.final_x() < normal.final_x());
    assert!(fast.dx(0.5) < normal.dx(0.5));
}

#[test]
fn fast_bouncing_user_drag_is_harder_and_eases_without_friction() {
    let normal = ScrollPhysics::clamping().bouncing();
    let fast = ScrollPhysics::clamping().bouncing_with(ScrollDecelerationRate::Fast);
    // 50 px past the start in a 400 px viewport, pulling 20 px further:
    // factor 0.26 * (1 - 50/400)^2 = 0.199063 -> 3.98125 px.
    let tension = fast.apply_delta_in_viewport(-50., -20., 0., 100., 400.);
    assert!(approx(tension.position, -53.98125, 1.0e-3), "{tension:?}");
    // Easing back is not resisted at all for the fast rate.
    let easing = fast.apply_delta_in_viewport(-50., 20., 0., 100., 400.);
    assert!(approx(easing.position, -30., 1.0e-4), "{easing:?}");
    // The normal rate still resists easing (see physics_audit).
    let normal_easing = normal.apply_delta_in_viewport(-50., 20., 0., 100., 400.);
    assert!(normal_easing.position < -30.);
}

#[test]
fn spring_steps_are_independent_of_how_frames_partition_the_motion() {
    // The spring is closed form in elapsed time, so stepping from the state
    // at 0.1 s for another 0.2 s lands where one 0.3 s step from the start
    // does.
    let physics = ScrollPhysics::clamping().bouncing();
    let whole = physics.spring_step(-80., 300., 0.3, 0., 100.);
    let first = physics.spring_step(-80., 300., 0.1, 0., 100.);
    let second = physics.spring_step(first.position, first.velocity, 0.2, 0., 100.);
    assert!(
        approx(whole.position, second.position, 0.05),
        "{whole:?} {second:?}"
    );
    assert!(approx(whole.velocity, second.velocity, 0.5));
    // It settles exactly on the extent.
    let settled = physics.spring_step(-80., 0., 10., 0., 100.);
    assert!(settled.settled);
    assert_eq!(settled.position, 0.);
    assert_eq!(settled.velocity, 0.);
}

#[test]
fn fixed_extent_ballistics_land_on_the_nearest_item() {
    let physics = ScrollPhysics::clamping();
    // Scenario 5: from 70 at 500 px/s the fling would stop at 128.32, nearest
    // item 4 (128). FrictionSimulation.through tunes the drag so the motion
    // reaches 128 exactly when its speed falls to the tolerance velocity.
    let simulation = physics
        .fixed_extent_ballistic_simulation(70., 500., 0., 320., 32.)
        .expect("runs");
    let mut time = 0.;
    while !simulation.is_done(time) {
        time += 0.001;
        assert!(time < 20., "the simulation never finishes");
    }
    assert!(
        approx(simulation.x(time), 128., 0.1),
        "{}",
        simulation.x(time)
    );

    // Scenario 4: too slow to leave item 2 (64) springs back onto it.
    let spring = physics
        .fixed_extent_ballistic_simulation(70., 25., 0., 320., 32.)
        .expect("runs");
    assert!(approx(spring.x(0.), 70., 1.0e-3));
    assert_eq!(spring.final_x(), 64.);

    // Scenario 3: at rest where it intends to land, nothing runs.
    assert!(
        physics
            .fixed_extent_ballistic_simulation(64., 0., 0., 320., 32.)
            .is_none()
    );

    // Scenario 1: pushing out of the leading extent defers to the parent
    // ballistics, which idle at the edge for clamping physics.
    assert!(
        physics
            .fixed_extent_ballistic_simulation(0., -100., 0., 320., 32.)
            .is_none()
    );
}
