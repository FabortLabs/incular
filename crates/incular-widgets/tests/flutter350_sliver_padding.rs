//! Flutter 3.50 SliverPadding geometry, constraints, and retained placement.

use std::{cell::Cell, rc::Rc};

use incular_config::{Axis, Constraints, EdgeInsets, EdgeInsetsDirectional, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_scroll::{ScrollController, SliverConstraints, SliverGeometry};
use incular_widgets::internal::{Key, RenderSliver, SliverLayout, WidgetTree};
use incular_widgets::{
    CustomScrollView, Directionality, EdgeInsetsGeometry, SizedBox, Sliver, SliverFillRemaining,
    SliverPadding, SliverToBoxAdapter, Viewport, Widget,
};

struct Probe {
    constraints: Rc<Cell<Option<SliverConstraints>>>,
    geometry: SliverGeometry,
}

impl Sliver for Probe {
    fn build(&self, _: &ScrollController) -> Widget {
        SizedBox::shrink().into()
    }

    fn create_render_sliver(
        &self,
        _: &ScrollController,
        _: Axis,
        _: bool,
    ) -> Box<dyn RenderSliver> {
        Box::new(Self {
            constraints: self.constraints.clone(),
            geometry: self.geometry,
        })
    }
}

impl RenderSliver for Probe {
    fn perform_layout(&mut self, constraints: SliverConstraints) -> SliverLayout {
        self.constraints.set(Some(constraints));
        SliverLayout {
            geometry: self.geometry,
            children: Vec::new(),
            absorbed_overlap: 0.,
        }
    }
}

fn constraints(axis: Axis, reverse: bool, overlap: f32) -> SliverConstraints {
    SliverConstraints::new(axis, reverse, 0., 0., overlap, 100., 100., 100., 100., 0.)
}

fn probe(
    context: SliverConstraints,
    padding: EdgeInsets,
    geometry: SliverGeometry,
) -> (SliverConstraints, SliverGeometry) {
    let recorded = Rc::new(Cell::new(None));
    let sliver = SliverPadding::new(
        padding,
        Probe {
            constraints: recorded.clone(),
            geometry,
        },
    );
    let mut render =
        sliver.create_render_sliver(&ScrollController::new(), context.axis, context.reverse);
    let result = render.perform_layout(context).geometry;
    (recorded.get().expect("inner sliver laid out"), result)
}

const MOCK: SliverGeometry = SliverGeometry {
    paint_origin: 10.,
    paint_extent: 10.,
    max_paint_extent: 10.,
    layout_extent: 10.,
    hit_test_extent: 10.,
    cache_extent: 10.,
    visible: true,
    ..SliverGeometry::ZERO
};

#[test]
fn positive_overlap_only_loses_the_visible_leading_padding() {
    let (inner, _) = probe(
        constraints(Axis::Vertical, false, 100.),
        EdgeInsets::only(0., 20., 0., 0.),
        MOCK,
    );
    assert_eq!(inner.overlap, 80.);
}

#[test]
fn negative_overlap_reaches_the_child_unchanged() {
    let (inner, _) = probe(
        constraints(Axis::Vertical, false, -100.),
        EdgeInsets::only(0., 20., 0., 0.),
        MOCK,
    );
    assert_eq!(inner.overlap, -100.);
}

#[test]
fn padding_forwards_the_render_slivers_paint_origin() {
    let (_, outer) = probe(
        constraints(Axis::Vertical, false, 100.),
        EdgeInsets::only(0., 20., 0., 0.),
        MOCK,
    );
    assert_eq!(outer.paint_origin, 10.);
}

#[test]
fn fill_remaining_receives_preceding_padding_before_it_measures_the_box() {
    let key = Key::from("fill");
    let fill = SliverFillRemaining::new(
        Widget::box_(Size::ZERO, Color::rgba(255, 0, 0, 255)).with_key(key.clone()),
    )
    .has_scroll_body(false);
    let slivers: Vec<Box<dyn Sliver>> = vec![Box::new(SliverPadding::new(
        EdgeInsets::only(0., 30., 0., 0.),
        fill,
    ))];
    let mut tree = WidgetTree::new();
    tree.mount(CustomScrollView::new(slivers).into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let child = tree.element_with_key(&key).expect("fill box mounted");
    assert_eq!(
        tree.element_bounds(child).unwrap(),
        Rect::from_origin_size(Offset::new(0., 30.), Size::new(800., 570.))
    );
}

#[test]
fn reversed_growth_consumes_physical_bottom_or_right_padding_first() {
    for (axis, reverse, before) in [
        (Axis::Vertical, false, 20.),
        (Axis::Vertical, true, 40.),
        (Axis::Horizontal, false, 11.),
        (Axis::Horizontal, true, 31.),
    ] {
        let (inner, outer) = probe(
            constraints(axis, reverse, 100.),
            EdgeInsets::only(11., 20., 31., 40.),
            MOCK,
        );
        assert_eq!(
            inner.preceding_scroll_extent, before,
            "{axis:?}, reverse={reverse}"
        );
        assert_eq!(inner.overlap, 100. - before);
        assert_eq!(inner.remaining_paint_extent, 100. - before);
        assert_eq!(outer.paint_origin, 10.);
    }
}

fn padding_then_box(target: Key) -> Vec<Box<dyn Sliver>> {
    use incular_widgets::SliverToBoxAdapter;
    vec![
        Box::new(SliverPadding::empty(EdgeInsets::all(100.))),
        Box::new(SliverToBoxAdapter::new(
            Widget::box_(Size::new(400., 400.), Color::WHITE).with_key(target),
        )),
    ]
}

#[test]
fn padding_without_a_child_still_moves_following_slivers() {
    use incular_widgets::Viewport;

    let key = Key::from("after empty padding");
    let controller = ScrollController::new();
    let mut tree = WidgetTree::new();
    tree.mount(
        Viewport::new(padding_then_box(key.clone()))
            .controller(controller)
            .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let child = tree.element_with_key(&key).unwrap();
    assert_eq!(
        tree.element_bounds(child).unwrap().origin,
        Offset::new(0., 200.)
    );
    assert_eq!(tree.children(tree.root().unwrap()).unwrap(), [child]);
}

#[test]
fn paint_extent_of_empty_padding_shrinks_by_the_scrolled_amount() {
    let key = Key::from("after empty padding");
    let controller = ScrollController::new();
    let mut tree = WidgetTree::new();
    tree.mount(
        CustomScrollView::new(padding_then_box(key.clone()))
            .controller(controller.clone())
            .into(),
    )
    .unwrap();
    let view = Constraints::tight(Size::new(800., 600.));
    tree.layout(view).unwrap();
    let child = tree.element_with_key(&key).unwrap();
    assert_eq!(
        tree.element_bounds(child).unwrap().origin,
        Offset::new(0., 200.)
    );

    let padding = SliverPadding::builder()
        .padding(EdgeInsets::all(100.))
        .build();
    let mut render = padding.create_render_sliver(&controller, Axis::Vertical, false);
    let mut parent = SliverConstraints::new(
        Axis::Vertical,
        false,
        0.,
        0.,
        0.,
        600.,
        800.,
        600.,
        600.,
        0.,
    );
    let initial = render.perform_layout(parent);
    assert_eq!(initial.geometry.paint_extent, 200.);
    assert!(initial.children.is_empty());

    assert!(controller.jump_to(50.));
    tree.layout(view).unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    assert_eq!(
        tree.element_bounds(child).unwrap().origin,
        Offset::new(0., 150.)
    );
    parent.scroll_offset = 50.;
    let scrolled = render.perform_layout(parent);
    assert_eq!(scrolled.geometry.paint_extent, 150.);
    assert!(scrolled.children.is_empty());
}

#[test]
fn padding_forwards_an_actual_child_scroll_correction_without_extra_geometry() {
    let (_, outer) = probe(
        constraints(Axis::Vertical, false, 0.),
        EdgeInsets::only(10., 30., 20., 40.),
        SliverGeometry {
            scroll_offset_correction: Some(200.),
            ..SliverGeometry::ZERO
        },
    );
    assert_eq!(
        outer,
        SliverGeometry {
            scroll_offset_correction: Some(200.),
            ..SliverGeometry::ZERO
        },
    );
}

#[test]
fn replacing_childless_padding_in_a_leftward_viewport_moves_the_following_box() {
    use incular_widgets::{SliverToBoxAdapter, Viewport};

    let key = Key::from("horizontally padded");
    let scene = |insets| {
        let slivers: Vec<Box<dyn Sliver>> = vec![
            Box::new(SliverPadding::empty(insets)),
            Box::new(SliverToBoxAdapter::new(
                Widget::box_(Size::new(201., 20.), Color::WHITE).with_key(key.clone()),
            )),
        ];
        Viewport::new(slivers)
            .axis_direction(Axis::Horizontal)
            .reverse(true)
            .controller(ScrollController::new())
            .into()
    };
    let mut tree = WidgetTree::new();
    let view = Constraints::tight(Size::new(800., 600.));
    let root = tree
        .mount(scene(EdgeInsets::only(90., 1., 110., 2.)))
        .unwrap();
    tree.layout(view).unwrap();
    let child = tree.element_with_key(&key).unwrap();
    assert_eq!(
        tree.element_bounds(child).unwrap().origin,
        Offset::new(399., 0.)
    );

    tree.update(root, scene(EdgeInsets::only(110., 1., 80., 2.)))
        .unwrap();
    tree.layout(view).unwrap();
    assert_eq!(
        tree.element_bounds(child).unwrap().origin,
        Offset::new(409., 0.)
    );
}

#[cfg(debug_assertions)]
#[test]
fn replacing_padding_with_negative_edges_trips_the_debug_contract() {
    use incular_widgets::Viewport;

    let controller = ScrollController::new();
    let scene = |insets| {
        let slivers: Vec<Box<dyn Sliver>> = vec![Box::new(SliverPadding::empty(insets))];
        Viewport::new(slivers).controller(controller.clone()).into()
    };
    let mut tree = WidgetTree::new();
    let view = Constraints::tight(Size::new(800., 600.));
    let root = tree.mount(scene(EdgeInsets::ZERO)).unwrap();
    tree.layout(view).unwrap();

    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tree.update(root, scene(EdgeInsets::all(-1.))).unwrap();
        tree.layout(view).unwrap();
    }))
    .expect_err("negative padding must be rejected by the debug layout");
    assert!(
        failure
            .downcast_ref::<&str>()
            .is_some_and(|message| message.contains("SliverPadding requires nonnegative padding"))
            || failure.downcast_ref::<String>().is_some_and(
                |message| message.contains("SliverPadding requires nonnegative padding")
            )
    );
}

