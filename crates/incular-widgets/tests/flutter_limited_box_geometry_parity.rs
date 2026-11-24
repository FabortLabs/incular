//! Flutter 3.50.0 LimitedBox geometry under finite and infinite per-axis bounds.

use incular_config::Constraints;
use incular_core::{Offset, Rect, Size};
use incular_widgets::{
    LimitedBox, OverflowBox, SizedBox, Widget,
    internal::{Key, WidgetTree},
};

const CHILD: u64 = 1;
const LIMITED: u64 = 2;
const INFINITE: f32 = f32::INFINITY;

fn inspect(
    min_width: f32,
    max_width: f32,
    min_height: f32,
    max_height: f32,
    child: SizedBox,
) -> (Rect, Rect) {
    let limited = LimitedBox::new(Widget::from(child).with_key(CHILD))
        .max_width(100.)
        .max_height(200.);
    let overflow = OverflowBox::new(Widget::from(limited).with_key(LIMITED))
        .min_width(min_width)
        .max_width(max_width)
        .min_height(min_height)
        .max_height(max_height);
    let mut tree = WidgetTree::new();
    tree.mount(overflow.into()).expect("mount");
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .expect("layout");
    let bounds = |key| {
        let element = tree.element_with_key(&Key::from(key)).expect("fixture key");
        tree.element_bounds(element).expect("fixture bounds")
    };
    (bounds(LIMITED), bounds(CHILD))
}

fn fixed_child() -> SizedBox {
    SizedBox::new().width(300.).height(400.)
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

// rendering/limited_box_test.dart:13, "LimitedBox: parent max size is unconstrained".
// Compares geometry, not Dart render-tree debug formatting.
#[test]
fn limited_box_caps_both_axes_when_both_parent_maxima_are_unbounded() {
    let expected = rect(350., 200., 100., 200.);
    assert_eq!(
        inspect(0., INFINITE, 0., INFINITE, fixed_child()),
        (expected, expected)
    );
}

// rendering/limited_box_test.dart:59, "LimitedBox: parent maxWidth is unconstrained".
#[test]
fn limited_box_caps_only_width_when_parent_already_sets_height() {
    let (_, child) = inspect(0., INFINITE, 500., 500., fixed_child());
    assert_eq!(child.size, Size::new(100., 500.));
}

// rendering/limited_box_test.dart:75, "LimitedBox: parent maxHeight is unconstrained".
#[test]
fn limited_box_caps_only_height_when_parent_already_sets_width() {
    let (_, child) = inspect(500., 500., 0., INFINITE, fixed_child());
    assert_eq!(child.size, Size::new(500., 200.));
}

// rendering/limited_box_test.dart:92, "LimitedBox: no child".
// Incular requires a child, so an inert SizedBox::shrink represents absence
// for geometry only. The Dart diagnostic formatting is not asserted.
#[test]
fn limited_box_with_inert_child_retains_the_parent_minimum() {
    let (limited, _) = inspect(10., 500., 0., INFINITE, SizedBox::shrink());
    assert_eq!(limited, rect(395., 300., 10., 0.));
}
