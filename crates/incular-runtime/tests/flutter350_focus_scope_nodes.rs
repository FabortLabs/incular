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

#[test]
fn ordinary_parent_focus_and_traversal_gates_have_different_descendant_effects() {
    use incular_config::Alignment;
    use incular_widgets::{Align, Column};
    let scope = FocusScopeNode::new();
    let left = FocusNode::new();
    let right = FocusNode::new();
    let a = FocusNode::new();
    let b = FocusNode::new();
    let branch = |parent: &FocusNode, child: &FocusNode| {
        Focus::new(Align::new(
            Alignment::CENTER,
            Focus::new(SizedBox::new().width(20.).height(20.)).node(child.clone()),
        ))
        .node(parent.clone())
    };
    let scene =
        FocusScope::new(Column::new([branch(&left, &a), branch(&right, &b)])).node(scope.clone());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(bounds).unwrap();
    let traversable = |runtime: &Runtime, node: &FocusNode| {
        runtime
            .tree()
            .focusable_elements()
            .into_iter()
            .any(|id| runtime.tree().focus_node_at(id).as_ref() == Some(node))
    };
    assert!(scope.request_focus(&a));
    runtime.run_frame(bounds).unwrap();
    assert_eq!(scope.focused_child(), Some(a.clone()));
    assert!(traversable(&runtime, &a) && traversable(&runtime, &b));

    right.set_descendants_are_traversable(false);
    assert!(!traversable(&runtime, &b));
    assert!(traversable(&runtime, &right));
    assert!(scope.request_focus(&b));
    runtime.run_frame(bounds).unwrap();
    assert!(b.has_primary_focus());

    right.set_descendants_are_traversable(true);
    right.set_can_request_focus(false);
    assert!(traversable(&runtime, &b));
    assert!(!traversable(&runtime, &right));
    runtime.run_frame(bounds).unwrap();
    assert!(b.has_primary_focus());
    right.set_can_request_focus(true);
    right.set_descendants_are_focusable(false);
    right.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(right.has_primary_focus());
    b.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(right.has_primary_focus());
    assert_eq!(scope.focused_child(), Some(right));
    assert!(traversable(&runtime, &a) && !traversable(&runtime, &b));

    left.set_descendants_are_focusable(false);
    runtime.run_frame(bounds).unwrap();
    assert!(!traversable(&runtime, &a) && !traversable(&runtime, &b));
}

