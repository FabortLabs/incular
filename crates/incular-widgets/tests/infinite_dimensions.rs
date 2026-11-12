//! Flutter idioms that use `double.infinity`: an infinite maximum is a
//! normal constraint, and an infinite `width`/`height` on `SizedBox` or
//! `Container` fills the parent's maximum on that axis.

use incular_config::{Alignment, Constraints};
use incular_core::{Color, Size};
use incular_widgets::{
    Align, ConstrainedBox, Container, Directionality, SizedBox, TextDirection, Widget,
    internal::{Key, WidgetTree},
};

fn probe() -> Key {
    Key::String("probe".into())
}

fn layout(widget: Widget) -> Size {
    let mut tree = WidgetTree::new();
    tree.mount(
        Directionality::new(
            TextDirection::Ltr,
            Align::new(
                Alignment::TOP_LEFT,
                SizedBox::new().width(300.).height(200.).child(widget),
            ),
        )
        .into(),
    )
    .expect("mount");
    tree.layout(Constraints::loose(Size::new(400., 400.)))
        .expect("layout");
    let element = tree.element_with_key(&probe()).expect("probe");
    tree.element_bounds(element).expect("bounds").size
}

#[test]
fn an_infinite_maximum_is_a_valid_constraint() {
    let constraints = Constraints::at_most(120., f32::INFINITY);
    assert_eq!(constraints.max_width(), 120.);
    assert!(!constraints.is_height_bounded());
    assert_eq!(constraints.min_width(), 0.);
    let widget = Align::new(
        Alignment::TOP_LEFT,
        Widget::from(ConstrainedBox::new(
            constraints,
            Widget::box_(Size::new(500., 50.), Color::BLACK),
        ))
        .with_key(probe()),
    );
    assert_eq!(layout(widget.into()), Size::new(120., 50.));
}

#[test]
fn copy_with_style_bounds_keep_the_other_axis() {
    let constraints = Constraints::unbounded().with_max_width(80.);
    assert_eq!(constraints.max_width(), 80.);
    assert!(!constraints.is_height_bounded());
    let constraints = constraints.with_max_height(30.).with_min_height(10.);
    assert_eq!(
        (constraints.min_height(), constraints.max_height()),
        (10., 30.)
    );
}

#[test]
fn container_with_an_infinite_width_fills_the_parent_width() {
    let widget = Align::new(
        Alignment::TOP_LEFT,
        Widget::from(
            Container::new()
                .width(f32::INFINITY)
                .height(40.)
                .color(Color::BLACK),
        )
        .with_key(probe()),
    );
    assert_eq!(layout(widget.into()), Size::new(300., 40.));
}

#[test]
fn container_with_infinite_dimensions_and_constraints_clamps_into_them() {
    let widget = Align::new(
        Alignment::TOP_LEFT,
        Widget::from(
            Container::new()
                .constraints(Constraints::new(0., 150., 0., 100.))
                .width(f32::INFINITY)
                .height(f32::INFINITY)
                .color(Color::BLACK),
        )
        .with_key(probe()),
    );
    assert_eq!(layout(widget.into()), Size::new(150., 100.));
}

#[test]
fn sized_box_with_an_infinite_width_fills_the_parent_width() {
    let widget = Align::new(
        Alignment::TOP_LEFT,
        Widget::from(
            SizedBox::new()
                .width(f32::INFINITY)
                .height(40.)
                .child(Widget::box_(Size::new(10., 10.), Color::BLACK)),
        )
        .with_key(probe()),
    );
    assert_eq!(layout(widget.into()), Size::new(300., 40.));
}
