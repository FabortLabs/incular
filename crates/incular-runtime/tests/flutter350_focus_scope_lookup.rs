use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Focus, FocusNode, FocusScope, FocusScopeNode, GlobalKey, SizedBox, Widget,
};

#[test]
fn scope_lookup_uses_the_context_boundary_then_the_retained_focus_parent_chain() {
    let left = FocusScopeNode::new();
    let right = FocusScopeNode::new();
    let branch = FocusNode::new();
    let nested = FocusScopeNode::new();
    let inside_branch = GlobalKey::new();
    let outside_branch = GlobalKey::new();
    let inside_nested = GlobalKey::new();
    let without_boundary = GlobalKey::new();
    let box_at = |key: GlobalKey| -> Widget { Widget::from(SizedBox::shrink()).with_key(key) };
    let scene = |parent: &FocusScopeNode| -> Widget {
        Column::new([
            FocusScope::new(Column::new([
                Focus::new(box_at(inside_branch))
                    .node(branch.clone())
                    .parent_node(parent.scope_focus_node())
                    .into(),
                box_at(outside_branch),
                FocusScope::new(box_at(inside_nested))
                    .node(nested.clone())
                    .parent_node(right.scope_focus_node())
                    .into(),
            ]))
            .node(left.clone())
            .into(),
            FocusScope::new(SizedBox::shrink())
                .node(right.clone())
                .into(),
            box_at(without_boundary),
        ])
        .into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(&right)).unwrap();
    let root = runtime.tree().root().unwrap();

    for (index, parent) in [&right, &left].into_iter().enumerate() {
        if index > 0 {
            runtime.schedule_update(root, scene(parent)).unwrap();
        }
        runtime.run_frame(bounds).unwrap();
        let tree = runtime.tree();
        let id = |key: GlobalKey| tree.element_with_key(&key.into()).unwrap();
        assert_eq!(Focus::of(tree, id(inside_branch), true), branch);
        assert_eq!(branch.host_enclosing_scope().as_ref(), Some(parent));
        assert_eq!(
            FocusScope::maybe_of(tree, id(inside_branch)).as_ref(),
            Some(parent),
            "after assigning explicit focus parent {index}"
        );
        assert_eq!(
            FocusScope::maybe_of(tree, id(outside_branch)),
            Some(left.clone())
        );
        assert_eq!(
            FocusScope::maybe_of(tree, id(inside_nested)),
            Some(nested.clone())
        );
        assert!(FocusScope::maybe_of(tree, id(without_boundary)).is_none());
    }
}
