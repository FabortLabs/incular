//! `NestedScrollViewCoordinator`: Flutter 3.41.7
//! widgets/nested_scroll_view.dart `_NestedScrollCoordinator.applyUserOffset`
//! (1057-1116) and `goBallistic` / `_getMetrics` (729-889).

use incular_scroll::{NestedScrollViewCoordinator, ScrollController, ScrollPhysics};

/// Outer: a 200 px header over a 600 px viewport (max 200). Inner: forty
/// 50 px rows in a 600 px viewport (max 1400).
fn pair() -> (
    NestedScrollViewCoordinator,
    ScrollController,
    ScrollController,
) {
    let outer = ScrollController::new();
    let inner = ScrollController::new();
    outer
        .update_extents(800., 600.)
        .expect("free outer publishes");
    inner
        .update_extents(2000., 600.)
        .expect("free inner publishes");
    let coordinator = NestedScrollViewCoordinator::new(false);
    coordinator.attach_outer(&outer);
    coordinator.bind_inner(&inner);
    (coordinator, outer, inner)
}

#[test]
fn drag_forward_collapses_the_header_before_scrolling_the_body() {
    let (_coordinator, outer, inner) = pair();
    let physics = ScrollPhysics::clamping();
    let result = inner.apply_user_drag(physics, 150.);
    assert_eq!((outer.offset(), inner.offset()), (150., 0.));
    assert_eq!(result.unconsumed, 0., "the coordinator owns the delta");
    inner.apply_user_drag(physics, 100.);
    assert_eq!((outer.offset(), inner.offset()), (200., 50.));
    // A drag from the header routes the same way.
    outer.apply_user_drag(physics, 25.);
    assert_eq!((outer.offset(), inner.offset()), (200., 75.));
}

#[test]
fn drag_back_returns_the_body_to_its_start_before_expanding_the_header() {
    let (_coordinator, outer, inner) = pair();
    let physics = ScrollPhysics::clamping();
    inner.apply_user_drag(physics, 300.);
    assert_eq!((outer.offset(), inner.offset()), (200., 100.));
    inner.apply_user_drag(physics, -150.);
    assert_eq!((outer.offset(), inner.offset()), (150., 0.));
}

#[test]
fn floating_header_moves_first_on_a_drag_back() {
    let (coordinator, outer, inner) = pair();
    coordinator.set_float_header_slivers(true);
    let physics = ScrollPhysics::clamping();
    inner.apply_user_drag(physics, 300.);
    inner.apply_user_drag(physics, -50.);
    assert_eq!((outer.offset(), inner.offset()), (150., 100.));
}

#[test]
fn fling_forward_with_a_collapsed_header_runs_on_the_body() {
    let (_coordinator, outer, inner) = pair();
    let physics = ScrollPhysics::clamping();
    inner.apply_user_drag(physics, 200.);
    let runs = inner
        .delegated_ballistic(physics, 3000.)
        .expect("the coordinator replaces the ballistic");
    let (_, outer_run) = runs.iter().find(|(c, _)| c == &outer).expect("outer run");
    let (_, inner_run) = runs.iter().find(|(c, _)| c == &inner).expect("inner run");
    // The outer position already rests on its edge: it ends at once.
    assert_eq!(outer_run.x(0.), 200.);
    assert!(outer_run.is_done(0.016));
    // The inner position follows the shared simulation past the header.
    assert_eq!(inner_run.x(0.), 0.);
    assert!(inner_run.x(0.1) > 0.);
    assert!(inner_run.final_x() <= 1400.);
}

#[test]
fn fling_back_crosses_from_the_body_into_the_header() {
    let (_coordinator, outer, inner) = pair();
    let physics = ScrollPhysics::clamping();
    inner.apply_user_drag(physics, 300.);
    let runs = inner
        .delegated_ballistic(physics, -4000.)
        .expect("the coordinator replaces the ballistic");
    let (_, outer_run) = runs.iter().find(|(c, _)| c == &outer).expect("outer run");
    let (_, inner_run) = runs.iter().find(|(c, _)| c == &inner).expect("inner run");
    assert_eq!(inner_run.x(0.), 100.);
    assert_eq!(outer_run.x(0.), 200.);
    // The body reaches its start, then the header expands.
    assert_eq!(inner_run.final_x(), 0.);
    assert!(outer_run.final_x() < 200.);
    assert!(
        outer_run.x(0.01) == 200.,
        "the header holds until the body is back"
    );
}

#[test]
fn a_detached_coordinator_leaves_the_controllers_alone() {
    let (coordinator, outer, inner) = pair();
    coordinator.detach();
    assert!(!outer.has_drive_delegate());
    assert!(!inner.has_drive_delegate());
    inner.apply_user_drag(ScrollPhysics::clamping(), 100.);
    assert_eq!((outer.offset(), inner.offset()), (0., 100.));
}
