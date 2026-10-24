//! Scroll-physics audit: boundary conservation, range shapes, snap
//! thresholds, builder consistency, controller paths, and coordination.
//! Expected values below are derived by hand from the documented rules,
//! never by re-running the implementation under test.

use std::{cell::RefCell, rc::Rc};

use incular_scroll::{
    BoundaryPhysics, ClampingScrollPhysics, NestedScrollCoordinator, ScrollController,
    ScrollDecelerationRate, ScrollNotificationType, ScrollPhysics, Scrollability, SnapPhysics,
};

fn approx(left: f32, right: f32) -> bool {
    (left - right).abs() < 1.0e-3
}

#[test]
fn clamping_conserves_delta_across_positions_and_ranges() {
    // Hand-derived: position clamps, consumed is the clamped travel, and
    // the two parts always sum back to the input delta.
    for (current, delta, min, max) in [
        (50., 20., 0., 100.),
        (95., 20., 0., 100.),
        (5., -20., 0., 100.),
        (0., -7., 0., 100.),
        (100., 7., 0., 100.),
        (150., -30., 0., 100.),
        (-50., 30., 0., 100.),
        (0., 0., 0., 100.),
        (30., 500., 0., 40.),
    ] {
        let result = ScrollPhysics::clamping().apply_delta(current, delta, min, max);
        let lo = min.min(max);
        let hi = max.max(min);
        assert!(
            result.position >= lo - 1.0e-4 && result.position <= hi + 1.0e-4,
            "position leaves range for ({current}, {delta})"
        );
        assert!(
            approx(result.consumed + result.unconsumed, delta),
            "delta not conserved for ({current}, {delta})"
        );
        assert_eq!(result.consumed != 0., result.accepted);
    }
    assert_eq!(
        ScrollPhysics::clamping().apply_delta(95., 20., 0., 100.),
        incular_scroll::ScrollDelta {
            position: 100.,
            consumed: 5.,
            unconsumed: 15.,
            overscroll: 0.,
            accepted: true,
        }
    );
}

#[test]
fn empty_reversed_and_outside_ranges() {
    // Empty range under WhenScrollable: clamped in place, all unconsumed.
    let empty = ScrollPhysics::clamping().apply_delta(50., 25., 40., 40.);
    assert_eq!(empty.position, 40.);
    assert_eq!(empty.unconsumed, 25.);
    assert!(!empty.accepted);

    // Reversed bounds normalize to the same range.
    let forward = ScrollPhysics::clamping().apply_delta(50., 30., 0., 100.);
    let reversed = ScrollPhysics::clamping().apply_delta(50., 30., 100., 0.);
    assert_eq!(forward, reversed);

    // Starting outside pulls back to the bound, consuming the difference.
    let outside = ScrollPhysics::clamping().apply_delta(150., -30., 0., 100.);
    assert_eq!(outside.position, 100.);
    assert_eq!(outside.consumed, -50.);
    assert_eq!(outside.unconsumed, 20.);
}

#[test]
fn nonfinite_input_is_rejected_without_movement() {
    let nan_delta = ScrollPhysics::clamping().apply_delta(50., f32::NAN, 0., 100.);
    assert_eq!(nan_delta.position, 50.);
    assert_eq!(nan_delta.consumed, 0.);
    assert!(nan_delta.unconsumed.is_nan());

    let nan_current = ScrollPhysics::clamping().apply_delta(f32::NAN, 10., 0., 100.);
    assert!(nan_current.position.is_nan());
    assert_eq!(nan_current.unconsumed, 10.);

    let inf_delta = ScrollPhysics::clamping().apply_delta(50., f32::INFINITY, 0., 100.);
    assert_eq!(inf_delta.position, 50.);
    assert_eq!(inf_delta.unconsumed, f32::INFINITY);

    // The low-level helper passes NaN through without panicking and
    // normalizes reversed bounds itself.
    assert!(
        ClampingScrollPhysics
            .apply(50., f32::NAN, 0., 100.)
            .is_nan()
    );
    assert_eq!(ClampingScrollPhysics.apply(50., 10., 100., 0.), 60.);
}

