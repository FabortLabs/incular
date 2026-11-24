//! Lifecycle regressions from Flutter 3.50's synchronous broadcast comparison.
//! See `audits/flutter-3.50.0-parity/stream-comparisons.md`.

use incular_widgets::{StreamEvent, UiStreamController};
use std::{cell::RefCell, rc::Rc};

type Event = StreamEvent<i32, ()>;

#[test]
fn ordinary_broadcast_delivers_data_then_done_to_both_listeners() {
    let controller = UiStreamController::<i32, ()>::new();
    let first = Rc::new(RefCell::new(Vec::new()));
    let second = Rc::new(RefCell::new(Vec::new()));
    let a = first.clone();
    let b = second.clone();
    let _first = controller.stream().listen(move |event| {
        a.borrow_mut().push(event.clone());
    });
    let _second = controller.stream().listen(move |event| {
        b.borrow_mut().push(event.clone());
    });

    controller.add(1);
    controller.add(2);
    controller.close();

    let expected = [Event::Data(1), Event::Data(2), Event::Done];
    assert_eq!(first.borrow().as_slice(), &expected);
    assert_eq!(second.borrow().as_slice(), &expected);
}

#[test]
fn cancellation_during_broadcast_skips_an_unreached_listener() {
    let controller = UiStreamController::<i32, ()>::new();
    let second = Rc::new(RefCell::new(None));
    let cancel = second.clone();
    let first_calls = Rc::new(RefCell::new(Vec::new()));
    let first_observed = first_calls.clone();
    let _first = controller.stream().listen(move |event| {
        first_observed.borrow_mut().push(event.clone());
        drop(cancel.borrow_mut().take());
    });
    let second_calls = Rc::new(RefCell::new(Vec::new()));
    let observed = second_calls.clone();
    *second.borrow_mut() = Some(controller.stream().listen(move |event| {
        observed.borrow_mut().push(event.clone());
    }));

    controller.add(1);

    assert_eq!(first_calls.borrow().as_slice(), &[Event::Data(1)]);
    assert!(
        second.borrow().is_none(),
        "the first callback dropped the subscription"
    );
    assert_eq!(second_calls.borrow().as_slice(), &[]);
}

#[test]
fn synchronous_broadcast_does_not_publish_a_reentrant_add() {
    let controller = Rc::new(UiStreamController::<i32, ()>::new());
    let sender = controller.clone();
    let _first = controller.stream().listen(move |event| {
        if matches!(event, Event::Data(1)) {
            sender.add(2);
        }
    });
    let second = Rc::new(RefCell::new(Vec::new()));
    let observed = second.clone();
    let _second = controller.stream().listen(move |event| {
        observed.borrow_mut().push(event.clone());
    });

    controller.add(1);

    assert_eq!(second.borrow().as_slice(), &[Event::Data(1)]);
}

#[test]
fn done_is_terminal_across_broadcast_listeners() {
    let controller = Rc::new(UiStreamController::<i32, ()>::new());
    let sender = controller.clone();
    let first = Rc::new(RefCell::new(Vec::new()));
    let second = Rc::new(RefCell::new(Vec::new()));
    let a = first.clone();
    let b = second.clone();
    let _first = controller.stream().listen(move |event| {
        a.borrow_mut().push(event.clone());
        if matches!(event, Event::Done) {
            sender.add(2);
        }
    });
    let _second = controller.stream().listen(move |event| {
        b.borrow_mut().push(event.clone());
    });

    controller.close();

    assert!(controller.stream().is_closed());
    assert_eq!(
        (first.borrow().as_slice(), second.borrow().as_slice()),
        (&[Event::Done][..], &[Event::Done][..]),
    );
}
