use std::{cell::RefCell, rc::Rc};

use incular_core::{MergedListenable, reactivity::DependencySource};

fn recording(log: &Rc<RefCell<Vec<u8>>>, value: u8) -> impl Fn() + 'static {
    let log = log.clone();
    move || log.borrow_mut().push(value)
}

#[test]
fn every_selected_source_notifies_in_listener_order() {
    let a = DependencySource::default();
    let b = DependencySource::default();
    let unrelated = DependencySource::default();
    let merged = MergedListenable::from_sources([a.clone(), b.clone()]);
    let log = Rc::new(RefCell::new(Vec::new()));
    let emit = || {
        a.notify();
        b.notify();
        unrelated.notify();
    };

    let first = merged.add_listener(recording(&log, 1));
    emit();
    assert_eq!(log.take(), [1, 1]);
    drop(first);
    emit();
    assert!(log.take().is_empty());
    let _first = merged.add_listener(recording(&log, 1));
    let _second = merged.add_listener(recording(&log, 2));
    emit();
    assert_eq!(log.take(), [1, 2, 1, 2]);
}

#[test]
fn accepts_non_array_iterables() {
    let a = DependencySource::default();
    let b = DependencySource::default();
    let sources = std::collections::VecDeque::from([a.clone(), b.clone()]);
    let merged = MergedListenable::from_sources(sources);
    let log = Rc::new(RefCell::new(Vec::new()));
    let _listener = merged.add_listener(recording(&log, 1));
    a.notify();
    b.notify();
    assert_eq!(log.take(), [1, 1]);
}

#[test]
fn ignores_absent_sources() {
    let a = DependencySource::default();
    let b = DependencySource::default();
    let merged = MergedListenable::new([None, Some(a.clone()), None, Some(b.clone()), None]);
    let log = Rc::new(RefCell::new(Vec::new()));
    let _listener = merged.add_listener(recording(&log, 1));
    a.notify();
    b.notify();
    assert_eq!(log.take(), [1, 1]);
}

#[test]
fn dropping_subscription_removes_it_from_every_source() {
    let a = DependencySource::default();
    let b = DependencySource::default();
    let merged = MergedListenable::from_sources([a.clone(), b.clone()]);
    let log = Rc::new(RefCell::new(Vec::new()));
    let listener = merged.add_listener(recording(&log, 1));
    a.notify();
    b.notify();
    assert_eq!(log.take(), [1, 1]);
    drop(listener);
    a.notify();
    b.notify();
    assert!(log.take().is_empty());
}

#[test]
fn merge_never_holds_eager_or_stale_registrations() {
    let a = DependencySource::default();
    let b = DependencySource::default();
    let merged = MergedListenable::from_sources([a.clone(), b.clone()]);
    assert_eq!((a.subscriber_count(), b.subscriber_count()), (0, 0));
    let listener = merged.add_listener(|| {});
    assert_eq!((a.subscriber_count(), b.subscriber_count()), (1, 1));
    drop(listener);
    assert_eq!((a.subscriber_count(), b.subscriber_count()), (0, 0));
}

#[test]
fn repeated_sources_register_each_occurrence() {
    let source = DependencySource::default();
    let merged = MergedListenable::from_sources([source.clone(), source.clone()]);
    let log = Rc::new(RefCell::new(Vec::new()));
    let listener = merged.add_listener(recording(&log, 1));
    assert_eq!(source.subscriber_count(), 2);
    source.notify();
    assert_eq!(log.take(), [1, 1]);
    drop(listener);
    assert_eq!(source.subscriber_count(), 0);
}

#[test]
fn formatting_preserves_empty_absent_and_duplicate_source_slots() {
    let a = DependencySource::default();
    let b = DependencySource::default();
    for (sources, expected) in [
        (vec![], "Listenable.merge([])"),
        (vec![None], "Listenable.merge([null])"),
        (
            vec![Some(a.clone())],
            "Listenable.merge([DependencySource])",
        ),
        (
            vec![Some(a.clone()), Some(b.clone())],
            "Listenable.merge([DependencySource, DependencySource])",
        ),
        (
            vec![None, Some(b), Some(a.clone()), Some(a)],
            "Listenable.merge([null, DependencySource, DependencySource, DependencySource])",
        ),
    ] {
        let merged = MergedListenable::new(sources);
        assert_eq!(merged.to_string(), expected);
        assert_eq!(format!("{merged:?}"), expected);
    }
}