#[test]
fn padding_trailing_extent_tracks_direction_even_far_outside_the_viewport() {
    let recorded = Rc::new(Cell::new(None));
    let sliver = SliverPadding::new(
        EdgeInsets::only(1., 2., 4., 8.),
        Probe {
            constraints: recorded.clone(),
            geometry: SliverGeometry::ZERO,
        },
    );
    let mut render = sliver.create_render_sliver(&ScrollController::new(), Axis::Vertical, false);
    for (axis, reverse, scroll, expected_after) in [
        (Axis::Vertical, true, 0., 2.),
        (Axis::Vertical, false, 0., 8.),
        (Axis::Horizontal, false, 0., 4.),
        (Axis::Horizontal, true, 0., 1.),
        (Axis::Horizontal, true, 99999.9, 1.),
    ] {
        let mut input = constraints(axis, reverse, 0.);
        input.scroll_offset = scroll;
        let layout = render.perform_layout(input);
        let before = recorded.get().unwrap().preceding_scroll_extent;
        assert_eq!(layout.geometry.scroll_extent - before, expected_after);
    }
}

fn compare_three_box_padding_offsets(
    padding: impl Into<EdgeInsetsGeometry>,
    direction: Option<TextDirection>,
    expected_left: f32,
) {
    let row = |index: u64| {
        SliverToBoxAdapter::new(
            Widget::box_(Size::new(400., 400.), Color::WHITE).with_key(Key::from(index)),
        )
    };
    let slivers: Vec<Box<dyn Sliver>> = vec![
        Box::new(row(1)),
        Box::new(SliverPadding::new(padding, row(2))),
        Box::new(row(3)),
    ];
    let controller = ScrollController::new();
    let mut tree = WidgetTree::new();
    let viewport: Widget = Viewport::new(slivers).controller(controller.clone()).into();
    let root = tree
        .mount(match direction {
            Some(direction) => Directionality::new(direction, viewport).into(),
            None => viewport,
        })
        .unwrap();
    let view = Constraints::tight(Size::new(800., 600.));
    tree.layout(view).unwrap();
    assert_eq!(
        tree.element_bounds(root).unwrap().size,
        Size::new(800., 600.)
    );

    for offset in [0., 200., 390., 490., 10000.] {
        controller.jump_to(offset);
        tree.layout(view).unwrap();
        tree.update_compositor(incular_core::time::Instant::now())
            .unwrap();
        let mut rects = Vec::new();
        for index in 1..=3 {
            let id = tree
                .element_with_key(&Key::from(index as u64))
                .expect("box adapter is retained");
            rects.push(tree.element_bounds(id).unwrap());
        }
        let rect = |x, y, width| Rect::from_origin_size(Offset::new(x, y), Size::new(width, 400.));
        assert_eq!(
            rects,
            [
                rect(0., -offset, 800.),
                rect(expected_left, 420. - offset, 760.),
                rect(0., 855. - offset, 800.),
            ],
            "at offset {offset}"
        );
    }
}