#[test]
fn external_focus_scope_owns_both_key_callbacks_and_permissions_across_rebuilds() {
    use incular_core::{Code, KeyboardEvent, KeyboardKey, NamedKey};
    use incular_widgets::{FocusKeyEventHandler, FocusRawKeyEventHandler, KeyEventResult};
    use std::{cell::Cell, rc::Rc};

    let scope = FocusScopeNode::new();
    let node = scope.scope_focus_node();
    node.set_skip_traversal(false);
    scope.set_descendants_are_focusable(false);
    scope.set_descendants_are_traversable(false);
    let ignored: FocusKeyEventHandler = Rc::new(|_, _| KeyEventResult::Ignored);
    let ignored_raw: FocusRawKeyEventHandler = Rc::new(|_, _| KeyEventResult::Ignored);
    node.set_on_key_event(Some(ignored.clone()));
    node.set_on_key(Some(ignored_raw.clone()));

    let scene = || FocusScope::with_external_focus_node(SizedBox::shrink(), scope.clone());
    let check = |typed: &FocusKeyEventHandler, raw: &FocusRawKeyEventHandler, descendants| {
        let descriptor = scene();
        assert!(Rc::ptr_eq(
            descriptor.key_event_handler().as_ref().unwrap(),
            typed
        ));
        assert!(Rc::ptr_eq(node.on_key_event().as_ref().unwrap(), typed));
        assert!(Rc::ptr_eq(
            descriptor.raw_key_handler().as_ref().unwrap(),
            raw
        ));
        assert!(Rc::ptr_eq(node.on_key().as_ref().unwrap(), raw));
        assert_eq!(descriptor.allows_descendant_focus(), descendants);
        assert_eq!(scope.descendants_are_focusable(), descendants);
        assert_eq!(descriptor.allows_descendant_traversal(), descendants);
        assert_eq!(scope.descendants_are_traversable(), descendants);
        assert!(!descriptor.skips_traversal() && !node.skip_traversal());
        assert!(descriptor.allows_focus_requests() && scope.can_request_focus());
    };
    let bounds = Constraints::tight(Size::new(120., 80.));
    let mut runtime = Runtime::new(scene().into()).unwrap();
    runtime.run_frame(bounds).unwrap();
    check(&ignored, &ignored_raw, false);
    assert!(scope.request_focus_in_scope());
    runtime.run_frame(bounds).unwrap();
    let focused = runtime.focused_element().unwrap();
    let send = |runtime: &mut Runtime| {
        let key = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Enter), Code::Enter);
        let handled = runtime.dispatch_keyboard_event(key.clone());
        let mut up = key;
        up.state = incular_core::KeyState::Up;
        runtime.dispatch_keyboard_event(up);
        handled
    };
    assert!(!send(&mut runtime));

    let seen = Rc::new(Cell::new(0));
    let typed: FocusKeyEventHandler = Rc::new({
        let seen = seen.clone();
        move |_, event| {
            if event.state == incular_core::KeyState::Down {
                seen.set(seen.get() + 1);
            }
            KeyEventResult::Handled
        }
    });
    let raw: FocusRawKeyEventHandler = Rc::new({
        let seen = seen.clone();
        move |_, event| {
            if event.is_key_down() {
                seen.set(seen.get() + 1);
            }
            KeyEventResult::Handled
        }
    });
    node.set_on_key_event(Some(typed.clone()));
    node.set_on_key(Some(raw.clone()));
    scope.set_descendants_are_focusable(true);
    scope.set_descendants_are_traversable(true);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene().into()).unwrap();
    runtime.run_frame(bounds).unwrap();
    check(&typed, &raw, true);
    assert_eq!(runtime.focused_element(), Some(focused));
    assert!(scope.has_primary_focus());
    assert!(send(&mut runtime));
    assert_eq!(seen.get(), 2);
}

#[test]
fn building_a_scope_with_different_permission_is_side_effect_free_until_mount() {
    let scope = FocusScopeNode::new();
    let scene = |allowed| {
        Widget::from(
            FocusScope::new(SizedBox::shrink())
                .node(scope.clone())
                .can_request_focus(allowed),
        )
    };
    let bounds = Constraints::tight(Size::new(120., 80.));
    let mut runtime = Runtime::new(scene(true)).unwrap();
    runtime.run_frame(bounds).unwrap();
    scope.request_scope_focus();
    runtime.run_frame(bounds).unwrap();
    let focused = runtime.focused_element().unwrap();
    assert!(scope.has_primary_focus());
    drop(scene(false));
    runtime.run_frame(bounds).unwrap();
    assert_eq!(runtime.focused_element(), Some(focused));
    assert!(scope.can_request_focus() && scope.has_primary_focus());

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(false)).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(!scope.can_request_focus() && !scope.has_primary_focus());
    assert_ne!(runtime.focused_element(), Some(focused));
}

#[test]
fn detaching_an_autofocused_node_restores_the_retained_scope() {
    let scope = FocusScopeNode::new();
    let first = FocusNode::new();
    let replacement = FocusNode::new();
    let make = |node: &FocusNode, autofocus| -> Widget {
        FocusScope::new(
            Focus::new(SizedBox::shrink())
                .node(node.clone())
                .autofocus(autofocus),
        )
        .node(scope.clone())
        .into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(make(&first, true)).unwrap();
    let root = runtime.tree().root().unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(first.has_primary_focus());

    runtime
        .schedule_update(root, make(&replacement, false))
        .unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(!first.has_primary_focus());
    assert!(!replacement.has_primary_focus());
    assert!(scope.has_primary_focus());
}

#[test]
fn a_deferred_autofocus_does_not_transfer_to_a_reused_focus_slot() {
    let scope = FocusScopeNode::new();
    let old = FocusNode::new();
    let replacement = FocusNode::new();
    let scene = |node: &FocusNode, autofocus| -> Widget {
        FocusScope::new(
            Focus::new(SizedBox::shrink())
                .node(node.clone())
                .autofocus(autofocus),
        )
        .node(scope.clone())
        .into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(&old, true)).unwrap();
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, scene(&replacement, false))
        .unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(!old.has_primary_focus());
    assert!(!replacement.has_primary_focus());
}

