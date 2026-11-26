use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Column, Focus, FocusNode, FocusScope, SizedBox, Widget, internal::Key};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(200., 100.)))
        .unwrap();
}

fn focus(node: &FocusNode, autofocus: bool) -> Focus {
    Focus::new(SizedBox::square(20.))
        .node(node.clone())
        .autofocus(autofocus)
}

fn repump(runtime: &mut Runtime, widget: impl Into<Widget>) {
    runtime
        .schedule_update(runtime.tree().root().unwrap(), widget.into())
        .unwrap();
    frame(runtime);
}

#[test]
fn updating_an_existing_nodes_autofocus_can_request_its_first_focus() {
    let node = FocusNode::new();
    let mut runtime = Runtime::new(focus(&node, false).into()).unwrap();
    frame(&mut runtime);
    assert!(!node.has_primary_focus());
    repump(&mut runtime, focus(&node, true));
    assert!(node.has_primary_focus());
}

#[test]
fn retained_focus_does_not_retry_autofocus_after_it_was_used() {
    let node = FocusNode::new();
    let mut runtime = Runtime::new(focus(&node, false).into()).unwrap();
    frame(&mut runtime);
    repump(&mut runtime, focus(&node, true));
    assert!(node.has_primary_focus());
    repump(&mut runtime, focus(&node, false));
    node.unfocus();
    frame(&mut runtime);
    assert!(!node.has_primary_focus());
    repump(&mut runtime, focus(&node, true));
    assert!(!node.has_primary_focus());
}

#[test]
fn retained_autofocus_in_a_scope_without_a_focused_child_is_eligible() {
    let outside = FocusNode::new();
    let inside = FocusNode::new();
    let make_view = |autofocus| {
        Column::new([
            Widget::from(focus(&outside, true)),
            Widget::from(FocusScope::new(focus(&inside, autofocus))),
        ])
    };
    let mut runtime = Runtime::new(make_view(false).into()).unwrap();
    frame(&mut runtime);
    assert!(outside.has_primary_focus());
    repump(&mut runtime, make_view(true));
    assert!(inside.has_primary_focus());
    assert!(!outside.has_primary_focus());
}

#[test]
fn a_new_autofocus_node_does_not_steal_an_existing_selection() {
    let first = FocusNode::new();
    let second = FocusNode::new();
    let mut runtime = Runtime::new(Column::new([focus(&first, true)]).into()).unwrap();
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    repump(
        &mut runtime,
        Column::new([focus(&first, false), focus(&second, true)]),
    );
    assert!(first.has_primary_focus());
    assert!(!second.has_primary_focus());
}

#[test]
fn a_mounted_root_scope_can_receive_focus_itself() {
    let key = Key::Value(30);
    let view = FocusScope::new(Focus::new(
        Widget::from(SizedBox::square(20.)).with_key(key.clone()),
    ));
    let mut runtime = Runtime::new(view.into()).unwrap();
    frame(&mut runtime);
    let element = runtime.tree().element_with_key(&key).unwrap();
    let root_scope = FocusScope::maybe_of(runtime.tree(), element).unwrap();
    assert!(root_scope.request_focus_in_scope());
    frame(&mut runtime);
    assert!(root_scope.scope_focus_node().has_focus());
}

#[test]
fn requesting_focus_on_a_mounted_node_notifies_its_wrapper() {
    let key = Key::Value(31);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let seen = changes.clone();
    let wrapper = Focus::new(Widget::from(SizedBox::square(20.)).with_key(key.clone()))
        .on_focus_change(move |value| seen.borrow_mut().push(value));
    let mut runtime = Runtime::new(wrapper.into()).unwrap();
    frame(&mut runtime);
    let element = runtime.tree().element_with_key(&key).unwrap();
    let node = Focus::of(runtime.tree(), element, false);
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_focus());
    assert_eq!(*changes.borrow(), [true]);
}

#[test]
fn focus_lookup_excludes_the_starting_widget_and_stops_at_a_scope() {
    let scope_key = Key::Value(40);
    let wrapper_key = Key::Value(41);
    let leaf_key = Key::Value(42);
    let outer = FocusNode::new();
    let inner = FocusNode::new();
    let deep = FocusNode::new();
    let leaf = Widget::from(SizedBox::square(20.)).with_key(leaf_key.clone());
    let deeper = Focus::new(leaf).node(deep.clone()).include_semantics(false);
    let wrapped =
        Widget::from(Focus::new(deeper).node(inner.clone())).with_key(wrapper_key.clone());
    let scoped = Widget::from(FocusScope::new(wrapped)).with_key(scope_key.clone());
    let mut runtime = Runtime::new(Focus::new(scoped).node(outer.clone()).into()).unwrap();
    frame(&mut runtime);
    let tree = runtime.tree();
    let scope = tree.element_with_key(&scope_key).unwrap();
    let wrapper = tree.element_with_key(&wrapper_key).unwrap();
    let leaf = tree.element_with_key(&leaf_key).unwrap();
    assert!(Focus::of(tree, scope, false) == outer);
    assert!(Focus::maybe_of(tree, wrapper, false).is_none());
    let scope_owner = FocusScope::maybe_of(tree, wrapper).unwrap();
    assert!(Focus::of(tree, wrapper, true) == scope_owner.scope_focus_node());
    assert!(Focus::of(tree, leaf, false) == deep);
}
