use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, EdgeInsets, RuntimeEnvironment};
use incular_core::{Color, Offset, Rect, Size};
use incular_scroll::ScrollController;
use incular_widgets::{
    Builder, MediaQuery, Sliver, SliverSafeArea, SliverToBoxAdapter, Viewport, Widget,
    internal::{Key, WidgetTree},
};

const FRAME: Size = Size {
    width: 800.,
    height: 600.,
};

fn sized(name: &'static str) -> Widget {
    Widget::box_(Size::new(800., 100.), Color::WHITE).with_key(Key::from(name))
}

fn viewport(safe: impl Sliver + 'static) -> Widget {
    Viewport::new([
        Box::new(SliverToBoxAdapter::new(sized("before"))) as Box<dyn Sliver>,
        Box::new(safe),
        Box::new(SliverToBoxAdapter::new(sized("after"))),
    ])
    .controller(ScrollController::new())
    .into()
}

fn scope(padding: EdgeInsets, child: Widget) -> Widget {
    MediaQuery::new(
        RuntimeEnvironment {
            safe_insets: padding,
            ..Default::default()
        },
        child,
    )
    .into()
}

fn mount(child: Widget, padding: EdgeInsets) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(scope(padding, child)).unwrap();
    tree.layout(Constraints::tight(FRAME)).unwrap();
    tree
}

fn rect(x: f32, y: f32, width: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, 100.))
}

fn areas(tree: &WidgetTree) -> [Rect; 3] {
    ["before", "padded", "after"].map(|name| {
        tree.element_bounds(tree.element_with_key(&Key::from(name)).unwrap())
            .unwrap()
    })
}

#[test]
fn sliver_safe_area_basic_and_minimum_match_before_inside_after_geometry() {
    let basic = mount(
        viewport(SliverSafeArea::new(SliverToBoxAdapter::new(sized("padded"))).left(false)),
        EdgeInsets::all(20.),
    );
    assert_eq!(
        areas(&basic),
        [
            rect(0., 0., 800.),
            rect(0., 120., 780.),
            rect(0., 240., 800.)
        ]
    );
    let minimum = mount(
        viewport(
            SliverSafeArea::new(SliverToBoxAdapter::new(sized("padded")))
                .top(false)
                .minimum(EdgeInsets::only(0., 10., 20., 30.)),
        ),
        EdgeInsets::all(20.),
    );
    assert_eq!(
        areas(&minimum),
        [
            rect(0., 0., 800.),
            rect(20., 110., 760.),
            rect(0., 240., 800.)
        ]
    );
}

#[test]
fn nested_sliver_safe_areas_consume_same_edges_once_and_hide_them_from_boxes() {
    let seen = Rc::new(Cell::new(None));
    let leaf = Builder::new({
        let seen = seen.clone();
        move |context| {
            seen.set(context.media_query_padding());
            sized("padded")
        }
    });
    let nested =
        SliverSafeArea::new(SliverSafeArea::new(SliverToBoxAdapter::new(leaf)).right(false))
            .top(false);
    let tree = mount(viewport(nested), EdgeInsets::all(20.));
    assert_eq!(
        areas(&tree),
        [
            rect(0., 0., 800.),
            rect(20., 120., 760.),
            rect(0., 240., 800.)
        ]
    );
    assert_eq!(seen.get(), Some(EdgeInsets::ZERO));
}

#[test]
fn sliver_safe_area_reacts_to_new_ambient_padding_on_same_viewport() {
    let nested = SliverSafeArea::new(
        SliverSafeArea::new(SliverToBoxAdapter::new(sized("padded")))
            .left(false)
            .bottom(false),
    )
    .bottom(false);
    let viewport = viewport(nested);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scope(EdgeInsets::all(20.), viewport.clone()))
        .unwrap();
    tree.layout(Constraints::tight(FRAME)).unwrap();
    assert_eq!(
        areas(&tree),
        [
            rect(0., 0., 800.),
            rect(20., 120., 760.),
            rect(0., 220., 800.)
        ]
    );
    let id = tree.element_with_key(&Key::from("padded"));
    tree.update(root, scope(EdgeInsets::only(100., 30., 0., 40.), viewport))
        .unwrap();
    tree.layout(Constraints::tight(FRAME)).unwrap();
    assert_eq!(
        areas(&tree),
        [
            rect(0., 0., 800.),
            rect(100., 130., 700.),
            rect(0., 230., 800.)
        ]
    );
    assert_eq!(tree.element_with_key(&Key::from("padded")), id);
}
