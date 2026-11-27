use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Focus, FocusNode, FocusScope, FocusScopeNode, GlobalKey, SizedBox, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

#[test]
fn updating_skip_traversal_removes_only_the_named_node_not_its_descendants() {
    let outer = FocusScopeNode::new();
    let inner = FocusScopeNode::new();
    let parent = FocusNode::new();
    let leaf = FocusNode::new();
    let scene = |skips: [bool; 4]| {
        Widget::from(
            FocusScope::new(
                FocusScope::new(
                    Focus::new(
                        Focus::new(SizedBox::shrink())
                            .node(leaf.clone())
                            .skip_traversal(skips[3]),
                    )
                    .node(parent.clone())
                    .skip_traversal(skips[2]),
                )
                .node(inner.clone())
                .skip_traversal(skips[1]),
            )
            .node(outer.clone())
            .skip_traversal(skips[0]),
        )
    };
    let mut runtime = Runtime::new(scene([false; 4])).unwrap();
    let root = runtime.tree().root().unwrap();
    frame(&mut runtime);
    let assert_snapshot = |runtime: &Runtime, expected: &[FocusNode]| {
        assert_eq!(
            runtime.tree().focus_traversal_descendants(Some(root)),
            expected
        );
    };
    let anchor = inner.scope_focus_node();
    assert_snapshot(&runtime, &[leaf.clone(), parent.clone(), anchor.clone()]);

    for (index, expected) in [
        (3, vec![parent.clone(), anchor.clone()]),
        (2, vec![leaf.clone(), anchor.clone()]),
        (1, vec![leaf.clone(), parent.clone()]),
        (0, vec![leaf.clone(), parent.clone(), anchor]),
    ] {
        let mut skips = [false; 4];
        skips[index] = true;
        runtime.schedule_update(root, scene(skips)).unwrap();
        frame(&mut runtime);
        assert_snapshot(&runtime, &expected);
    }
    assert!(outer.skip_traversal());
    assert!(!inner.skip_traversal());
}

#[test]
fn descendants_are_traversable_controls_inspection_but_not_explicit_focus() {
    let scope = FocusScopeNode::new();
    let first = FocusNode::new();
    let parent = FocusNode::new();
    let leaf = FocusNode::new();
    let parent_key = GlobalKey::new();
    let scene = FocusScope::new(Column::new([
        Focus::new(SizedBox::shrink()).node(first.clone()).into(),
        Widget::from(
            Focus::new(Focus::new(SizedBox::shrink()).node(leaf.clone()))
                .node(parent.clone())
                .descendants_are_traversable(false),
        )
        .with_key(parent_key),
    ]))
    .node(scope);
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root();
    let parent_element = runtime.tree().element_with_key(&parent_key.into()).unwrap();
    assert_eq!(
        runtime.tree().focus_traversal_descendants(root),
        [first, parent]
    );
    assert!(
        runtime
            .tree()
            .focus_traversal_descendants(Some(parent_element))
            .is_empty()
    );

    leaf.request_focus();
    frame(&mut runtime);
    assert!(leaf.has_primary_focus());
}

#[test]
fn scope_widget_flags_do_not_override_an_externally_owned_scope() {
    let scope = FocusScopeNode::new();
    let leaf = FocusNode::new();
    let scene = FocusScope::with_external_focus_node(
        Focus::new(SizedBox::shrink()).node(leaf.clone()),
        scope.clone(),
    )
    .skip_traversal(true)
    .descendants_are_focusable(false)
    .descendants_are_traversable(false);
    assert!(!scene.skips_traversal());
    assert!(scene.allows_descendant_focus() && scene.allows_descendant_traversal());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    leaf.request_focus();
    frame(&mut runtime);
    assert!(!scope.skip_traversal());
    assert_eq!(
        runtime
            .tree()
            .focus_traversal_descendants(runtime.tree().root()),
        [leaf.clone()]
    );
    assert!(leaf.has_primary_focus());
}

#[test]
fn mounted_owned_scopes_update_traversal_independently_of_focus_permission() {
    let leaf = FocusNode::new();
    let scene = |focusable, traversable| {
        Widget::from(
            FocusScope::new(Focus::new(SizedBox::shrink()).node(leaf.clone()))
                .descendants_are_focusable(focusable)
                .descendants_are_traversable(traversable),
        )
    };
    let mut runtime = Runtime::new(scene(true, false)).unwrap();
    let root = runtime.tree().root().unwrap();
    frame(&mut runtime);
    let anchor = runtime.tree().focus_node_at(root).unwrap().clone();
    assert!(anchor.descendants_are_focusable() && !anchor.descendants_are_traversable());
    assert!(
        runtime
            .tree()
            .focus_traversal_descendants(Some(root))
            .is_empty()
    );

    runtime.schedule_update(root, scene(false, true)).unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.tree().focus_node_at(root), Some(anchor.clone()));
    assert!(!anchor.descendants_are_focusable() && anchor.descendants_are_traversable());
    assert!(
        runtime
            .tree()
            .focus_traversal_descendants(Some(root))
            .is_empty()
    );
    leaf.request_focus();
    frame(&mut runtime);
    assert!(!leaf.has_focus());

    runtime.schedule_update(root, scene(true, true)).unwrap();
    frame(&mut runtime);
    assert_eq!(
        runtime.tree().focus_traversal_descendants(Some(root)),
        [leaf]
    );
}

#[test]
fn disabling_a_mounted_focus_node_notifies_its_reactive_builder() {
    use incular_widgets::Builder;
    use std::{cell::Cell, rc::Rc};

    let node = FocusNode::new();
    let builds = Rc::new(Cell::new(0));
    let focus_key = GlobalKey::new();
    let widget = Builder::new({
        let node = node.clone();
        let builds = Rc::clone(&builds);
        move |_| {
            // As in Flutter's TestFocus, the built value depends on hasFocus.
            // The listener needs to run on a permission change even if the
            // node was not focused and the displayed value remains unchanged.
            let _ = node.has_focus();
            builds.set(builds.get() + 1);
            Widget::from(Focus::new(SizedBox::shrink()).node(node.clone())).with_key(focus_key)
        }
    });
    let mut runtime = Runtime::new(widget.into()).unwrap();
    frame(&mut runtime);
    let before = builds.get();
    assert!(before > 0);
    let focus_element = runtime.tree().element_with_key(&focus_key.into()).unwrap();
    assert!(node.can_request_focus() && !node.has_focus());

    node.set_can_request_focus(false);
    frame(&mut runtime);
    assert!(builds.get() > before);
    assert_eq!(
        runtime.tree().focus_node_at(focus_element),
        Some(node.clone())
    );
    assert!(!node.can_request_focus());
    node.request_focus();
    frame(&mut runtime);
    assert!(!node.has_focus());
}