#[test]
fn bouncing_owns_its_sequence_with_hand_derived_values() {
    // Flutter 3.41.7 BouncingScrollPhysics.applyPhysicsToUserOffset: a
    // position that is in range takes the whole offset, even past an edge.
    let physics = ScrollPhysics::clamping().bouncing();
    let bounced = physics.apply_delta_in_viewport(0., -1_000., 0., 100., 400.);
    assert_eq!(bounced.position, -1_000.);
    assert_eq!(bounced.unconsumed, 0.);
    assert_eq!(bounced.overscroll, -1_000.);
    assert!(bounced.accepted);

    // In range, including at the edge: the offset applies in full.
    let top = physics.apply_delta_in_viewport(100., 30., 0., 100., 400.);
    assert_eq!(top.position, 130.);
    assert_eq!(top.overscroll, 30.);
    assert_eq!(top.unconsumed, 0.);
    let mid = physics.apply_delta_in_viewport(50., 10., 0., 100., 400.);
    assert_eq!(mid.position, 60.);
    assert_eq!(mid.overscroll, 0.);

    // 50 px past the start in a 400 px viewport, pulling further (tension):
    // frictionFactor = 0.52 * (1 - 50/400)^2 = 0.398125, and a 20 px drag
    // is shorter than the 50 / 0.398125 px that reaches the limit, so it
    // moves 20 * 0.398125 = 7.9625 px.
    let tension = physics.apply_delta_in_viewport(-50., -20., 0., 100., 400.);
    assert!(approx(tension.position, -57.9625), "{tension:?}");
    assert!(approx(tension.overscroll, -57.9625));
    // Easing back in applies less resistance: the fraction is
    // (50 - 20) / 400 = 0.075, so the factor is 0.52 * 0.925^2 = 0.444925
    // and 20 px moves 8.8985 px.
    let easing = physics.apply_delta_in_viewport(-50., 20., 0., 100., 400.);
    assert!(approx(easing.position, -41.1015), "{easing:?}");
    // A drag longer than the distance to the limit passes the rest through:
    // the fraction (50 - 200) / 400 = -0.375 gives a factor of
    // 0.52 * 1.375^2 = 0.983125, the first 50 / 0.983125 = 50.8582 px of
    // the drag recover the 50 px overscroll and the other 149.1418 px apply
    // in full, so 199.1418 px move the position to 149.1418.
    let recover = physics.apply_delta_in_viewport(-50., 200., 0., 100., 400.);
    assert!(approx(recover.position, 149.1418), "{recover:?}");

    // The spring returns exactly to the bound along Flutter's closed-form
    // ScrollSpringSimulation (mass 0.5, stiffness 100, ratio 1.1 is
    // overdamped), never overshooting it.
    let mut previous = -50.;
    let mut settled_at = None;
    for frame in 1..=600 {
        let elapsed = frame as f32 / 120.;
        let step = physics.spring_step(-50., 0., elapsed, 0., 100.);
        assert!(step.position >= previous - 1.0e-4, "monotonic return");
        assert!(step.position <= 0. + 1.0e-4, "no overshoot");
        previous = step.position;
        if step.settled {
            settled_at = Some(elapsed);
            assert_eq!(step.position, 0.);
            assert_eq!(step.velocity, 0.);
            break;
        }
    }
    assert!(settled_at.is_some());

    // Non-bouncing policies settle trivially to the clamped position.
    let plain = ScrollPhysics::clamping().spring_step(150., 999., 0.016, 0., 100.);
    assert_eq!(plain.position, 100.);
    assert_eq!(plain.velocity, 0.);
    assert!(plain.settled);
}

#[test]
fn snap_velocity_thresholds_and_exact_multiples_are_pinned() {
    let page = ScrollPhysics::clamping().page_snapping(20.);
    // Flutter 3.41.7 widgets/page_view.dart PageScrollPhysics._getTargetPixels:
    // page = pixels / extent; a velocity beyond Tolerance.velocity (20 px/s at
    // DPR 1, widgets/scroll_physics.dart:442) moves the page by 0.5 in its
    // direction; then page.roundToDouble() (half away from zero) picks it.
    // Off-multiple, fling past the tolerance: 105/20 = 5.25 + 0.5 = 5.75 -> 6.
    assert_eq!(page.snap_target(105., 121., 0., 500.), 120.);
    // Off-multiple, slow (under 20 px/s): 5.25 -> 5.
    assert_eq!(page.snap_target(105., 19., 0., 500.), 100.);
    // 95/20 = 4.75 - 0.5 = 4.25 -> 4.
    assert_eq!(page.snap_target(95., -121., 0., 500.), 80.);
    // Slow: 4.75 -> 5.
    assert_eq!(page.snap_target(95., -19., 0., 500.), 100.);
    // Exact multiple 100/20 = 5.0. A forward fling gives 5.5 -> 6 -> 120.
    for velocity in [21., 120., 121., 500.] {
        assert_eq!(page.snap_target(100., velocity, 0., 500.), 120.);
    }
    // A backward fling gives 4.5, which rounds half away from zero to 5 -> 100;
    // slow velocities leave 5.0 -> 100.
    for velocity in [-500., -121., -120., -21., 0., 19., -19.] {
        assert_eq!(page.snap_target(100., velocity, 0., 500.), 100.);
    }
    // Targets clamp to the range.
    assert_eq!(page.snap_target(495., 500., 0., 500.), 500.);
    assert_eq!(page.snap_target(5., -500., 0., 500.), 0.);
    // Degenerate extents fall back to a plain clamp.
    for bad in [0., -20., f32::NAN] {
        let policy = ScrollPhysics::clamping().page_snapping(bad);
        assert_eq!(policy.snap_target(149., 500., 0., 500.), 149.);
        assert_eq!(policy.snap_extent(100.), None);
    }
    assert_eq!(
        ScrollPhysics::clamping()
            .page_snapping(100.)
            .snap_extent(100.),
        Some(100.)
    );
    assert_eq!(ScrollPhysics::clamping().snap_extent(100.), None);
    assert_eq!(ScrollPhysics::clamping().page().snap_extent(75.), Some(75.));
    // Viewport pages resolve against the live viewport extent.
    assert_eq!(
        ScrollPhysics::clamping()
            .page()
            .snap_target_for_extent(149., 500., 0., 500., 100.),
        ScrollPhysics::clamping()
            .page_snapping(100.)
            .snap_target(149., 500., 0., 500.)
    );
}

