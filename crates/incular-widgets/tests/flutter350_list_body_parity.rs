//! ListBody cross-axis constraints and directional placement from Flutter 3.50.0.
use incular_config::{Axis, Constraints};
use incular_core::{Offset, Size};
use incular_widgets::{
    Column, Directionality, ListBody, Row, ScrollController, SingleChildScrollView, SizedBox,
    TextDirection, Widget,
    internal::{Key, TreeError, WidgetTree},
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
fn children() -> [Widget; 4] {
    std::array::from_fn(|i| {
        Widget::from(SizedBox::new().width(200.).height(150.)).with_key(Key::Value(i as u64))
    })
}
fn directionality(d: TextDirection, child: impl Into<Widget>) -> Widget {
    Directionality::new(d, child).into()
}
fn mount(widget: impl Into<Widget>, constraints: Constraints) -> WidgetTree {
    let mut t = WidgetTree::new();
    t.mount(widget.into()).unwrap();
    t.layout(constraints).unwrap();
    t
}
fn boxes(t: &WidgetTree) -> [(Offset, Size); 4] {
    std::array::from_fn(|i| {
        let r = t
            .element_bounds(t.element_with_key(&Key::Value(i as u64)).unwrap())
            .unwrap();
        (r.origin, r.size)
    })
}
fn horizontal(d: TextDirection) -> Widget {
    directionality(d, ListBody::new(children()).main_axis(Axis::Horizontal))
}
#[test]
fn vertical_list_body_stretches_every_child_to_cross_axis_width() {
    let t = mount(
        Column::new([ListBody::new(children())]),
        Constraints::tight(VIEW),
    );
    assert_eq!(
        boxes(&t),
        [0., 150., 300., 450.].map(|y| (Offset::new(0., y), Size::new(800., 150.)))
    );
}
#[test]
fn horizontal_ltr_list_body_stretches_children_and_orders_them_left_to_right() {
    let r = Row::new([horizontal(TextDirection::Ltr)]).text_direction(TextDirection::Ltr);
    let t = mount(r, Constraints::tight(VIEW));
    assert_eq!(
        boxes(&t),
        [0., 200., 400., 600.].map(|x| (Offset::new(x, 0.), Size::new(200., 600.)))
    );
}
#[test]
fn horizontal_rtl_list_body_stretches_children_and_orders_them_right_to_left() {
    let r = Row::new([horizontal(TextDirection::Rtl)]).text_direction(TextDirection::Ltr);
    let t = mount(r, Constraints::tight(VIEW));
    assert_eq!(
        boxes(&t),
        [600., 400., 200., 0.].map(|x| (Offset::new(x, 0.), Size::new(200., 600.)))
    );
}

#[test]
fn vertical_up_reverses_layout_without_reordering_retained_children() {
    let frame = |reverse| Column::new([ListBody::new(children()).reverse(reverse)]);
    let mut t = mount(frame(false), Constraints::tight(VIEW));
    let ids: [_; 4] = std::array::from_fn(|i| t.element_with_key(&Key::Value(i as u64)).unwrap());
    t.update(t.root().unwrap(), frame(true).into()).unwrap();
    t.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(
        boxes(&t),
        [450., 300., 150., 0.].map(|y| (Offset::new(0., y), Size::new(800., 150.)))
    );
    assert_eq!(
        ids,
        std::array::from_fn(|i| t.element_with_key(&Key::Value(i as u64)).unwrap())
    );
}

#[test]
fn list_body_rejects_a_bounded_main_axis() {
    let widget = SizedBox::new()
        .width(100.)
        .height(100.)
        .child(horizontal(TextDirection::Rtl));
    let mut t = WidgetTree::new();
    t.mount(widget.into()).unwrap();
    assert!(matches!(
        t.layout(Constraints::tight(VIEW)),
        Err(TreeError::InvalidWidgetConfiguration {widget: "ListBody", reason})
            if reason.starts_with("RenderListBody must have unlimited space along its main axis.")
    ));
}

#[test]
fn nested_list_body_rejects_an_unbounded_cross_axis() {
    let inner = directionality(TextDirection::Ltr, ListBody::new(children()));
    let outer = directionality(
        TextDirection::Ltr,
        ListBody::new([Column::new([inner])]).main_axis(Axis::Horizontal),
    );
    let scene = Row::new([outer]).text_direction(TextDirection::Ltr);
    let mut t = WidgetTree::new();
    t.mount(scene.into()).unwrap();
    assert!(matches!(
        t.layout(Constraints::tight(VIEW)),
        Err(TreeError::InvalidWidgetConfiguration {widget: "ListBody", reason})
            if reason.starts_with("RenderListBody must have a bounded constraint for its cross axis.")
    ));
}

#[test]
fn horizontal_reverse_combines_with_live_inherited_direction() {
    let frame = |direction| {
        Row::new([directionality(
            direction,
            ListBody::new(children())
                .main_axis(Axis::Horizontal)
                .reverse(true),
        )])
        .text_direction(TextDirection::Ltr)
    };
    let mut t = mount(frame(TextDirection::Ltr), Constraints::tight(VIEW));
    let xs = |t: &WidgetTree| boxes(t).map(|(origin, _)| origin.x);
    assert_eq!(xs(&t), [600., 400., 200., 0.]);
    t.update(t.root().unwrap(), frame(TextDirection::Rtl).into())
        .unwrap();
    t.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(xs(&t), [0., 200., 400., 600.]);
}

#[test]
fn matching_scroll_view_keeps_the_full_child_extent_and_stretches_width() {
    let controller = ScrollController::new();
    let view = SingleChildScrollView::new(ListBody::new(children())).controller(controller.clone());
    let t = mount(view, Constraints::tight(Size::new(320., 250.)));
    assert_eq!(
        boxes(&t),
        [0., 150., 300., 450.].map(|y| (Offset::new(0., y), Size::new(320., 150.)))
    );
    assert_eq!(controller.max_offset(), 350.);
}

#[test]
fn empty_vertical_body_still_fills_the_cross_axis() {
    let t = mount(
        ListBody::default(),
        Constraints::at_most(320., f32::INFINITY),
    );
    assert_eq!(t.root_layout_size(), Some(Size::new(320., 0.)));
}

#[test]
fn measuring_intrinsics_can_supply_an_otherwise_unbounded_cross_axis() {
    use incular_widgets::{IntrinsicHeight, IntrinsicWidth};
    let children = || {
        [
            SizedBox::new().width(40.).height(20.),
            SizedBox::new().width(60.).height(30.),
        ]
    };
    let unconstrained = Constraints::at_most(f32::INFINITY, f32::INFINITY);
    let vertical = mount(
        IntrinsicWidth::new(ListBody::new(children())),
        unconstrained,
    );
    assert_eq!(vertical.root_layout_size(), Some(Size::new(60., 50.)));
    let horizontal = mount(
        directionality(
            TextDirection::Ltr,
            IntrinsicHeight::new(ListBody::new(children()).main_axis(Axis::Horizontal)),
        ),
        unconstrained,
    );
    assert_eq!(horizontal.root_layout_size(), Some(Size::new(100., 50.)));
}
