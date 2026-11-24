//! Focus requests on mounted, hidden, and detached nodes; paired with the
//! pinned Flutter 3.50.0 hosted_focus_requests_test.dart probe.
use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Column, Focus, FocusNode, IndexedStack, SizedBox, Widget};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
}

fn change(runtime: &mut Runtime, widget: impl Into<Widget>) {
    runtime
        .schedule_update(runtime.tree().root().unwrap(), widget.into())
        .unwrap();
    frame(runtime);
}

fn focus(node: &FocusNode) -> Focus {
    Focus::new(SizedBox::shrink()).node(node.clone())
}

#[test]
fn refused_hidden_request_does_not_publish_focus_or_retry_after_selection() {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let _subscription = b.observe({
        let b = b.clone();
        let events = events.clone();
        let previous = Cell::new(b.has_primary_focus());
        move || {
            let focused = b.has_primary_focus();
            if previous.replace(focused) != focused {
                events.borrow_mut().push(focused);
            }
        }
    });
    let stack = |i| IndexedStack::new([focus(&a), focus(&b)]).index(i);
    let mut runtime = Runtime::new(stack(0).into()).unwrap();
    a.request_focus();
    assert!(!a.has_primary_focus(), "a request is not yet a grant");
    frame(&mut runtime);
    assert!(a.has_primary_focus());

    b.request_focus();
    assert!(!b.has_primary_focus());
    frame(&mut runtime);
    assert!(a.has_primary_focus());
    assert!(!b.has_primary_focus());
    change(&mut runtime, stack(1));
    assert!(
        !b.has_primary_focus(),
        "a denied request cannot be replayed"
    );
    assert!(events.borrow().is_empty());

    b.request_focus();
    frame(&mut runtime);
    assert!(b.has_primary_focus());
    assert_eq!(events.borrow().as_slice(), [true]);
}

#[test]
fn detached_hosted_node_loses_focus_and_waits_to_reattach() {
    let node = FocusNode::new();
    let show = |visible| {
        let child: Widget = if visible {
            focus(&node).into()
        } else {
            SizedBox::shrink().into()
        };
        Column::new([child])
    };
    let mut runtime = Runtime::new(show(true).into()).unwrap();
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());

    change(&mut runtime, show(false));
    assert!(!node.has_primary_focus());
    node.request_focus();
    assert!(!node.has_primary_focus());
    frame(&mut runtime);
    assert!(!node.has_primary_focus());
    assert_eq!(runtime.focused_element(), None);

    change(&mut runtime, show(true));
    assert!(node.has_primary_focus());
}

#[test]
fn last_eligible_request_wins_even_when_skipped_by_traversal() {
    let a = FocusNode::new();
    let b = FocusNode::new();
    b.set_skip_traversal(true);
    let mut runtime = Runtime::new(Column::new([focus(&a), focus(&b)]).into()).unwrap();
    a.request_focus();
    b.request_focus();
    frame(&mut runtime);
    assert!(!a.has_primary_focus());
    assert!(b.has_primary_focus());

    frame(&mut runtime);
    assert!(!a.has_primary_focus(), "an older request must not replay");
    assert!(b.has_primary_focus());
}
