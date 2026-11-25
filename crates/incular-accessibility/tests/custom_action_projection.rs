use accesskit::{Action, ActionData, ActionRequest, NodeId, TreeId};
use incular_accessibility::{
    AccessKitProjection, CustomSemanticsAction, CustomSemanticsActionId,
    MobileAccessibilityProjection, Role, SemanticAction, SemanticActionKind, SemanticNode,
    SemanticNodeId, SemanticState, SemanticsTree,
};
use incular_core::{ArenaId, Rect};

fn fixture(action: CustomSemanticsActionId) -> (SemanticsTree, SemanticNodeId) {
    let mut tree = SemanticsTree::new();
    let node = tree.insert(SemanticNode {
        id: SemanticNodeId(ArenaId::from_parts(0, 0)),
        role: Role::Button,
        label: Some("Sort".into()),
        value: None,
        description: None,
        bounds: Rect::default(),
        state: SemanticState {
            custom_actions: Box::new([action]),
            ..SemanticState::default()
        },
        actions: vec![SemanticActionKind::Custom],
        children: vec![],
    });
    tree.set_root(Some(node));
    (tree, node)
}

#[test]
fn desktop_exports_named_actions_and_validates_the_requested_id() {
    let id = CustomSemanticsAction::new("Move up").get_identifier();
    let other = CustomSemanticsAction::new("Move down").get_identifier();
    let (mut tree, semantic) = fixture(id);
    let mut projection = AccessKitProjection::new();
    let change = projection.sync(&tree, 1.0).unwrap().into_accesskit();
    let native = NodeId(projection.native_node_id(semantic).unwrap());
    let announced = &change.nodes.iter().find(|(id, _)| *id == native).unwrap().1;
    assert!(announced.supports_action(Action::CustomAction));
    assert_eq!(announced.custom_actions().len(), 1);
    assert_eq!(
        announced.custom_actions()[0].id,
        i32::try_from(id.0).unwrap()
    );
    assert_eq!(
        announced.custom_actions()[0].description.as_ref(),
        "Move up"
    );

    let request = |data| ActionRequest {
        action: Action::CustomAction,
        target_tree: TreeId::ROOT,
        target_node: native,
        data,
    };
    let valid = request(Some(ActionData::CustomAction(i32::try_from(id.0).unwrap())));
    let mapped = projection.translate_action(&valid).unwrap();
    assert_eq!(mapped.node, semantic);
    assert_eq!(mapped.action, SemanticAction::Custom(id));
    for data in [
        None,
        Some(ActionData::CustomAction(-1)),
        Some(ActionData::CustomAction(i32::try_from(other.0).unwrap())),
    ] {
        assert!(projection.translate_action(&request(data)).is_none());
    }

    let mut updated = tree.node(semantic).unwrap().clone();
    updated.state.custom_actions = Box::default();
    updated.actions.clear();
    assert!(tree.update(semantic, updated));
    let change = projection.sync(&tree, 1.0).unwrap().into_accesskit();
    let announced = &change.nodes.iter().find(|(id, _)| *id == native).unwrap().1;
    assert!(announced.custom_actions().is_empty());
    assert!(!announced.supports_action(Action::CustomAction));
    assert!(projection.translate_action(&valid).is_none());
}

#[test]
fn mobile_projection_rejects_an_id_not_advertised_by_this_node() {
    let id = CustomSemanticsAction::new("Move up").get_identifier();
    let other = CustomSemanticsAction::new("Move down").get_identifier();
    let hint = CustomSemanticsAction::overriding_action(SemanticActionKind::Activate, "Choose")
        .get_identifier();
    let (mut tree, semantics) = fixture(id);
    let mut configured = tree.node(semantics).unwrap().clone();
    configured.state.custom_actions = Box::new([id, hint]);
    assert!(tree.update(semantics, configured));
    let mut projection = MobileAccessibilityProjection::new();
    projection.activate();
    let update = projection.sync(&tree).unwrap();
    let node = update
        .nodes
        .iter()
        .find(|node| node.semantic.id == semantics)
        .unwrap();
    assert_eq!(node.semantic.state.custom_actions.as_ref(), &[id, hint]);
    assert!(
        projection
            .translate_action(node.native_id, SemanticAction::Custom(other))
            .is_none()
    );
    assert!(
        projection
            .translate_action(node.native_id, SemanticAction::Custom(hint))
            .is_none()
    );
    assert_eq!(
        projection
            .translate_action(node.native_id, SemanticAction::Custom(id))
            .unwrap()
            .action,
        SemanticAction::Custom(id),
    );
}
