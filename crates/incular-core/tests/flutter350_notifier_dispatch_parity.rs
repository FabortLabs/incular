//! Additional synchronous dispatch comparisons with Flutter 3.50.0
//! `foundation/change_notifier_test.dart`.

use std::{
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

use incular_core::reactivity::DependencySource;

type Log = Rc<RefCell<Vec<&'static str>>>;

fn append(log: &Log, entry: &'static str) -> impl Fn() + Clone + 'static {
    let log = log.clone();
    move || log.borrow_mut().push(entry)
}

// Upstream :168. Dropping the Rust token corresponds to removeListener.
// Each registration of the fourth callback remains independently live.
#[test]
fn mutations_during_notification_do_not_skip_live_or_run_new_listeners_early() {
    let source = DependencySource::default();
    let log = Log::default();
    let first = Rc::new(RefCell::new(Some(
        source.subscribe((), append(&log, "listener1")),
    )));
    let third = Rc::new(RefCell::new(None));
    let fourth_tokens = Rc::new(RefCell::new(Vec::new()));

    let _second = source.subscribe((), {
        let source = source.clone();
        let record = append(&log, "listener2");
        let fourth = append(&log, "listener4");
        let third = third.clone();
        move || {
            record();
            drop(first.borrow_mut().take());
            drop(third.borrow_mut().take());
            fourth_tokens
                .borrow_mut()
                .push(source.subscribe((), fourth.clone()));
        }
    });
    *third.borrow_mut() = Some(source.subscribe((), append(&log, "listener3")));

    source.notify();
    assert_eq!(log.take(), ["listener1", "listener2"]);
    source.notify();
    assert_eq!(log.take(), ["listener2", "listener4"]);
    source.notify();
    assert_eq!(log.take(), ["listener2", "listener4", "listener4"]);
}

// Upstream :457. Only presence is compared; Flutter does not expose
// the exact listener count in this assertion.
#[test]
fn listener_presence_tracks_individual_registrations_including_duplicates() {
    let source = DependencySource::default();
    let has_listeners = || source.subscriber_count() != 0;
    let callback_1 = || {};
    let callback_2 = || {};

    assert!(!has_listeners());
    let first = source.subscribe((), callback_1);
    assert!(has_listeners());
    let duplicate = source.subscribe((), callback_1);
    assert!(has_listeners());
    drop(first);
    assert!(has_listeners());
    drop(duplicate);
    assert!(!has_listeners());

    let first = source.subscribe((), callback_1);
    assert!(has_listeners());
    let second = source.subscribe((), callback_2);
    assert!(has_listeners());
    drop(first);
    assert!(has_listeners());
    drop(second);
    assert!(!has_listeners());
}

// Callback dispatch subset of the upstream "ChangeNotifier" (:83)
// registration, assertions :150–164. Dart's exception reporter is not
// asserted here. The test boundary catches Rust's unwind so it can inspect
// which observers the source reached before the unwind.
#[test]
fn an_observer_failure_does_not_skip_later_observers() {
    let source = DependencySource::default();
    let log = Log::default();
    let ordinary = source.subscribe((), append(&log, "listener"));
    let second = source.subscribe((), append(&log, "listener2"));
    let listener1 = append(&log, "listener1");
    let first = source.subscribe((), listener1.clone());
    let _bad = source.subscribe((), {
        let record = append(&log, "badListener");
        move || {
            record();
            panic!("badListener failed");
        }
    });

    let _ = catch_unwind(AssertUnwindSafe(|| source.notify()));
    assert_eq!(
        log.take(),
        ["listener", "listener2", "listener1", "badListener"]
    );

    let _duplicate = source.subscribe((), listener1);
    drop(ordinary);
    drop(first);
    drop(second);
    let _second = source.subscribe((), append(&log, "listener2"));
    let _ = catch_unwind(AssertUnwindSafe(|| source.notify()));
    assert_eq!(log.take(), ["badListener", "listener1", "listener2"]);
}