#[test]
fn builders_are_consistent_with_constructors_and_defaults() {
    assert_eq!(ScrollPhysics::clamping(), ScrollPhysics::default());
    // Flutter 3.41.7 widgets/scroll_physics.dart BouncingScrollPhysics.spring:
    // SpringDescription.withDampingRatio(mass: 0.5, stiffness: 100, ratio: 1.1)
    // gives damping = ratio * 2 * sqrt(mass * stiffness) = 15.5563.
    assert_eq!(
        ScrollPhysics::clamping().bouncing().boundary,
        BoundaryPhysics::Bouncing {
            deceleration_rate: ScrollDecelerationRate::Normal,
        }
    );
    let spring = ScrollPhysics::clamping().bouncing().spring();
    assert_eq!((spring.mass, spring.stiffness), (0.5, 100.));
    assert!(approx(spring.damping, 15.5563));
    let carousel = ScrollPhysics::clamping().carousel(32.);
    let fixed = ScrollPhysics::clamping().fixed_extent_snapping(32.);
    assert_eq!(carousel.snap, fixed.snap);
    assert_eq!(
        carousel.snap_target(70., -500., 0., 320.),
        fixed.snap_target(70., -500., 0., 320.)
    );
    assert_eq!(
        ScrollPhysics::clamping().page().snap,
        SnapPhysics::PageViewport
    );
    // Composition: explicit child choices win, defaults inherit, range
    // maintenance ORs, and operand order does not matter.
    let first = ScrollPhysics::clamping()
        .bouncing()
        .then(ScrollPhysics::clamping().always_scrollable())
        .range_maintaining();
    let second = ScrollPhysics::clamping()
        .range_maintaining()
        .then(ScrollPhysics::clamping().bouncing().always_scrollable());
    assert_eq!(first.scrollability, Scrollability::Always);
    assert_eq!(first.boundary, second.boundary);
    assert_eq!(first.scrollability, second.scrollability);
    assert_eq!(first.snap, second.snap);
    assert!(first.is_range_maintaining());
    assert!(second.is_range_maintaining());
    assert!(!ScrollPhysics::clamping().is_range_maintaining());
    let parented = ScrollPhysics::clamping().parent(ScrollPhysics::clamping().bouncing());
    assert_eq!(
        parented.boundary,
        BoundaryPhysics::Bouncing {
            deceleration_rate: ScrollDecelerationRate::Normal,
        }
    );
    // Threshold constants are locked API.
    assert_eq!(ScrollPhysics::clamping().min_fling_velocity(), 50.0);
    assert_eq!(ScrollPhysics::clamping().max_fling_velocity(), 8000.0);
    assert_eq!(ScrollPhysics::clamping().min_fling_distance(), 18.0);
    // Flutter scroll_physics.dart: only BouncingScrollPhysics overrides
    // dragStartDistanceMotionThreshold (3.5); the base and clamping are null.
    assert_eq!(
        ScrollPhysics::clamping().drag_start_distance_motion_threshold(),
        None
    );
    assert_eq!(
        ScrollPhysics::clamping()
            .bouncing()
            .drag_start_distance_motion_threshold(),
        Some(3.5)
    );
}

