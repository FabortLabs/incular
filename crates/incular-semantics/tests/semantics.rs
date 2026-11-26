use incular_core::{ArenaId, Offset, Rect, Size};
use incular_semantics::{
    OrdinalSortKey, Role, SemanticNode, SemanticNodeId, SemanticState, SemanticsTree,
};
use std::cmp::Ordering::{Equal, Greater, Less};

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

#[test]
fn a_role_change_marks_the_same_semantic_node_dirty() {
    let mut tree = SemanticsTree::new();
    let id = tree.insert(SemanticNode {
        role: Role::GenericContainer,
        ..node(placeholder())
    });
    let clean = tree.revision();
    assert_eq!(tree.node(id).unwrap().role, Role::GenericContainer);
    assert_eq!(tree.changed_since(clean).count(), 0);
    let mut updated = tree.node(id).unwrap().clone();
    updated.role = Role::Tab;
    assert!(tree.update(id, updated));
    assert_eq!(
        tree.changed_since(clean)
            .map(|(id, node)| (id, node.role))
            .collect::<Vec<_>>(),
        [(id, Role::Tab)]
    );
    assert_eq!(tree.diagnostics().property_updates, 1);
    assert_eq!(tree.diagnostics().geometry_updates, 0);
}

#[test]
fn ordinal_sort_keys_compare_numbers_only_within_the_same_name() {
    for name in [None, Some("a")] {
        for (left, right, expected) in [
            (0., 0., Equal),
            (0., 1., Less),
            (1., 0., Greater),
            (1., 1., Equal),
        ] {
            assert_eq!(
                ordinal(left, name).compare_to(&ordinal(right, name)),
                expected
            );
        }
    }
}

#[test]
fn ordinal_sort_keys_compare_names_before_numbers() {
    for (left_name, right_name, expected) in [
        (None, Some("bar"), Less),
        (Some("foo"), None, Greater),
        (Some("foo"), Some("bar"), Greater),
        (Some("bar"), Some("foo"), Less),
    ] {
        for left in [0., 1.] {
            for right in [0., 1.] {
                assert_eq!(
                    ordinal(left, left_name).compare_to(&ordinal(right, right_name)),
                    expected
                );
            }
        }
    }
}

#[test]
fn ordinal_sort_key_ties_signed_zero_and_compares_unicode_names_in_utf16_order() {
    assert_eq!(ordinal(0., None).compare_to(&ordinal(-0., None)), Equal);
    assert_eq!(ordinal(-0., None).compare_to(&ordinal(0., None)), Equal);
    assert_eq!(
        ordinal(9., Some("😀")).compare_to(&ordinal(-9., Some("\u{e000}"))),
        Less
    );
}

fn ordinal(order: f64, name: Option<&str>) -> OrdinalSortKey {
    match name {
        Some(name) => OrdinalSortKey::named(order, name),
        None => OrdinalSortKey::new(order),
    }
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

#[test]
fn debug_dump_reports_tags_on_an_unattached_node() {
    let mut tree = SemanticsTree::new();
    let mut tagged = node(placeholder());
    tagged.state.add_tag(incular_semantics::SemanticsTag::new(
        "RenderViewport.twoPane",
    ));
    tree.insert(tagged);
    assert!(
        tree.debug_dump()
            .contains("\n  tags: RenderViewport.twoPane\n")
    );
}

#[test]
fn debug_dump_shows_tags_and_current_child_index() {
    let mut tree = SemanticsTree::new();
    let children = (0..=10)
        .map(|index| {
            let mut child = node(placeholder());
            child.label = Some(format!("child {index}").into());
            if index == 10 {
                child.state.tags = vec![
                    incular_semantics::SemanticsTag::new("RenderViewport.twoPane"),
                    incular_semantics::SemanticsTag::new("neighbor"),
                ]
                .into_boxed_slice();
            }
            tree.insert(child)
        })
        .collect::<Vec<_>>();
    let root = tree.insert(SemanticNode {
        children: children.clone(),
        ..node(placeholder())
    });
    tree.set_root(Some(root));

    let dump = tree.debug_dump();
    let tagged_child = dump
        .lines()
        .find(|line| line.contains("\"child 10\""))
        .unwrap();
    assert!(tagged_child.contains("indexInParent: 10"), "{dump}");
    assert!(
        dump.contains("\n    tags: RenderViewport.twoPane, neighbor\n"),
        "{dump}"
    );
    assert!(!dump.lines().next().unwrap().contains("indexInParent:"));
    assert!(tree.debug_dump_limited(12, 2, 4).contains("tags: Rend…"));

    let mut updated = tree.node(root).unwrap().clone();
    updated.children.rotate_right(1);
    tree.update(root, updated);
    let dump = tree.debug_dump();
    let tagged_child = dump
        .lines()
        .find(|line| line.contains("\"child 10\""))
        .unwrap();
    assert!(tagged_child.contains("indexInParent: 0"), "{dump}");

    let mut child = tree.node(children[10]).unwrap().clone();
    child.children.push(root);
    tree.update(children[10], child);
    let dump = tree.debug_dump_bounded(12);
    assert_eq!(dump.matches("\"child 10\"").count(), 1);
    assert!(!dump.contains("truncated"), "{dump}");
}

#[test]
#[cfg(debug_assertions)]
fn semantics_owner_dispatches_one_create_and_dispose_for_same_object() {
    use incular_core::memory::allocations::{MemoryAllocations, ObjectEventKind};
    use std::{cell::RefCell, rc::Rc};

    let seen = Rc::new(RefCell::new(Vec::new()));
    let _subscription = MemoryAllocations::current().add_listener({
        let seen = Rc::clone(&seen);
        move |event| seen.borrow_mut().push(event)
    });
    let mut tree = SemanticsTree::new();
    let root = tree.insert(node(placeholder()));
    tree.set_root(Some(root));
    assert_eq!(seen.borrow().len(), 1);
    drop(tree);

    let seen = seen.borrow();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].kind, ObjectEventKind::Created);
    assert_eq!(seen[1].kind, ObjectEventKind::Disposed);
    assert_eq!(seen[0].id, seen[1].id);
    assert!(
        seen.iter()
            .all(|event| event.library == "semantics" && event.class_name == "SemanticsOwner")
    );
}
