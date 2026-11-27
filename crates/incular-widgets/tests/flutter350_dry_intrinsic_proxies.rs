use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::internal::{Key, WidgetTree};
use incular_widgets::{
    AspectRatio, IntrinsicHeight, IntrinsicWidth, MaxIntrinsicSize, SizedBox, Widget,
};

const CHILD: Key = Key::Value(801);

fn custom_box() -> MaxIntrinsicSize {
    let child = SizedBox::builder().width(55.).height(110.).build();
    MaxIntrinsicSize::new(Size::new(100., 200.), Widget::from(child).with_key(CHILD))
        .with_minimum(Size::new(10., 20.))
}

fn verify(parent: Widget, layout: Size, widths: [f32; 2], heights: [f32; 2]) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(parent).unwrap();
    let id = tree.render_id(root).unwrap();
    tree.layout(Constraints::new(5., 500., 8., 800.)).unwrap();
    assert_eq!(tree.root_layout_size(), Some(layout));
    let child = tree.element_with_key(&CHILD).unwrap();
    assert_eq!(tree.element_bounds(child).unwrap().size, layout);

    for cross in [0., 10., 80., f32::INFINITY] {
        assert_eq!(tree.min_intrinsic_width(id, cross), Some(widths[0]));
        assert_eq!(tree.max_intrinsic_width(id, cross), Some(widths[1]));
        assert_eq!(tree.min_intrinsic_height(id, cross), Some(heights[0]));
        assert_eq!(tree.max_intrinsic_height(id, cross), Some(heights[1]));
    }
    assert_eq!(tree.root_layout_size(), Some(layout));
}

#[test]
fn width_shrink_wraps_to_the_maximum_and_queries_the_height_range() {
    verify(
        IntrinsicWidth::new(custom_box()).into(),
        Size::new(100., 110.),
        [100.; 2],
        [20., 200.],
    );
}

#[test]
fn width_step_rounds_the_width_but_not_height_queries() {
    verify(
        IntrinsicWidth::new(custom_box()).step_width(47.).into(),
        Size::new(141., 110.),
        [141.; 2],
        [20., 200.],
    );
}

#[test]
fn height_step_rounds_height_measurements_independently() {
    verify(
        IntrinsicWidth::new(custom_box()).step_height(47.).into(),
        Size::new(100., 235.),
        [100.; 2],
        [47., 235.],
    );
}

#[test]
fn both_steps_round_independent_axes() {
    let proxy = IntrinsicWidth::new(custom_box())
        .step_width(37.)
        .step_height(47.);
    verify(proxy.into(), Size::new(111., 235.), [111.; 2], [47., 235.]);
}

#[test]
fn height_shrink_wraps_to_the_maximum_and_queries_the_width_range() {
    verify(
        IntrinsicHeight::new(custom_box()).into(),
        Size::new(55., 200.),
        [10., 100.],
        [200.; 2],
    );
}

#[test]
fn unbounded_query_uses_the_measured_perpendicular_extent() {
    let child = || AspectRatio::new(2., SizedBox::builder().width(100.).height(40.).build());
    let mut tree = WidgetTree::new();
    let proxy = IntrinsicWidth::new(child())
        .step_width(47.)
        .step_height(10.);
    let root = tree.mount(proxy.into()).unwrap();
    let id = tree.render_id(root).unwrap();
    assert_eq!(tree.min_intrinsic_height(id, f32::INFINITY), Some(80.));
    assert_eq!(tree.max_intrinsic_height(id, f32::INFINITY), Some(80.));
    assert_eq!(tree.max_intrinsic_height(id, 120.), Some(60.));

    let mut tree = WidgetTree::new();
    let root = tree.mount(IntrinsicHeight::new(child()).into()).unwrap();
    let id = tree.render_id(root).unwrap();
    assert_eq!(tree.min_intrinsic_width(id, f32::INFINITY), Some(80.));
    assert_eq!(tree.max_intrinsic_width(id, f32::INFINITY), Some(80.));
    assert_eq!(tree.max_intrinsic_width(id, 30.), Some(60.));
}
