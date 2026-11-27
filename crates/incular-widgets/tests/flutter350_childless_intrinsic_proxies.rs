use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_widgets::internal::WidgetTree;
use incular_widgets::{IntrinsicHeight, IntrinsicWidth, Text, Widget};

fn expect_empty(tree: &WidgetTree, root: incular_widgets::internal::ElementId) {
    let id = tree.render_id(root).unwrap();
    assert_eq!(tree.element_count(), 1);
    assert!(tree.children(root).unwrap().is_empty());
    assert_eq!(tree.root_layout_size(), Some(Size::new(5., 8.)));
    assert_eq!(tree.baseline(id), None);
    for cross in [0., 10., 80., f32::INFINITY] {
        assert_eq!(tree.min_intrinsic_width(id, cross), Some(0.));
        assert_eq!(tree.max_intrinsic_width(id, cross), Some(0.));
        assert_eq!(tree.min_intrinsic_height(id, cross), Some(0.));
        assert_eq!(tree.max_intrinsic_height(id, cross), Some(0.));
    }
}

fn verify_empty(widget: Widget) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget).unwrap();
    tree.layout(Constraints::new(5., 500., 8., 800.)).unwrap();
    expect_empty(&tree, root);
}

#[test]
fn width_without_a_child_uses_incoming_minima_but_reports_zero_intrinsics() {
    verify_empty(IntrinsicWidth::builder().build().into());
}

#[test]
fn height_without_a_child_uses_incoming_minima_but_reports_zero_intrinsics() {
    verify_empty(IntrinsicHeight::default().into());
}

#[test]
fn removing_a_baselined_child_leaves_the_same_proxy_without_a_baseline() {
    let child = || Text::new("Baseline").text_direction(TextDirection::Ltr);
    let mut tree = WidgetTree::new();
    let root = tree.mount(IntrinsicHeight::new(child()).into()).unwrap();
    let id = tree.render_id(root).unwrap();
    let constraints = Constraints::new(5., 500., 8., 800.);
    tree.layout(constraints).unwrap();
    assert!(tree.baseline(id).is_some());

    tree.update(root, IntrinsicHeight::empty().into()).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(tree.render_id(root), Some(id));
    expect_empty(&tree, root);

    tree.update(root, IntrinsicHeight::new(child()).into())
        .unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(tree.render_id(root), Some(id));
    assert!(tree.baseline(id).is_some());
}
