//! Semantic widget behavior tests.

use incular_config::Constraints;
use incular_core::Size;
use incular_semantics::{SemanticAction, SemanticActionKind, SemanticRole};
use incular_widgets::{
    Column, Semantics, SizedBox, Text, Widget,
    internal::{ExplicitSemantics, Key, WidgetTree},
};

#[test]
fn semantic_state_and_explicit_actions_survive_widget_conversion() {
    let semantics = Semantics::new(SizedBox::shrink())
        .role(SemanticRole::Checkbox)
        .label("Remember me")
        .value("on")
        .enabled(true)
        .selected(true)
        .checked(true)
        .focused(true)
        .read_only(true)
        .multiline(true)
        .action(SemanticAction::Focus)
        .action(SemanticAction::Activate);

    let explicit = semantics
        .explicit()
        .expect("role creates explicit semantics");
    assert_eq!(explicit.role, SemanticRole::Checkbox);
    assert_eq!(explicit.label.as_deref(), Some("Remember me"));
    assert_eq!(explicit.value.as_deref(), Some("on"));
    assert!(explicit.state.enabled);
    assert!(explicit.state.selected);
    assert_eq!(explicit.state.checked, Some(true.into()));
    assert!(explicit.state.focused);
    assert!(explicit.state.read_only);
    assert!(explicit.state.multiline);
    assert_eq!(
        explicit.actions,
        vec![SemanticActionKind::Focus, SemanticActionKind::Activate]
    );
}

#[test]
fn default_semantics_remain_independent_when_a_cloned_descriptor_is_labeled() {
    let plain: Widget = Text::new("visible text").into();
    let sibling = plain.clone();
    let labeled = plain.clone().accessibility_label("accessible name");

    assert!(plain == sibling);
    assert!(plain != labeled);
    assert!(labeled == plain.clone().accessibility_label("accessible name"));

    let mut tree = WidgetTree::new();
    tree.mount(Widget::from(Column::new([plain, labeled])))
        .expect("mount default and labeled descriptors");
    tree.layout(Constraints::tight(Size::new(160.0, 80.0)))
        .expect("layout descriptors");
    tree.update_semantics();
    let labels: Vec<_> = tree
        .semantics()
        .iter()
        .filter_map(|(_, node)| node.label.as_deref())
        .collect();
    assert!(labels.contains(&"visible text"));
    assert!(labels.contains(&"accessible name"));
}

#[test]
fn first_semantics_pass_wires_parent_child_edges() {
    let child: Widget = Text::new("child").into();
    let root: Widget = Widget::from(Column::new([child]))
        .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label("parent"));
    let mut tree = WidgetTree::new();
    tree.mount(root).expect("mount semantics tree");
    tree.layout(Constraints::tight(Size::new(160.0, 80.0)))
        .expect("layout semantics tree");

    tree.update_semantics();

    let root = tree.semantics().root().expect("semantic root");
    let root = tree.semantics().node(root).expect("semantic root node");
    assert_eq!(root.label.as_deref(), Some("parent"));
    assert_eq!(root.children.len(), 1);
    let child = tree
        .semantics()
        .node(root.children[0])
        .expect("semantic child node");
    assert_eq!(child.label.as_deref(), Some("child"));
}

