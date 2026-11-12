//! Flutter's animation graph (`animation/animations.dart`, `tween.dart`) and
//! the controller API beyond `forward`/`reverse`.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_animation::{
    AlwaysStoppedAnimation, AnimatableExt, AnimationController, AnimationSource, AnimationStatus,
    AnimationStyle, CompoundAnimation, ConstantTween, Curve, CurveTween, DoubleAnimation,
    LinearTween, ProxyAnimation, ReverseAnimation, ReverseTween, StepTween, TrainHoppingAnimation,
    always_complete, always_dismissed,
};
use incular_core::time::Instant;

fn at(value: f32) -> AnimationController {
    AnimationController::new(Duration::from_millis(100)).with_value(value)
}

#[test]
fn reverse_animation_mirrors_value_and_status() {
    let controller = at(0.25);
    let reversed = ReverseAnimation::new(controller.clone());
    assert_eq!(reversed.value(), 0.75);
    controller.forward_on_next_tick();
    assert_eq!(reversed.status(), AnimationStatus::Reverse);
    controller.set_value(1.0);
    assert_eq!(reversed.status(), AnimationStatus::Dismissed);
}

#[test]
fn compound_animations_combine_values_and_prefer_the_running_status() {
    let first = at(0.2);
    let next = at(0.6);
    assert_eq!(
        CompoundAnimation::max(first.clone(), next.clone()).value(),
        0.6
    );
    assert_eq!(
        CompoundAnimation::min(first.clone(), next.clone()).value(),
        0.2
    );
    assert!((CompoundAnimation::mean(first.clone(), next.clone()).value() - 0.4).abs() < 1e-6);
    // A controller resting mid-range reports `forward` (Flutter
    // `_internalSetValue`), so the next animation rests at an end here.
    let next = at(1.0);
    let compound = CompoundAnimation::max(first.clone(), next.clone());
    first.forward_on_next_tick();
    assert_eq!(
        compound.status(),
        AnimationStatus::Forward,
        "next rests: first's"
    );
    next.reverse_on_next_tick();
    assert_eq!(
        compound.status(),
        AnimationStatus::Reverse,
        "next runs: next's"
    );
    assert_eq!(compound.controllers().len(), 2);
}

#[test]
fn stopped_animations_hold_their_value() {
    assert_eq!(AlwaysStoppedAnimation(0.3).value(), 0.3);
    assert_eq!(
        AlwaysStoppedAnimation(0.3).status(),
        AnimationStatus::Forward
    );
    assert_eq!(always_complete().status(), AnimationStatus::Completed);
    assert_eq!(always_dismissed().value(), 0.0);
    assert_eq!(DoubleAnimation::from(0.5).value(), 0.5);
}

#[test]
fn a_proxy_keeps_the_last_value_of_a_detached_parent() {
    let controller = at(0.4);
    let proxy = ProxyAnimation::new(Some(controller.clone().into()));
    assert_eq!(proxy.value(), 0.4);
    proxy.set_parent(None);
    controller.set_value(0.9);
    assert_eq!(proxy.value(), 0.4);
    proxy.set_parent(Some(controller.into()));
    assert_eq!(proxy.value(), 0.9);
}

#[test]
fn train_hopping_switches_once_the_next_train_catches_up() {
    let current = at(0.2);
    let next = at(0.8);
    let switched = Rc::new(RefCell::new(false));
    let hopping = TrainHoppingAnimation::new(current.clone(), Some(next.clone().into()))
        .on_switched_train({
            let switched = switched.clone();
            move || *switched.borrow_mut() = true
        });
    assert_eq!(hopping.value(), 0.2);
    next.set_value(0.1);
    assert_eq!(hopping.value(), 0.1, "next crossed below current");
    assert!(*switched.borrow());
    current.set_value(0.05);
    assert_eq!(hopping.value(), 0.1, "it stays on the next train");
}

#[test]
fn tweens_chain_and_animate() {
    let controller = at(0.5);
    let eased = LinearTween {
        begin: 0.0_f32,
        end: 100.0,
    }
    .chain(CurveTween(Curve::EaseIn))
    .animate(controller.clone());
    assert!((eased.value() - 100.0 * Curve::EaseIn.apply(0.5)).abs() < 1e-4);
    assert_eq!(StepTween { begin: 0, end: 10 }.transform(0.99), 9);
    assert_eq!(StepTween { begin: 0, end: 10 }.transform(1.0), 10);
    assert_eq!(ConstantTween("x").transform(0.3), "x");
    let reversed = ReverseTween(LinearTween {
        begin: 0.0_f32,
        end: 1.0,
    });
    assert!((reversed.transform(0.25) - 0.75).abs() < 1e-6);
    let curved = DoubleAnimation::from(controller).curved(Curve::EaseOut);
    assert!((curved.value() - Curve::EaseOut.apply(0.5)).abs() < 1e-6);
}

#[test]
fn controller_api_beyond_forward_and_reverse() {
    let statuses = Rc::new(RefCell::new(Vec::new()));
    let controller = AnimationController::new(Duration::from_millis(100))
        .with_reverse_duration(Duration::from_millis(50));
    controller.add_status_listener({
        let statuses = statuses.clone();
        move |status| statuses.borrow_mut().push(status)
    });
    let start = Instant::now();
    controller.toggle(start);
    controller.tick(start + Duration::from_millis(50));
    assert_eq!(controller.value(), 0.5);
    // Linear over 100 ms: one unit per 100 ms.
    assert!(
        (controller.velocity() - 10.0).abs() < 0.1,
        "{}",
        controller.velocity()
    );
    // At the end Flutter's central difference sees half the slope.
    controller.tick(start + Duration::from_millis(100));
    assert!((controller.velocity() - 5.0).abs() < 0.1);
    controller.tick(start + Duration::from_millis(101));
    assert_eq!(controller.velocity(), 0.0);
    // `toggle` from completed reverses, over the reverse duration.
    controller.toggle(start + Duration::from_millis(101));
    controller.tick(start + Duration::from_millis(126));
    assert_eq!(controller.value(), 0.5);
    controller.tick(start + Duration::from_millis(152));
    assert_eq!(
        statuses.borrow().as_slice(),
        &[
            AnimationStatus::Forward,
            AnimationStatus::Completed,
            AnimationStatus::Reverse,
            AnimationStatus::Dismissed,
        ]
    );
}

#[test]
fn repeat_starts_from_the_current_value() {
    // Flutter `_RepeatingSimulation`: the first cycle resumes at the value.
    let controller = at(0.5);
    let start = Instant::now();
    controller.repeat(start, false);
    controller.tick(start + Duration::from_millis(25));
    assert!((controller.value() - 0.75).abs() < 1e-5);
    controller.stop_repeat();
    controller.tick(start + Duration::from_millis(60));
    assert_eq!(controller.status(), AnimationStatus::Completed);
}

#[test]
fn animation_style_overrides_and_lerps() {
    let base = AnimationStyle::new().duration(Duration::from_millis(200));
    let merged = base.merge(AnimationStyle::new().curve(Curve::EaseIn));
    assert_eq!(merged.duration, Some(Duration::from_millis(200)));
    assert_eq!(merged.curve, Some(Curve::EaseIn));
    assert_eq!(AnimationStyle::NO_ANIMATION.duration, Some(Duration::ZERO));
    assert_eq!(
        AnimationStyle::lerp(Some(base), Some(AnimationStyle::NO_ANIMATION), 0.75),
        Some(AnimationStyle::NO_ANIMATION)
    );
}
