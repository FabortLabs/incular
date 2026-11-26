use incular_config::Constraints;
use incular_core::Size;
use incular_text::TextEditingController;
use incular_widgets::{
    AutofillGroup, Column, EditableText, GlobalKey, TextDirection, Widget,
    internal::{ElementId, WidgetTree},
};

fn field(hint: Option<&str>) -> Widget {
    EditableText::new(TextEditingController::new())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(180.0, 30.0))
        .autofill_hints(hint.map(|hint| [hint]))
        .into()
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::loose(Size::new(320.0, 400.0)))
        .unwrap();
}

fn scopes(tree: &WidgetTree, root: ElementId) -> Vec<ElementId> {
    let mut work = vec![root];
    let mut result = Vec::new();
    while let Some(id) = work.pop() {
        if tree.autofill_scope(id) == Some(id) {
            result.push(id);
        }
        work.extend(tree.children(id).into_iter().flatten().rev().copied());
    }
    result
}

fn hints(tree: &WidgetTree, scope: ElementId) -> Vec<String> {
    tree.autofill_clients(scope)
        .into_iter()
        .map(|id| {
            tree.text_field_input_snapshot(id)
                .unwrap()
                .traits
                .autofill_hints
                .unwrap()
                .remove(0)
        })
        .collect()
}

#[test]
fn nearest_groups_own_only_their_enabled_mounted_clients() {
    let inner: Widget = AutofillGroup::new(Column::new([field(Some("2")), field(None)])).into();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(AutofillGroup::new(Column::new([field(Some("1")), inner])).into())
        .unwrap();
    layout(&mut tree);
    let found = scopes(&tree, root);
    assert_eq!(found.len(), 2);
    assert_eq!(hints(&tree, found[0]), ["1"]);
    assert_eq!(hints(&tree, found[1]), ["2"]);
    assert_eq!(
        tree.autofill_scope(tree.autofill_clients(found[1])[0]),
        Some(found[1])
    );
}

#[test]
fn enabling_disabling_and_removing_fields_updates_the_same_scope() {
    let state = |second: Option<Option<&str>>| -> Widget {
        let mut items = vec![field(Some("1"))];
        if let Some(second) = second {
            items.push(field(second));
        }
        AutofillGroup::new(Column::new(items)).into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(state(Some(None))).unwrap();
    layout(&mut tree);
    let original = tree.autofill_clients(root)[0];
    assert_eq!(hints(&tree, root), ["1"]);
    tree.update(root, state(Some(Some("2")))).unwrap();
    layout(&mut tree);
    assert_eq!(hints(&tree, root), ["1", "2"]);
    let formerly_enabled = tree.autofill_clients(root)[1];
    tree.update(root, state(Some(None))).unwrap();
    layout(&mut tree);
    assert_eq!(tree.autofill_clients(root), [original]);
    assert_eq!(tree.autofill_scope(formerly_enabled), Some(root));
    tree.update(root, state(None)).unwrap();
    layout(&mut tree);
    assert_eq!(tree.autofill_scope(formerly_enabled), None);
    assert_eq!(tree.autofill_clients(root), [original]);
}

#[test]
fn a_reparented_global_key_keeps_its_client_identity_but_changes_scope() {
    let key = GlobalKey::new();
    let state = |inner| -> Widget {
        let moved = field(Some("3")).with_key(key.clone());
        let mut inside = vec![field(Some("2"))];
        let mut outside = vec![field(Some("1"))];
        if inner {
            inside.push(moved);
        } else {
            outside.push(moved);
        }
        outside.push(AutofillGroup::new(Column::new(inside)).into());
        AutofillGroup::new(Column::new(outside)).into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(state(true)).unwrap();
    layout(&mut tree);
    let inner = scopes(&tree, root)[1];
    let moved = tree.autofill_clients(inner)[1];
    let tag_before = tree.text_field_input_snapshot(moved).unwrap().client_id;
    assert_eq!(hints(&tree, root), ["1"]);
    assert_eq!(hints(&tree, inner), ["2", "3"]);
    tree.update(root, state(false)).unwrap();
    layout(&mut tree);
    let inner = scopes(&tree, root)[1];
    assert_eq!(hints(&tree, root), ["1", "3"]);
    assert_eq!(hints(&tree, inner), ["2"]);
    let moved_after = tree.autofill_clients(root)[1];
    assert_eq!(moved_after, moved);
    assert_eq!(
        tree.text_field_input_snapshot(moved_after)
            .unwrap()
            .client_id,
        tag_before
    );
    assert!(tree.take_autofill_context_completions().is_empty());
}

#[test]
fn registration_order_survives_sibling_reorder_and_cross_group_move() {
    let moving = GlobalKey::new();
    let screen = |phase| -> Widget {
        let first = field(Some("first")).with_key(1_u64);
        let second = field(Some("second")).with_key(2_u64);
        let third = field(Some("moving")).with_key(moving.clone());
        let mut outside = if phase == 0 {
            vec![first, second]
        } else {
            vec![second, first]
        };
        let inside: Widget = AutofillGroup::new(Column::new(if phase < 2 {
            vec![third.clone()]
        } else {
            Vec::new()
        }))
        .into();
        if phase == 2 {
            outside.insert(0, third);
        }
        outside.push(inside.with_key(3_u64));
        AutofillGroup::new(Column::new(outside)).into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(screen(0)).unwrap();
    layout(&mut tree);
    let before = tree.autofill_clients(root);
    assert_eq!(hints(&tree, root), ["first", "second"]);
    tree.update(root, screen(1)).unwrap();
    layout(&mut tree);
    assert_eq!(tree.autofill_clients(root), before);
    assert_eq!(hints(&tree, root), ["first", "second"]);
    tree.update(root, screen(2)).unwrap();
    layout(&mut tree);
    assert_eq!(hints(&tree, root), ["first", "second", "moving"]);
    assert!(hints(&tree, scopes(&tree, root)[1]).is_empty());
}
