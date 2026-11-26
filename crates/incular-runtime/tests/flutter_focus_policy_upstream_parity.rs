//! Mounted focus policy counterparts from Flutter 3.50.0 focus_scope_test.
//! Focus widget callbacks and attached nodes are driven through Runtime.

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, ExcludeFocus, Focus, FocusNode, FocusScope, FocusScopeNode, SizedBox, Widget,
    internal::Key,
};
use std::{cell::RefCell, rc::Rc};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .expect("lay out focus tree");
}

#[test]
fn retained_focus_is_lost_when_can_request_focus_is_disabled() {
    let transitions = Rc::new(RefCell::new(Vec::new()));
    let build = |can_request| {
        Focus::new(SizedBox::shrink())
            .can_request_focus(can_request)
            .autofocus(can_request)
            .on_focus_change({
                let transitions = transitions.clone();
                move |focused| transitions.borrow_mut().push(focused)
            })
            .into()
    };
    let mut runtime = Runtime::new(build(true)).expect("mount focusable node");
    frame(&mut runtime);
    let root = runtime.tree().root().expect("mounted focus");
    let node = runtime
        .tree()
        .focus_node_at(root)
        .expect("owned focus node");
    node.request_focus();
    frame(&mut runtime);
    assert_eq!(runtime.focused_element(), Some(root));
    assert!(node.has_focus());
    assert_eq!(*transitions.borrow(), [true]);

    runtime
        .tree_mut()
        .update(root, build(false))
        .expect("disable the same retained focus element");
    assert_eq!(runtime.tree().focus_node_at(root), Some(node.clone()));
    node.request_focus();
    frame(&mut runtime);
    assert!(runtime.tree().element_exists(root));
    assert_eq!(runtime.focused_element(), None);
    assert!(!node.has_focus());
    assert_eq!(*transitions.borrow(), [true, false]);
}

#[test]
fn unfocusable_parent_has_focus_within_when_its_child_has_primary_focus() {
    let child = FocusNode::new();
    let transitions = Rc::new(RefCell::new(Vec::new()));
    let key = Key::from("focusable child");
    let child_widget =
        Widget::from(Focus::new(SizedBox::shrink()).node(child.clone())).with_key(key.clone());
    let root = Focus::new(child_widget)
        .can_request_focus(false)
        .on_focus_change({
            let transitions = transitions.clone();
            move |focused| transitions.borrow_mut().push(focused)
        });
    let mut runtime = Runtime::new(root.into()).expect("mount nested focus");
    frame(&mut runtime);
    let parent = runtime
        .tree()
        .focus_node_at(runtime.tree().root().unwrap())
        .unwrap();
    let target = runtime
        .tree()
        .element_with_key(&key)
        .expect("mounted child");

    parent.request_focus();
    frame(&mut runtime);
    assert!(!parent.has_focus());
    assert!(!child.has_focus());
    assert_eq!(runtime.focused_element(), None);
    assert!(transitions.borrow().is_empty());

    child.request_focus();
    frame(&mut runtime);
    assert_eq!(runtime.focused_element(), Some(target));
    assert!(child.has_primary_focus());
    assert!(!parent.has_primary_focus());
    assert!(
        parent.has_focus(),
        "being unable to take primary focus does not suppress descendant focus membership"
    );
    assert_eq!(*transitions.borrow(), [true]);
}

#[test]
fn disabling_either_scope_blocks_nested_requests_where_disabling_an_ordinary_parent_does_not() {
    let outer = FocusScopeNode::new();
    let inner = FocusScopeNode::new();
    let parent = FocusNode::new();
    let child = FocusNode::new();
    let layout = |[allow_outer, allow_inner, allow_parent, allow_child]: [bool; 4]| {
        let leaf = Focus::new(SizedBox::shrink())
            .node(child.clone())
            .can_request_focus(allow_child);
        let column = Focus::new(leaf)
            .node(parent.clone())
            .can_request_focus(allow_parent);
        let inner_view = FocusScope::new(column)
            .node(inner.clone())
            .can_request_focus(allow_inner);
        FocusScope::new(inner_view)
            .node(outer.clone())
            .can_request_focus(allow_outer)
            .into()
    };
    let enabled = [true; 4];
    let mut runtime = Runtime::new(layout(enabled)).unwrap();
    frame(&mut runtime);
    let update = |runtime: &mut Runtime, flags| {
        runtime
            .schedule_update(runtime.tree().root().unwrap(), layout(flags))
            .unwrap();
        frame(runtime);
    };
    let request = |runtime: &mut Runtime, node: &FocusNode| {
        node.request_focus();
        frame(runtime);
    };

    request(&mut runtime, &child);
    assert!(child.has_primary_focus());
    update(&mut runtime, [true, true, true, false]);
    assert!(!child.has_focus());
    request(&mut runtime, &child);
    assert!(!child.has_focus());
    update(&mut runtime, enabled);
    request(&mut runtime, &child);
    assert!(child.has_primary_focus());

    update(&mut runtime, [true, true, false, true]);
    assert!(child.has_primary_focus());
    request(&mut runtime, &parent);
    assert!(child.has_primary_focus() && !parent.has_primary_focus());
    request(&mut runtime, &child);
    assert!(child.has_primary_focus());
    update(&mut runtime, enabled);
    child.unfocus();
    frame(&mut runtime);
    assert!(!child.has_focus());
    request(&mut runtime, &child);
    assert!(child.has_primary_focus());

    for (flags, denied_scope) in [
        ([true, false, true, true], &inner),
        ([false, true, true, true], &outer),
    ] {
        update(&mut runtime, flags);
        assert!(!child.has_focus());
        assert!(!denied_scope.request_focus_in_scope());
        frame(&mut runtime);
        if denied_scope == &outer {
            let _ = inner.request_focus_in_scope();
            frame(&mut runtime);
            assert!(!inner.has_focus());
        }
        request(&mut runtime, &parent);
        assert!(!parent.has_focus() && !child.has_focus());
        request(&mut runtime, &child);
        assert!(!parent.has_focus() && !child.has_focus());
        update(&mut runtime, enabled);
        request(&mut runtime, &child);
        assert!(child.has_primary_focus());
    }
}

