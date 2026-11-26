use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{Role, SemanticsInputType};
use incular_widgets::{
    Directionality, Padding, Placeholder, Semantics, SizedBox, Text, TextDirection, Widget,
};

fn mount(scene: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(scene.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    runtime
}

fn ids(runtime: &Runtime) -> Vec<&str> {
    let tree = runtime.tree().semantics();
    tree.node(tree.roots()[0])
        .unwrap()
        .state
        .controls_nodes
        .as_ref()
        .unwrap()
        .iter()
        .map(String::as_str)
        .collect()
}

#[test]
fn a_single_control_identifier_is_preserved_even_without_a_target() {
    let runtime = mount(Semantics::new(Placeholder::new()).controls_nodes(["abc"]));
    assert_eq!(ids(&runtime), ["abc"]);
}

#[test]
fn compatible_nested_controls_merge_in_discovery_order_and_remove_duplicates() {
    let inside = Semantics::new(Placeholder::new()).controls_nodes(["abc", "def"]);
    let scene = Semantics::new(inside).controls_nodes(["abc", "ghi"]);
    let runtime = mount(scene);
    assert_eq!(ids(&runtime), ["abc", "ghi", "def"]);
    assert_eq!(runtime.tree().semantics().len(), 1);
}

#[test]
fn explicit_child_nodes_keep_control_relationships_separate() {
    let inside = Semantics::new(Placeholder::new()).controls_nodes(["b"]);
    let scene = Semantics::new(inside)
        .explicit_child_nodes(true)
        .controls_nodes(["a"]);
    let runtime = mount(scene);
    let tree = runtime.tree().semantics();
    assert_eq!(ids(&runtime), ["a"]);
    let parent = tree.node(tree.roots()[0]).unwrap();
    let [child] = parent.children.as_slice() else {
        panic!("the child is its own node");
    };
    assert_eq!(
        tree.node(*child)
            .unwrap()
            .state
            .controls_nodes
            .as_deref()
            .unwrap(),
        [String::from("b")]
    );
}

#[test]
fn a_declarative_phone_input_hint_does_not_make_the_node_editable() {
    let scene = Semantics::new(SizedBox::square(10.)).input_type(SemanticsInputType::Phone);
    let runtime = mount(scene);
    let tree = runtime.tree().semantics();
    let node = tree.node(tree.roots()[0]).unwrap();
    assert_eq!(node.state.input_type, SemanticsInputType::Phone);
    assert_eq!(node.role, Role::GenericContainer);
    assert!(!node.state.editable);
    assert!(node.actions.is_empty());
}

#[test]
fn rebuilding_only_a_compatible_child_refreshes_the_retained_parent_controls() {
    let inside = |names| {
        Widget::from(Semantics::new(Placeholder::new()).controls_nodes(names))
            .with_key("controls-child")
    };
    let scene = Semantics::new(inside(["abc", "def"])).controls_nodes(["abc", "ghi"]);
    let mut runtime = mount(scene);
    let parent = runtime.tree().semantics().roots()[0];
    let inside_id = runtime
        .tree()
        .element_with_key(&"controls-child".into())
        .unwrap();
    assert_eq!(ids(&runtime), ["abc", "ghi", "def"]);

    runtime
        .schedule_update(inside_id, inside(["ghi", "new"]))
        .unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert_eq!(runtime.tree().semantics().roots(), [parent]);
    assert_eq!(ids(&runtime), ["abc", "ghi", "new"]);
}

#[test]
fn retaining_control_relationships_does_not_drop_accessible_text() {
    let scene = Directionality::new(
        TextDirection::Ltr,
        Semantics::new(Padding::zero(Text::new("Show preferences")))
            .controls_nodes(["preferences"]),
    );
    let runtime = mount(scene);
    assert_eq!(ids(&runtime), ["preferences"]);
    assert_eq!(
        runtime
            .tree()
            .semantics()
            .iter()
            .filter(|(_, node)| node.label.as_deref() == Some("Show preferences"))
            .count(),
        1
    );
}
