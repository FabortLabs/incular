//! Flutter 3.50.0 foundation/change_notifier_test.dart callback parity.
//! Incular keeps each listener registered through an owned subscription token.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_core::reactivity::{DependencySource, Subscription};

type Log = Rc<RefCell<Vec<&'static str>>>;

fn append(log: &Log, event: &'static str) -> impl Fn() + Clone + 'static {
    let log = log.clone();
    move || log.borrow_mut().push(event)
}

// Upstream :207.
#[test]
fn adding_then_removing_a_listener_during_notification_does_not_run_it() {
    let source = DependencySource::default();
    let log = Log::default();
    let added = Rc::new(RefCell::new(None));
    let _first = source.subscribe((), {
        let source = source.clone();
        let log = log.clone();
        let added = added.clone();
        move || {
            log.borrow_mut().push("listener1");
            let second = source.subscribe((), append(&log, "listener2"));
            drop(second);
            *added.borrow_mut() = Some(source.subscribe((), append(&log, "listener3")));
        }
    });

    source.notify();

    assert_eq!(log.borrow().as_slice(), ["listener1"]);
}

fn subscribe_self_removing(
    source: &DependencySource,
    log: &Log,
) -> Rc<RefCell<Option<Subscription>>> {
    let owner = Rc::new(RefCell::new(None));
    let weak_owner = Rc::downgrade(&owner);
    let record = append(log, "selfRemovingListener");
    *owner.borrow_mut() = Some(source.subscribe((), move || {
        record();
        weak_owner.upgrade().unwrap().borrow_mut().take();
    }));
    owner
}

// Upstream :233.
#[test]
fn middle_listener_removing_itself_does_not_skip_either_duplicate_subscription() {
    let source = DependencySource::default();
    let log = Log::default();
    let listener = append(&log, "listener1");
    let _first = source.subscribe((), listener.clone());
    let _middle = subscribe_self_removing(&source, &log);
    let _last = source.subscribe((), listener);

    source.notify();

    assert_eq!(
        log.borrow().as_slice(),
        ["listener1", "selfRemovingListener", "listener1"]
    );
}

// Upstream :256.
#[test]
fn first_listener_removing_itself_does_not_skip_the_next_listener() {
    let source = DependencySource::default();
    let log = Log::default();
    let _first = subscribe_self_removing(&source, &log);
    let _second = source.subscribe((), append(&log, "listener1"));

    source.notify();

    assert_eq!(
        log.borrow().as_slice(),
        ["selfRemovingListener", "listener1"]
    );
}

// The upstream :42 Counter provides the equality guard; the behavior under
// test is the source delivering nested notifications synchronously.
#[derive(Default)]
struct Counter {
    value: Cell<i32>,
    source: DependencySource,
}

impl Counter {
    fn set(&self, value: i32) {
        if self.value.replace(value) != value {
            self.source.notify();
        }
    }
}

// Upstream :539.
#[test]
fn notification_can_reenter_synchronously_when_a_listener_corrects_the_counter() {
    let counter = Rc::new(Counter::default());
    let log = Log::default();
    let _listener = counter.source.subscribe((), {
        let counter = counter.clone();
        let record = append(&log, "listener1");
        move || {
            record();
            if counter.value.get() < 0 {
                counter.set(0);
            }
        }
    });

    counter.source.notify();
    assert_eq!(log.take(), ["listener1"]);
    counter.set(3);
    assert_eq!(log.take(), ["listener1"]);
    counter.set(-2);
    assert_eq!(log.take(), ["listener1", "listener1"]);
}

// Upstream :564. The externally observable ordering and removal are tested;
// the Rust subscription registry does not use Dart's backing-array capacity.
#[test]
fn removing_many_listeners_mid_notification_keeps_surviving_order() {
    let source = DependencySource::default();
    let log = Rc::new(RefCell::new(Vec::new()));
    let listeners = Rc::new(RefCell::new(Vec::<Option<Subscription>>::new()));
    let cleanup = Rc::new(RefCell::new(None));
    {
        let cleanup_owner = Rc::downgrade(&cleanup);
        let listeners = listeners.clone();
        *cleanup.borrow_mut() = Some(source.subscribe((), move || {
            let mut listeners = listeners.borrow_mut();
            for index in [1, 3, 4] {
                listeners[index].take();
            }
            cleanup_owner.upgrade().unwrap().borrow_mut().take();
        }));
    }
    for index in 0..12 {
        let log = log.clone();
        let subscription = source.subscribe((), move || log.borrow_mut().push(index));
        listeners.borrow_mut().push(Some(subscription));
    }
    let remaining = [0, 2, 5, 6, 7, 8, 9, 10, 11];

    source.notify();
    assert_eq!(log.take(), remaining);
    source.notify();
    assert_eq!(log.take(), remaining);
    for index in remaining {
        listeners.borrow_mut()[index].take();
    }
    source.notify();
    assert!(log.borrow().is_empty());
}
