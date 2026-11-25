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

fn empty_box_under(parent: impl Fn(Widget) -> OverflowBox) -> WidgetTree {
    let child = LimitedBox::builder()
        .max_width(100.)
        .max_height(200.)
        .build();
    let mut tree = WidgetTree::new();
    tree.mount(parent(Widget::from(child).with_key(LIMITED)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree
}

fn check_empty(tree: &WidgetTree, expected: Rect) {
    let limited = tree.element_with_key(&Key::from(LIMITED)).unwrap();
    assert_eq!(tree.children(limited).unwrap(), &[]);
    assert_eq!(tree.element_bounds(limited), Some(expected));
}

// rendering/limited_box_test.dart:92. Dart diagnostic formatting is separate.
#[test]
fn childless_limited_box_keeps_parent_minimum_under_an_unbounded_axis() {
    let tree = empty_box_under(|child| {
        OverflowBox::new(child)
            .min_width(10.)
            .max_width(500.)
            .min_height(0.)
            .max_height(INFINITE)
    });
    check_empty(&tree, rect(395., 300., 10., 0.));
}

// rendering/limited_box_test.dart:129. Also recheck a retained child insertion/removal.
#[test]
fn childless_limited_box_uses_inherited_tight_height() {
    let parent = |child| OverflowBox::new(child).min_width(10.);
    let mut tree = empty_box_under(parent);
    let root = tree.root().unwrap();
    let limited = tree.element_with_key(&Key::from(LIMITED)).unwrap();
    check_empty(&tree, rect(395., 0., 10., 600.));
    let make = |with_child| {
        let mut box_ = LimitedBox::empty().max_width(100.).max_height(200.);
        if with_child {
            box_ = box_.child(fixed_child());
        }
        parent(Widget::from(box_).with_key(LIMITED)).into()
    };
    let constraints = Constraints::tight(Size::new(800., 600.));
    tree.update(root, make(true)).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(tree.children(limited).unwrap().len(), 1);
    assert_eq!(
        tree.element_bounds(limited),
        Some(rect(250., 0., 300., 600.))
    );
    tree.update(root, make(false)).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(tree.element_with_key(&Key::from(LIMITED)), Some(limited));
    check_empty(&tree, rect(395., 0., 10., 600.));
}

// Flutter limits the incoming maximum without constructing inverted constraints.
#[test]
fn limited_box_caps_cannot_undercut_the_parent_minimum() {
    let expected = rect(325., 175., 150., 250.);
    assert_eq!(
        inspect(150., INFINITE, 250., INFINITE, fixed_child()),
        (expected, expected)
    );

    let tree = empty_box_under(|child| {
        OverflowBox::new(child)
            .min_width(150.)
            .max_width(INFINITE)
            .min_height(250.)
            .max_height(INFINITE)
    });
    check_empty(&tree, expected);
}
