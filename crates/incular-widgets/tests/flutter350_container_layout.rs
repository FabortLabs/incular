use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::Size;
use incular_widgets::{
    Align, BorderRadius, BoxDecoration, Container, DecoratedBox, Directionality, ListView, Radius,
    SizedBox, Text, Widget,
    internal::{ElementId, Key, RenderKind, WidgetTree},
};

fn layout(widget: Widget, constraints: Constraints) -> (WidgetTree, ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget).expect("mount");
    tree.layout(constraints).expect("layout");
    let _ = tree.paint();
    (tree, root)
}

fn has_align(tree: &WidgetTree, node: ElementId) -> bool {
    matches!(
        tree.render_id(node)
            .and_then(|id| tree.render_object_kind(id)),
        Some(RenderKind::Align { .. })
    ) || tree
        .children(node)
        .is_some_and(|children| children.iter().any(|child| has_align(tree, *child)))
}

#[test]
fn childless_nontight_container_does_not_insert_an_align_widget() {
    let rounded = BoxDecoration::new().border_radius(BorderRadius::all(Radius::circular(1.0)));
    let (tree, root) = layout(
        Container::new()
            .decoration(rounded.clone())
            .alignment(Alignment::CENTER_LEFT)
            .into(),
        Constraints::tight(Size::new(100.0, 60.0)),
    );
    assert!(!has_align(&tree, root));

    let (tree, root) = layout(
        Container::new()
            .decoration(rounded)
            .width(40.0)
            .height(30.0)
            .alignment(Alignment::CENTER_LEFT)
            .into(),
        Constraints::loose(Size::new(100.0, 60.0)),
    );
    assert!(
        has_align(&tree, root),
        "tight explicit sizing keeps the Align"
    );
}

#[test]
fn childless_container_fills_the_other_axis_when_only_width_is_fixed() {
    let (tree, root) = layout(
        Container::new().width(40.0).into(),
        Constraints::loose(Size::new(100.0, 60.0)),
    );
    assert_eq!(
        tree.element_geometry(root).unwrap().1,
        Size::new(40.0, 60.0)
    );
}

#[test]
fn an_empty_container_in_a_vertical_list_finishes_layout() {
    let row = Key::from("childless list row");
    let widget = Widget::from(Container::new()).with_key(row.clone());
    let list = Directionality::new(TextDirection::Ltr, ListView::new([widget]));
    let (tree, _) = layout(list.into(), Constraints::tight(Size::new(100.0, 60.0)));
    let row = tree.element_with_key(&row).expect("list row");
    assert_eq!(tree.element_geometry(row).unwrap().1, Size::new(100.0, 0.0));
}

fn zero_sized_decoration_is_laid_out(decorated: impl Into<Widget>) {
    let key = Key::from("decorated");
    let centered = Align::new(
        Alignment::CENTER,
        SizedBox::shrink().child(decorated.into().with_key(key.clone())),
    );
    let (tree, _) = layout(
        Directionality::new(TextDirection::Ltr, centered).into(),
        Constraints::tight(Size::new(100.0, 60.0)),
    );
    let decorated = tree.element_with_key(&key).expect("decorated");
    assert_eq!(tree.element_geometry(decorated).unwrap().1, Size::ZERO);
}

#[test]
fn a_decorated_container_can_size_text_to_zero() {
    zero_sized_decoration_is_laid_out(
        Container::new()
            .decoration(BoxDecoration::default())
            .child(Text::new("X")),
    );
}

#[test]
fn a_decorated_box_can_be_zero_sized() {
    zero_sized_decoration_is_laid_out(
        DecoratedBox::new(SizedBox::shrink()).decoration(BoxDecoration::default()),
    );
}