#[test]
fn semantic_node_ids_survive_reorder_and_prune_removal() {
    let labeled = |key: u64, label: &str| {
        Widget::from(Text::new(label))
            .with_key(key)
            .semantics(ExplicitSemantics::new(SemanticRole::Group).label(label.to_owned()))
    };
    let key_for = |label: &str| match label {
        "a" => 1,
        "b" => 2,
        "c" => 3,
        _ => 0,
    };
    let build = |labels: &[&str]| {
        Widget::from(Column::new(
            labels
                .iter()
                .map(|label| labeled(key_for(label), label))
                .collect::<Vec<_>>(),
        ))
        .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label("parent"))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(&["a", "b"])).expect("mount");
    tree.layout(Constraints::tight(Size::new(160., 80.)))
        .unwrap();
    tree.update_semantics();
    let node_for = |tree: &WidgetTree, label: &str| {
        tree.semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some(label))
            .map(|(id, _)| id)
    };
    let a_before = node_for(&tree, "a").expect("a node");
    let b_before = node_for(&tree, "b").expect("b node");
    tree.update(root, build(&["b", "a", "c"])).unwrap();
    tree.layout(Constraints::tight(Size::new(160., 80.)))
        .unwrap();
    tree.update_semantics();
    assert_eq!(node_for(&tree, "a"), Some(a_before));
    assert_eq!(node_for(&tree, "b"), Some(b_before));
    assert!(node_for(&tree, "c").is_some());
    tree.update(root, build(&["a"])).unwrap();
    tree.layout(Constraints::tight(Size::new(160., 80.)))
        .unwrap();
    tree.update_semantics();
    assert_eq!(node_for(&tree, "a"), Some(a_before));
    assert!(node_for(&tree, "b").is_none());
}

#[test]
fn large_semantic_sibling_graph_preserves_order_and_ids_after_reorder() {
    const CHILD_COUNT: usize = 10_000;

    let build = |order: &[usize]| {
        let children = order
            .iter()
            .map(|&index| {
                Widget::from(SizedBox::shrink())
                    .with_key(Key::Value(index as u64))
                    .semantics(
                        ExplicitSemantics::new(SemanticRole::Group).label(format!("item-{index}")),
                    )
            })
            .collect::<Vec<_>>();
        Widget::from(Column::new(children))
            .semantics(ExplicitSemantics::new(SemanticRole::GenericContainer).label("parent"))
    };
    let forward: Vec<_> = (0..CHILD_COUNT).collect();
    let reversed: Vec<_> = forward.iter().copied().rev().collect();
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(&forward)).expect("mount large graph");
    tree.layout(Constraints::tight(Size::new(160.0, 80.0)))
        .expect("layout large graph");
    tree.update_semantics();

    let parent = tree.semantics().root().expect("semantic root");
    let original_ids = tree
        .semantics()
        .node(parent)
        .expect("semantic parent")
        .children
        .clone();
    assert_eq!(original_ids.len(), CHILD_COUNT);
    let original_labels: Vec<_> = original_ids
        .iter()
        .map(|id| {
            tree.semantics()
                .node(*id)
                .expect("semantic child")
                .label
                .as_deref()
                .expect("child label")
                .to_owned()
        })
        .collect();
    let expected_labels: Vec<_> = forward
        .iter()
        .map(|index| format!("item-{index}"))
        .collect();
    assert_eq!(original_labels, expected_labels);

    tree.update(root, build(&reversed))
        .expect("reorder semantic children");
    tree.layout(Constraints::tight(Size::new(160.0, 80.0)))
        .expect("layout reordered graph");
    tree.update_semantics();

    let parent_after = tree.semantics().root().expect("semantic root after update");
    assert_eq!(parent_after, parent);
    let reordered_ids = &tree
        .semantics()
        .node(parent_after)
        .expect("semantic parent after update")
        .children;
    assert_eq!(reordered_ids.len(), CHILD_COUNT);
    assert_eq!(
        reordered_ids.to_vec(),
        original_ids.iter().copied().rev().collect::<Vec<_>>(),
        "reordering preserves each keyed semantic node identity"
    );
    let reordered_labels: Vec<_> = reordered_ids
        .iter()
        .map(|id| {
            tree.semantics()
                .node(*id)
                .expect("reordered semantic child")
                .label
                .as_deref()
                .expect("reordered child label")
                .to_owned()
        })
        .collect();
    let expected_reversed: Vec<_> = reversed
        .iter()
        .map(|index| format!("item-{index}"))
        .collect();
    assert_eq!(reordered_labels, expected_reversed);
}
