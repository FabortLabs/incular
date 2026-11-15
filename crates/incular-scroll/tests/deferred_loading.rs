//! Flutter's `ScrollPhysics.recommendDeferredLoading` and
//! `ScrollPosition.recommendDeferredLoading` (scroll-008): a position
//! recommends deferring expensive work while its activity velocity plus the
//! distance of a jump in the current frame exceeds the longest side of the
//! view in physical pixels.

use std::time::Instant;

use incular_scroll::{ActivityOrigin, ScrollController, ScrollPhysics};

#[test]
fn physics_defer_above_the_longest_physical_side() {
    for physics in [ScrollPhysics::CLAMPING, ScrollPhysics::BOUNCING] {
        assert!(!physics.recommend_deferred_loading(2000., 2000.));
        assert!(physics.recommend_deferred_loading(2000.5, 2000.));
        assert!(physics.recommend_deferred_loading(-2400., 2000.));
        assert!(!physics.recommend_deferred_loading(0., 2000.));
    }
}

#[test]
fn the_activity_velocity_lapses_with_its_activity() {
    let controller = ScrollController::new();
    assert_eq!(controller.activity_velocity(), 0.);
    assert!(!controller.recommend_deferred_loading(1000.));

    let activity = controller.start_owned_activity(ActivityOrigin::Animation);
    assert!(activity.set_velocity(-3000.));
    assert_eq!(controller.activity_velocity(), -3000.);
    assert!(controller.recommend_deferred_loading(1000.));
    assert!(!controller.recommend_deferred_loading(4000.));

    // A takeover starts with no velocity, and the stale token cannot set one.
    let takeover = controller.start_owned_activity(ActivityOrigin::Drag);
    assert_eq!(controller.activity_velocity(), 0.);
    assert!(!activity.set_velocity(5000.));
    assert_eq!(controller.activity_velocity(), 0.);

    assert!(takeover.set_velocity(2500.));
    assert!(takeover.finish());
    assert_eq!(controller.activity_velocity(), 0.);
    assert!(!controller.recommend_deferred_loading(1000.));
}

#[test]
fn a_jump_counts_as_velocity_until_the_frame_ends() {
    let controller = ScrollController::new();
    controller.update_extents(10_000., 100.).expect("extents");
    assert!(controller.jump_to(5000.));
    assert!(controller.recommend_deferred_loading(2000.));
    assert!(!controller.recommend_deferred_loading(6000.));
    // Flutter clears `_impliedVelocity` in a post-frame callback.
    incular_animation::scheduler::end_frame(Instant::now());
    assert!(!controller.recommend_deferred_loading(2000.));

    // A short jump stays below the threshold.
    assert!(controller.jump_to(4900.));
    assert!(!controller.recommend_deferred_loading(2000.));
    incular_animation::scheduler::end_frame(Instant::now());
}
