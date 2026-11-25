//! A semantic graph is a forest whenever a layout-only widget has several
//! semantic children. Every root must hang off the native window node:
//! AccessKit rejects an update that publishes a node it cannot reach.
//!
//! Each update here is applied to the real `accesskit_consumer` tree, the
//! same validation the native adapters run.

use accesskit::{NodeId, TreeUpdate};
use accesskit_consumer::{Node, Tree, TreeChangeHandler};
use incular_accessibility::*;
use incular_core::{ArenaId, Offset, Rect, Size};

struct Ignore;

impl TreeChangeHandler for Ignore {
    fn node_added(&mut self, _: &Node) {}
    fn node_updated(&mut self, _: &Node, _: &Node) {}
    fn focus_moved(&mut self, _: Option<&Node>, _: Option<&Node>) {}
    fn node_removed(&mut self, _: &Node) {}
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn insert(tree: &mut SemanticsTree, label: &str, children: Vec<SemanticNodeId>) -> SemanticNodeId {
    tree.insert(SemanticNode {
        id: SemanticNodeId(ArenaId::from_parts(0, 0)),
        role: Role::Button,
        label: Some(label.into()),
        value: None,
        description: None,
        bounds: rect(0.0, 0.0, 10.0, 10.0),
        state: SemanticState::default(),
        actions: vec![],
        children,
    })
}

fn update(projection: &mut AccessKitProjection, semantics: &SemanticsTree) -> TreeUpdate {
    projection
        .sync(semantics, 1.0)
        .expect("semantic change publishes")
        .into_accesskit()
}

/// Labels of the window node's children, in order.
fn window_children(tree: &Tree) -> Vec<String> {
    tree.state()
        .root()
        .children()
        .map(|node| node.label().expect("labeled root"))
        .collect()
}

/// Every published node is reachable from the window, so the consumer's
/// node count equals the semantic node count plus the window itself.
fn assert_connected(tree: &Tree, semantics: &SemanticsTree) {
    fn count(node: Node<'_>) -> usize {
        1 + node.children().map(count).sum::<usize>()
    }
    assert_eq!(count(tree.state().root()), semantics.len() + 1);
}

#[test]
fn every_root_is_a_window_child() {
    let mut semantics = SemanticsTree::new();
    let first = insert(&mut semantics, "first", vec![]);
    let child = insert(&mut semantics, "child", vec![]);
    let second = insert(&mut semantics, "second", vec![child]);
    semantics.set_roots([second, first]);

    let mut projection = AccessKitProjection::new();
    let tree = Tree::new(update(&mut projection, &semantics), true);
    assert_eq!(window_children(&tree), ["second", "first"]);
    assert_connected(&tree, &semantics);
    assert!(projection.sync(&semantics, 1.0).is_none());
}

/// The update shape published before roots were tracked: the whole forest,
/// with only the designated root under the window. This is the panic the
/// native adapter hit.
#[test]
fn a_window_with_only_the_first_root_is_rejected() {
    let mut semantics = SemanticsTree::new();
    let first = insert(&mut semantics, "first", vec![]);
    let second = insert(&mut semantics, "second", vec![]);
    semantics.set_roots([first, second]);

    let mut projection = AccessKitProjection::new();
    let mut single_root = update(&mut projection, &semantics);
    let first = NodeId(projection.native_node_id(first).unwrap());
    let window = &mut single_root
        .nodes
        .iter_mut()
        .find(|(id, _)| *id == single_root.tree.as_ref().unwrap().root)
        .unwrap()
        .1;
    window.set_children(vec![first]);

    let panic = std::panic::catch_unwind(|| Tree::new(single_root, true)).unwrap_err();
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .unwrap_or_default();
    assert!(message.contains("neither in the current tree"), "{message}");
}

#[test]
fn roots_added_reparented_and_removed_incrementally() {
    let mut semantics = SemanticsTree::new();
    let first = insert(&mut semantics, "first", vec![]);
    semantics.set_roots([first]);
    let mut projection = AccessKitProjection::new();
    let mut tree = Tree::new(update(&mut projection, &semantics), true);

    let apply =
        |projection: &mut AccessKitProjection, tree: &mut Tree, semantics: &SemanticsTree| {
            let next = projection
                .sync(semantics, 1.0)
                .expect("root change publishes");
            assert_eq!(next.kind(), AccessKitUpdateKind::Incremental);
            tree.update_and_process_changes(next.into_accesskit(), &mut Ignore);
            assert_connected(tree, semantics);
        };

    let second = insert(&mut semantics, "second", vec![]);
    let third = insert(&mut semantics, "third", vec![]);
    semantics.set_roots([first, second, third]);
    apply(&mut projection, &mut tree, &semantics);
    assert_eq!(window_children(&tree), ["first", "second", "third"]);

    // A root moves under another root without being republished itself.
    semantics.update_geometry(first, rect(0.0, 0.0, 10.0, 10.0), &[second]);
    semantics.set_roots([first, third]);
    apply(&mut projection, &mut tree, &semantics);
    assert_eq!(window_children(&tree), ["first", "third"]);

    semantics.update_geometry(first, rect(0.0, 0.0, 10.0, 10.0), &[]);
    semantics.set_roots([third, second, first]);
    apply(&mut projection, &mut tree, &semantics);
    assert_eq!(window_children(&tree), ["third", "second", "first"]);

    // A removed root's storage slot is reused under a new generation.
    semantics.remove(second);
    apply(&mut projection, &mut tree, &semantics);
    assert_eq!(window_children(&tree), ["third", "first"]);
    let replacement = insert(&mut semantics, "replacement", vec![]);
    assert_ne!(replacement, second);
    semantics.set_roots([third, replacement, first]);
    apply(&mut projection, &mut tree, &semantics);
    assert_eq!(window_children(&tree), ["third", "replacement", "first"]);
    assert_eq!(projection.native_node_id(second), None);

    for id in [first, third, replacement] {
        semantics.remove(id);
    }
    apply(&mut projection, &mut tree, &semantics);
    assert!(window_children(&tree).is_empty());
}

#[test]
fn focus_and_reactivation_reach_every_root() {
    let mut semantics = SemanticsTree::new();
    let first = insert(&mut semantics, "first", vec![]);
    let second = insert(&mut semantics, "second", vec![]);
    let mut focused = semantics.node(second).unwrap().clone();
    focused.state.focused = true;
    semantics.update(second, focused);
    semantics.set_roots([first, second]);

    let mut projection = AccessKitProjection::new();
    let mut tree = Tree::new(update(&mut projection, &semantics), true);
    assert_eq!(
        tree.state()
            .focus()
            .and_then(|node| node.label())
            .as_deref(),
        Some("second")
    );

    projection.deactivate();
    projection.activate();
    let full = projection.sync(&semantics, 1.0).expect("reactivation");
    assert_eq!(full.kind(), AccessKitUpdateKind::Full);
    tree.update_and_process_changes(full.into_accesskit(), &mut Ignore);
    assert_eq!(window_children(&tree), ["first", "second"]);

    let third = insert(&mut semantics, "third", vec![]);
    semantics.set_roots([first, second, third]);
    // A scale change republishes every node alongside the new root.
    tree.update_and_process_changes(
        projection.sync(&semantics, 2.0).unwrap().into_accesskit(),
        &mut Ignore,
    );
    assert_eq!(window_children(&tree), ["first", "second", "third"]);
    assert_connected(&tree, &semantics);
}

#[test]
fn window_bounds_enclose_every_root() {
    let mut semantics = SemanticsTree::new();
    let mut at = |label: &str, bounds: Rect| {
        let id = insert(&mut semantics, label, vec![]);
        let mut node = semantics.node(id).unwrap().clone();
        node.bounds = bounds;
        semantics.update(id, node);
        id
    };
    let top = at("top", rect(10.0, 5.0, 20.0, 10.0));
    let bottom = at("bottom", rect(0.0, 40.0, 50.0, 10.0));
    semantics.set_roots([top, bottom]);

    let mut projection = AccessKitProjection::new();
    let tree = Tree::new(update(&mut projection, &semantics), true);
    let bounds = tree.state().root().bounding_box().expect("window bounds");
    assert_eq!(
        (bounds.x0, bounds.y0, bounds.x1, bounds.y1),
        (0.0, 5.0, 50.0, 50.0)
    );

    // Moving a non-designated root moves the window bounds.
    let mut moved = semantics.node(bottom).unwrap().clone();
    moved.bounds = rect(0.0, 40.0, 80.0, 10.0);
    semantics.update(bottom, moved);
    let next = update(&mut projection, &semantics);
    assert!(next.nodes.iter().any(|(id, _)| *id == NodeId(0)));
}

#[test]
fn mobile_updates_list_every_root() {
    let mut semantics = SemanticsTree::new();
    let first = insert(&mut semantics, "first", vec![]);
    let second = insert(&mut semantics, "second", vec![]);
    semantics.set_roots([first, second]);

    let mut projection = MobileAccessibilityProjection::new();
    let full = projection.sync(&semantics).expect("initial tree");
    let (first_native, second_native) = (
        projection.native_node_id(first).unwrap(),
        projection.native_node_id(second).unwrap(),
    );
    assert_eq!(full.roots, [first_native, second_native]);
    assert_eq!(full.root, Some(first_native));

    // Reordering roots changes no node, but still reaches the bridge.
    semantics.set_roots([second, first]);
    let reordered = projection.sync(&semantics).expect("root order change");
    assert!(reordered.nodes.is_empty());
    assert_eq!(reordered.roots, [second_native, first_native]);
    assert!(projection.sync(&semantics).is_none());

    let snapshot = SemanticsSnapshot::from(&semantics);
    assert_eq!(snapshot.roots, [second, first]);
    assert_eq!(snapshot.root, Some(second));
}

#[test]
fn native_projection_keeps_table_rows_cells_and_headers() {
    let mut semantics = SemanticsTree::new();
    let mut add = |role, label, children| {
        semantics.insert(SemanticNode {
            id: SemanticNodeId(ArenaId::from_parts(0, 0)),
            role,
            label,
            value: None,
            description: None,
            bounds: rect(0., 0., 20., 10.),
            state: SemanticState::default(),
            actions: Vec::new(),
            children,
        })
    };
    let header = add(Role::ColumnHeader, Some("Name".into()), Vec::new());
    let cell = add(Role::Cell, Some("Ada".into()), Vec::new());
    let row = add(Role::Row, None, vec![header, cell]);
    let table = add(Role::Table, None, vec![row]);
    semantics.set_root(Some(table));

    let mut projection = AccessKitProjection::new();
    let native = Tree::new(update(&mut projection, &semantics), true);
    assert_connected(&native, &semantics);
    let table = native.state().root().children().next().unwrap();
    assert_eq!(table.role(), accesskit::Role::Table);
    let row = table.children().next().unwrap();
    assert_eq!(row.role(), accesskit::Role::Row);
    let roles: Vec<_> = row
        .children()
        .map(|node| (node.role(), node.label()))
        .collect();
    assert_eq!(
        roles,
        [
            (accesskit::Role::ColumnHeader, Some("Name".into())),
            (accesskit::Role::Cell, Some("Ada".into()))
        ]
    );
}
