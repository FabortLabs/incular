//! Default FocusScope nodes and their mounted descendant history.

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Focus, FocusNode, FocusScope, FocusScopeNode, SizedBox, Widget, internal::Key,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(400., 300.)))
        .unwrap();
}

fn anchor(node: &FocusNode, key: &str) -> Widget {
    let child = Widget::from(SizedBox::from_dimensions(
        Some(40.),
        Some(20.),
        None::<Widget>,
    ))
    .with_key(Key::from(key));
    Focus::new(child).node(node.clone()).into()
}

fn enclosing(runtime: &Runtime, child: &str) -> Option<FocusScopeNode> {
    let tree = runtime.tree();
    let child = tree.element_with_key(&Key::from(child)).unwrap();
    FocusScope::maybe_of(tree, child)
}

#[test]
fn a_default_scope_owns_and_retains_a_distinct_node_and_child_history() {
    let first = FocusNode::new();
    let second = FocusNode::new();
    let compose = || {
        FocusScope::new(Column::new([
            Widget::from(FocusScope::new(anchor(&first, "first"))),
            Widget::from(FocusScope::new(anchor(&second, "second"))),
        ]))
        .into()
    };
    let mut runtime = Runtime::new(compose()).unwrap();
    frame(&mut runtime);
    let first_scope = enclosing(&runtime, "first").expect("first scope owns a node");
    let second_scope = enclosing(&runtime, "second").expect("second scope owns a node");
    assert_ne!(first_scope, second_scope);
    let outer = first_scope
        .scope_focus_node()
        .host_enclosing_scope()
        .expect("outer scope");
    assert_eq!(
        second_scope.scope_focus_node().host_enclosing_scope(),
        Some(outer)
    );

    first.request_focus();
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    second.request_focus();
    frame(&mut runtime);
    assert!(second.has_primary_focus());
    assert_eq!(first_scope.focused_child(), Some(first.clone()));

    runtime
        .schedule_update(runtime.tree().root().unwrap(), compose())
        .unwrap();
    frame(&mut runtime);
    assert_eq!(enclosing(&runtime, "first"), Some(first_scope.clone()));
    assert_eq!(enclosing(&runtime, "second"), Some(second_scope.clone()));
    assert!(second.has_primary_focus());

    assert!(first_scope.request_focus_in_scope());
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    assert!(!second.has_focus());
}

#[test]
fn removing_external_and_owned_scopes_detaches_their_nodes_without_sibling_focus() {
    let external = FocusScopeNode::new();
    let first = FocusNode::new();
    let second = FocusNode::new();
    let scene = FocusScope::new(Column::new([
        Widget::from(FocusScope::new(anchor(&first, "external")).node(external.clone())),
        Widget::from(FocusScope::new(anchor(&second, "owned"))),
    ]));
    let mut runtime = Runtime::new(Column::new([Widget::from(scene)]).into()).unwrap();
    frame(&mut runtime);
    let owned = enclosing(&runtime, "owned").expect("owned scope participates in host tree");
    assert_eq!(enclosing(&runtime, "external"), Some(external.clone()));
    second.request_focus();
    frame(&mut runtime);
    first.request_focus();
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    assert!(!second.has_focus());
    assert!(external.scope_focus_node().host_enclosing_scope().is_some());
    assert!(owned.scope_focus_node().host_enclosing_scope().is_some());

    runtime
        .schedule_update(
            runtime.tree().root().unwrap(),
            Column::new(Vec::<Widget>::new()).into(),
        )
        .unwrap();
    frame(&mut runtime);
    assert!(external.scope_focus_node().host_enclosing_scope().is_none());
    assert!(owned.scope_focus_node().host_enclosing_scope().is_none());
    assert!(external.focused_child().is_none() && owned.focused_child().is_none());
    assert!(!first.has_focus() && !second.has_focus());
}

#[test]
fn removing_the_focused_child_does_not_focus_its_sibling_in_the_same_scope() {
    let scope = FocusScopeNode::new();
    let first = FocusNode::new();
    let second = FocusNode::new();
    let build = |include_first: bool| {
        let mut entries = Vec::with_capacity(2);
        if include_first {
            entries.push(anchor(&first, "first").with_key(Key::from("first-focus")));
        }
        entries.push(anchor(&second, "second").with_key(Key::from("second-focus")));
        FocusScope::new(Column::new(entries))
            .node(scope.clone())
            .autofocus(include_first)
            .into()
    };
    let mut runtime = Runtime::new(build(true)).unwrap();
    frame(&mut runtime);
    assert!(scope.request_focus(&first));
    frame(&mut runtime);
    assert!(first.has_primary_focus());
    assert!(!second.has_focus());
    let second_key = Key::from("second-focus");
    let original = runtime.tree().element_with_key(&second_key).unwrap();

    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(false))
        .unwrap();
    frame(&mut runtime);
    assert!(!first.has_focus() && !second.has_focus());
    assert_eq!(runtime.tree().element_with_key(&second_key), Some(original));
}

#[test]
fn inserting_a_parent_scope_reparents_a_focused_child_scope_without_blurring_it() {
    use incular_widgets::GlobalKey;

    let parent = FocusScopeNode::new();
    let child = FocusScopeNode::new();
    let leaf = FocusNode::new();
    let scope_key = GlobalKey::new();
    let build = |insert_parent| {
        let inside = Widget::from(FocusScope::new(anchor(&leaf, "leaf")).node(child.clone()))
            .with_key(scope_key);
        let body = if insert_parent {
            Widget::from(FocusScope::new(inside).node(parent.clone()))
        } else {
            inside
        };
        Column::new([body]).into()
    };
    let mut runtime = Runtime::new(build(false)).unwrap();
    frame(&mut runtime);
    assert_eq!(enclosing(&runtime, "leaf"), Some(child.clone()));
    assert!(child.request_focus(&leaf));
    frame(&mut runtime);
    assert!(leaf.has_primary_focus());
    let target = runtime.focused_element().unwrap();
    let blur_count = std::rc::Rc::new(std::cell::Cell::new(0));
    let _subscription = leaf.observe({
        let blur_count = blur_count.clone();
        let leaf = leaf.clone();
        move || {
            if !leaf.has_primary_focus() {
                blur_count.set(blur_count.get() + 1);
            }
        }
    });

    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(true))
        .unwrap();
    frame(&mut runtime);
    assert_eq!(enclosing(&runtime, "leaf"), Some(child.clone()));
    assert_eq!(
        child.scope_focus_node().host_enclosing_scope(),
        Some(parent.clone())
    );
    assert_eq!(parent.focused_child(), Some(child.scope_focus_node()));
    assert_eq!(runtime.focused_element(), Some(target));
    assert!(leaf.has_primary_focus());
    assert_eq!(blur_count.get(), 0);
}
