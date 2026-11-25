use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Directionality, Focus, FocusNode, FocusScope, FocusScopeNode, SizedBox, Text, Widget,
};

#[test]
fn external_scope_can_take_primary_focus_instead_of_its_previous_child() {
    let outer = FocusScopeNode::new();
    let child = FocusNode::new();
    let tree = FocusScope::new(Focus::new(FocusScope::new(
        Focus::new(SizedBox::shrink()).node(child.clone()),
    )))
    .node(outer.clone());
    let mut runtime = Runtime::new(tree.into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    assert!(
        outer.request_focus(&child),
        "initial mount has already attached descendants"
    );
    assert!(!child.has_primary_focus());
    runtime.run_frame(bounds).unwrap();
    assert!(outer.has_focus() && !outer.has_primary_focus());
    assert!(child.has_primary_focus());
    let traversal = runtime.tree().focusable_elements();
    assert!(!traversal.iter().any(|&id| runtime.tree().focus_node_at(id).as_ref() == Some(&outer.scope_focus_node())));
    assert!(
        traversal
            .iter()
            .any(|&id| runtime.tree().focus_node_at(id).as_ref() == Some(&child))
    );

    outer.request_scope_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(outer.has_primary_focus());
    assert!(!child.has_primary_focus());
    assert!(outer.request_focus_in_scope());
    runtime.run_frame(bounds).unwrap();
    assert!(child.has_primary_focus());
}

#[test]
fn an_external_focus_scope_can_be_focused_on_a_zero_area_view() {
    let scope = FocusScopeNode::new();
    let key = incular_widgets::GlobalKey::new();
    let scene = Directionality::new(
        TextDirection::Ltr,
        Center::new(
            Widget::from(FocusScope::new(Text::new("X")).node(scope.clone())).with_key(key),
        ),
    );
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let empty = Constraints::tight(Size::new(0., 0.));
    runtime.run_frame(empty).unwrap();
    let element = runtime.tree().element_with_key(&key.into()).unwrap();
    assert_eq!(
        runtime.tree().element_bounds(element).unwrap().size,
        Size::new(0., 0.)
    );
    scope.request_scope_focus();
    runtime.run_frame(empty).unwrap();
    assert!(scope.has_primary_focus());
}

#[test]
fn a_scopes_focus_permission_combines_with_its_independent_descendant_flag() {
    let scope = FocusScopeNode::new();
    scope.set_descendants_are_focusable(false);
    assert!(!scope.descendants_are_focusable());
    assert!(scope.can_request_focus());
    scope.set_descendants_are_focusable(true);
    assert!(scope.descendants_are_focusable());
    scope.set_can_request_focus(false);
    assert!(!scope.descendants_are_focusable());
    assert!(!scope.can_request_focus());
    scope.set_can_request_focus(true);
    assert!(scope.descendants_are_focusable());
    assert!(scope.can_request_focus());

    scope.set_descendants_are_focusable(false);
    scope.set_can_request_focus(false);
    scope.set_can_request_focus(true);
    assert!(
        !scope.descendants_are_focusable(),
        "the local opt-out is not overwritten"
    );
    assert!(scope.can_request_focus());
}

#[test]
fn disabling_a_scope_revokes_current_child_focus_without_replaying_rejected_requests() {
    let scope = FocusScopeNode::new();
    let child = FocusNode::new();
    let disabled_child = FocusNode::new();
    disabled_child.set_can_request_focus(false);
    let scene = FocusScope::new(incular_widgets::Column::new([
        Focus::new(SizedBox::shrink()).node(child.clone()),
        Focus::new(SizedBox::shrink()).node(disabled_child.clone()),
    ]))
    .node(scope.clone());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(bounds).unwrap();
    child.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(child.has_primary_focus());

    scope.set_can_request_focus(false);
    assert!(
        runtime.frame_requested(),
        "a permission change must wake an idle scope"
    );
    runtime.run_frame(bounds).unwrap();
    assert!(runtime.focused_element().is_none() && !child.has_focus());
    assert!(runtime.tree().focusable_elements().is_empty());
    child.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(runtime.focused_element().is_none());

    scope.set_can_request_focus(true);
    runtime.run_frame(bounds).unwrap();
    assert!(runtime.focused_element().is_none());
    assert_eq!(runtime.tree().focusable_elements().len(), 1);
    disabled_child.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(runtime.focused_element().is_none());
    child.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(child.has_primary_focus());
    runtime.run_frame(bounds).unwrap();
    scope.set_descendants_are_focusable(false);
    assert!(
        runtime.frame_requested(),
        "a descendant permission change must request a frame"
    );
    runtime.run_frame(bounds).unwrap();
    assert!(!child.has_focus());
}

#[test]
fn focus_history_survives_direct_scope_focus_and_clears_after_global_key_scope_move() {
    use incular_core::{Code, KeyboardEvent, KeyboardKey};
    use incular_widgets::{Column, GlobalKey, KeyEventResult};
    use std::{cell::RefCell, rc::Rc};

    let left = FocusScopeNode::new();
    let right = FocusScopeNode::new();
    let child = FocusNode::new();
    let key = GlobalKey::new();
    let routes = Rc::new(RefCell::new(Vec::new()));
    let scene = |on_left| {
        let slot = |here, node: &FocusScopeNode, route| {
            let contents: Widget = if here {
                Widget::from(Focus::new(Text::new("a")).node(child.clone())).with_key(key)
            } else {
                SizedBox::shrink().into()
            };
            let routes = routes.clone();
            FocusScope::new(Focus::new(contents).on_key_event(move |_, _| {
                routes.borrow_mut().push(route);
                KeyEventResult::Ignored
            }))
            .node(node.clone())
        };
        Directionality::new(
            TextDirection::Ltr,
            Column::new([slot(on_left, &left, 0), slot(!on_left, &right, 1)]),
        )
        .into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(true)).unwrap();
    runtime.run_frame(bounds).unwrap();
    child.request_focus();
    runtime.run_frame(bounds).unwrap();
    let original = runtime.tree().element_with_key(&key.into()).unwrap();
    assert_eq!(left.focused(), Some(child.clone()));
    assert_eq!(left.focused_child(), Some(child.clone()));
    assert_eq!(right.focused_child(), None);

    left.request_scope_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(left.has_primary_focus());
    assert_eq!(left.focused(), None);
    assert_eq!(left.focused_child(), Some(child.clone()));
    child.request_focus();
    runtime.run_frame(bounds).unwrap();

    runtime
        .schedule_update(runtime.tree().root().unwrap(), scene(false))
        .unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(child.has_primary_focus());
    assert_eq!(runtime.focused_element(), Some(original));
    assert_eq!(left.focused_child(), None);
    assert!(!left.request_focus(&child));
    assert_eq!(right.focused(), Some(child.clone()));
    assert_eq!(right.focused_child(), Some(child.clone()));
    assert!(!runtime.dispatch_keyboard_event(KeyboardEvent::key_down(
        KeyboardKey::Character("1".into()),
        Code::Digit1
    )));
    assert_eq!(routes.take(), [1]);

    right.request_scope_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(right.has_primary_focus());
    runtime
        .schedule_update(runtime.tree().root().unwrap(), scene(true))
        .unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(!child.has_focus());
    assert_eq!(left.focused_child(), None, "old history must not reappear");
    assert_eq!(right.focused_child(), None);
}

#[test]
fn a_parent_scope_remembers_its_nested_scope_and_not_the_nested_leaf() {
    let outer = FocusScopeNode::new();
    let nested = FocusScopeNode::new();
    let leaf = FocusNode::new();
    let scene = FocusScope::new(
        FocusScope::new(Focus::new(SizedBox::shrink()).node(leaf.clone())).node(nested.clone()),
    )
    .node(outer.clone());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    leaf.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(leaf.has_primary_focus());
    assert_eq!(outer.focused_child(), Some(nested.scope_focus_node()));
    assert_eq!(nested.focused_child(), Some(leaf.clone()));

    nested.request_scope_focus();
    runtime.run_frame(bounds).unwrap();
    assert_eq!(outer.focused(), Some(nested.scope_focus_node()));
    assert_eq!(nested.focused(), None);
    assert_eq!(nested.focused_child(), Some(leaf.clone()));

    assert!(outer.request_focus_in_scope());
    runtime.run_frame(bounds).unwrap();
    assert!(leaf.has_primary_focus());
    leaf.set_can_request_focus(false);
    assert!(outer.request_focus_in_scope());
    runtime.run_frame(bounds).unwrap();
    assert!(nested.has_primary_focus());
    assert!(!leaf.has_focus());
}

#[test]
fn descendants_inherit_scope_permission_without_losing_their_local_setting() {
    use incular_widgets::Column;
    let child = FocusNode::new();
    let local_opt_out = FocusNode::new();
    local_opt_out.set_can_request_focus(false);
    let scene = |allowed| {
        FocusScope::new(Column::new([
            Focus::new(SizedBox::shrink()).node(child.clone()),
            Focus::new(SizedBox::shrink()).node(local_opt_out.clone()),
        ]))
        .can_request_focus(allowed)
        .into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(true)).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(child.can_request_focus());
    assert!(!local_opt_out.can_request_focus());
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(false)).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(!child.can_request_focus());
    assert!(!local_opt_out.can_request_focus());
    child.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(!child.has_focus());

    runtime.schedule_update(root, scene(true)).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(child.can_request_focus());
    assert!(!child.has_focus());
    assert!(!local_opt_out.can_request_focus());
}
