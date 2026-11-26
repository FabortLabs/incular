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

fn directional_group(
    axis: Axis,
    reverse: bool,
    slivers: Vec<Box<dyn Sliver>>,
    following: Vec<Box<dyn Sliver>>,
) -> (WidgetTree, ScrollController) {
    let controller = ScrollController::new();
    let mut tree = WidgetTree::new();
    let slivers = std::iter::once(Box::new(SliverCrossAxisGroup::new(slivers)) as Box<dyn Sliver>)
        .chain(following);
    tree.mount(
        Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(slivers)
                .scroll_direction(axis)
                .reverse(reverse)
                .controller(controller.clone()),
        )
        .into(),
    )
    .unwrap();
    layout_group(&mut tree);
    (tree, controller)
}

const GROUP_VIEW: Size = Size {
    width: 300.,
    height: 600.,
};

fn layout_group(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(GROUP_VIEW)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
}

fn variable_lane(axis: Axis, key: GlobalKey, extent: f32) -> incular_widgets::SliverList {
    incular_widgets::SliverList::builder(20, move |index| {
        let child = if axis.is_vertical() {
            SizedBox::new().height(extent)
        } else {
            SizedBox::new().width(extent)
        };
        let child = Widget::from(child);
        if index == 0 {
            child.with_key(key)
        } else {
            child
        }
    })
}

#[test]
fn constrained_and_flex_lanes_keep_cross_axis_order_on_both_scroll_axes() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            let keys = [GlobalKey::new(), GlobalKey::new(), GlobalKey::new()];
            let first = variable_lane(axis, keys[0], 300.);
            let second = variable_lane(axis, keys[1], 200.);
            let third = variable_lane(axis, keys[2], 200.);
            let (tree, controller) = directional_group(
                axis,
                reverse,
                vec![
                    Box::new(SliverConstrainedCrossAxis::new(30., first)),
                    Box::new(SliverCrossAxisExpanded::new(2, second)),
                    Box::new(third),
                ],
                vec![],
            );
            let widths = if axis.is_vertical() {
                [30., 180., 90.]
            } else {
                [30., 380., 190.]
            };
            let mut cross = 0.;
            for ((key, main), width) in keys.into_iter().zip([300., 200., 200.]).zip(widths) {
                let leading = if reverse {
                    axis.main_extent(GROUP_VIEW) - main
                } else {
                    0.
                };
                assert_eq!(
                    bounds(&tree, key),
                    Rect::from_origin_size(axis.offset(leading, cross), axis.size(main, width)),
                    "{axis:?}, reverse={reverse}, lane starts at {cross}",
                );
                cross += width;
            }
            assert_eq!(
                controller.content_extent(),
                6000.,
                "{axis:?}, reverse={reverse}"
            );
        }
    }
}

#[test]
fn final_fixed_lane_is_capped_at_the_remaining_cross_extent() {
    let a = GlobalKey::new();
    let b = GlobalKey::new();
    let (tree, controller) = directional_group(
        Axis::Vertical,
        false,
        vec![
            Box::new(SliverConstrainedCrossAxis::new(
                200.,
                variable_lane(Axis::Vertical, a, 300.),
            )),
            Box::new(SliverConstrainedCrossAxis::new(
                200.,
                variable_lane(Axis::Vertical, b, 200.),
            )),
        ],
        vec![],
    );
    assert_eq!(
        bounds(&tree, a),
        Rect::from_origin_size(Offset::ZERO, Size::new(200., 300.))
    );
    assert_eq!(
        bounds(&tree, b),
        Rect::from_origin_size(Offset::new(200., 0.), Size::new(100., 200.))
    );
    assert_eq!(controller.content_extent(), 6000.);
}

#[test]
fn flex_weights_divide_the_cross_extent_without_changing_scroll_range() {
    let a = GlobalKey::new();
    let b = GlobalKey::new();
    let (tree, controller) = directional_group(
        Axis::Vertical,
        false,
        vec![
            Box::new(SliverCrossAxisExpanded::new(
                3,
                variable_lane(Axis::Vertical, a, 300.),
            )),
            Box::new(SliverCrossAxisExpanded::new(
                2,
                variable_lane(Axis::Vertical, b, 200.),
            )),
        ],
        vec![],
    );
    assert_eq!(
        bounds(&tree, a),
        Rect::from_origin_size(Offset::ZERO, Size::new(180., 300.))
    );
    assert_eq!(
        bounds(&tree, b),
        Rect::from_origin_size(Offset::new(180., 0.), Size::new(120., 200.))
    );
    assert_eq!(controller.content_extent(), 6000.);
}

#[test]
fn unweighted_lanes_share_cross_extent_and_jump_to_the_longer_lanes_end() {
    let a: [GlobalKey; 20] = std::array::from_fn(|_| GlobalKey::new());
    let b: [GlobalKey; 20] = std::array::from_fn(|_| GlobalKey::new());
    let list = |keys: [GlobalKey; 20], height| {
        incular_widgets::SliverList::builder(keys.len(), move |i| {
            Widget::from(SizedBox::new().height(height)).with_key(keys[i])
        })
    };
    let (mut tree, controller) = directional_group(
        Axis::Vertical,
        false,
        vec![Box::new(list(a, 300.)), Box::new(list(b, 200.))],
        vec![],
    );
    assert_eq!(controller.offset(), 0.);
    assert_eq!(controller.content_extent(), 6000.);
    let viewport = Rect::from_origin_size(Offset::ZERO, GROUP_VIEW);
    let visible = |tree: &WidgetTree, key: GlobalKey| {
        tree.element_with_key(&key.into())
            .and_then(|id| tree.element_bounds(id))
            .is_some_and(|rect| rect.intersects(viewport))
    };
    for (key, expected) in [
        (a[0], true),
        (a[1], true),
        (a[2], false),
        (b[0], true),
        (b[2], true),
        (b[3], false),
    ] {
        assert_eq!(visible(&tree, key), expected);
    }
    assert_eq!(
        bounds(&tree, a[0]),
        Rect::from_origin_size(Offset::ZERO, Size::new(150., 300.))
    );
    assert_eq!(
        bounds(&tree, b[0]),
        Rect::from_origin_size(Offset::new(150., 0.), Size::new(150., 200.))
    );

    assert!(controller.jump_to(5400.));
    layout_group(&mut tree);
    assert_eq!(controller.offset(), 5400.);
    assert_eq!(controller.content_extent(), 6000.);
    assert!(!visible(&tree, a[17]));
    assert_eq!(
        bounds(&tree, a[18]),
        Rect::from_origin_size(Offset::ZERO, Size::new(150., 300.))
    );
    assert_eq!(
        bounds(&tree, a[19]),
        Rect::from_origin_size(Offset::new(0., 300.), Size::new(150., 300.))
    );
    assert!(!visible(&tree, b[19]));
}
