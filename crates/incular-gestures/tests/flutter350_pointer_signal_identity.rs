use std::{cell::RefCell, rc::Rc};

use incular_core::{Offset, Transform, time::Instant};
use incular_gestures::{
    PointerDeviceKind, PointerSignalError, PointerSignalEvent, PointerSignalResolver,
    RawPointerEvent,
};

fn raw() -> RawPointerEvent {
    RawPointerEvent::scroll(
        1,
        1,
        PointerDeviceKind::Mouse,
        Offset::ZERO,
        Offset::new(0., 12.),
        Instant::now(),
    )
}

#[test]
fn two_transformed_views_register_for_one_original_and_preserve_first_identity() {
    let original = PointerSignalEvent::new(raw());
    let first = original.transformed(Some(Transform::translation(Offset::new(10., 20.))));
    let second = original.transformed(Some(Transform::translation(Offset::new(30., 50.))));
    assert!(original.original().is_none());
    assert!(!first.same_instance(&original) && !second.same_instance(&original));
    assert!(first.original().unwrap().same_instance(&original));
    assert!(second.original().unwrap().same_instance(&original));
    assert_eq!(first.raw().local_position, Offset::new(10., 20.));
    assert_eq!(second.raw().local_position, Offset::new(30., 50.));
    assert_eq!(first.raw().position, Offset::ZERO);
    assert_eq!(first.raw().scroll_delta, Offset::new(0., 12.));

    let events = Rc::new(RefCell::new(Vec::new()));
    let winners = events.clone();
    let mut resolver = PointerSignalResolver::new();
    assert_eq!(
        resolver.register(&first, move |event| winners.borrow_mut().push(event)),
        Ok(true)
    );
    assert_eq!(
        resolver.register(&second, |_| panic!("the second view cannot win")),
        Ok(false)
    );
    assert_eq!(resolver.resolve(&original), Ok(true));
    let events = events.borrow();
    let [winner] = events.as_slice() else {
        panic!("expected exactly one winner")
    };
    assert!(winner.same_instance(&first));
    assert!(!winner.same_instance(&second));
}

#[test]
fn equal_raw_content_is_not_identity_and_a_resolver_can_be_reused() {
    let sample = raw();
    let first = PointerSignalEvent::new(sample);
    let second = PointerSignalEvent::new(sample);
    assert_eq!(first.raw(), second.raw());
    let mut resolver = PointerSignalResolver::new();
    assert_eq!(
        resolver.register(&first, |_| panic!("intentional listener failure")),
        Ok(true)
    );
    assert_eq!(
        resolver.register(&second, |_| {}),
        Err(PointerSignalError::DifferentSignal)
    );
    assert_eq!(
        resolver.resolve(&second),
        Err(PointerSignalError::DifferentSignal)
    );
    assert_eq!(
        resolver.resolve(&first),
        Err(PointerSignalError::CallbackPanicked)
    );
    assert_eq!(resolver.register(&second, |_| {}), Ok(true));
    assert_eq!(resolver.resolve(&second), Ok(true));
}

#[test]
fn unclaimed_resolution_allows_the_original_native_default() {
    let responses = Rc::new(RefCell::new(Vec::new()));
    let observer = responses.clone();
    let first =
        PointerSignalEvent::with_response(raw(), move |allow| observer.borrow_mut().push(allow));
    let transformed = first.transformed(Some(Transform::translation(Offset::new(5., 9.))));
    let mut resolver = PointerSignalResolver::new();
    assert_eq!(resolver.resolve(&transformed), Ok(false));
    assert_eq!(*responses.borrow(), [true]);
    assert_eq!(resolver.register(&transformed, |_| {}), Ok(true));
    assert_eq!(resolver.resolve(&first), Ok(true));
    assert_eq!(*responses.borrow(), [true]);
}
