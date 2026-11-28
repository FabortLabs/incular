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

#[test]
fn sort_keys_reject_incompatible_concrete_types_before_consulting_names() {
    use incular_semantics::{
        CustomSemanticsSortKey as Custom, IncompatibleSemanticsSortKeyTypes,
        SemanticsSortKeyRef as Key,
    };
    let ordinal = OrdinalSortKey::new(0.);
    let number = Custom::new(0_i32);
    let other_type = Custom::named(0_i64, "elsewhere");
    assert_eq!(Key::Ordinal(&ordinal).compare_to(Key::Custom(&number)),
        Err(IncompatibleSemanticsSortKeyTypes));
    assert_eq!(Key::Custom(&number).compare_to(Key::Ordinal(&ordinal)),
        Err(IncompatibleSemanticsSortKeyTypes));
    assert_eq!(Key::Custom(&number).compare_to(Key::Custom(&other_type)),
        Err(IncompatibleSemanticsSortKeyTypes));
    assert!(!Key::Custom(&number).shares_group_with(Key::Custom(&other_type)));

    let another_group = Custom::named(-99_i32, "group");
    assert_eq!(Key::Custom(&number).compare_to(Key::Custom(&another_group)), Ok(Less));
    assert!(!Key::Custom(&number).shares_group_with(Key::Custom(&another_group)));
}

#[test]
fn ordinal_comparison_within_the_same_name_has_all_eight_cases() {
    use incular_semantics::SemanticsSortKeyRef as Key;
    for name in [None, Some("a")] {
        for (left, right, expected) in [
            (0., 0., Equal), (0., 1., Less), (1., 0., Greater), (1., 1., Equal),
        ] {
            let left = ordinal(left, name);
            let right = ordinal(right, name);
            assert!(Key::Ordinal(&left).shares_group_with(Key::Ordinal(&right)));
            assert_eq!(Key::Ordinal(&left).compare_to(Key::Ordinal(&right)), Ok(expected));
        }
    }
}

#[test]
fn visibility_inspection_allows_only_a_childless_empty_root_or_consumed_empty_child() {
    use incular_semantics::InvisibleSemanticsNode;

    let empty = Rect::from_origin_size(Offset::ZERO, Size::new(0., 10.));
    let visible = Rect::from_origin_size(Offset::ZERO, Size::new(10., 10.));
    let mut tree = SemanticsTree::new();
    let root = tree.insert(SemanticNode { bounds: empty, ..node(placeholder()) });
    let child = tree.insert(SemanticNode { bounds: empty, ..node(placeholder()) });
    let collapsed_grandchild = tree.insert(SemanticNode { bounds: empty, ..node(placeholder()) });
    tree.update_geometry(child, empty, &[collapsed_grandchild]);
    tree.set_root(Some(root));
    assert!(tree.validate_visibility().is_ok(), "empty root alone");

    tree.update_geometry(root, visible, &[child]);
    assert!(tree.validate_visibility_with_merge_boundaries(|node| node.id == root).is_ok());

    tree.update_geometry(child, visible, &[]);
    assert!(tree.validate_visibility().is_ok());

    tree.update_geometry(root, empty, &[child]);
    tree.update_geometry(child, empty, &[collapsed_grandchild]);
    let error = tree.validate_visibility_with_merge_boundaries(|node| node.id == root).unwrap_err();
    assert_eq!(error.nodes(), &[InvisibleSemanticsNode { node: root, parent: None, bounds: empty }]);
    assert!(error.to_string().contains("root with children"));

    tree.update_geometry(root, visible, &[child]);
    let error = tree.validate_visibility().unwrap_err();
    assert_eq!(error.nodes(), &[InvisibleSemanticsNode { node: child, parent: Some(root), bounds: empty }]);
    assert!(error.to_string().contains("is a child of"));

    // Hidden viewport cache entries are valid when their geometry is nonempty.
    let mut child_state = tree.node(child).unwrap().clone();
    child_state.bounds = visible;
    child_state.children.clear();
    child_state.state.hidden = true;
    tree.update(child, child_state);
    assert!(tree.validate_visibility().is_ok());
}

