//! Flutter 3.50.0 ChangeNotifier disposal and the Rust subscription boundary.

use incular_core::ChangeNotifier;
use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

#[test]
fn disposal_rejects_registration_notification_and_a_second_disposal() {
    let notifier = ChangeNotifier::new();
    notifier.dispose();
    assert!(catch_unwind(AssertUnwindSafe(|| notifier.add_listener(|| {}))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| notifier.dispose())).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| notifier.notify_listeners())).is_err());
}

#[test]
fn removing_a_listener_after_disposal_is_safe() {
    let notifier = ChangeNotifier::new();
    let listener = notifier.add_listener(|| {});
    notifier.dispose();
    drop(listener);
    assert!(!notifier.has_listeners());
}

#[test]
fn disposing_clears_has_listeners_even_with_an_outstanding_subscription() {
    let notifier = ChangeNotifier::new();
    let _listener = notifier.add_listener(|| {});
    assert!(notifier.has_listeners());
    notifier.dispose();
    assert!(!notifier.has_listeners());
}

#[test]
fn disposal_releases_captures_even_if_the_subscription_is_still_held() {
    let notifier = ChangeNotifier::new();
    let capture = Rc::new(Cell::new(0));
    let weak = Rc::downgrade(&capture);
    let listener = notifier.add_listener(move || capture.set(capture.get() + 1));
    assert!(weak.upgrade().is_some());
    notifier.notify_listeners();
    assert_eq!(weak.upgrade().unwrap().get(), 1);
    notifier.dispose();
    assert!(weak.upgrade().is_none());
    drop(listener);
}

#[cfg(debug_assertions)]
#[test]
fn the_debug_assertion_distinguishes_an_alive_notifier_from_a_disposed_one() {
    let notifier = ChangeNotifier::new();
    assert!(notifier.debug_assert_not_disposed());
    notifier.dispose();
    let error =
        catch_unwind(AssertUnwindSafe(|| notifier.debug_assert_not_disposed())).unwrap_err();
    let message = error.downcast::<&'static str>().unwrap();
    assert!(message.contains("A ChangeNotifier was used after being disposed"));
}

#[test]
fn a_second_dispose_identifies_the_disposed_notifier() {
    let notifier = ChangeNotifier::new();
    notifier.dispose();
    let error = catch_unwind(AssertUnwindSafe(|| notifier.dispose())).unwrap_err();
    let message = error.downcast::<&'static str>().unwrap();
    assert_eq!(
        *message,
        "A ChangeNotifier was used after being disposed.\nOnce you have called dispose() on a ChangeNotifier, it can no longer be used."
    );
}

#[test]
fn disposing_during_notification_is_rejected_without_closing_the_notifier() {
    let notifier = ChangeNotifier::new();
    let reentrant = notifier.clone();
    let _first = notifier.add_listener(move || {
        assert!(catch_unwind(AssertUnwindSafe(|| reentrant.dispose())).is_err());
    });
    let count = Rc::new(Cell::new(0));
    let callback_count = count.clone();
    let _second = notifier.add_listener(move || callback_count.set(callback_count.get() + 1));
    notifier.notify_listeners();
    assert_eq!(count.get(), 1);
    assert!(notifier.has_listeners());
    notifier.dispose();
    assert!(!notifier.has_listeners());
}

