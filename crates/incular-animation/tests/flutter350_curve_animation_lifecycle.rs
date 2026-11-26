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

#[test]
fn animation_wrappers_have_one_line_diagnostics_in_either_direction() {
    use incular_animation::{
        AlwaysStoppedAnimation, ProxyAnimation, always_complete, always_dismissed,
    };

    fn one_line(value: impl std::fmt::Debug) -> String {
        let text = format!("{value:?}");
        assert!(!text.is_empty() && !text.contains('\n'), "{text}");
        text
    }
    one_line(always_complete());
    one_line(always_dismissed());
    one_line(AlwaysStoppedAnimation(0.5));
    one_line(CurvedAnimation::new(always_dismissed(), Curve::Ease));
    one_line(
        CurvedAnimation::new(always_dismissed(), Curve::Ease)
            .reverse_curve(incular_animation::Curves::ELASTIC_OUT),
    );
    let controller = AnimationController::new(ms(500)).with_value(0.5);
    let _ = controller.reverse_on_next_tick();
    one_line(
        CurvedAnimation::new(&controller, Curve::Ease)
            .reverse_curve(incular_animation::Curves::ELASTIC_OUT),
    );
    controller.stop();

    let proxy = ProxyAnimation::new(None);
    assert_eq!(proxy.value(), 0.);
    assert_eq!(proxy.status(), AnimationStatus::Dismissed);
    assert!(one_line(&proxy).contains("null;"));
    proxy.set_parent(Some(always_dismissed()));
    let description = one_line(&proxy);
    assert!(description.ends_with("➩ProxyAnimation"));
    assert!(!description.contains("null;"));
}

#[test]
fn disposing_a_train_disconnects_both_clocks_before_they_can_cross() {
    use incular_animation::TrainHoppingAnimation;

    let current = AnimationController::new(ms(100)).with_value(0.5);
    let next = AnimationController::new(ms(100)).with_value(0.75);
    let train = TrainHoppingAnimation::new(&current, Some((&next).into()))
        .on_switched_train(|| panic!("a disposed train hopped"));
    let _subscription = train
        .changes()
        .subscribe((), || panic!("a disposed train emitted an update"));
    assert!(format!("{train:?}").contains("next:"));
    train.dispose();
    train.dispose();
    current.set_value(0.8);
    next.set_value(0.25);
    assert!(train.controllers().is_empty());
    assert!(format!("{train:?}").contains("disposed"));
}

#[cfg(debug_assertions)]
#[test]
fn wrapped_animation_lifetimes_emit_exactly_one_create_and_dispose() {
    use incular_animation::{AlwaysStoppedAnimation, TrainHoppingAnimation};
    use incular_core::memory::allocations::{MemoryAllocations, ObjectEventKind};

    let events = Rc::new(RefCell::new(Vec::new()));
    let _subscription = MemoryAllocations::current().add_listener({
        let events = events.clone();
        move |event| {
            if event.library == "animation"
                && matches!(
                    event.class_name,
                    "CurvedAnimation" | "TrainHoppingAnimation"
                )
            {
                events.borrow_mut().push(event);
            }
        }
    });
    let curve = CurvedAnimation::new(AnimationController::new(ms(100)), Curve::Linear);
    assert!(!curve.is_disposed());
    curve.dispose();
    curve.dispose();
    assert!(curve.is_disposed());
    drop(curve);
    let train = TrainHoppingAnimation::new(
        AlwaysStoppedAnimation(1.0),
        Some(AlwaysStoppedAnimation(1.0).into()),
    );
    train.dispose();
    train.dispose();
    drop(train);

    let events = events.borrow();
    assert_eq!(events.len(), 4);
    for (pair, expected) in events
        .chunks_exact(2)
        .zip(["CurvedAnimation", "TrainHoppingAnimation"])
    {
        assert_eq!(pair[0].class_name, expected);
        assert_eq!(pair[1].class_name, expected);
        assert_eq!(pair[0].id, pair[1].id);
        assert_eq!(pair[0].kind, ObjectEventKind::Created);
        assert_eq!(pair[1].kind, ObjectEventKind::Disposed);
    }
    assert_ne!(events[0].id, events[2].id);
}

#[test]
#[cfg(debug_assertions)]
fn a_bad_custom_curve_names_the_endpoint_it_maps_incorrectly() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let controller = AnimationController::without_duration();
    let curve = Curve::custom("BogusCurve", |_| 100.);
    assert_eq!(curve, curve.clone());
    assert_ne!(curve, Curve::custom("BogusCurve", |_| 100.));
    let animation = CurvedAnimation::new(&controller, curve);
    let error = catch_unwind(AssertUnwindSafe(|| animation.value())).unwrap_err();
    let message = error.downcast_ref::<String>().expect("diagnostic message");
    assert_eq!(
        message,
        "Invalid curve endpoint at 0.0.\nCurves must map 0.0 to near zero and 1.0 to near one but\nBogusCurve mapped 0.0 to 100.0, which is near 100.0."
    );
}

#[test]
fn valid_custom_curve_samples_the_interior_but_keeps_the_exact_animation_endpoints() {
    let controller = AnimationController::without_duration();
    let curve = Curve::custom("NearEndpointCurve", |t| {
        if t == 0. { 0.001 } else if t == 1. { 0.999 } else { t * t }
    });
    assert_eq!(curve.transform(0.), 0.001);
    assert_eq!(curve.apply(0.), 0.);
    assert_eq!(curve.apply(0.5), 0.25);
    let animation = CurvedAnimation::new(&controller, curve);
    assert_eq!(animation.value(), 0.);
    controller.set_value(0.5);
    assert_eq!(animation.value(), 0.25);
    controller.set_value(1.);
    assert_eq!(animation.value(), 1.);
}
