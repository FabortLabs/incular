#![cfg(debug_assertions)]

use std::{cell::RefCell, rc::Rc};

use incular_core::memory::allocations::{MemoryAllocations, ObjectEventKind};
use incular_scroll::ScrollController;

#[test]
fn the_controller_reports_one_creation_and_one_final_disposal_for_the_shared_state() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let _observer = MemoryAllocations::current().add_listener({
        let events = events.clone();
        move |event| {
            if event.class_name == "ScrollController" {
                events.borrow_mut().push(event);
            }
        }
    });
    let first = ScrollController::new();
    let handle = first.clone();
    assert_eq!(events.borrow().len(), 1);
    assert_eq!(events.borrow()[0].kind, ObjectEventKind::Created);
    drop(first);
    assert_eq!(events.borrow().len(), 1);
    drop(handle);

    let second = ScrollController::new();
    drop(second);
    let events = events.borrow();
    assert_eq!(events.len(), 4);
    assert_eq!(events[1].kind, ObjectEventKind::Disposed);
    assert_eq!(events[0].id, events[1].id);
    assert_eq!(events[2].kind, ObjectEventKind::Created);
    assert_eq!(events[3].kind, ObjectEventKind::Disposed);
    assert_eq!(events[2].id, events[3].id);
    assert_ne!(events[0].id, events[2].id);
}