#[test]
fn detaching_a_focused_child_restores_history_unless_a_new_request_is_pending() {
    use incular_widgets::Column;
    for request_replacement in [false, true] {
        let scope = FocusScopeNode::new();
        let earlier = FocusNode::new();
        let old = FocusNode::new();
        let replacement = FocusNode::new();
        let scene = |last: &FocusNode| -> Widget {
            FocusScope::new(Column::new([
                Focus::new(SizedBox::shrink()).node(earlier.clone()),
                Focus::new(SizedBox::shrink()).node(last.clone()),
            ]))
            .node(scope.clone())
            .into()
        };
        let bounds = Constraints::tight(Size::new(800., 600.));
        let mut runtime = Runtime::new(scene(&old)).unwrap();
        let root = runtime.tree().root().unwrap();
        runtime.run_frame(bounds).unwrap();
        earlier.request_focus();
        runtime.run_frame(bounds).unwrap();
        assert!(earlier.has_primary_focus());
        old.request_focus();
        runtime.run_frame(bounds).unwrap();
        assert!(old.has_primary_focus());

        if request_replacement {
            replacement.request_focus();
        }
        runtime.schedule_update(root, scene(&replacement)).unwrap();
        runtime.run_frame(bounds).unwrap();
        assert!(!old.has_primary_focus());
        assert_eq!(replacement.has_primary_focus(), request_replacement);
        assert_eq!(earlier.has_primary_focus(), !request_replacement);
        assert_eq!(
            scope.focused_child(),
            Some(if request_replacement {
                replacement
            } else {
                earlier
            }),
        );
    }
}

#[test]
fn programmatic_scope_autofocus_yields_to_an_explicit_child_request() {
    use incular_widgets::Column;
    let scope = FocusScopeNode::new();
    let parent = FocusNode::new();
    let first = FocusNode::new();
    let second = FocusNode::new();
    let scene = FocusScope::new(
        Focus::new(Column::new([
            Focus::new(SizedBox::shrink()).node(first.clone()),
            Focus::new(SizedBox::shrink()).node(second.clone()),
        ]))
        .node(parent.clone()),
    )
    .node(scope.clone());
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(scope.autofocus(&second));
    assert!(!second.has_primary_focus());
    runtime.run_frame(bounds).unwrap();
    assert_eq!(scope.focused_child(), Some(second.clone()));
    assert!(parent.has_focus() && !parent.has_primary_focus());
    assert!(!first.has_focus());
    assert!(second.has_primary_focus());

    first.request_focus();
    assert!(scope.autofocus(&second));
    runtime.run_frame(bounds).unwrap();
    assert_eq!(scope.focused_child(), Some(first.clone()));
    assert!(parent.has_focus() && !parent.has_primary_focus());
    assert!(first.has_primary_focus());
    assert!(!second.has_focus());
}

