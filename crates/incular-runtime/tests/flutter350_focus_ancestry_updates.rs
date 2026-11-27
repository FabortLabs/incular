use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Column, Focus, FocusNode, FocusScope, FocusScopeNode, SizedBox, Widget};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(320., 220.)))
        .unwrap();
}

fn leaf(node: &FocusNode, key: &str) -> Widget {
    Widget::from(Focus::new(SizedBox::square(20.)).node(node.clone())).with_key(key)
}

fn parent_tree(parent: &FocusNode, children: Vec<Widget>) -> Widget {
    Focus::new(Column::new(children))
        .node(parent.clone())
        .into()
}

fn update(runtime: &mut Runtime, replacement: Widget) {
    runtime
        .schedule_update(runtime.tree().root().unwrap(), replacement)
        .unwrap();
    frame(runtime);
}

#[test]
fn adding_mounted_focus_children_preserves_order_and_both_parent_links() {
    let [parent, first, second] = std::array::from_fn(|_| FocusNode::new());
    let mut runtime = Runtime::new(parent_tree(&parent, vec![leaf(&first, "first")])).unwrap();
    frame(&mut runtime);
    assert_eq!(parent.children(), [first.clone()]);
    assert_eq!(first.parent(), Some(parent.clone()));
    update(
        &mut runtime,
        parent_tree(
            &parent,
            vec![leaf(&first, "first"), leaf(&second, "second")],
        ),
    );
    assert_eq!(parent.children(), [first.clone(), second.clone()]);
    assert_eq!(first.parent(), Some(parent.clone()));
    assert_eq!(second.parent(), Some(parent));
}

#[test]
fn removing_focus_children_clears_links_without_disturbing_the_surviving_sibling() {
    let [parent, first, second] = std::array::from_fn(|_| FocusNode::new());
    let mut runtime = Runtime::new(parent_tree(
        &parent,
        vec![leaf(&first, "first"), leaf(&second, "second")],
    ))
    .unwrap();
    frame(&mut runtime);
    assert_eq!(parent.children(), [first.clone(), second.clone()]);
    let survivor = runtime.tree().element_with_key(&"second".into()).unwrap();
    update(
        &mut runtime,
        parent_tree(&parent, vec![leaf(&second, "second")]),
    );
    assert_eq!(first.parent(), None);
    assert_eq!(parent.children(), [second.clone()]);
    assert_eq!(second.parent(), Some(parent.clone()));
    assert_eq!(
        runtime.tree().element_with_key(&"second".into()),
        Some(survivor)
    );
    update(&mut runtime, parent_tree(&parent, vec![]));
    assert_eq!(first.parent(), None);
    assert_eq!(second.parent(), None);
    assert!(parent.children().is_empty());
}

#[test]
fn removing_a_focused_leaf_clears_scope_memory_without_focusing_its_sibling() {
    let scope = FocusScopeNode::new();
    let [parent, first, second] = std::array::from_fn(|_| FocusNode::new());
    let scene = |with_first| {
        let mut children = Vec::new();
        if with_first {
            children.push(leaf(&first, "first"));
        }
        children.push(leaf(&second, "second"));
        FocusScope::new(parent_tree(&parent, children))
            .node(scope.clone())
            .into()
    };
    let mut runtime = Runtime::new(scene(true)).unwrap();
    frame(&mut runtime);
    first.request_focus();
    frame(&mut runtime);
    assert!(first.has_primary_focus() && scope.has_focus());
    assert_eq!(scope.focused_child(), Some(first.clone()));
    update(&mut runtime, scene(false));
    assert!(!first.has_focus() && !second.has_focus());
    // Flutter clears its primary leaf at raw detachment, then requests the
    // scope itself when the next frame settles an empty focus history.
    assert!(scope.has_primary_focus());
    assert_eq!(scope.focused_child(), None);
}

#[test]
fn focus_through_an_ordinary_parent_updates_the_scope_to_the_active_leaf() {
    let scope = FocusScopeNode::new();
    let [parent, first, second] = std::array::from_fn(|_| FocusNode::new());
    let root = FocusScope::new(parent_tree(
        &parent,
        vec![leaf(&first, "first"), leaf(&second, "second")],
    ))
    .node(scope.clone());
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime);
    assert_eq!(scope.scope_focus_node().children(), [parent.clone()]);
    assert_eq!(parent.parent(), Some(scope.scope_focus_node()));
    assert_eq!(parent.children(), [first.clone(), second.clone()]);
    for (active, inactive) in [(&first, &second), (&second, &first)] {
        active.request_focus();
        frame(&mut runtime);
        assert_eq!(scope.focused_child(), Some(active.clone()));
        assert!(parent.has_focus() && !parent.has_primary_focus());
        assert!(active.has_focus() && active.has_primary_focus());
        assert!(!inactive.has_focus() && !inactive.has_primary_focus());
    }
}

#[test]
fn removing_a_middle_ancestor_is_enough_to_clear_focused_grandchild_history() {
    let scope = FocusScopeNode::new();
    let middle = FocusNode::new();
    let leaf_node = FocusNode::new();
    let build = |subtree| FocusScope::new(subtree).node(scope.clone()).into();
    let tree = build(parent_tree(&middle, vec![leaf(&leaf_node, "leaf")]));
    let mut runtime = Runtime::new(tree).unwrap();
    frame(&mut runtime);
    leaf_node.request_focus();
    frame(&mut runtime);
    assert_eq!(scope.focused_child(), Some(leaf_node.clone()));
    update(&mut runtime, build(SizedBox::shrink().into()));
    assert_eq!(scope.focused_child(), None);
    assert_eq!(middle.parent(), None);
    assert!(!leaf_node.has_focus());
}
