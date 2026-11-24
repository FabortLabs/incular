//! Mounted focus policy counterparts from Flutter 3.50.0 focus_scope_test.
//! FocusNode observers report state/property changes, not the Flutter
//! Focus widget's onFocusChange hook; only comparable state is asserted.

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Column, ExcludeFocus, Focus, FocusNode, SizedBox, Widget, internal::Key};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .expect("lay out focus tree");
}

#[test]
fn retained_focus_is_lost_when_can_request_focus_is_disabled() {
    let node = FocusNode::new();
    let observed = node.clone();
    let previous = Cell::new(false);
    let transitions = Rc::new(RefCell::new(Vec::new()));
    let changes = Rc::clone(&transitions);
    let _subscription = node.observe(move || {
        let now = observed.has_focus();
        if previous.replace(now) != now {
            changes.borrow_mut().push(now);
        }
    });
    let build = |can_request| {
        Focus::new(SizedBox::shrink())
            .node(node.clone())
            .autofocus(can_request)
            .can_request_focus(can_request)
            .into()
    };
    let mut runtime = Runtime::new(build(true)).expect("mount focusable node");
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    let root = runtime.tree().root().expect("mounted focus");
    assert_eq!(runtime.focused_element(), Some(root));
    assert!(node.has_focus());
    assert_eq!(*transitions.borrow(), [true]);

    runtime
        .tree_mut()
        .update(root, build(false))
        .expect("disable the same retained focus element");
    node.request_focus();
    frame(&mut runtime);
    assert!(runtime.tree().element_exists(root));
    assert_eq!(runtime.focused_element(), None);
    assert!(!node.has_focus());
    assert_eq!(*transitions.borrow(), [true, false]);
}

#[test]
fn unfocusable_parent_has_focus_within_when_its_child_has_primary_focus() {
    let parent = FocusNode::new();
    let child = FocusNode::new();
    let key = Key::from("focusable child");
    let child_widget =
        Widget::from(Focus::new(SizedBox::shrink()).node(child.clone())).with_key(key.clone());
    let root = Focus::new(child_widget)
        .node(parent.clone())
        .can_request_focus(false);
    let mut runtime = Runtime::new(root.into()).expect("mount nested focus");
    frame(&mut runtime);
    let target = runtime
        .tree()
        .element_with_key(&key)
        .expect("mounted child");

    parent.request_focus();
    frame(&mut runtime);
    assert!(!parent.has_focus());
    assert!(!child.has_focus());
    assert_eq!(runtime.focused_element(), None);

    child.request_focus();
    frame(&mut runtime);
    assert_eq!(runtime.focused_element(), Some(target));
    assert!(child.has_primary_focus());
    assert!(!parent.has_primary_focus());
    assert!(
        parent.has_focus(),
        "being unable to take primary focus does not suppress descendant focus membership"
    );
}

#[test]
fn descendants_are_focusable_blocks_both_nodes_and_runtime_target() {
    let boundary = FocusNode::new();
    boundary.set_descendants_are_focusable(false);
    let middle = FocusNode::new();
    let leaf = FocusNode::new();
    let root = Focus::new(
        Focus::new(Focus::new(SizedBox::shrink()).node(leaf.clone())).node(middle.clone()),
    )
    .node(boundary);
    let mut runtime = Runtime::new(root.into()).expect("mount focus boundary");
    frame(&mut runtime);
    assert!(!middle.has_focus());
    assert!(!leaf.has_focus());

    let states = [&middle, &leaf].map(|node| {
        node.request_focus();
        frame(&mut runtime);
        (
            middle.has_focus(),
            leaf.has_focus(),
            runtime.focused_element().is_some(),
        )
    });
    assert_eq!(states, [(false, false, false), (false, false, false)]);
}

#[test]
fn excluding_focus_clears_focused_descendant_without_refocusing_its_sibling() {
    let parent = FocusNode::new();
    let first = FocusNode::new();
    let second = FocusNode::new();
    let subtree = || {
        let column = Column::new([
            Focus::new(SizedBox::shrink())
                .node(first.clone())
                .autofocus(true),
            Focus::new(SizedBox::shrink()).node(second.clone()),
        ]);
        Focus::new(column).node(parent.clone())
    };
    let build = |excluding| ExcludeFocus::new(subtree()).excluding(excluding).into();
    let mut runtime = Runtime::new(build(false)).expect("mount focus group");
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    assert!(!second.has_focus());
    second.request_focus();
    frame(&mut runtime);
    assert!(!first.has_focus());
    assert!(second.has_primary_focus());

    let root = runtime.tree().root().expect("mounted subtree");
    runtime
        .tree_mut()
        .update(root, build(true))
        .expect("exclude the same retained subtree");
    frame(&mut runtime);
    assert!(runtime.tree().element_exists(root));
    assert!(!first.has_focus());
    assert!(!second.has_focus());
    assert!(!parent.has_focus());
    // Incular's mounted FocusScope is a traversal boundary rather than
    // a separately focusable node; only absence of a retained target maps.
    assert_eq!(runtime.focused_element(), None);
}