#[test]
fn controller_distinguishes_input_from_programmatic_movement() {
    let controller = ScrollController::new();
    controller
        .update_extents(300., 100.)
        .expect("free controller publishes");
    let kinds = Rc::new(RefCell::new(Vec::new()));
    let kinds_for_listener = kinds.clone();
    let _subscription = controller.add_listener(move |notification| {
        kinds_for_listener.borrow_mut().push(notification.kind);
        false
    });

    // User input path persists and reports Update.
    let input = controller.apply_physics(ScrollPhysics::clamping(), 40.);
    assert!(input.accepted);
    assert_eq!(controller.offset(), 40.);
    // Programmatic path persists and reports Update too; both move state,
    // only policy-gated input can refuse.
    assert!(controller.jump_to(80.));
    assert!(controller.scroll_by(-30.));
    assert_eq!(controller.offset(), 50.);
    assert_eq!(
        *kinds.borrow(),
        vec![
            ScrollNotificationType::Update,
            ScrollNotificationType::Update,
            ScrollNotificationType::Update,
        ]
    );

    // Policy swap on the same controller takes effect on the next input:
    // Never refuses while jump_to still programs movement (W5 owns
    // attachment policy, not per-call policy selection).
    let refused = controller.apply_physics(ScrollPhysics::clamping().never_scrollable(), 40.);
    assert!(!refused.accepted);
    assert_eq!(controller.offset(), 50.);
    assert!(controller.jump_to(90.));
    let moved = controller.apply_physics(ScrollPhysics::clamping(), 40.);
    assert!(moved.accepted);
    assert_eq!(controller.offset(), 130.);
}

#[test]
fn extents_sanitize_ranges_and_apply_pending_jumps() {
    let controller = ScrollController::new();
    // Negative content/viewport collapse to an empty range at zero.
    controller
        .update_extents(-50., -10.)
        .expect("free controller publishes");
    assert_eq!(controller.max_offset(), 0.);
    assert_eq!(controller.offset(), 0.);

    // A pre-layout jump defers instead of clamping away, then applies
    // once the range exists.
    let controller = ScrollController::new();
    assert!(controller.deferred_jump_to(150.));
    controller
        .update_extents(300., 100.)
        .expect("free controller publishes");
    assert_eq!(controller.offset(), 150.);

    // Shrinking the range clamps a live offset back inside.
    assert!(controller.jump_to(200.));
    controller
        .update_extents(120., 100.)
        .expect("free controller publishes");
    assert_eq!(controller.max_offset(), 20.);
    assert_eq!(controller.offset(), 20.);
}

#[test]
fn settle_and_anchor_adjustment_follow_their_gates() {
    let controller = ScrollController::new();
    controller
        .update_extents(300., 100.)
        .expect("free controller publishes");
    // Viewport-relative settle uses the live viewport extent.
    assert!(controller.jump_to(149.));
    assert!(controller.settle_physics(ScrollPhysics::clamping().page(), 0.));
    assert_eq!(controller.offset(), 100.);

    // Anchor adjustment requires a range-maintaining policy and a finite
    // delta; otherwise it reports no change.
    let plain = ScrollPhysics::clamping();
    let maintaining = ScrollPhysics::clamping().range_maintaining();
    assert!(!controller.adjust_for_content_change(48., plain));
    assert!(!controller.adjust_for_content_change(f32::NAN, maintaining));
    assert!(controller.adjust_for_content_change(48., maintaining));
    assert_eq!(controller.offset(), 148.);
    assert!(controller.adjust_for_content_change(-24., maintaining));
    assert_eq!(controller.offset(), 124.);
}

#[test]
fn coordinator_spans_ranges_innermost_first_with_conservation() {
    let inner = ScrollController::new();
    let mid = ScrollController::new();
    let outer = ScrollController::new();
    inner
        .update_extents(120., 100.)
        .expect("free controller publishes");
    mid.update_extents(300., 100.)
        .expect("free controller publishes");
    outer
        .update_extents(1_000., 100.)
        .expect("free controller publishes");
    let mut coordinator = NestedScrollCoordinator::new([inner.clone(), mid.clone(), outer.clone()]);
    // 20 + 200 + remainder: inner takes 20, mid takes 200, outer the rest.
    let result = coordinator.apply_drag_delta(400.);
    assert_eq!(inner.offset(), 20.);
    assert_eq!(mid.offset(), 200.);
    assert_eq!(outer.offset(), 180.);
    assert!(approx(result.consumed + result.unconsumed, 400.));
    assert_eq!(result.unconsumed, 0.);
    assert_eq!(coordinator.active_index(), Some(2));

    // Reversing hands back down the same chain without duplication.
    let result = coordinator.apply_drag_delta(-400.);
    assert_eq!(inner.offset(), 0.);
    assert_eq!(mid.offset(), 0.);
    assert_eq!(outer.offset(), 0.);
    assert_eq!(result.unconsumed, 0.);
    coordinator.cancel();
    assert_eq!(coordinator.active_index(), None);
}