#[test]
fn nested_notification_observes_removals_and_its_own_registration_boundary() {
    use std::cell::RefCell;

    let notifier = ChangeNotifier::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let old = Rc::new(RefCell::new(None));
    let added = Rc::new(RefCell::new(None));
    let initial = Rc::new(Cell::new(true));
    let append = |value| {
        let log = log.clone();
        move || log.borrow_mut().push(value)
    };
    let _first = notifier.add_listener({
        let reentrant = notifier.clone();
        let old = old.clone();
        let added = added.clone();
        let record = append('A');
        let new_listener = append('D');
        move || {
            record();
            if initial.replace(false) {
                old.borrow_mut().take();
                *added.borrow_mut() = Some(reentrant.add_listener(new_listener.clone()));
                reentrant.notify_listeners();
            }
        }
    });
    *old.borrow_mut() = Some(notifier.add_listener(append('B')));
    let _third = notifier.add_listener(append('C'));
    notifier.notify_listeners();
    assert_eq!(*log.borrow(), ['A', 'A', 'C', 'D', 'C']);
    log.borrow_mut().clear();
    notifier.notify_listeners();
    assert_eq!(*log.borrow(), ['A', 'C', 'D']);
}

#[test]
fn disposal_inside_a_callback_reports_error_before_returning_to_that_callback() {
    let notifier = ChangeNotifier::new();
    let attempted = notifier.clone();
    let finished = Rc::new(Cell::new(false));
    let _registration = notifier.add_listener({
        let finished = finished.clone();
        move || {
            attempted.dispose();
            finished.set(true);
        }
    });
    let errors = notifier.notify_listeners_collecting_panics();
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&'static str>().copied())
        .unwrap();
    assert!(message.contains("Cannot dispose a ChangeNotifier during notification."));
    assert!(!finished.get());
    assert!(notifier.has_listeners());
    notifier.dispose();
    assert!(!notifier.has_listeners());
}

#[test]
fn one_reported_observer_failure_never_blocks_later_registrations() {
    use std::{cell::RefCell, panic::panic_any};

    let notifier = ChangeNotifier::new();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let register = |label| {
        let trace = trace.clone();
        notifier.add_listener(move || trace.borrow_mut().push(label))
    };
    let one = register("listener");
    let duplicate = register("listener");
    assert!(notifier.notify_listeners_collecting_panics().is_empty());
    assert_eq!(trace.take(), ["listener", "listener"]);
    drop(duplicate);
    assert!(notifier.notify_listeners_collecting_panics().is_empty());
    assert_eq!(trace.take(), ["listener"]);
    drop(one);
    assert!(notifier.notify_listeners_collecting_panics().is_empty());
    assert!(trace.take().is_empty());

    let one = register("listener");
    let two = register("listener1");
    let three = register("listener2");
    drop(two);
    let moved = register("listener1");
    let failing = notifier.add_listener({
        let trace = trace.clone();
        move || {
            trace.borrow_mut().push("badListener");
            panic_any(23_u8);
        }
    });
    let errors = notifier.notify_listeners_collecting_panics();
    assert_eq!(
        trace.take(),
        ["listener", "listener2", "listener1", "badListener"]
    );
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].downcast_ref::<u8>(), Some(&23));

    let still_armed = register("listener1");
    drop(one);
    drop(moved);
    drop(three);
    let last = register("listener2");
    let errors = notifier.notify_listeners_collecting_panics();
    assert_eq!(trace.take(), ["badListener", "listener1", "listener2"]);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].downcast_ref::<u8>(), Some(&23));
    drop((still_armed, last, failing));
    notifier.dispose();
}

#[test]
fn an_embedded_notifier_delivers_before_the_owner_finishes_its_operation() {
    #[derive(Default)]
    struct Owner {
        notifications: ChangeNotifier,
        result: Rc<Cell<bool>>,
    }

    impl Owner {
        fn run(&self) {
            self.notifications.notify_listeners();
            self.result.set(true);
        }
    }

    let owner = Owner::default();
    let notifications = Rc::new(Cell::new(0));
    let _listener = owner.notifications.add_listener({
        let result = owner.result.clone();
        let notifications = notifications.clone();
        move || {
            assert!(!result.get());
            notifications.set(notifications.get() + 1);
        }
    });
    assert!(!owner.result.get());
    assert_eq!(notifications.get(), 0);
    owner.run();
    assert!(owner.result.get());
    assert_eq!(notifications.get(), 1);
}
