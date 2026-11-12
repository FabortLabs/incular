//! Flutter's `ButtonStyle.alignment` defaults to `Alignment.center`: a button
//! taller or wider than its content centres it, whether the extra room comes
//! from `height`, `minimum_size`, `fixed_size` or tight parent constraints.

use incular_config::Constraints;
use incular_controls::prelude::*;
use incular_core::{Rect, Size};
use incular_widgets::{
    Directionality, SizedBox, TextDirection, Widget,
    internal::{Key, WidgetTree},
};

const CONTENT: Size = Size {
    width: 20.,
    height: 10.,
};

fn content() -> Widget {
    Widget::from(SizedBox::new().width(CONTENT.width).height(CONTENT.height))
        .with_key(Key::String("content".into()))
}

/// Lays out `outer(button)` and returns the button's and its content's
/// bounds.
fn layout(style: ButtonStyle, outer: impl Fn(Widget) -> Widget) -> (Rect, Rect) {
    let mut tree = WidgetTree::new();
    let button = Widget::from(Button::new("").child(content()).style(style))
        .with_key(Key::String("button".into()));
    tree.mount(Directionality::new(TextDirection::Ltr, outer(button)).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(400., 400.)))
        .expect("layout");
    let bounds = |key: &str| {
        let element = tree.element_with_key(&Key::String(key.into())).expect(key);
        tree.element_bounds(element).expect(key)
    };
    (bounds("button"), bounds("content"))
}

fn assert_centered(label: &str, (button, content): (Rect, Rect)) {
    assert!(
        button.size.width > CONTENT.width + 1. || button.size.height > CONTENT.height + 1.,
        "{label}: the button {button:?} must be larger than its content"
    );
    let center = |rect: Rect| {
        (
            rect.origin.x + rect.size.width / 2.,
            rect.origin.y + rect.size.height / 2.,
        )
    };
    let (bx, by) = center(button);
    let (cx, cy) = center(content);
    assert!(
        (bx - cx).abs() < 0.51 && (by - cy).abs() < 0.51,
        "{label}: content {content:?} must be centred in the button {button:?}"
    );
}

#[test]
fn a_set_height_centres_the_content() {
    let style = ButtonStyle {
        height: Some(60.),
        ..ButtonStyle::default()
    };
    assert_centered("height", layout(style, |button| button));
}

#[test]
fn a_minimum_size_centres_the_content() {
    let style = ButtonStyle {
        minimum_size: Some(Size::new(160., 60.)),
        ..ButtonStyle::default()
    };
    assert_centered("minimum_size", layout(style, |button| button));
}

#[test]
fn a_fixed_size_centres_the_content() {
    let style = ButtonStyle {
        fixed_size: Some(Size::new(160., 60.)),
        ..ButtonStyle::default()
    };
    assert_centered("fixed_size", layout(style, |button| button));
}

#[test]
fn tight_parent_constraints_centre_the_content() {
    assert_centered(
        "tight parent",
        layout(ButtonStyle::default(), |button| {
            SizedBox::new().width(200.).height(80.).child(button).into()
        }),
    );
}

#[test]
fn a_height_in_a_tight_width_centres_the_content() {
    let style = ButtonStyle {
        height: Some(60.),
        ..ButtonStyle::default()
    };
    assert_centered(
        "height + tight width",
        layout(style, |button| {
            SizedBox::new().width(200.).child(button).into()
        }),
    );
}
