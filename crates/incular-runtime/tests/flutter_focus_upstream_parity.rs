//! Public mounted focus cases from Flutter 3.50.0. Incular binds external
//! nodes through Focus widgets, not Flutter's manual FocusAttachment API.

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Align, Alignment, Column, Focus, FocusNode, FocusScope, SizedBox, Widget, internal::Key,
};
use std::{cell::Cell, rc::Rc};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .expect("lay out focus tree");
}

fn scoped_slot(child: impl Into<Widget>) -> Widget {
    FocusScope::new(Align::new(Alignment::CENTER, child)).into()
}

#[test]
fn unattached_focus_request_waits_for_host_attachment() {
    let child = FocusNode::new();
    child.request_focus();
    let primary_before_scope = child.has_primary_focus();

    let mut runtime = Runtime::new(scoped_slot(SizedBox::shrink())).expect("mount empty scope");
    frame(&mut runtime);
    let primary_before_child_mount = child.has_primary_focus();
    assert_eq!(runtime.focused_element(), None);

    let root = runtime.tree().root().expect("stable scope slot");
    let key = Key::from("pending focus");
    let focus =
        Widget::from(Focus::new(SizedBox::shrink()).node(child.clone())).with_key(key.clone());
    runtime
        .tree_mut()
        .update(root, scoped_slot(focus))
        .expect("mount requested node");
    frame(&mut runtime);
    let target = runtime
        .tree()
        .element_with_key(&key)
        .expect("focus element mounted");

    assert_eq!(
        (
            primary_before_scope,
            primary_before_child_mount,
            child.has_primary_focus(),
            runtime.focused_element() == Some(target),
        ),
        (false, false, true, true),
        "only a mounted focus target can become primary"
    );
}

#[test]
fn replacement_autofocus_works_after_focused_child_is_detached() {
    let original = FocusNode::new();
    let replacement = FocusNode::new();
    let focus = |node: &FocusNode| {
        Focus::new(SizedBox::shrink())
            .node(node.clone())
            .autofocus(true)
    };
    let mut runtime = Runtime::new(scoped_slot(focus(&original))).expect("mount original");
    frame(&mut runtime);
    assert!(original.has_primary_focus());
    let old_target = runtime.focused_element().expect("initial autofocus");

    let root = runtime.tree().root().expect("stable scope slot");
    // Insert a sized wrapper so the previous Focus element really unmounts.
    let key = Key::from("replacement focus");
    let new_focus = Widget::from(focus(&replacement)).with_key(key.clone());
    runtime
        .tree_mut()
        .update(
            root,
            scoped_slot(SizedBox::new().width(100.).height(100.).child(new_focus)),
        )
        .expect("replace original");
    frame(&mut runtime);
    assert!(!runtime.tree().element_exists(old_target));
    let target = runtime
        .tree()
        .element_with_key(&key)
        .expect("replacement element mounted");

    assert_eq!(
        (
            replacement.has_primary_focus(),
            runtime.focused_element() == Some(target)
        ),
        (true, true),
        "newly mounted autofocus target takes focus after the old target is removed"
    );
}

#[test]
fn first_mounted_autofocus_candidate_wins() {
    let nodes: [FocusNode; 3] = std::array::from_fn(|_| FocusNode::new());
    let children = nodes.iter().map(|node| {
        Focus::new(SizedBox::shrink())
            .node(node.clone())
            .autofocus(true)
    });
    let mut runtime = Runtime::new(Column::new(children).into()).expect("mount three candidates");
    frame(&mut runtime);

    assert_eq!(
        nodes.each_ref().map(FocusNode::has_primary_focus),
        [true, false, false]
    );
}

#[test]
fn unfocusable_mounted_node_ignores_request_and_does_not_notify() {
    let node = FocusNode::new();
    let focus = Focus::new(SizedBox::shrink())
        .node(node.clone())
        .can_request_focus(false);
    let mut runtime = Runtime::new(focus.into()).expect("mount unfocusable node");
    frame(&mut runtime);
    assert!(!node.can_request_focus());
    let calls = Rc::new(Cell::new(0));
    let notified = Rc::clone(&calls);
    let _subscription = node.observe(move || notified.set(notified.get() + 1));

    node.request_focus();
    frame(&mut runtime);
    assert!(!node.has_focus());
    assert_eq!(runtime.focused_element(), None);
    assert_eq!(calls.get(), 0);
}
