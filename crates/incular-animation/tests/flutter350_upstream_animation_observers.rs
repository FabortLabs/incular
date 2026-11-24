//! Behavioral assertions from Flutter 3.50.0 animation/animations_test.dart.

use incular_animation::{
    AnimationController, AnimationSource, AnimationStatus, DoubleAnimation, DrivenAnimation,
    LinearTween, ProxyAnimation, TrainHoppingAnimation,
};
use incular_core::reactivity::DependencySource;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

fn controller(value: f32) -> AnimationController {
    let controller = AnimationController::new(Duration::from_millis(100));
    controller.set_value(value);
    controller
}

#[test]
fn parentless_proxy_has_dismissed_default_value_and_status() {
    // Upstream :51. The Dart diagnostic-string checks have no direct Rust equivalent.
    let proxy: DoubleAnimation = ProxyAnimation::new(None).into();
    assert_eq!(proxy.value(), 0.0);
    assert_eq!(proxy.status(), AnimationStatus::Dismissed);
}

#[test]
fn proxy_parent_assignment_and_updates_notify_value_listener() {
    // Upstream :60.
    let parent = controller(0.5);
    let proxy = ProxyAnimation::new(None);
    let animation: DoubleAnimation = proxy.clone().into();
    let called = Rc::new(Cell::new(false));
    let flag = Rc::clone(&called);
    let _subscription = animation.observe(move |_| flag.set(true));
    assert!(!called.get());
    proxy.set_parent(Some((&parent).into()));
    assert!(called.get());
    called.set(false);
    assert!(!called.get());
    parent.set_value(0.6);
    assert!(called.get());
}

#[test]
fn reverse_animation_stops_notifying_when_listener_is_removed() {
    // Upstream :77 (behavioral assertions, not Dart diagnostic formatting).
    let parent = controller(0.5);
    let animation = DoubleAnimation::from(&parent).reversed();
    let called = Rc::new(Cell::new(false));
    let flag = Rc::clone(&called);
    let subscription = animation.observe(move |_| flag.set(true));
    assert!(!called.get());
    parent.set_value(0.6);
    assert!(called.get());
    called.set(false);
    drop(subscription);
    assert!(!called.get());
    parent.set_value(0.7);
    assert!(!called.get());
}

#[test]
fn train_switches_on_crossing_before_a_value_read() {
    // Upstream :97 (behavioral assertions, not Dart diagnostic formatting).
    let current = controller(0.5);
    let next = controller(0.75);
    let next_animation: DoubleAnimation = (&next).into();
    let called = Rc::new(Cell::new(false));
    let flag = Rc::clone(&called);
    let train = TrainHoppingAnimation::new(&current, Some(next_animation.clone()))
        .on_switched_train(move || flag.set(true));
    assert!(!called.get());
    assert_eq!(train.value(), 0.5);
    next.set_value(0.25);
    assert!(called.get());
    assert_eq!(train.value(), 0.25);
    assert_eq!(train.current_train(), next_animation);
}

// A typed DrivenAnimation has no conversion to DoubleAnimation. Preserve its
// real public tween results/status and expose only the parent's public change
// source so the two trains can share the exact original single controller.
struct DrivenTrain(DrivenAnimation<f32>);

impl AnimationSource for DrivenTrain {
    fn value(&self) -> f32 {
        self.0.value()
    }
    fn status(&self) -> AnimationStatus {
        self.0.status()
    }
    fn controllers(&self) -> Vec<AnimationController> {
        self.0.controllers()
    }
    fn changes(&self) -> DependencySource {
        self.0.parent().changes()
    }
}

#[test]
fn train_notifies_status_listener_on_direction_start_and_immediate_return() {
    // Upstream :136: two oppositely driven tweens on a single controller.
    let parent = controller(0.0);
    let train_a = DoubleAnimation::new(DrivenTrain(DoubleAnimation::from(&parent).drive(
        LinearTween {
            begin: 1.0_f32,
            end: -1.0,
        },
    )));
    let train_b = DoubleAnimation::new(DrivenTrain(DoubleAnimation::from(&parent).drive(
        LinearTween {
            begin: -1.0_f32,
            end: 1.0,
        },
    )));
    let animation: DoubleAnimation = TrainHoppingAnimation::new(train_a, Some(train_b)).into();
    let statuses = Rc::new(RefCell::new(Vec::new()));
    let recorded = Rc::clone(&statuses);
    let _subscription = animation.observe_status(move |status| recorded.borrow_mut().push(status));
    assert!(statuses.borrow().is_empty());

    let _ = parent.forward_on_next_tick();
    let forward = statuses.take();
    let _ = parent.reverse_on_next_tick();
    let reverse = statuses.take();
    assert_eq!(
        (forward, reverse),
        (
            vec![AnimationStatus::Forward],
            vec![AnimationStatus::Dismissed]
        ),
    );
}

#[test]
fn proxy_channels_follow_only_changed_values_and_statuses_when_swapping_parents() {
    // Authored paired probe; see proxy_parent_swap_reference_test.dart.
    use AnimationStatus::{Forward, Reverse};
    let first = controller(0.5);
    let equal = controller(0.5);
    let backwards = controller(0.5);
    let changed = controller(0.75);
    let _ = backwards.reverse_on_next_tick();
    let _ = changed.reverse_on_next_tick();

    let first: DoubleAnimation = first.into();
    let proxy = ProxyAnimation::new(Some(first.clone()));
    let animation: DoubleAnimation = proxy.clone().into();
    let values = Rc::new(RefCell::new(Vec::new()));
    let statuses = Rc::new(RefCell::new(Vec::new()));
    let value_log = Rc::clone(&values);
    let status_log = Rc::clone(&statuses);
    let _value = animation.observe(move |value| value_log.borrow_mut().push(value));
    let _status = animation.observe_status(move |status| status_log.borrow_mut().push(status));
    let mut snapshots = Vec::new();
    let mut capture = || {
        snapshots.push((
            (animation.value(), animation.status()),
            (values.take(), statuses.take()),
        ))
    };

    proxy.set_parent(Some(first)); // no-op: identical object
    capture();
    proxy.set_parent(Some((&equal).into())); // equal value and status, different parent
    capture();
    proxy.set_parent(Some((&backwards).into())); // only status changed
    capture();
    proxy.set_parent(Some((&changed).into())); // only value changed
    capture();
    proxy.set_parent(None); // detach preserves value and status; no notification
    capture();
    changed.set_value(0.25); // former parent cannot notify the detached proxy
    capture();
    assert_eq!(
        snapshots,
        [
            ((0.5, Forward), (vec![], vec![])),
            ((0.5, Forward), (vec![], vec![])),
            ((0.5, Reverse), (vec![], vec![Reverse])),
            ((0.75, Reverse), (vec![0.75], vec![])),
            ((0.75, Reverse), (vec![], vec![])),
            ((0.75, Reverse), (vec![], vec![])),
        ],
    );
}