#[test]
fn scoped_autofocus_drops_a_moved_target_and_keeps_the_first_valid_request() {
    use incular_widgets::Column;
    let scope = FocusScopeNode::new();
    let unrelated = FocusScopeNode::new();
    let moving = FocusNode::new();
    let first = FocusNode::new();
    let later = FocusNode::new();
    let view = |inside| -> Widget {
        let candidates = if inside {
            vec![Focus::new(SizedBox::shrink()).node(moving.clone())]
        } else {
            vec![
                Focus::new(SizedBox::shrink()).node(first.clone()),
                Focus::new(SizedBox::shrink()).node(later.clone()),
            ]
        };
        let outside = if inside {
            Widget::from(SizedBox::shrink())
        } else {
            Focus::new(SizedBox::shrink()).node(moving.clone()).into()
        };
        Column::new([
            FocusScope::new(Column::new(candidates)).node(scope.clone()),
            FocusScope::new(outside).node(unrelated.clone()),
        ])
        .into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(view(true)).unwrap();
    let root = runtime.tree().root().unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(scope.autofocus(&moving));
    assert!(scope.autofocus(&first));
    assert!(scope.autofocus(&later));
    // Change the mounted ancestry itself before the focus boundary,
    // like Flutter's manual FocusAttachment.reparent control.
    runtime.tree_mut().update(root, view(false)).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert!(!moving.has_primary_focus());
    assert!(first.has_primary_focus());
    assert!(!later.has_primary_focus());
}

#[test]
fn a_runtime_cannot_drain_another_hosts_scoped_autofocus() {
    let scene = |scope: &FocusScopeNode, child: &FocusNode| -> Widget {
        FocusScope::new(Focus::new(SizedBox::shrink()).node(child.clone()))
            .node(scope.clone())
            .into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let left_scope = FocusScopeNode::new();
    let right_scope = FocusScopeNode::new();
    let left_child = FocusNode::new();
    let right_child = FocusNode::new();
    let mut left = Runtime::new(scene(&left_scope, &left_child)).unwrap();
    let mut right = Runtime::new(scene(&right_scope, &right_child)).unwrap();
    left.run_frame(bounds).unwrap();
    right.run_frame(bounds).unwrap();

    assert!(right_scope.autofocus(&right_child));
    assert!(left_scope.autofocus(&left_child));
    left.run_frame(bounds).unwrap();
    assert!(left_child.has_primary_focus());
    assert!(!right_child.has_primary_focus());
    right.run_frame(bounds).unwrap();
    assert!(right_child.has_primary_focus());
}

#[test]
fn skipping_a_mounted_scope_does_not_skip_its_children_or_change_focus() {
    use incular_widgets::{Column, GlobalKey};

    let scope = FocusScopeNode::new();
    let parent1 = FocusNode::new();
    let parent2 = FocusNode::new();
    let child1 = FocusNode::new();
    let child2 = FocusNode::new();
    let key = GlobalKey::new();
    let scene = Widget::from(FocusScope::with_external_focus_node(
        Column::new(vec![
            Focus::new(Column::new(vec![
                Focus::new(SizedBox::shrink()).node(child1.clone()),
                Focus::new(SizedBox::shrink()).node(child2.clone()),
            ]))
            .node(parent1.clone()),
            Focus::new(SizedBox::shrink()).node(parent2.clone()),
        ]),
        scope.clone(),
    ))
    .with_key(key);
    let mut runtime = Runtime::new(scene).unwrap();
    let bounds = Constraints::tight(Size::new(160., 100.));
    child1.request_focus();
    runtime.run_frame(bounds).unwrap();
    let scope_id = runtime.tree().element_with_key(&key.into()).unwrap();
    let snapshot = |runtime: &Runtime, root| runtime.tree().focus_traversal_descendants(root);
    let children = vec![child1.clone(), child2.clone(), parent1.clone(), parent2.clone()];

    assert!(child1.has_primary_focus());
    assert_eq!(scope.focused_child(), Some(child1.clone()));
    assert!(!scope.skip_traversal());
    assert_eq!(snapshot(&runtime, Some(scope_id)), children);
    assert_eq!(
        snapshot(&runtime, None),
        [children.clone(), vec![scope.scope_focus_node()]].concat()
    );
    assert!(!runtime.tree().focusable_elements().contains(&scope_id));

    scope.set_skip_traversal(true);
    runtime.run_frame(bounds).unwrap();
    assert!(scope.skip_traversal() && child1.has_primary_focus());
    assert_eq!(scope.focused_child(), Some(child1.clone()));
    assert_eq!(snapshot(&runtime, Some(scope_id)), children);
    assert_eq!(snapshot(&runtime, None), children);

    scope.set_skip_traversal(false);
    scope.set_descendants_are_traversable(false);
    runtime.run_frame(bounds).unwrap();
    assert!(child1.has_primary_focus());
    assert_eq!(scope.focused_child(), Some(child1));
    assert!(snapshot(&runtime, Some(scope_id)).is_empty());
    assert_eq!(snapshot(&runtime, None), vec![scope.scope_focus_node()]);
    child2.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(child2.has_primary_focus(), "traversal restrictions are not focus restrictions");
}

#[test]
fn queued_autofocus_precedes_a_global_key_move_to_a_different_scope() {
    use incular_widgets::{Column, GlobalKey};

    let first = FocusScopeNode::new();
    let second = FocusScopeNode::new();
    let node = FocusNode::new();
    let key = GlobalKey::new();
    let scene = |in_first| -> Widget {
        let leaf = Widget::from(Focus::new(SizedBox::shrink()).node(node.clone())).with_key(key);
        let empty = Widget::from(SizedBox::shrink());
        let (left, right) = if in_first { (leaf, empty) } else { (empty, leaf) };
        Column::new([
            FocusScope::new(left).node(first.clone()),
            FocusScope::new(right).node(second.clone()),
        ])
        .into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(true)).unwrap();
    runtime.run_frame(bounds).unwrap();
    let element = runtime.tree().element_with_key(&key.into()).unwrap();
    assert!(!node.has_focus());

    assert!(first.autofocus(&node));
    runtime
        .schedule_update(runtime.tree().root().unwrap(), scene(false))
        .unwrap();
    runtime.run_frame(bounds).unwrap();
    assert_eq!(runtime.tree().element_with_key(&key.into()), Some(element));
    assert!(node.has_primary_focus());
    assert!(second.has_focus());
    assert_eq!(second.focused_child(), Some(node));
}

#[test]
fn an_inactive_scope_keeps_its_focused_child_when_the_child_is_reparented_inside_it() {
    use incular_widgets::{Column, GlobalKey};

    let first = FocusScopeNode::new();
    let second = FocusScopeNode::new();
    let owner = FocusNode::new();
    let leaf = FocusNode::new();
    let key = GlobalKey::new();
    let scene = |wrapped| -> Widget {
        let content = Widget::from(Focus::new(SizedBox::shrink()).node(leaf.clone())).with_key(key);
        let content = if wrapped {
            Widget::from(Focus::new(content).node(owner.clone()))
        } else {
            content
        };
        Column::new([
            FocusScope::new(content).node(first.clone()),
            FocusScope::new(SizedBox::shrink()).node(second.clone()),
        ])
        .into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(true)).unwrap();
    runtime.run_frame(bounds).unwrap();
    let element = runtime.tree().element_with_key(&key.into()).unwrap();

    assert!(first.autofocus(&leaf));
    runtime.run_frame(bounds).unwrap();
    assert!(leaf.has_primary_focus());
    assert!(second.request_focus_in_scope());
    runtime.run_frame(bounds).unwrap();
    assert!(second.has_primary_focus());
    assert_eq!(first.focused_child(), Some(leaf.clone()));

    runtime
        .schedule_update(runtime.tree().root().unwrap(), scene(false))
        .unwrap();
    runtime.run_frame(bounds).unwrap();
    assert_eq!(runtime.tree().element_with_key(&key.into()), Some(element));
    assert_eq!(first.focused_child(), Some(leaf.clone()));
    assert!(second.has_primary_focus());

    assert!(first.request_focus_in_scope());
    runtime.run_frame(bounds).unwrap();
    assert!(leaf.has_primary_focus());
    assert_eq!(first.focused_child(), Some(leaf));
}

#[test]
fn a_disabled_scope_keeps_child_history_but_blocks_all_traversal_queries() {
    use incular_widgets::{Column, GlobalKey};

    let scope = FocusScopeNode::new();
    let parent1 = FocusNode::new();
    let parent2 = FocusNode::new();
    let child1 = FocusNode::new();
    let child2 = FocusNode::new();
    let scope_key = GlobalKey::new();
    let parent_key = GlobalKey::new();
    let branch = Widget::from(
        Focus::new(Column::new([
            Focus::new(SizedBox::shrink()).node(child1.clone()),
            Focus::new(SizedBox::shrink()).node(child2.clone()),
        ]))
        .node(parent1.clone()),
    )
    .with_key(parent_key);
    let scene = Widget::from(FocusScope::with_external_focus_node(
        Column::new([
            branch,
            Focus::new(SizedBox::shrink()).node(parent2.clone()).into(),
        ]),
        scope.clone(),
    ))
    .with_key(scope_key);
    let mut runtime = Runtime::new(scene).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    child1.request_focus();
    runtime.run_frame(bounds).unwrap();
    let ids =
        |runtime: &Runtime, key: GlobalKey| runtime.tree().element_with_key(&key.into()).unwrap();
    let scope_id = ids(&runtime, scope_key);
    let parent_id = ids(&runtime, parent_key);

    assert!(child1.has_primary_focus());
    assert_eq!(scope.focused_child(), Some(child1.clone()));
    assert_eq!(
        runtime.tree().focus_traversal_descendants(Some(scope_id)),
        [
            child1.clone(),
            child2.clone(),
            parent1.clone(),
            parent2.clone()
        ]
    );
    assert_eq!(
        runtime.tree().focus_traversal_children(Some(scope_id)),
        [parent1.clone(), parent2.clone()]
    );
    assert_eq!(
        runtime.tree().focus_traversal_children(Some(parent_id)),
        [child1.clone(), child2.clone()]
    );

    parent1.set_skip_traversal(true);
    assert_eq!(
        runtime.tree().focus_traversal_children(Some(scope_id)),
        [parent2]
    );
    assert!(
        runtime
            .tree()
            .focus_traversal_descendants(Some(scope_id))
            .contains(&child2)
    );
    parent1.set_skip_traversal(false);

    scope.set_can_request_focus(false);
    runtime.run_frame(bounds).unwrap();
    child2.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(!child1.has_primary_focus() && !child2.has_primary_focus());
    assert_eq!(scope.focused_child(), Some(child1));
    assert!(
        runtime
            .tree()
            .focus_traversal_descendants(Some(scope_id))
            .is_empty()
    );
    assert!(
        runtime
            .tree()
            .focus_traversal_children(Some(scope_id))
            .is_empty()
    );
    assert!(
        runtime
            .tree()
            .focus_traversal_children(Some(parent_id))
            .is_empty()
    );
}

#[test]
fn tab_skips_a_group_with_only_disabled_controls_in_one_keypress() {
    use incular_core::{Code, KeyboardEvent, KeyboardKey, NamedKey};
    use incular_widgets::{Column, FocusTraversalGroup, FocusableActionDetector};

    let first = FocusNode::new();
    let second = FocusNode::new();
    let disabled1 = FocusNode::new();
    let disabled2 = FocusNode::new();
    let control = |label, node: &FocusNode, enabled| -> Widget {
        FocusableActionDetector::new(Text::new(label))
            .focus_node(node.clone())
            .enabled(enabled)
            .into()
    };
    let dead_group = FocusTraversalGroup::new(Column::new([
        control("Disabled Button 1", &disabled1, false),
        SizedBox::new().height(16.).into(),
        control("Disabled Button 2", &disabled2, false),
    ]));
    let scene = Directionality::new(
        TextDirection::Ltr,
        Center::new(Column::new([
            control("Enabled Button 1", &first, true),
            dead_group.into(),
            control("Enabled Button 2", &second, true),
        ])),
    );
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime.run_frame(bounds).unwrap();
    let tab = |runtime: &mut Runtime| {
        let key = KeyboardKey::Named(NamedKey::Tab);
        assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key.clone(), Code::Tab)));
        let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(key, Code::Tab));
        runtime.run_frame(bounds).unwrap();
    };

    assert!(!first.has_primary_focus() && !second.has_primary_focus());
    tab(&mut runtime);
    assert!(first.has_primary_focus() && !second.has_primary_focus());
    tab(&mut runtime);
    assert!(second.has_primary_focus() && !first.has_primary_focus());
    assert!(!disabled1.has_focus() && !disabled2.has_focus());
}
