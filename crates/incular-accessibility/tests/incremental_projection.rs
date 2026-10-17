//! A projection publishes only the nodes the retained tree changed, and a
//! removal still reaches it when nothing else changed.

use incular_accessibility::*;
use incular_core::{ArenaId, Offset, Rect, Size};

fn node(label: &str, children: Vec<SemanticNodeId>) -> SemanticNode {
    SemanticNode {
        id: SemanticNodeId(ArenaId::from_parts(0, 0)),
        role: Role::Button,
        label: Some(label.into()),
        value: None,
        description: None,
        bounds: Rect::from_origin_size(Offset::new(2.0, 3.0), Size::new(20.0, 10.0)),
        state: SemanticState::default(),
        actions: vec![],
        children,
    }
}

fn wide_tree() -> (SemanticsTree, SemanticNodeId, Vec<SemanticNodeId>) {
    let mut tree = SemanticsTree::new();
    let leaves: Vec<_> = (0..200)
        .map(|index| tree.insert(node(&format!("leaf {index}"), vec![])))
        .collect();
    let root = tree.insert(node("root", leaves.clone()));
    tree.set_root(Some(root));
    (tree, root, leaves)
}

fn published(update: NativeAccessibilityUpdate) -> usize {
    update.into_accesskit().nodes.len()
}

#[test]
fn one_changed_node_publishes_one_node() {
    let (mut tree, _, leaves) = wide_tree();
    let mut projection = AccessKitProjection::new();
    let full = projection.sync(&tree, 1.0).expect("initial tree");
    assert_eq!(published(full), 202);

    let mut changed = tree.node(leaves[7]).unwrap().clone();
    changed.label = Some("renamed".into());
    assert!(tree.update(leaves[7], changed));
    let update = projection.sync(&tree, 1.0).expect("label change");
    assert_eq!(update.kind(), AccessKitUpdateKind::Incremental);
    assert_eq!(published(update), 1);
    assert!(projection.sync(&tree, 1.0).is_none());
}

#[test]
fn removal_reaches_the_projection_without_other_changes() {
    let (mut tree, root, leaves) = wide_tree();
    let mut projection = AccessKitProjection::new();
    let _ = projection.sync(&tree, 1.0);
    assert!(projection.native_node_id(leaves[3]).is_some());

    let mut parent = tree.node(root).unwrap().clone();
    parent.children.retain(|child| *child != leaves[3]);
    tree.update(root, parent);
    tree.remove(leaves[3]);
    let update = projection.sync(&tree, 1.0).expect("removal");
    assert_eq!(published(update), 1);
    assert_eq!(projection.native_node_id(leaves[3]), None);
}

#[test]
fn mobile_projection_publishes_only_changed_nodes() {
    let (mut tree, _, leaves) = wide_tree();
    let mut projection = MobileAccessibilityProjection::new();
    assert_eq!(projection.sync(&tree).expect("initial").nodes.len(), 201);

    let mut changed = tree.node(leaves[1]).unwrap().clone();
    changed.value = Some("on".into());
    tree.update(leaves[1], changed);
    let update = projection.sync(&tree).expect("value change");
    assert_eq!(update.nodes.len(), 1);
    assert!(update.removed.is_empty());
}