#[test]
fn descendants_are_focusable_blocks_both_nodes_and_runtime_target() {
    let leaf = FocusNode::new();
    let transitions = Rc::new(RefCell::new(Vec::new()));
    let mid = Key::from("middle focus");
    let root = Focus::new(
        Widget::from(
            Focus::new(Focus::new(SizedBox::shrink()).node(leaf.clone())).on_focus_change({
                let transitions = transitions.clone();
                move |focused| transitions.borrow_mut().push(focused)
            }),
        )
        .with_key(mid.clone()),
    )
    .descendants_are_focusable(false);
    let mut runtime = Runtime::new(root.into()).expect("mount focus boundary");
    frame(&mut runtime);
    let middle = runtime
        .tree()
        .focus_node_at(runtime.tree().element_with_key(&mid).unwrap())
        .unwrap();
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
    assert!(transitions.borrow().is_empty());

    // The flag excludes descendants, not the Focus widget itself.
    let focusable = runtime.tree().focusable_elements();
    assert_eq!(focusable, [runtime.tree().root().unwrap()]);
}

#[test]
fn an_owned_focus_observer_does_not_flap_when_its_children_take_turns() {
    let first = FocusNode::new();
    let second = FocusNode::new();
    let outside = FocusNode::new();
    let transitions = Rc::new(RefCell::new(Vec::new()));
    let branch = Focus::new(Column::new([
        Focus::new(SizedBox::shrink()).node(first.clone()),
        Focus::new(SizedBox::shrink()).node(second.clone()),
    ]))
    .on_focus_change({
        let transitions = transitions.clone();
        move |focused| transitions.borrow_mut().push(focused)
    });
    let mut runtime = Runtime::new(
        Column::new([branch, Focus::new(SizedBox::shrink()).node(outside.clone())]).into(),
    )
    .unwrap();
    frame(&mut runtime);
    first.request_focus();
    frame(&mut runtime);
    second.request_focus();
    frame(&mut runtime);
    assert_eq!(*transitions.borrow(), [true]);
    outside.request_focus();
    frame(&mut runtime);
    assert_eq!(*transitions.borrow(), [true, false]);
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

#[test]
fn an_explicit_widget_permission_can_reenable_an_existing_external_node() {
    let node = FocusNode::new();
    node.set_can_request_focus(false);
    let focus = || Focus::new(SizedBox::shrink()).node(node.clone());
    let mut runtime = Runtime::new(focus().into()).unwrap();
    frame(&mut runtime);
    assert!(
        !node.can_request_focus(),
        "omitted override preserves node policy"
    );

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, focus().can_request_focus(true).into())
        .unwrap();
    frame(&mut runtime);
    assert!(node.can_request_focus());
    assert!(!node.has_primary_focus(), "reenabling is not autofocus");
    node.request_focus();
    frame(&mut runtime);
    assert!(node.has_primary_focus());
}

#[test]
fn an_implicitly_owned_but_disabled_focus_ignores_requests_without_notifying() {
    let notifications = Rc::new(RefCell::new(Vec::new()));
    let widget = Focus::new(SizedBox::shrink())
        .can_request_focus(false)
        .on_focus_change({
            let notifications = notifications.clone();
            move |focused| notifications.borrow_mut().push(focused)
        });
    let mut runtime = Runtime::new(widget.into()).unwrap();
    frame(&mut runtime);
    let node = runtime
        .tree()
        .focus_node_at(runtime.tree().root().unwrap())
        .unwrap();
    node.request_focus();
    frame(&mut runtime);
    assert!(!node.has_focus());
    assert!(notifications.borrow().is_empty());
}

#[test]
fn removing_a_focus_widget_detaches_its_callback_before_the_node_is_reused() {
    let node = FocusNode::new();
    let old = Rc::new(RefCell::new(Vec::new()));
    let new = Rc::new(RefCell::new(Vec::new()));
    let build = |notices: &Rc<RefCell<Vec<bool>>>| {
        let notices = notices.clone();
        Focus::new(SizedBox::shrink())
            .node(node.clone())
            .on_focus_change(move |focused| notices.borrow_mut().push(focused))
    };
    let mut runtime = Runtime::new(Column::new([build(&old)]).into()).unwrap();
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    assert_eq!(*old.borrow(), [true]);

    runtime
        .schedule_update(
            runtime.tree().root().unwrap(),
            Column::new([SizedBox::shrink()]).into(),
        )
        .unwrap();
    frame(&mut runtime);
    assert!(!node.has_primary_focus());
    assert_eq!(*old.borrow(), [true], "removed callback received detach");
    runtime
        .schedule_update(
            runtime.tree().root().unwrap(),
            Column::new([build(&new)]).into(),
        )
        .unwrap();
    frame(&mut runtime);
    node.request_focus();
    frame(&mut runtime);
    assert_eq!(*old.borrow(), [true], "removed callback received new focus");
    assert_eq!(*new.borrow(), [true]);
}
