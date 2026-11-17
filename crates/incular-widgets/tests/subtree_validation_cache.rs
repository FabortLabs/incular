//! Cached immutable topology must retain transactional key and scope checks.

use incular_core::{Color, Size};
use incular_widgets::internal::{Key, TreeError, Widget, WidgetTree};
use incular_widgets::{Column, Directionality, GlobalKey, Text, TextDirection};

fn leaf() -> Widget {
    Widget::box_(Size::new(10., 10.), Color::WHITE)
}

fn column(children: Vec<Widget>) -> Widget {
    Column::new(children).into()
}

fn stable_subtree(count: usize) -> Widget {
    column((0..count).map(|i| leaf().with_key(i as u64)).collect())
}

#[test]
fn reused_root_validates_one_descriptor_regardless_of_subtree_size() {
    let shared = stable_subtree(10_000);
    let mut tree = WidgetTree::new();
    let root = tree.mount(shared.clone()).expect("valid initial subtree");
    let before = tree.diagnostics();
    tree.update(root, shared)
        .expect("same immutable description");
    let after = tree.diagnostics();
    assert_eq!(after.prevalidation_visits - before.prevalidation_visits, 1);
    assert_eq!(after.rebuilds - before.rebuilds, 0);
    assert_eq!(after.child_list_scans - before.child_list_scans, 0);
}

#[test]
fn changed_leaf_does_not_revalidate_its_cached_sibling_subtree() {
    let shared = stable_subtree(10_000);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(column(vec![shared.clone(), leaf()]))
        .expect("initial subtree");
    let before = tree.diagnostics();
    tree.update(
        root,
        column(vec![
            shared,
            Widget::box_(Size::new(20., 10.), Color::WHITE),
        ]),
    )
    .expect("one changed leaf");
    let after = tree.diagnostics();
    assert_eq!(after.prevalidation_visits - before.prevalidation_visits, 3);
    assert_eq!(after.rebuilds - before.rebuilds, 2);
    assert_eq!(after.child_list_scans - before.child_list_scans, 2);
}

#[test]
fn cached_branch_does_not_hide_nested_duplicate_local_keys() {
    let shared = stable_subtree(100);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(column(vec![shared.clone(), column(vec![leaf()])]))
        .expect("initial tree");
    let previous = tree.children(root).unwrap().to_vec();
    let count = tree.element_count();
    let before = tree.diagnostics();
    let malformed = column(vec![
        shared,
        column(vec![leaf().with_key(9_u64), leaf().with_key(9_u64)]),
    ]);
    for _ in 0..2 {
        assert!(matches!(
            tree.update(root, malformed.clone()),
            Err(TreeError::DuplicateKey { .. })
        ));
    }
    assert_eq!(tree.children(root).unwrap(), previous);
    assert_eq!(tree.element_count(), count);
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
    assert_eq!(tree.diagnostics().mounts, before.mounts);
    assert_eq!(tree.diagnostics().unmounts, before.unmounts);
}

#[test]
fn global_keys_are_checked_across_reused_static_scope_subtrees() {
    let key = GlobalKey::new();
    let shared = Widget::environment_scope(7_u32, column(vec![leaf().with_key(key)]));
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(column(vec![shared.clone()]))
        .expect("one occurrence of global key");
    let before = tree.diagnostics();
    let previous = tree.children(root).unwrap().to_vec();
    assert!(matches!(
        tree.update(root, column(vec![shared.clone(), shared])),
        Err(TreeError::DuplicateGlobalKey { key: Key::Global(found) }) if found == key
    ));
    assert_eq!(tree.children(root).unwrap(), previous);
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
    assert_eq!(tree.diagnostics().mounts, before.mounts);
}

#[test]
fn copy_on_write_key_change_invalidates_validation() {
    let shared = leaf();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(column(vec![shared.clone()]))
        .expect("cache original descriptor");
    let key = GlobalKey::new();
    let changed = shared.clone().with_key(key);
    assert!(matches!(
        tree.update(
            root,
            column(vec![column(vec![changed.clone()]), column(vec![changed])]),
        ),
        Err(TreeError::DuplicateGlobalKey { .. })
    ));
    // The original allocation is still valid and keeps its independent cache.
    tree.update(root, column(vec![shared]))
        .expect("original remains valid");
}

#[test]
fn uniquely_owned_key_change_also_invalidates_validation() {
    let shared = leaf();
    {
        let mut tree = WidgetTree::new();
        tree.mount(shared.clone()).expect("cache descriptor");
    }
    // The temporary tree is gone: mutation need not trigger an Rc clone.
    let changed = shared.with_key(GlobalKey::new());
    let mut tree = WidgetTree::new();
    assert!(matches!(
        tree.mount(column(vec![
            column(vec![changed.clone()]),
            column(vec![changed])
        ])),
        Err(TreeError::DuplicateGlobalKey { .. })
    ));
    assert_eq!(tree.element_count(), 0);
}

#[test]
fn cached_directional_subtree_is_revalidated_below_a_lookup_boundary() {
    let shared: Widget = Text::new("needs ambient direction").into();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Directionality::new(TextDirection::Ltr, shared.clone()).into())
        .expect("direction supplied by scope");
    let before = tree.diagnostics();
    assert!(matches!(
        tree.update(root, Widget::environment_boundary(shared)),
        Err(TreeError::MissingDirectionality { widget: "Text", .. })
    ));
    assert_eq!(tree.diagnostics().rebuilds, before.rebuilds);
    assert_eq!(tree.diagnostics().unmounts, before.unmounts);
}

#[test]
fn validation_cache_does_not_leak_ambient_direction_between_trees() {
    let shared = column(vec![Text::new("direction is contextual").into()]);
    let mut first = WidgetTree::new();
    first
        .mount(Directionality::new(TextDirection::Rtl, shared.clone()).into())
        .expect("first tree supplies direction");
    let mut second = WidgetTree::new();
    assert!(matches!(
        second.mount(shared),
        Err(TreeError::MissingDirectionality { widget: "Text", .. })
    ));
    assert_eq!(second.element_count(), 0);
}
