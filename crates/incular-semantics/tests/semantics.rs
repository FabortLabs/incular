use incular_core::{ArenaId, Offset, Rect, Size};
use incular_semantics::{Role, SemanticNode, SemanticNodeId, SemanticState, SemanticsTree};

fn node(id: SemanticNodeId) -> SemanticNode {
    SemanticNode {
        id,
        role: Role::Text,
        label: None,
        value: None,
        description: None,
        bounds: Rect::from_origin_size(Offset::ZERO, Size::ZERO),
        state: SemanticState::default(),
        actions: vec![],
        children: vec![],
    }
}

#[test]
fn stale_ids_do_not_address_reused_nodes() {
    let mut tree = SemanticsTree::new();
    let first = tree.insert(node(SemanticNodeId(ArenaId::from_parts(0, 0))));
    tree.remove(first);
    let replacement = tree.insert(node(first));
    assert_ne!(first, replacement);
    assert!(tree.node(first).is_none());
    assert!(tree.node(replacement).is_some());
}

#[test]
fn updates_distinguish_property_and_geometry_changes() {
    let mut tree = SemanticsTree::new();
    let id = tree.insert(node(SemanticNodeId(ArenaId::from_parts(0, 0))));
    let mut updated = tree.node(id).expect("inserted node").clone();
    updated.label = Some("Heading".into());
    tree.update(id, updated.clone());
    updated.bounds = Rect::from_origin_size(Offset::new(4.0, 8.0), Size::new(10.0, 20.0));
    tree.update(id, updated);
    assert_eq!(tree.diagnostics().nodes_updated, 2);
    assert_eq!(tree.diagnostics().property_updates, 1);
    assert_eq!(tree.diagnostics().geometry_updates, 1);
}

fn placeholder() -> SemanticNodeId {
    SemanticNodeId(ArenaId::from_parts(0, 0))
}

#[test]
fn a_forest_keeps_every_root_in_order() {
    let mut tree = SemanticsTree::new();
    let first = tree.insert(node(placeholder()));
    let second = tree.insert(node(placeholder()));
    let third = tree.insert(node(placeholder()));
    tree.set_roots([second, first, third]);
    assert_eq!(tree.roots(), [second, first, third]);
    assert_eq!(tree.root(), Some(second));

    tree.remove(first);
    assert_eq!(tree.roots(), [second, third]);
    tree.remove(second);
    assert_eq!(tree.root(), Some(third));

    tree.set_root(None);
    assert!(tree.roots().is_empty());
    assert_eq!(tree.root(), None);
}

#[test]
fn only_a_root_change_advances_the_revision() {
    let mut tree = SemanticsTree::new();
    let first = tree.insert(node(placeholder()));
    let second = tree.insert(node(placeholder()));
    tree.set_roots([first, second]);
    let revision = tree.revision();

    tree.set_roots([first, second]);
    assert_eq!(tree.revision(), revision);
    tree.set_roots([second, first]);
    assert!(tree.revision() > revision);
    let revision = tree.revision();
    tree.set_roots([second]);
    assert!(tree.revision() > revision);
    let revision = tree.revision();
    tree.set_roots([second, first]);
    assert!(tree.revision() > revision);
}

#[test]
fn debug_dump_lists_every_root() {
    let mut tree = SemanticsTree::new();
    let mut labeled = |label: &str| {
        let mut semantic = node(placeholder());
        semantic.label = Some(label.into());
        tree.insert(semantic)
    };
    let first = labeled("first");
    let second = labeled("second");
    tree.set_roots([first, second]);
    let dump = tree.debug_dump();
    let first_at = dump.find("\"first\"").expect("first root dumped");
    let second_at = dump.find("\"second\"").expect("second root dumped");
    assert!(first_at < second_at, "{dump}");
}
