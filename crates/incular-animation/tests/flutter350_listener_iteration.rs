use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::{AnimationController, AnimationListenerChannel, AnimationStatus};

type Log = Rc<RefCell<Vec<&'static str>>>;

fn record<T>(log: &Log, name: &'static str) -> impl Fn(T) + 'static {
    let log = log.clone();
    move |_| log.borrow_mut().push(name)
}

fn controller() -> AnimationController {
    AnimationController::new(Duration::from_millis(100))
}

#[test]
fn value_listeners_skip_removed_entries_and_snapshot_one_shared_identity() {
    let controller = controller();
    let log = Log::default();
    let removals = Rc::new(Cell::new([0; 2]));
    let shared_id = Rc::new(Cell::new(None));
    let repeated: Rc<dyn Fn(f32)> = Rc::new(record(&log, "listener4"));

    let first = controller.add_listener(record(&log, "listener1"));
    let middle = controller.add_listener({
        let controller = controller.clone();
        let log = log.clone();
        let removals = removals.clone();
        let shared_id = shared_id.clone();
        move |_| {
            log.borrow_mut().push("listener2");
            for id in removals.get() {
                controller.remove_listener(id);
            }
            shared_id.set(Some(controller.add_shared_listener(repeated.clone())));
        }
    });
    let third = controller.add_listener(record(&log, "listener3"));
    removals.set([first, third]);
    for (value, callbacks) in [
        (0.2, vec!["listener1", "listener2"]),
        (0.3, vec!["listener2", "listener4"]),
        (0.4, vec!["listener2", "listener4"]),
    ] {
        controller.set_value(value);
        assert_eq!(log.take(), callbacks);
    }

    assert!(controller.remove_listener(middle));
    let id = shared_id.get().unwrap();
    for value in [0.5, 0.6] {
        assert!(controller.remove_listener(id));
        controller.set_value(value);
        assert_eq!(log.take(), ["listener4"]);
    }
    assert!(controller.remove_listener(id));
    controller.set_value(0.7);
    assert!(log.borrow().is_empty());
    assert!(!controller.remove_listener(id));
}

#[test]
fn status_listener_iteration_retains_duplicate_registrations_in_order() {
    let controller = controller();
    let log = Log::default();
    let removals = Rc::new(Cell::new([0; 2]));
    let added = Rc::new(RefCell::new(Vec::new()));
    let repeated: Rc<dyn Fn(AnimationStatus)> = Rc::new(record(&log, "listener4"));
    let first = controller.add_status_listener(record(&log, "listener1"));
    let middle = controller.add_status_listener({
        let controller = controller.clone();
        let log = log.clone();
        let removals = removals.clone();
        let added = added.clone();
        move |_| {
            log.borrow_mut().push("listener2");
            for id in removals.get() {
                controller.remove_status_listener(id);
            }
            added
                .borrow_mut()
                .push(controller.add_shared_status_listener(repeated.clone()));
        }
    });
    let third = controller.add_status_listener(record(&log, "listener3"));
    removals.set([first, third]);
    let _ = controller.forward_on_next_tick();
    assert_eq!(log.take(), ["listener1", "listener2"]);
    let _ = controller.reverse_on_next_tick();
    assert_eq!(log.take(), ["listener2", "listener4"]);
    let _ = controller.forward_on_next_tick();
    assert_eq!(log.take(), ["listener2", "listener4", "listener4"]);

    assert!(controller.remove_status_listener(middle));
    let ids = added.take();
    assert_eq!(ids.len(), 3);
    assert!(controller.remove_status_listener(ids[1]));
    let _ = controller.reverse_on_next_tick();
    assert_eq!(log.take(), ["listener4", "listener4"]);
}

#[cfg(panic = "unwind")]
#[test]
fn a_throwing_listener_is_reported_without_skipping_other_callbacks() {
    for channel in [
        AnimationListenerChannel::Value,
        AnimationListenerChannel::Status,
    ] {
        let controller = controller();
        let log = Log::default();
        let failures = Rc::new(RefCell::new(Vec::new()));
        controller.set_listener_error_handler({
            let failures = failures.clone();
            move |error| {
                failures
                    .borrow_mut()
                    .push((error.channel, *error.payload.downcast::<usize>().unwrap()))
            }
        });
        let bad = {
            let log = log.clone();
            move || {
                log.borrow_mut().push("badListener");
                std::panic::panic_any(17_usize);
            }
        };
        match channel {
            AnimationListenerChannel::Value => {
                controller.add_listener(record(&log, "listener1"));
                controller.add_listener(move |_| bad());
                controller.add_listener(record(&log, "listener2"));
                let status_count = Rc::new(Cell::new(0));
                let _ = controller.add_status_listener({
                    let count = status_count.clone();
                    move |_| count.set(count.get() + 1)
                });
                controller.set_value(0.2);
                assert_eq!(
                    status_count.get(),
                    1,
                    "value failure must not interrupt status delivery"
                );
            }
            AnimationListenerChannel::Status => {
                controller.add_status_listener(record(&log, "listener1"));
                controller.add_status_listener(move |_| bad());
                controller.add_status_listener(record(&log, "listener2"));
                let _ = controller.forward_on_next_tick();
            }
        }
        assert_eq!(log.take(), ["listener1", "badListener", "listener2"]);
        assert_eq!(failures.take(), [(channel, 17)]);
        controller.clear_listener_error_handler();
    }
}
