use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_animation::{
    AnimationStatus, AnimationStatusListener, AnimationValueListener, LazyAnimationListeners,
};

fn lazy() -> (Rc<LazyAnimationListeners>, Rc<Cell<usize>>, Rc<Cell<usize>>) {
    let starts = Rc::new(Cell::new(0));
    let stops = Rc::new(Cell::new(0));
    let listeners = Rc::new(LazyAnimationListeners::new(
        {
            let starts = starts.clone();
            move || starts.set(starts.get() + 1)
        },
        {
            let stops = stops.clone();
            move || stops.set(stops.get() + 1)
        },
    ));
    (listeners, starts, stops)
}

#[test]
fn removing_unregistered_status_listener_does_not_start_or_stop_parent() {
    let (listeners, starts, stops) = lazy();
    let fake: AnimationStatusListener = Rc::new(|_| {});
    assert!(!listeners.remove_status_listener(&fake));
    assert_eq!(starts.get(), 0);
    assert_eq!(stops.get(), 0);
    assert!(!listeners.is_listening());
}

#[test]
fn removing_unregistered_value_listener_does_not_start_or_stop_parent() {
    let (listeners, starts, stops) = lazy();
    let fake: AnimationValueListener = Rc::new(|| {});
    assert!(!listeners.remove_listener(&fake));
    assert_eq!(starts.get(), 0);
    assert_eq!(stops.get(), 0);
    assert!(!listeners.is_listening());
}

#[test]
fn duplicates_share_one_parent_lifecycle_and_each_list_uses_its_own_delivery_policy() {
    let (listeners, starts, stops) = lazy();
    let values = Rc::new(Cell::new(0));
    let statuses = Rc::new(RefCell::new(Vec::new()));
    let value: AnimationValueListener = Rc::new({
        let values = values.clone();
        move || values.set(values.get() + 1)
    });
    let status: AnimationStatusListener = Rc::new({
        let statuses = statuses.clone();
        move |s| statuses.borrow_mut().push(s)
    });
    let fake_value: AnimationValueListener = Rc::new(|| {});
    let fake_status: AnimationStatusListener = Rc::new(|_| {});

    listeners.add_listener(value.clone());
    listeners.add_listener(value.clone());
    listeners.add_status_listener(status.clone());
    listeners.add_status_listener(status.clone());
    assert_eq!((starts.get(), stops.get()), (1, 0));
    assert!(listeners.is_listening());
    assert!(!listeners.remove_status_listener(&fake_status));
    assert!(!listeners.remove_listener(&fake_value));
    assert_eq!((starts.get(), stops.get()), (1, 0));

    assert!(listeners.notify_listeners().is_empty());
    assert!(
        listeners
            .notify_status_listeners(AnimationStatus::Forward)
            .is_empty()
    );
    assert_eq!(values.get(), 1);
    assert_eq!(
        statuses.take(),
        [AnimationStatus::Forward, AnimationStatus::Forward]
    );

    assert!(listeners.remove_listener(&value));
    assert!(listeners.remove_status_listener(&status));
    assert!(listeners.notify_listeners().is_empty());
    assert!(
        listeners
            .notify_status_listeners(AnimationStatus::Completed)
            .is_empty()
    );
    assert_eq!(values.get(), 2);
    assert_eq!(statuses.take(), [AnimationStatus::Completed]);

    assert!(listeners.remove_listener(&value));
    assert_eq!(stops.get(), 0);
    assert!(listeners.is_listening());
    assert!(listeners.remove_status_listener(&status));
    assert!(!listeners.is_listening());
    assert!(!listeners.remove_status_listener(&status));
    assert_eq!((starts.get(), stops.get()), (1, 1));

    listeners.add_status_listener(status.clone());
    assert_eq!((starts.get(), stops.get()), (2, 1));
    assert!(listeners.remove_status_listener(&status));
    assert_eq!((starts.get(), stops.get()), (2, 2));
}

#[test]
fn removal_during_notification_skips_the_removed_listener_and_defers_additions() {
    let (listeners, _, stops) = lazy();
    let events = Rc::new(RefCell::new(Vec::new()));
    let later = |tag| -> AnimationValueListener {
        let events = events.clone();
        Rc::new(move || events.borrow_mut().push(tag))
    };
    let old = later("old");
    let new = later("new");
    listeners.add_listener(Rc::new({
        let events = events.clone();
        let old = old.clone();
        let new = new.clone();
        let listeners = Rc::downgrade(&listeners);
        move || {
            events.borrow_mut().push("first");
            let listeners = listeners.upgrade().unwrap();
            if listeners.remove_listener(&old) {
                listeners.add_listener(new.clone());
            }
        }
    }));
    listeners.add_listener(old);
    assert!(listeners.notify_listeners().is_empty());
    assert_eq!(events.take(), ["first"]);
    assert!(listeners.notify_listeners().is_empty());
    assert_eq!(events.take(), ["first", "new"]);
    assert_eq!(stops.get(), 0);
}

#[cfg(panic = "unwind")]
#[test]
fn a_failed_callback_does_not_abort_the_snapshot() {
    let (listeners, _, _) = lazy();
    let count = Rc::new(Cell::new(0));
    listeners.add_status_listener(Rc::new(|_| std::panic::panic_any(7_u8)));
    listeners.add_status_listener(Rc::new({
        let count = count.clone();
        move |status| {
            assert_eq!(status, AnimationStatus::Reverse);
            count.set(count.get() + 1);
        }
    }));
    let failures = listeners.notify_status_listeners(AnimationStatus::Reverse);
    assert_eq!(count.get(), 1);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].downcast_ref::<u8>(), Some(&7));
}
