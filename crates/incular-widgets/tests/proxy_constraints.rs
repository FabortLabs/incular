//! Proxy widgets pass their constraints to their child unchanged, as
//! Flutter's `RenderProxyBox` subclasses do: under a tight parent the child
//! is sized tightly too, so a shrink-wrapping child (an `Align` with size
//! factors, a `Text`) fills the proxy and positions, paints and hit-tests
//! across all of it.

use incular_config::{Alignment, Constraints};
use incular_core::{Color, Offset, Size};
use incular_widgets::{
    Align, BackdropFilter, BoxDecoration, Container, DecoratedBox, Directionality,
    FractionalTranslation, GestureDetector, Opacity, SizedBox, TextDirection, Transform,
    Visibility, Widget,
    internal::{Key, WidgetTree},
};

/// An `Align` that shrink-wraps a 20x10 box when it is free to.
fn shrink_wrapping() -> Widget {
    Widget::from(
        Align::new(Alignment::CENTER, SizedBox::new().width(20.).height(10.))
            .width_factor(1.)
            .height_factor(1.),
    )
    .with_key(Key::String("probe".into()))
}

fn probe_size(label: &str, proxy: impl Fn(Widget) -> Widget) -> Size {
    let mut tree = WidgetTree::new();
    let root = SizedBox::new()
        .width(200.)
        .height(80.)
        .child(proxy(shrink_wrapping()));
    tree.mount(
        Directionality::new(TextDirection::Ltr, Align::new(Alignment::TOP_LEFT, root)).into(),
    )
    .expect("mount");
    tree.layout(Constraints::loose(Size::new(400., 400.)))
        .expect("layout");
    let probe = tree
        .element_with_key(&Key::String("probe".into()))
        .unwrap_or_else(|| panic!("{label}: probe"));
    tree.element_bounds(probe)
        .unwrap_or_else(|| panic!("{label}: bounds"))
        .size
}

/// Wraps the probe in the proxy under test.
type Wrap = dyn Fn(Widget) -> Widget;

#[test]
fn proxies_pass_tight_constraints_through() {
    let cases: Vec<(&str, Box<Wrap>)> = vec![
        (
            "DecoratedBox",
            Box::new(|child| DecoratedBox::new(child).background(Color::BLACK).into()),
        ),
        (
            "Container(color)",
            Box::new(|child| Container::new().color(Color::BLACK).child(child).into()),
        ),
        (
            "Container(decoration)",
            Box::new(|child| {
                Container::new()
                    .decoration(BoxDecoration::new().color(Color::BLACK))
                    .child(child)
                    .into()
            }),
        ),
        ("Opacity", Box::new(|child| Opacity::new(0.5, child).into())),
        (
            "Transform",
            Box::new(|child| Transform::translation(Offset::new(4., 4.), child).into()),
        ),
        (
            "FractionalTranslation",
            Box::new(|child| FractionalTranslation::new(Offset::new(0.1, 0.), child).into()),
        ),
        (
            "BackdropFilter",
            Box::new(|child| BackdropFilter::blur(2., child).into()),
        ),
        (
            "GestureDetector",
            Box::new(|child| GestureDetector::new(child).on_tap(|| {}).into()),
        ),
        (
            "Visibility",
            Box::new(|child| Visibility::new(child).into()),
        ),
    ];
    for (label, proxy) in cases {
        assert_eq!(
            probe_size(label, proxy),
            Size::new(200., 80.),
            "{label} must hand its tight constraints to its child"
        );
    }
}
