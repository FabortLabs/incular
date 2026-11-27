use incular_config::{Constraints, EdgeInsets, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{
    Directionality, EditableText, Focus, FocusNode, FocusScope, FocusScopeNode, Padding, SizedBox,
    TraversalDirection,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn start(runtime: &mut Runtime, parent: &FocusScopeNode) {
    assert!(runtime.focus_in_direction_from(&parent.scope_focus_node(), TraversalDirection::Down,));
    frame(runtime);
}

#[test]
fn downward_focus_from_an_empty_scope_can_land_on_the_nested_scope_not_its_leaf() {
    let parent = FocusScopeNode::new();
    let nested = FocusScopeNode::new();
    let leaf = FocusNode::new();
    let padded = Padding::new(
        EdgeInsets::only(0., 10., 0., 0.),
        Focus::new(SizedBox::square(24.)).node(leaf.clone()),
    );
    let scene = Directionality::new(
        TextDirection::Ltr,
        FocusScope::new(FocusScope::new(padded).node(nested.clone())).node(parent.clone()),
    );
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    assert!(!nested.has_focus());
    assert!(!leaf.has_focus());
    start(&mut runtime, &parent);
    assert!(nested.has_focus());
    assert!(nested.has_primary_focus());
    assert!(!leaf.has_focus());
}

#[test]
fn equal_edges_preserve_descendant_order_and_skip_traversal_excludes_scope_only() {
    for skip_scope in [false, true] {
        let parent = FocusScopeNode::new();
        let nested = FocusScopeNode::new();
        let leaf = FocusNode::new();
        let padded = Padding::new(
            EdgeInsets::only(0., if skip_scope { 10. } else { 0. }, 0., 0.),
            Focus::new(SizedBox::square(24.)).node(leaf.clone()),
        );
        let scene = Directionality::new(
            TextDirection::Ltr,
            FocusScope::new(
                FocusScope::new(padded)
                    .node(nested.clone())
                    .skip_traversal(skip_scope),
            )
            .node(parent.clone()),
        );
        let mut runtime = Runtime::new(scene.into()).unwrap();
        frame(&mut runtime);
        start(&mut runtime, &parent);
        assert!(leaf.has_primary_focus(), "skipped scope: {skip_scope}");
        assert!(!nested.has_primary_focus());
    }
}

#[test]
fn no_descendants_falls_back_to_the_origin_scope_itself() {
    let parent = FocusScopeNode::new();
    let scene = FocusScope::new(SizedBox::square(24.)).node(parent.clone());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    start(&mut runtime, &parent);
    assert!(parent.has_primary_focus());
}

#[test]
fn an_implicit_editor_remains_an_initial_directional_stop() {
    let parent = FocusScopeNode::new();
    let editor = EditableText::new(TextEditingController::with_text("editable"))
        .text_direction(TextDirection::Ltr);
    let scene = FocusScope::new(editor).node(parent.clone());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    start(&mut runtime, &parent);
    let focused = runtime.focused_element().unwrap();
    assert!(runtime.tree().text_controller(focused).is_some());
    assert!(!parent.has_primary_focus());
}