#[test]
fn physical_sliver_padding_preserves_all_three_boxes_at_five_scroll_offsets() {
    compare_three_box_padding_offsets(EdgeInsets::only(25., 20., 15., 35.), None, 25.);
}

#[test]
fn ambient_directional_padding_places_the_center_box_against_the_correct_edge() {
    let insets = EdgeInsetsDirectional::only(25., 20., 15., 35.);
    for (direction, left) in [(TextDirection::Ltr, 25.), (TextDirection::Rtl, 15.)] {
        compare_three_box_padding_offsets(insets, Some(direction), left);
    }
}

#[test]
fn retained_nested_padding_reacts_to_ambient_direction_changes() {
    let controller = ScrollController::new();
    let child_key = Key::from(42_u64);
    let viewport: Widget = Viewport::new([Box::new(SliverPadding::new(
        EdgeInsetsDirectional::only(11., 8., 7., 2.),
        SliverPadding::new(
            EdgeInsetsDirectional::only(13., 3., 3., 9.),
            SliverToBoxAdapter::new(
                Widget::box_(Size::new(100., 50.), Color::WHITE).with_key(child_key.clone()),
            ),
        ),
    )) as Box<dyn Sliver>])
    .controller(controller.clone())
    .into();
    let scene = |direction| Directionality::new(direction, viewport.clone()).into();
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(TextDirection::Ltr)).unwrap();
    let view = Constraints::tight(Size::new(100., 80.));
    tree.layout(view).unwrap();
    let child = tree.element_with_key(&child_key).unwrap();
    for (direction, left) in [
        (TextDirection::Ltr, 24.),
        (TextDirection::Rtl, 10.),
        (TextDirection::Ltr, 24.),
    ] {
        tree.update(root, scene(direction)).unwrap();
        tree.layout(view).unwrap();
        assert_eq!(tree.element_with_key(&child_key), Some(child));
        assert_eq!(
            tree.element_bounds(child).unwrap(),
            Rect::from_origin_size(Offset::new(left, 11.), Size::new(66., 50.))
        );
        assert_eq!(controller.offset(), 0.);
    }
}

