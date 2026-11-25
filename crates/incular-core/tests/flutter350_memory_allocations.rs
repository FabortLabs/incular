#![cfg(debug_assertions)]

use std::{cell::RefCell, rc::Rc};

use incular_core::memory::allocations::{
    MEMORY_ALLOCATIONS_ENABLED, MemoryAllocations, ObjectEventKind, ObjectLifetime,
};

#[test]
fn owned_debug_lifetime_dispatches_one_create_and_dispose() {
    assert!(MEMORY_ALLOCATIONS_ENABLED);
    let registry = MemoryAllocations::current();
    let log = Rc::new(RefCell::new(Vec::new()));
    assert!(!registry.has_listeners());

    let subscription = registry.add_listener({
        let log = log.clone();
        move |event| log.borrow_mut().push(event)
    });
    assert!(registry.has_listeners());
    let object = ObjectLifetime::new("foundation", "Fixture");
    let id = object.id().unwrap();
    drop(object);
    let expected = [ObjectEventKind::Created, ObjectEventKind::Disposed];
    assert_eq!(
        log.borrow().iter().map(|e| e.kind).collect::<Vec<_>>(),
        expected
    );
    assert!(
        log.borrow()
            .iter()
            .all(|e| e.id == id && e.class_name == "Fixture")
    );
    drop(subscription);
    assert!(!registry.has_listeners());
    drop(ObjectLifetime::new("foundation", "Fixture"));
    assert_eq!(log.borrow().len(), 2);
}

#[test]
fn listeners_added_mid_dispatch_do_not_see_that_event() {
    let global = MemoryAllocations::current();
    let registry = MemoryAllocations::default();
    let event = Rc::new(RefCell::new(None));
    let collector = global.add_listener({
        let event = event.clone();
        move |item| *event.borrow_mut() = Some(item)
    });
    let _object = ObjectLifetime::new("foundation", "Fixture");
    let event = event.borrow().unwrap();
    drop(collector);

    let order = Rc::new(RefCell::new(Vec::new()));
    let late = Rc::new(RefCell::new(Vec::new()));
    let _first = registry.add_listener({
        let order = order.clone();
        let late = late.clone();
        let registry = registry.clone();
        move |_| {
            order.borrow_mut().push("first");
            let order = order.clone();
            late.borrow_mut()
                .push(registry.add_listener(move |_| order.borrow_mut().push("late")));
        }
    });
    assert_eq!(registry.dispatch(event), 0);
    assert_eq!(*order.borrow(), ["first"]);
    order.borrow_mut().clear();
    assert_eq!(registry.dispatch(event), 0);
    assert_eq!(*order.borrow(), ["first", "late"]);
}

#[test]
fn removal_skips_later_listener_and_panics_do_not_truncate_delivery() {
    let registry = MemoryAllocations::current();
    let order = Rc::new(RefCell::new(Vec::new()));
    let removed = Rc::new(RefCell::new(None));
    let first = registry.add_listener({
        let order = order.clone();
        let removed = removed.clone();
        move |_| {
            order.borrow_mut().push("first");
            drop(removed.borrow_mut().take());
            panic!("listener one failed")
        }
    });
    *removed.borrow_mut() =
        Some(registry.add_listener(|_| panic!("removed listener must not run")));
    let second = registry.add_listener({
        let order = order.clone();
        move |_| order.borrow_mut().push("second")
    });
    let bad = registry.add_listener({
        let order = order.clone();
        move |_| {
            order.borrow_mut().push("bad");
            panic!("another listener failed");
        }
    });
    let third = registry.add_listener({
        let order = order.clone();
        move |_| order.borrow_mut().push("third")
    });
    let captured = Rc::new(RefCell::new(None));
    let tap = registry.add_listener({
        let captured = captured.clone();
        move |event| *captured.borrow_mut() = Some(event)
    });
    drop(ObjectLifetime::new("foundation", "Fixture"));
    assert_eq!(&order.borrow()[..4], ["first", "second", "bad", "third"]);
    order.borrow_mut().clear();
    let event = captured.borrow().unwrap();
    assert_eq!(registry.dispatch(event), 2);
    assert_eq!(*order.borrow(), ["first", "second", "bad", "third"]);
    drop((first, second, bad, third, tap));
    assert!(!registry.has_listeners());
}
