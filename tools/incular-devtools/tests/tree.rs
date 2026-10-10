use incular_devtools_protocol::{DevWidgetId, DevWindowId, TreeDelta, WidgetNode};
use incular_devtools_ui::inspector::{InspectorModel, TreeNavigation};

fn id(index: u64) -> DevWidgetId {
    DevWidgetId::new(index, 0)
}

fn tree() -> InspectorModel {
    let node = |index: u64,
                parent: Option<u64>,
                kind: &str,
                text: Option<&str>,
                key: Option<&str>,
                children: &[u64]| WidgetNode {
        id: id(index),
        parent: parent.map(id),
        type_name: kind.into(),
        label: text.map(str::to_owned),
        key: key.map(str::to_owned),
        child_ids: children.iter().copied().map(id).collect(),
        revision: 0,
    };
    let mut model = InspectorModel::default();
    model.apply_tree(
        1,
        [TreeDelta::Snapshot {
            window: DevWindowId::new(1, 0),
            root: Box::new(node(1, None, "Row", None, None, &[2, 4])),
            nodes: vec![
                node(2, Some(1), "Column", None, Some("editor"), &[3]),
                node(3, Some(2), "Text", Some("Hello café"), Some("title"), &[]),
                node(4, Some(1), "Column", None, Some("actions"), &[5]),
                node(5, Some(4), "Button", Some("Save"), Some("save"), &[]),
            ],
            truncated: false,
        }],
    );
    model
}

#[test]
fn scoped_search_finds_collapsed_descendants_and_combines_terms() {
    let mut model = tree();
    model.set_search("type:Text text:CAFÉ key:title");
    assert_eq!(model.row_count(), 1);
    assert_eq!(model.navigate_tree(TreeNavigation::First), Some(id(3)));
    model.set_search("key:save");
    assert_eq!(model.row_count(), 1);
    assert_eq!(model.navigate_tree(TreeNavigation::First), Some(id(5)));
    model.set_search("has:children");
    assert_eq!(model.row_count(), 3);
    model.set_search("type:Text key:missing");
    assert_eq!(model.row_count(), 0);
}

#[test]
fn search_navigation_starts_at_the_first_match_wraps_and_preserves_the_filter() {
    let mut model = tree();
    model.set_search("type:Column");
    assert_eq!(model.navigate_tree(TreeNavigation::Next), Some(id(2)));
    assert_eq!(model.navigate_tree(TreeNavigation::Next), Some(id(4)));
    assert_eq!(model.navigate_tree(TreeNavigation::Next), Some(id(2)));
    assert_eq!(model.navigate_tree(TreeNavigation::Previous), Some(id(4)));
    assert_eq!(model.navigate_tree(TreeNavigation::CollapseOrParent), None);
    assert_eq!(model.navigate_tree(TreeNavigation::ExpandOrChild), None);
    assert_eq!(model.row_count(), 2);
    assert_eq!(model.selected_node(), Some(id(4)));
}

#[test]
fn keyboard_navigation_expands_collapses_and_keeps_parent_context() {
    let mut model = tree();
    assert_eq!(model.navigate_tree(TreeNavigation::Next), Some(id(2)));
    assert_eq!(model.navigate_tree(TreeNavigation::ExpandOrChild), None);
    assert_eq!(model.selected_node(), Some(id(2)));
    assert_eq!(
        model.navigate_tree(TreeNavigation::ExpandOrChild),
        Some(id(3))
    );
    assert_eq!(
        model.navigate_tree(TreeNavigation::CollapseOrParent),
        Some(id(2))
    );
    assert_eq!(model.navigate_tree(TreeNavigation::CollapseOrParent), None);
    assert_eq!(
        model.navigate_tree(TreeNavigation::CollapseOrParent),
        Some(id(1))
    );
    assert_eq!(model.navigate_tree(TreeNavigation::Last), Some(id(4)));
}

#[test]
fn picked_node_escapes_an_unrelated_subtree_and_search() {
    let mut model = tree();
    model.select_node(id(2));
    model.focus_subtree(Some(id(2)));
    model.expand_branch(id(2), true);
    assert_eq!(model.row_count(), 2);
    model.set_search("type:Text");
    assert!(model.select_node(id(5)));
    assert_eq!(model.selected_node(), Some(id(5)));
    assert_eq!(model.ancestor_path(id(5)), vec![id(1), id(4), id(5)]);
    assert_eq!(model.row_count(), 5);
}

#[test]
fn selection_history_skips_removed_nodes_and_replaces_the_forward_branch() {
    let mut model = tree();
    model.select_node(id(2));
    model.select_node(id(3));
    model.select_node(id(5));
    model.apply_tree(2, [TreeDelta::Remove { id: id(3) }]);
    assert_eq!(model.selection_history_step(false), Some(id(2)));
    assert_eq!(model.selection_history_step(true), Some(id(5)));
    assert_eq!(model.selection_history_step(false), Some(id(2)));
    model.select_node(id(4));
    assert_eq!(model.selection_history_step(true), None);
    assert_eq!(model.selection_history_step(false), Some(id(2)));
    assert!(!model.select_node(id(99)));
}
