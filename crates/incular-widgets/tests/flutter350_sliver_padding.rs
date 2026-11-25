//! Flutter 3.50 SliverPadding geometry, constraints, and retained placement.

use std::{cell::Cell, rc::Rc};

use incular_config::{Axis, Constraints, EdgeInsets};
use incular_core::{Color, Offset, Rect, Size};
use incular_scroll::{ScrollController, SliverConstraints, SliverGeometry};
use incular_widgets::internal::{Key, RenderSliver, SliverLayout, WidgetTree};
use incular_widgets::{
    CustomScrollView, SizedBox, Sliver, SliverFillRemaining, SliverPadding, Widget,
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
