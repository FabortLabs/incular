//! Framework-level probe of Flutter 3.50.0 fillOverscroll in bouncing physics.
use incular_config::{Axis, Constraints};
use incular_core::{Color, Offset, Rect, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    CustomScrollView, Directionality, ScrollController, ScrollPhysics, SizedBox, Sliver,
    SliverFillRemaining, SliverToBoxAdapter, TextDirection, Widget, internal::Key,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const FILL: u64 = 15;
fn frame(r: &mut Runtime) {
    r.run_frame(Constraints::tight(VIEW)).unwrap();
}
fn rect(r: &Runtime) -> Rect {
    let t = r.tree();
    t.element_bounds(t.element_with_key(&Key::Value(FILL)).expect("fill mounted"))
        .expect("fill bounds")
}
fn host(preceding: bool, axis: Axis, reverse: bool, c: &ScrollController) -> Runtime {
    let leaf = Widget::box_(Size::ZERO, Color::WHITE).with_key(Key::Value(FILL));
    let fill = SliverFillRemaining::new(SizedBox::expand().child(leaf))
        .has_scroll_body(false)
        .fill_overscroll(true);
    let mut slivers: Vec<Box<dyn Sliver>> = Vec::new();
    if preceding {
        slivers.push(Box::new(SliverToBoxAdapter::new(Widget::box_(
            Size::new(150., 150.),
            Color::BLACK,
        ))));
    }
    slivers.push(Box::new(fill));
    let v = CustomScrollView::new(slivers)
        .scroll_direction(axis)
        .reverse(reverse)
        .controller(c.clone())
        .physics(ScrollPhysics::BOUNCING);
    let mut r = Runtime::new(Directionality::new(TextDirection::Ltr, v).into()).unwrap();
    frame(&mut r);
    r
}
fn compare(preceding: bool, axis: Axis, reverse: bool, offset: f32, want: Rect, check_hit: bool) {
    let c = ScrollController::new();
    let mut r = host(preceding, axis, reverse, &c);
    let initial = if preceding {
        Rect::from_origin_size(Offset::new(0., 150.), Size::new(800., 450.))
    } else {
        Rect::from_origin_size(Offset::ZERO, VIEW)
    };
    assert_eq!(rect(&r), initial, "rest geometry");
    assert!(c.jump_to(offset));
    frame(&mut r);
    let actual = rect(&r);
    let hit = if check_hit {
        let tree = r.tree();
        let node = tree.element_with_key(&Key::Value(FILL)).unwrap();
        Some(
            tree.hit_test(Offset::new(400., 25.))
                .and_then(|id| tree.element_for_render(id))
                == Some(node),
        )
    } else {
        None
    };
    assert!(c.jump_to(0.));
    frame(&mut r);
    assert_eq!(rect(&r), initial, "restored after returning to zero");
    assert_eq!((actual, hit), (want, check_hit.then_some(true)));
}
#[test]
fn overscroll_beyond_the_preceding_sibling_uses_the_remaining_scroll_offset() {
    compare(
        true,
        Axis::Vertical,
        false,
        200.,
        Rect::from_origin_size(Offset::new(0., -50.), Size::new(800., 650.)),
        false,
    );
}
#[test]
fn lone_child_covers_the_entire_trailing_overscroll() {
    compare(
        false,
        Axis::Vertical,
        false,
        50.,
        Rect::from_origin_size(Offset::new(0., -50.), Size::new(800., 650.)),
        false,
    );
}
#[test]
fn reversed_vertical_stretches_from_top_and_remains_hittable() {
    compare(
        false,
        Axis::Vertical,
        true,
        50.,
        Rect::from_origin_size(Offset::ZERO, Size::new(800., 650.)),
        true,
    );
}
#[test]
fn reversed_horizontal_stretches_from_left() {
    compare(
        false,
        Axis::Horizontal,
        true,
        50.,
        Rect::from_origin_size(Offset::ZERO, Size::new(850., 600.)),
        false,
    );
}
