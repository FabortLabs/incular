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
    runtime.run_frame(bounds).unwrap();
    child.request_focus();
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
