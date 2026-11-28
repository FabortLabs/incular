#![cfg(debug_assertions)]

use std::{cell::RefCell, rc::Rc};

use incular_core::memory::allocations::{
    AllocationSubscription, MemoryAllocations, ObjectEvent, ObjectEventKind,
};
use incular_gestures::{FocusManager, FocusNode, FocusScopeNode, FocusTraversalPolicyKind};

fn observe() -> (AllocationSubscription, Rc<RefCell<Vec<ObjectEvent>>>) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let capture = events.clone();
    let subscription = MemoryAllocations::current().add_listener(move |event| {
        if event.library == "widgets" {
            capture.borrow_mut().push(event);
        }
    });
    (subscription, events)
}

fn matched(events: &[ObjectEvent], class: &str) {
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].class_name, class);
    assert_eq!(events[1].class_name, class);
    assert_eq!(events[0].kind, ObjectEventKind::Created);
    assert_eq!(events[1].kind, ObjectEventKind::Disposed);
    assert_eq!(events[0].id, events[1].id);
}

#[test]
fn focus_manager_notifies_on_construction_and_matching_disposal() {
    let (_subscription, events) = observe();
    let manager = FocusManager::new();
    assert_eq!(events.borrow()[0].kind, ObjectEventKind::Created);
    assert_eq!(events.borrow()[0].class_name, "FocusManager");
    drop(manager);
    matched(&events.borrow(), "FocusManager");

    let alternate = FocusManager::with_policy(FocusTraversalPolicyKind::ReadingOrder);
    assert_eq!(alternate.policy(), FocusTraversalPolicyKind::ReadingOrder);
    drop(alternate);
    let observed = events.borrow();
    matched(&observed[2..], "FocusManager");
    assert_ne!(observed[0].id, observed[2].id);
}

#[test]
fn focus_node_notifies_on_construction_and_disposes_only_on_final_drop() {
    let (_subscription, events) = observe();
    let node = FocusNode::new();
    assert_eq!(events.borrow()[0].class_name, "FocusNode");
    assert_eq!(events.borrow()[0].kind, ObjectEventKind::Created);
    let alias = node.clone();
    drop(node);
    assert_eq!(events.borrow().len(), 1);
    drop(alias);
    matched(&events.borrow(), "FocusNode");
}

#[test]
fn scope_storage_does_not_publish_a_second_manager() {
    let (_subscription, events) = observe();
    let scope = FocusScopeNode::new();
    let _ = scope.scope_focus_node();
    assert_eq!(events.borrow().len(), 1);
    drop(scope);
    matched(&events.borrow(), "FocusScopeNode");
}
