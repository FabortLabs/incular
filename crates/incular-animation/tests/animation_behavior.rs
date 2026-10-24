//! Flutter `AnimationController.animationBehavior` under
//! `SemanticsBinding.disableAnimations`.

use std::time::{Duration, Instant};

use incular_animation::{
    AnimationBehavior, AnimationController, AnimationStatus, disable_animations,
    set_disable_animations,
};

/// Restores the thread's flag even when an assertion fails.
struct Disabled;
impl Disabled {
    fn new() -> Self {
        set_disable_animations(true);
        Self
    }
}
impl Drop for Disabled {
    fn drop(&mut self) {
        set_disable_animations(false);
    }
}

#[test]
fn controllers_default_to_normal_behavior() {
    assert_eq!(
        AnimationController::new(Duration::from_millis(300)).animation_behavior(),
        AnimationBehavior::Normal
    );
    assert!(!disable_animations());
}

#[test]
fn normal_controllers_run_at_five_percent_while_animations_are_disabled() {
    let _disabled = Disabled::new();
    let controller = AnimationController::new(Duration::from_millis(300));
    let start = Instant::now();
    controller.forward(start);
    // `_animateToInternal` scales 300 ms by 0.05: 15 ms. Halfway is 7.5 ms.
    controller.tick(start + Duration::from_micros(7_500));
    assert!((controller.value() - 0.5).abs() < 1e-4);
    controller.tick(start + Duration::from_millis(15));
    assert_eq!(controller.value(), 1.0);
    assert_eq!(controller.status(), AnimationStatus::Completed);
}

#[test]
fn preserve_controllers_keep_their_duration() {
    let _disabled = Disabled::new();
    let controller = AnimationController::new(Duration::from_millis(300))
        .with_animation_behavior(AnimationBehavior::Preserve);
    let start = Instant::now();
    controller.forward(start);
    controller.tick(start + Duration::from_millis(15));
    assert!((controller.value() - 0.05).abs() < 1e-4);
    assert_eq!(controller.status(), AnimationStatus::Forward);
}

#[test]
fn enabled_animations_keep_normal_durations() {
    let controller = AnimationController::new(Duration::from_millis(300));
    let start = Instant::now();
    controller.forward(start);
    controller.tick(start + Duration::from_millis(150));
    assert!((controller.value() - 0.5).abs() < 1e-4);
}

#[test]
fn next_tick_starts_time_the_run_from_the_first_tick() {
    // Flutter's `forward()` starts its ticker, whose elapsed time counts from
    // the next frame rather than from the call.
    let controller = AnimationController::new(Duration::from_millis(100));
    controller.forward_on_next_tick();
    assert!(controller.is_active());
    assert_eq!(controller.status(), AnimationStatus::Forward);
    let first_frame = Instant::now() + Duration::from_secs(5);
    controller.tick(first_frame);
    assert_eq!(controller.value(), 0.0);
    controller.tick(first_frame + Duration::from_millis(50));
    assert!((controller.value() - 0.5).abs() < 1e-4);
    controller.tick(first_frame + Duration::from_millis(100));
    assert_eq!(controller.status(), AnimationStatus::Completed);
    controller.reverse_on_next_tick();
    controller.tick(first_frame + Duration::from_secs(1));
    controller.tick(first_frame + Duration::from_millis(1_100));
    assert_eq!(controller.status(), AnimationStatus::Dismissed);
}
