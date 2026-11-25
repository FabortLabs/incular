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
