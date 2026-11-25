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

#[test]
fn indexed_changes_and_focus_match_arena_order_after_reuse() {
    let (mut tree, _, ids) = wide_tree();
    let at = tree.revision();
    for index in [190, 63, 64] {
        let mut changed = tree.node(ids[index]).unwrap().clone();
        changed.state.focused = true;
        tree.update(ids[index], changed);
    }
    tree.remove(ids[63]);
    let replacement = tree.insert(node("replacement", vec![]));
    tree.update_geometry(ids[2], Rect::default(), &[]);
    let expected: Vec<_> = tree
        .iter()
        .map(|(id, _)| id)
        .filter(|id| [ids[2], replacement, ids[64], ids[190]].contains(id))
        .collect();
    assert_eq!(
        tree.changed_since(at).map(|(id, _)| id).collect::<Vec<_>>(),
        expected
    );
    for index in [64, 190] {
        assert_eq!(
            tree.focused_node(),
            tree.iter()
                .find_map(|(id, n)| n.state.focused.then_some(id))
        );
        let mut changed = tree.node(ids[index]).unwrap().clone();
        changed.state.focused = false;
        tree.update(ids[index], changed);
    }
    assert_eq!(tree.focused_node(), None);
}

#[test]
fn framework_tags_do_not_cross_the_mobile_projection_boundary() {
    let mut tree = SemanticsTree::new();
    let tag = SemanticsTag::new("framework route marker");
    let mut subject = node("tagged target", vec![]);
    subject.state.add_tag(tag.clone());
    let id = tree.insert(subject);
    tree.set_root(Some(id));
    let mut project = MobileAccessibilityProjection::new();
    let update = project.sync(&tree).unwrap();
    assert!(tree.node(id).unwrap().is_tagged(&tag));
    assert!(update.nodes[0].semantic.tags().is_empty());
    assert!(SemanticsSnapshot::from(&tree).nodes[0].tags().is_empty());
}

#[test]
fn changing_only_tags_does_not_publish_a_mobile_state_event_or_accesskit_patch() {
    let mut tree = SemanticsTree::new();
    let id = tree.insert(node("retained target", vec![]));
    tree.set_root(Some(id));
    let mut mobile = MobileAccessibilityProjection::new();
    let mut desktop = AccessKitProjection::new();
    let _ = mobile.sync(&tree).unwrap();
    let _ = desktop.sync(&tree, 1.).unwrap();
    let mobile_id = mobile.native_node_id(id).unwrap();
    let desktop_id = desktop.native_node_id(id).unwrap();
    let tag = SemanticsTag::new("new framework tag");
    let mut changed = tree.node(id).unwrap().clone();
    changed.state.add_tag(tag.clone());
    assert!(tree.update(id, changed));
    assert!(tree.node(id).unwrap().is_tagged(&tag));
    let mobile_update = mobile.sync(&tree);
    let desktop_update = desktop.sync(&tree, 1.);
    assert_eq!(
        (
            mobile_update.as_ref().map(|update| update.events.len()),
            desktop_update.map(published)
        ),
        (None, None),
        "tag-only changes must not produce native events or patches",
    );

    let mut changed = tree.node(id).unwrap().clone();
    changed.state.set_enabled_state(Some(false));
    assert!(tree.update(id, changed));
    let update = mobile.sync(&tree).expect("real accessibility change");
    assert_eq!(update.nodes[0].native_id, mobile_id);
    assert!(!update.nodes[0].semantic.state.enabled);
    assert!(update.nodes[0].semantic.tags().is_empty());
    assert!(update.events.iter().any(|event| {
        event.native_id == mobile_id && event.kind == MobileAccessibilityEventKind::StateChanged
    }));
    let update = desktop
        .sync(&tree, 1.)
        .expect("real accessibility change")
        .into_accesskit();
    assert_eq!(update.nodes.len(), 1);
    assert_eq!(update.nodes[0].0.0, desktop_id);
    assert!(update.nodes[0].1.is_disabled());
}

#[test]
fn web_projection_receives_hit_test_policy_updates_at_unchanged_bounds() {
    let mut tree = SemanticsTree::new();
    let id = tree.insert(node("overlay", vec![]));
    tree.set_root(Some(id));
    let mut projection = MobileAccessibilityProjection::new();
    let initial = projection.sync(&tree).unwrap();
    let native_id = initial.nodes[0].native_id;
    let bounds = initial.nodes[0].semantic.bounds;

    for policy in [
        SemanticsHitTestBehavior::Opaque,
        SemanticsHitTestBehavior::Transparent,
        SemanticsHitTestBehavior::Defer,
    ] {
        let mut changed = tree.node(id).unwrap().clone();
        changed.state.hit_test_behavior = policy;
        assert!(tree.update(id, changed));
        let changed = projection
            .sync(&tree)
            .expect("semantic overlay style changed");
        assert_eq!(changed.nodes.len(), 1);
        assert_eq!(changed.nodes[0].native_id, native_id);
        assert_eq!(changed.nodes[0].semantic.bounds, bounds);
        assert_eq!(changed.nodes[0].semantic.state.hit_test_behavior, policy);
    }
}
