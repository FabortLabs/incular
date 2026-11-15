use incular_animation::AnimationController;
use incular_core::BuildContext;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

#[test]
fn animation_values_track_and_owned_observers_unsubscribe() {
    let context = BuildContext::new();
    let controller = AnimationController::new(Duration::from_secs(1));
    let calls = Rc::new(Cell::new(0));
    let subscription = controller.observe({
        let calls = calls.clone();
        move |_| calls.set(calls.get() + 1)
    });
    context.build(|_| {
        let _ = controller.value();
    });
    controller.set_value(0.5);
    assert!(context.take_dirty());
    assert_eq!(calls.get(), 1);
    drop(subscription);
    controller.set_value(1.0);
    assert!(context.is_dirty());
    assert_eq!(calls.get(), 1);
}

#[test]
fn animation_listener_can_mutate_after_notification_releases_state_borrow() {
    let controller = AnimationController::new(Duration::from_secs(1));
    let listener = controller.add_listener({
        let controller = controller.clone();
        move |value| {
            if value == 0.0 {
                controller.set_value(0.5);
            }
        }
    });
    // Flutter `forward` notifies no value listener when the value stays put
    // (`_startSimulation`); the first tick does.
    let now = std::time::Instant::now();
    controller.forward(now);
    controller.tick(now);
    assert_eq!(controller.value(), 0.5);
    assert!(controller.remove_listener(listener));
}

/// anim_core-001: Flutter's `notifyListeners` runs on every tick and on every
/// value assignment, even when the value does not change.
#[test]
fn value_listeners_hear_every_tick_and_assignment() {
    let controller = AnimationController::new(Duration::from_millis(100)).with_value(0.25);
    let count = Rc::new(Cell::new(0));
    let id = controller.add_listener({
        let count = count.clone();
        move |_| count.set(count.get() + 1)
    });
    let start = incular_core::time::Instant::now();
    controller.forward_on_next_tick();
    controller.tick(start);
    assert_eq!(count.get(), 1, "the zero-elapsed first tick notifies");
    controller.set_value(0.25);
    assert_eq!(count.get(), 2, "an unchanged assignment notifies");
    assert!(controller.remove_listener(id));
}

/// anim_core-002: the first forward status reaches status listeners even when
/// the controller starts at an interior value.
#[test]
fn first_forward_reports_status_from_an_interior_value() {
    let controller = AnimationController::new(Duration::from_millis(100)).with_value(0.25);
    let statuses = Rc::new(RefCell::new(Vec::new()));
    let _id = controller.add_status_listener({
        let statuses = statuses.clone();
        move |status| statuses.borrow_mut().push(status)
    });
    controller.forward_on_next_tick();
    assert_eq!(
        *statuses.borrow(),
        vec![incular_animation::AnimationStatus::Forward]
    );
}

/// anim_core-001 residual: Flutter `_startSimulation` notifies no value
/// listener, so reversing from the value the controller already holds is
/// heard only by status listeners; its first tick then notifies (harness
/// anim_core__controller_listener_notifications, frame 6).
#[test]
fn a_reversal_from_the_current_value_does_not_notify_value_listeners() {
    let controller = AnimationController::new(Duration::from_millis(100));
    controller.set_value(0.75);
    let count = Rc::new(Cell::new(0));
    controller.add_listener({
        let count = count.clone();
        move |_| count.set(count.get() + 1)
    });
    let statuses = Rc::new(Cell::new(0));
    controller.add_status_listener({
        let statuses = statuses.clone();
        move |_| statuses.set(statuses.get() + 1)
    });
    controller.reverse_on_next_tick();
    assert_eq!(
        count.get(),
        0,
        "starting the run notifies no value listener"
    );
    assert_eq!(statuses.get(), 1, "the reverse status is reported");
    controller.tick(incular_core::time::Instant::now());
    assert_eq!(count.get(), 1, "the first tick notifies");
}