#[test]
fn clamping_simulation_matches_flutter_clamping_scroll_simulation() {
    // Flutter 3.41.7 widgets/scroll_simulation.dart ClampingScrollSimulation,
    // friction 0.015, with _kDecelerationRate = ln(0.78)/ln(0.9) = 2.3582,
    // _kInflexion = 0.35 and _physicalCoeff = 9.80665 * 39.37 * 160 * 0.84.
    // For 2,000 px/s: referenceVelocity = 0.015 * 51890.2 / 0.35 = 2223.87,
    // duration = 2.3582 * 0.35 * (2000 / 2223.87)^(1 / 1.3582) = 0.763348 s,
    // distance = 2000 * 0.763348 / 2.3582 = 647.399 px. Evaluated by hand in
    // f64 so the f32 implementation never grades its own homework.
    let simulation = ScrollPhysics::clamping()
        .fling_simulation(0., 2000.)
        .expect("2,000 px/s is above the tolerance velocity");
    assert!(approx(simulation.x(0.), 0.));
    assert!(approx(simulation.x(0.016), 31.545_65));
    assert!(approx(simulation.dx(0.016), 1_943.278));
    assert!(approx(simulation.x(0.5), 594.769_2));
    assert!(!simulation.is_done(0.763));
    assert!(simulation.is_done(0.763_35));
    assert!(approx(simulation.x(10.), 647.398_8));
    assert!(approx(simulation.dx(10.), 0.));
    // Positions are offset by the start, and both signs are symmetric.
    let back = ScrollPhysics::clamping()
        .fling_simulation(100., -2000.)
        .expect("fling");
    assert!(approx(back.x(0.016), 100. - 31.545_65));
    assert!(approx(back.dx(0.016), -1_943.278));
}

#[test]
fn bouncing_simulation_is_friction_simulation_with_drag_0_135() {
    // Flutter 3.41.7 BouncingScrollSimulation runs FrictionSimulation(0.135)
    // inside the extents: x(t) = x0 + v0 * (0.135^t - 1) / ln(0.135) and
    // dx(t) = v0 * 0.135^t, done when |dx| < 20 px/s (the DPR 1 tolerance),
    // here at t = ln(20 / 2000) / ln(0.135) = 2.2997 s.
    let simulation = ScrollPhysics::clamping()
        .bouncing()
        .fling_simulation(0., 2000.)
        .expect("fling");
    assert!(approx(simulation.x(0.016), 31.492_8));
    assert!(approx(simulation.dx(0.016), 1_936.936));
    assert!(!simulation.is_done(2.29));
    assert!(simulation.is_done(2.30));
    assert!(approx(simulation.x(2.299_732_8), 988.773_7));
}

#[test]
fn simulation_agrees_with_itself_across_frame_partitions() {
    // x(t) is a closed form in elapsed time, so the distance covered by a
    // sequence of frame deltas telescopes to x(total) however frames fall.
    let simulation = ScrollPhysics::clamping()
        .fling_simulation(0., 2000.)
        .expect("fling");
    for partitions in [1usize, 2, 10, 16] {
        let mut applied = 0.;
        let mut previous = simulation.x(0.);
        for frame in 1..=partitions {
            let position = simulation.x(0.16 * frame as f32 / partitions as f32);
            applied += position - previous;
            previous = position;
        }
        assert!((applied - simulation.x(0.16)).abs() < 0.01);
    }
}

#[test]
fn no_simulation_under_the_tolerance_velocity_or_for_bad_input() {
    // Flutter createBallisticSimulation returns null when
    // |velocity| < tolerance.velocity (20 px/s at DPR 1): the scroll goes
    // idle rather than starting a fling.
    assert!(
        ScrollPhysics::clamping()
            .fling_simulation(0., 19.)
            .is_none()
    );
    assert!(
        ScrollPhysics::clamping()
            .fling_simulation(0., -19.)
            .is_none()
    );
    assert!(
        ScrollPhysics::clamping()
            .fling_simulation(0., 20.)
            .is_some()
    );
    assert!(
        ScrollPhysics::clamping()
            .fling_simulation(0., f32::NAN)
            .is_none()
    );
    assert!(
        ScrollPhysics::clamping()
            .fling_simulation(f32::NAN, 100.)
            .is_none()
    );
}
