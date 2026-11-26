//! Flutter 3.50.0 SliverConstrainedCrossAxis and fixed cross-axis lanes.
use std::time::Instant;

use incular_config::{Axis, Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_scroll::{ScrollController, SliverConstraints};
use incular_widgets::{
    CustomScrollView, Directionality, GlobalKey, SizedBox, Sliver, SliverConstrainedCrossAxis,
    SliverCrossAxisExpanded, SliverCrossAxisGroup, SliverFixedExtentList, SliverToBoxAdapter,
    Widget, internal::WidgetTree,
};

const VIEW: Size = Size {
    width: 300.,
    height: 500.,
};

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(VIEW)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
}

fn bounds(tree: &WidgetTree, key: GlobalKey) -> Rect {
    tree.element_bounds(tree.element_with_key(&key.into()).unwrap())
        .unwrap()
}

#[test]
fn constrained_sliver_caps_cross_extent_without_reducing_main_paint_extent() {
    let child = SliverToBoxAdapter::new(SizedBox::new().height(100.));
    let sliver = SliverConstrainedCrossAxis::new(50., child);
    assert_eq!(sliver.cross_axis_flex(), Some(0));
    let mut renderer = sliver.create_render_sliver(&ScrollController::new(), Axis::Vertical, false);
    let constraints = SliverConstraints::new(
        Axis::Vertical,
        false,
        0.,
        0.,
        0.,
        VIEW.height,
        VIEW.width,
        VIEW.height,
        750.,
        0.,
    );
    let initial = renderer.perform_layout(constraints);
    assert_eq!(initial.children.len(), 1);
    let child = &initial.children[0];
    let mut box_tree = WidgetTree::new();
    let id = box_tree.mount(child.widget.clone()).unwrap();
    box_tree.layout(child.constraints).unwrap();
    let size = box_tree
        .render_size(box_tree.render_id(id).unwrap())
        .unwrap();
    assert_eq!(size, Size::new(50., 100.));
    assert!(renderer.set_child_extent(child.id, size.height));

    let layout = renderer.perform_layout(constraints);
    assert_eq!(renderer.reported_cross_axis_extent(), Some(50.));
    assert_eq!(layout.geometry.paint_extent, 100.);
    assert_eq!(layout.geometry.scroll_extent, 100.);
}

#[test]
fn cross_extent_changes_in_place_and_cannot_exceed_the_viewport() {
    let key = GlobalKey::new();
    let controller = ScrollController::new();
    let scene = |max, axis: Axis| {
        let leaf = if axis.is_vertical() {
            SizedBox::new().height(100.)
        } else {
            SizedBox::new().width(100.)
        };
        let leaf = Widget::from(leaf).with_key(key);
        let sliver = SliverConstrainedCrossAxis::new(max, SliverToBoxAdapter::new(leaf));
        Widget::from(Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(vec![Box::new(sliver) as Box<dyn Sliver>])
                .scroll_direction(axis)
                .controller(controller.clone()),
        ))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(50., Axis::Vertical)).unwrap();
    layout(&mut tree);
    let first = tree.element_with_key(&key.into());
    assert_eq!(bounds(&tree, key).size, Size::new(50., 100.));

    for (max, axis, expected) in [
        (80., Axis::Vertical, Size::new(80., 100.)),
        (400., Axis::Vertical, Size::new(300., 100.)),
        (50., Axis::Horizontal, Size::new(100., 50.)),
    ] {
        tree.update(root, scene(max, axis)).unwrap();
        layout(&mut tree);
        assert_eq!(bounds(&tree, key).size, expected);
        assert_eq!(tree.element_with_key(&key.into()), first);
    }
}

fn lane(key: GlobalKey, height: f32) -> SliverFixedExtentList {
    SliverFixedExtentList::new(20, height, move |index| {
        let row = Widget::from(SizedBox::new().height(height));
        if index == 0 { row.with_key(key) } else { row }
    })
}

fn group(slivers: Vec<Box<dyn Sliver>>) -> WidgetTree {
    let group = SliverCrossAxisGroup::new(slivers);
    let mut tree = WidgetTree::new();
    tree.mount(
        Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(vec![Box::new(group) as Box<dyn Sliver>]),
        )
        .into(),
    )
    .unwrap();
    layout(&mut tree);
    tree
}

#[test]
fn fixed_lanes_use_reported_width_and_flex_only_divides_the_remainder() {
    let a = GlobalKey::new();
    let b = GlobalKey::new();
    let tree = group(vec![
        Box::new(SliverConstrainedCrossAxis::new(60., lane(a, 300.))),
        Box::new(SliverConstrainedCrossAxis::new(120., lane(b, 200.))),
    ]);
    assert_eq!(
        bounds(&tree, a),
        Rect::from_origin_size(Offset::ZERO, Size::new(60., 300.))
    );
    assert_eq!(
        bounds(&tree, b),
        Rect::from_origin_size(Offset::new(60., 0.), Size::new(120., 200.))
    );

    let c = GlobalKey::new();
    let d = GlobalKey::new();
    let mixed = group(vec![
        Box::new(SliverCrossAxisExpanded::new(1, lane(c, 100.))),
        Box::new(SliverConstrainedCrossAxis::new(60., lane(a, 100.))),
        Box::new(SliverCrossAxisExpanded::new(2, lane(d, 100.))),
        Box::new(SliverConstrainedCrossAxis::new(120., lane(b, 100.))),
    ]);
    for (key, x, width) in [(c, 0., 40.), (a, 40., 60.), (d, 100., 80.), (b, 180., 120.)] {
        assert_eq!(
            bounds(&mixed, key),
            Rect::from_origin_size(Offset::new(x, 0.), Size::new(width, 100.))
        );
    }
}