#[test]
fn merge_membership_follows_ancestor_policy_without_merging_the_owner_into_itself() {
    let mut tree = SemanticsTree::new();
    let root = tree.insert(node(placeholder()));
    let child = tree.insert(node(placeholder()));
    let first = tree.insert(node(placeholder()));
    let second = tree.insert(node(placeholder()));
    tree.update_geometry(root, tree.node(root).unwrap().bounds, &[child]);
    tree.update_geometry(child, tree.node(child).unwrap().bounds, &[first, second]);
    tree.set_root(Some(root));

    for (boundaries, child_owner, grandchild_owner) in [
        (vec![], None, None),
        (vec![root], Some(root), Some(root)),
        (vec![root, child], Some(root), Some(root)),
        (vec![child], None, Some(child)),
    ] {
        let merges = |node: &SemanticNode| boundaries.contains(&node.id);
        assert_eq!(tree.merge_owner(root, merges), None);
        assert_eq!(tree.merge_owner(child, merges), child_owner);
        assert_eq!(tree.merge_owner(first, merges), grandchild_owner);
        assert_eq!(tree.merge_owner(second, merges), grandchild_owner);
    }
}

#[test]
fn diagnostic_reports_effective_validation_and_empty_geometry() {
    use incular_semantics::SemanticValidationResult;
    let mut tree = SemanticsTree::new();
    let mut candidate = node(placeholder());
    candidate.state.validation_result = SemanticValidationResult::Valid;
    let id = tree.insert(candidate);
    tree.set_root(Some(id));
    let stamp = tree.revision();
    let dump = tree.debug_dump();
    assert!(dump.contains("validationResult: valid"), "{dump}");
    assert!(dump.contains("invisible"), "{dump}");
    assert_eq!(tree.revision(), stamp);

    let mut visible = tree.node(id).unwrap().clone();
    visible.bounds = Rect::from_origin_size(Offset::new(50., 10.), Size::new(20., 30.));
    visible.state.invalid = true;
    assert!(tree.update(id, visible.clone()));
    let dump = tree.debug_dump();
    assert!(dump.contains("validationResult: invalid"), "{dump}");
    assert!(!dump.contains("validationResult: valid"));
    assert!(!dump.contains("invisible"));

    visible.state.invalid = false;
    visible.state.validation_result = SemanticValidationResult::None;
    assert!(tree.update(id, visible));
    assert!(!tree.debug_dump().contains("validationResult"));
}

#[test]
fn diagnostic_names_custom_actions_in_order_and_bounds_the_list() {
    use incular_semantics::{CustomSemanticsAction, SemanticActionKind};
    let mut tree = SemanticsTree::new();
    let mut candidate = node(placeholder());
    candidate.actions.push(SemanticActionKind::Custom);
    candidate.state.custom_actions = ["action1", "action2", "action3"]
        .into_iter()
        .map(|label| CustomSemanticsAction::new(label).get_identifier())
        .collect();
    let id = tree.insert(candidate);
    tree.set_root(Some(id));
    let stamp = tree.revision();
    let dump = tree.debug_dump();
    assert!(dump.contains("actions=[Custom]"), "{dump}");
    assert!(dump.contains("customActions: action1, action2, action3"), "{dump}");
    let limited = tree.debug_dump_limited(1, 4, 11);
    assert!(limited.contains("customActions: action1, ac…"), "{limited}");
    assert!(!limited.contains("action2"));
    assert_eq!(tree.revision(), stamp);
}

#[test]
fn exhausting_the_flutter_id_range_never_aliases_a_recycled_semantic_node() {
    use std::collections::HashSet;

    let mut tree = SemanticsTree::new();
    // Flutter caps framework identifiers at u16::MAX and the pinned
    // original removes ID 1400 after filling that range.
    let count = usize::from(u16::MAX);
    let mut live = HashSet::with_capacity(count + 1);
    let mut removed = None;
    for index in 0..count {
        let id = tree.insert(node(placeholder()));
        assert!(live.insert(id), "two live nodes shared ID {id:?}");
        if index == 1400 {
            removed = Some(id);
        }
    }
    let removed = removed.unwrap();
    let cutover = tree.revision();
    assert_eq!(tree.iter().count(), count);
    assert!(tree.remove(removed).is_some());
    assert!(live.remove(&removed));

    let replacement = tree.insert(node(placeholder()));
    assert!(live.insert(replacement));
    assert_eq!(replacement.0.index(), removed.0.index());
    assert_ne!(replacement, removed, "the recycled slot has a new generation");
    assert_eq!(tree.iter().count(), count);
    assert!(tree.node(removed).is_none());
    assert!(!tree.update(removed, node(removed)));
    assert!(tree.remove(removed).is_none());
    assert!(tree.node(replacement).is_some());
    assert_eq!(
        tree.changed_since(cutover).map(|(id, _)| id).collect::<Vec<_>>(),
        [replacement]
    );
}
