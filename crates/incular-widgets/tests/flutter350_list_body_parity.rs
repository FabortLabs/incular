//! ListBody cross-axis constraints and directional placement from Flutter 3.50.0.
use incular_config::{Axis, Constraints};
use incular_core::{Offset, Size};
use incular_widgets::{
    Column, Directionality, ListBody, Row, SizedBox, TextDirection, Widget,
    internal::{Key, WidgetTree},
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
fn list_body_rejects_a_bounded_main_axis() {
    let attempted = std::panic::catch_unwind(|| {
        let widget = SizedBox::new()
            .width(100.)
            .height(100.)
            .child(horizontal(TextDirection::Rtl));
        let mut t = WidgetTree::new();
        t.mount(widget.into())?;
        t.layout(Constraints::tight(VIEW))
    });
    assert!(
        !matches!(attempted, Ok(Ok(()))),
        "bounded main-axis ListBody reached layout"
    );
}
#[test]
fn nested_list_body_rejects_an_unbounded_cross_axis() {
    let attempted = std::panic::catch_unwind(|| {
        let inner = directionality(TextDirection::Ltr, ListBody::new(children()));
        let outer = directionality(
            TextDirection::Ltr,
            ListBody::new([Column::new([inner])]).main_axis(Axis::Horizontal),
        );
        let scene = Row::new([outer]).text_direction(TextDirection::Ltr);
        let mut t = WidgetTree::new();
        t.mount(scene.into())?;
        t.layout(Constraints::tight(VIEW))
    });
    assert!(
        !matches!(attempted, Ok(Ok(()))),
        "unbounded cross-axis ListBody reached layout"
    );
}
