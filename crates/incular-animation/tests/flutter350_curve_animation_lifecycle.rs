use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_animation::{
    AnimationController, AnimationSource, AnimationStatus, Curve, CurvedAnimation, DoubleAnimation,
    animation_max, animation_mean, animation_min,
};
use incular_core::time::Instant;

fn ms(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

#[test]
fn compound_mean_max_and_min_notify_and_then_unsubscribe() {
    type Combine = fn(AnimationController, AnimationController) -> DoubleAnimation;
    let cases: [(Combine, f32, f32, f32, f32); 3] = [
        (animation_mean, 0.25, 0.75, 0., 0.50),
        (animation_max, 0.50, 1.00, 0., 1.00),
        (animation_min, 0.00, 0.50, 0.25, 0.25),
    ];
    for (combine, initial, after_right, after_left_change, final_value) in cases {
        let left = AnimationController::new(ms(100)).with_value(0.5);
        let right = AnimationController::new(ms(100));
        let derived: DoubleAnimation = combine(left.clone(), right.clone());
        assert!(!format!("{derived:?}").contains('\n'));
        assert_eq!(derived.value(), initial);
        let log = Rc::new(RefCell::new(Vec::new()));
        let subscription = derived.observe({
            let log = log.clone();
            move |value| log.borrow_mut().push(value)
        });
        assert!(log.borrow().is_empty());
        right.set_value(1.);
        assert_eq!(derived.value(), after_right);
        assert_eq!(log.take(), [after_right]);
        drop(subscription);
        left.set_value(after_left_change);
        assert_eq!(derived.value(), final_value);
        assert!(log.borrow().is_empty());
    }
}

#[test]
fn linear_curved_and_reversed_wrappers_inherit_different_direction_durations() {
    let parent = AnimationController::new(ms(100)).with_reverse_duration(ms(50));
    let curved: DoubleAnimation = CurvedAnimation::new(&parent, Curve::Linear)
        .reverse_curve(Curve::Linear)
        .into();
    let reversed = curved.clone().reversed();
    let now = Instant::now();
    let _ = parent.forward_on_next_tick();
    assert_eq!(curved.status(), AnimationStatus::Forward);
    assert_eq!(reversed.status(), AnimationStatus::Reverse);
    let compare = |expected: f32| {
        assert!(
            (curved.value() - expected).abs() < 1e-6,
            "curved {}",
            curved.value()
        );
        assert!(
            (reversed.value() - (1. - expected)).abs() < 1e-6,
            "reversed {}",
            reversed.value()
        );
    };
    for index in 0..=10 {
        parent.tick(now + ms(10 * index));
        compare(index as f32 / 10.);
    }
    assert_eq!(curved.status(), AnimationStatus::Forward);
    compare(1.);
    parent.tick(now + ms(101));
    assert_eq!(curved.status(), AnimationStatus::Completed);
    assert_eq!(reversed.status(), AnimationStatus::Dismissed);
    let _ = parent.reverse_on_next_tick();
    assert_eq!(curved.status(), AnimationStatus::Reverse);
    assert_eq!(reversed.status(), AnimationStatus::Forward);
    for index in 0..=5 {
        parent.tick(now + ms(110 + 10 * index));
        compare(1. - index as f32 / 5.);
    }
    assert_eq!(curved.status(), AnimationStatus::Reverse);
    compare(0.);
    parent.tick(now + ms(161));
    assert_eq!(curved.status(), AnimationStatus::Dismissed);
    assert_eq!(reversed.status(), AnimationStatus::Completed);
}

#[test]
fn disposing_a_curved_animation_freezes_its_curve_direction_not_parent_reads() {
    let parent = AnimationController::new(ms(100)).with_reverse_duration(ms(100));
    let forward = Curve::interval(0., 0.5, &Curve::Linear);
    let reverse = Curve::interval(0.5, 1., &Curve::Linear);
    assert_eq!(forward.apply(0.5), 1.);
    assert_eq!(reverse.apply(0.5), 0.);
    let curved = CurvedAnimation::new(&parent, forward).reverse_curve(reverse);
    let notifications = Rc::new(std::cell::Cell::new(0));
    let _subscription = curved.changes().subscribe((), {
        let notifications = notifications.clone();
        move || notifications.set(notifications.get() + 1)
    });
    let _ = parent.forward_from_on_next_tick(0.5);
    assert_eq!(parent.status(), AnimationStatus::Forward);
    assert_eq!(curved.value(), 1.);
    parent.set_value(1.);
    assert_eq!(parent.status(), AnimationStatus::Completed);
    let _ = parent.reverse_from_on_next_tick(0.5);
    assert_eq!(parent.status(), AnimationStatus::Reverse);
    assert_eq!(curved.value(), 0.);
    assert!(!curved.is_disposed());
    curved.dispose();
    curved.dispose();
    assert!(curved.is_disposed());
    let before = notifications.get();
    parent.set_value(0.);
    assert_eq!(parent.status(), AnimationStatus::Dismissed);
    let _ = parent.forward_from_on_next_tick(0.5);
    assert_eq!(parent.status(), AnimationStatus::Forward);
    assert_eq!(curved.value(), 0.);
    assert!(
        notifications.get() > before,
        "value reads still track the original parent"
    );
}