#[test]
fn mixed_padding_resolves_late_without_remounting_the_sliver_child() {
    let controller = ScrollController::new();
    let id = Key::from("mixed padding child");
    let padding = EdgeInsetsGeometry::only(3., 5., 7., 11.)
        + EdgeInsetsGeometry::directional(13., 17., 19., 23.);
    let viewport: Widget = Viewport::new([Box::new(SliverPadding::new(
        padding,
        SliverToBoxAdapter::new(
            Widget::box_(Size::new(100., 50.), Color::WHITE).with_key(id.clone()),
        ),
    )) as Box<dyn Sliver>])
    .controller(controller.clone())
    .into();
    let scene = |dir| Directionality::new(dir, viewport.clone()).into();
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(TextDirection::Ltr)).unwrap();
    let bounds = Constraints::tight(Size::new(100., 120.));
    tree.layout(bounds).unwrap();
    let child = tree.element_with_key(&id).unwrap();
    for (direction, left) in [
        (TextDirection::Ltr, 16.),
        (TextDirection::Rtl, 22.),
        (TextDirection::Ltr, 16.),
    ] {
        tree.update(root, scene(direction)).unwrap();
        tree.layout(bounds).unwrap();
        assert_eq!(tree.element_with_key(&id), Some(child));
        assert_eq!(
            tree.element_bounds(child),
            Some(Rect::from_origin_size(
                Offset::new(left, 22.),
                Size::new(58., 50.)
            ))
        );
        assert_eq!(controller.offset(), 0.);
    }
}
