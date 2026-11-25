//! Flutter 3.50 SliverPadding hit regions and unclaimed padding gaps.

use incular_config::{Axis, Constraints, EdgeInsets};
use incular_core::{Rect, Size};
use incular_scroll::ScrollController;
use incular_widgets::internal::{Key, WidgetTree};
use incular_widgets::{
    Directionality, SizedBox, Sliver, SliverPadding, SliverToBoxAdapter, Text, TextDirection,
    Viewport, Widget,
};

fn compare(axis: Axis, reverse: bool) {
    let tags = ["before", "padded", "after"];
    let row = |tag| {
        SliverToBoxAdapter::new(
            SizedBox::new()
                .width(400.)
                .height(400.)
                .child(Widget::from(Text::new(tag)).with_key(Key::from(tag))),
        )
    };
    let slivers: Vec<Box<dyn Sliver>> = vec![
        Box::new(row(tags[0])),
        Box::new(SliverPadding::new(EdgeInsets::all(30.), row(tags[1]))),
        Box::new(row(tags[2])),
    ];
    let controller = ScrollController::new();
    let size = Size::new(800., 600.);
    let view = Constraints::tight(size);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            Directionality::new(
                TextDirection::Ltr,
                Viewport::new(slivers)
                    .axis_direction(axis)
                    .reverse(reverse)
                    .controller(controller.clone()),
            )
            .into(),
        )
        .unwrap();
    tree.layout(view).unwrap();
    assert!(controller.jump_to(350.));
    tree.layout(view).unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    assert_eq!(tree.element_bounds(root).unwrap().size, size);

    let labels = tags.map(|tag| tree.element_with_key(&Key::from(tag)).unwrap());
    let viewport_main = axis.main_extent(size);
    for (label, logical, cross, cross_extent) in [
        (labels[0], -350., 0., axis.cross_extent(size)),
        (labels[1], 80., 30., axis.cross_extent(size) - 60.),
        (labels[2], 510., 0., axis.cross_extent(size)),
    ] {
        let main = if reverse {
            viewport_main - logical - 400.
        } else {
            logical
        };
        let expected =
            Rect::from_origin_size(axis.offset(main, cross), axis.size(400., cross_extent));
        assert_eq!(tree.element_bounds(label).unwrap(), expected);
    }
    for (main, cross, expected) in [
        (10., 10., Some(labels[0])),
        (60., 10., None),
        (100., 100., Some(labels[1])),
        (490., 100., None),
        (520., 10., Some(labels[2])),
    ] {
        let main = if reverse { viewport_main - main } else { main };
        let point = axis.offset(main, cross);
        let element = tree
            .hit_test(point)
            .and_then(|render| tree.element_for_render(render));
        assert_eq!(
            element, expected,
            "{axis:?}, reverse={reverse}, point={point:?}, target={element:?}"
        );
    }
}

#[test]
fn downwards_padding_hit_regions() {
    compare(Axis::Vertical, false);
}
#[test]
fn upwards_padding_hit_regions() {
    compare(Axis::Vertical, true);
}
#[test]
fn rightwards_padding_hit_regions() {
    compare(Axis::Horizontal, false);
}
#[test]
fn leftwards_padding_hit_regions() {
    compare(Axis::Horizontal, true);
}

#[test]
fn higher_level_scroll_view_still_accepts_hits_when_its_sliver_has_no_child() {
    use incular_widgets::CustomScrollView;

    let slivers: Vec<Box<dyn Sliver>> = vec![Box::new(SliverPadding::empty(EdgeInsets::all(30.)))];
    let mut tree = WidgetTree::new();
    let root = tree.mount(CustomScrollView::new(slivers).into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert_eq!(
        tree.hit_test(Axis::Vertical.offset(20., 20.)),
        tree.render_id(root)
    );
}
