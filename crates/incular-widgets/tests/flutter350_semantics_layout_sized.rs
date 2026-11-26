use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_semantics::SemanticNode;
use incular_widgets::{
    Align, BlockSemantics, IndexedStack, Semantics, SizedBox, Stack, Widget,
    internal::{ElementId, Key, WidgetTree},
};

const LEAF: Key = Key::Value(10);
const BACK: Key = Key::Value(20);
const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn box_at_origin(child: impl Into<Widget>) -> Align {
    Align::new(Alignment::TOP_LEFT, child)
}

fn boundary(label: &str, child: impl Into<Widget>) -> Semantics {
    Semantics::new(child)
        .label(label)
        .container(true)
        .explicit_child_nodes(true)
}

fn leaf(label: &str, extent: f32) -> Widget {
    Widget::from(boundary(label, SizedBox::square(extent))).with_key(LEAF)
}

fn update(tree: &mut WidgetTree, root: ElementId, widget: impl Into<Widget>) {
    tree.update(root, widget.into()).unwrap();
    flush(tree);
}

fn flush(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(VIEW)).unwrap();
    tree.update_semantics();
}

fn mount(widget: impl Into<Widget>) -> (WidgetTree, ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget.into()).unwrap();
    flush(&mut tree);
    (tree, root)
}

fn node<'a>(tree: &'a WidgetTree, label: &str) -> Option<&'a SemanticNode> {
    tree.semantics()
        .iter()
        .map(|(_, value)| value)
        .find(|value| value.label.as_deref() == Some(label))
}

fn leaf_bounds(size: f32) -> Rect {
    Rect::from_origin_size(Offset::ZERO, Size::new(size, size))
}

#[test]
fn a_semantic_boundary_reuses_its_node_but_updates_its_layout_size() {
    let view = |size| box_at_origin(leaf("child", size));
    let (mut tree, root) = mount(view(100.));
    let element = tree.element_with_key(&LEAF).unwrap();
    let render = tree.render_id(element).unwrap();
    let old_semantics = node(&tree, "child").unwrap().id;
    assert_eq!(tree.element_bounds(element), Some(leaf_bounds(100.)));
    assert_eq!(node(&tree, "child").unwrap().bounds, leaf_bounds(100.));
    update(&mut tree, root, view(200.));
    assert_eq!(
        tree.render_id(tree.element_with_key(&LEAF).unwrap()),
        Some(render)
    );
    assert_eq!(tree.element_bounds(element), Some(leaf_bounds(200.)));
    let updated = node(&tree, "child").unwrap();
    assert_eq!(updated.id, old_semantics);
    assert_eq!(updated.bounds, leaf_bounds(200.));
}

#[test]
fn blocking_a_branch_while_its_boundaries_relayout_does_not_put_it_back() {
    let view = |size, blocked| {
        let behind = box_at_origin(boundary("middle", box_at_origin(leaf("child", size))));
        let mut children = vec![Widget::from(behind).with_key(BACK)];
        if blocked {
            children.push(
                BlockSemantics::new(box_at_origin(boundary("front", SizedBox::square(10.)))).into(),
            );
        }
        boundary(
            "parent",
            Stack::new(children).text_direction(TextDirection::Ltr),
        )
    };
    let (mut tree, root) = mount(view(100., false));
    assert_eq!(node(&tree, "child").unwrap().bounds, leaf_bounds(100.));
    let element = tree.element_with_key(&LEAF).unwrap();
    update(&mut tree, root, view(120., true));
    assert!(node(&tree, "front").is_some());
    assert!(node(&tree, "child").is_none());
    update(&mut tree, root, view(160., true));
    assert!(node(&tree, "middle").is_none());
    assert!(node(&tree, "child").is_none());
    assert_eq!(tree.element_bounds(element), Some(leaf_bounds(160.)));
}

#[test]
fn a_semantic_child_shrinking_to_zero_does_not_leave_old_hit_geometry() {
    let view = |size| boundary("outer", box_at_origin(leaf("inner", size)));
    let (mut tree, root) = mount(view(100.));
    assert_eq!(node(&tree, "inner").unwrap().bounds, leaf_bounds(100.));
    update(&mut tree, root, view(0.));
    let leaf = tree.element_with_key(&LEAF).unwrap();
    assert_eq!(tree.element_bounds(leaf), Some(leaf_bounds(0.)));
    assert!(node(&tree, "inner").is_none_or(|node| node.bounds == leaf_bounds(0.)));
}

fn last_exposed(extent: f32, replacement: bool) -> Semantics {
    let mut children = vec![
        Widget::from(boundary(
            "original",
            box_at_origin(leaf("old child", extent)),
        ))
        .with_key(BACK),
    ];
    if replacement {
        children.push(boundary("new child", SizedBox::square(20.)).into());
    }
    boundary(
        "root",
        IndexedStack::new(children)
            .index(usize::from(replacement))
            .text_direction(TextDirection::Ltr),
    )
}

fn child_count(tree: &WidgetTree) -> usize {
    node(tree, "root").unwrap().children.len()
}

#[test]
fn dirty_children_of_an_unvisited_branch_stay_hidden_until_it_is_selected_again() {
    let (mut tree, root) = mount(last_exposed(100., false));
    let element = tree.element_with_key(&LEAF).unwrap();
    assert_eq!(child_count(&tree), 1);
    assert!(node(&tree, "original").is_some());
    update(&mut tree, root, last_exposed(100., true));
    assert!(node(&tree, "original").is_none());
    assert!(node(&tree, "new child").is_some());
    update(&mut tree, root, last_exposed(160., true));
    assert_eq!(child_count(&tree), 1);
    assert!(node(&tree, "old child").is_none());
    assert_eq!(
        tree.element_bounds(element).unwrap().size,
        Size::new(160., 160.)
    );
    update(&mut tree, root, last_exposed(160., false));
    assert_eq!(tree.element_with_key(&LEAF), Some(element));
    assert_eq!(child_count(&tree), 1);
    assert!(node(&tree, "original").is_some());
    assert!(node(&tree, "new child").is_none());
    assert_eq!(
        node(&tree, "old child").unwrap().bounds.size,
        Size::new(160., 160.)
    );
}

#[test]
fn a_zero_size_child_can_be_dropped_with_its_dirty_parent() {
    let (mut tree, root) = mount(last_exposed(0., false));
    let element = tree.element_with_key(&LEAF).unwrap();
    assert_eq!(child_count(&tree), 1);
    assert!(node(&tree, "original").is_some());
    update(&mut tree, root, last_exposed(0., true));
    assert_eq!(child_count(&tree), 1);
    assert!(node(&tree, "new child").is_some());
    assert!(node(&tree, "original").is_none());
    assert!(node(&tree, "old child").is_none());
    assert_eq!(tree.element_with_key(&LEAF), Some(element));
}
