//! Flutter 3.50.0 fractional and overflow layout with direction and optional children.

use incular_config::{Alignment, AlignmentDirectional, Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_widgets::{
    AlignmentGeometry, Center, Container, Directionality, FractionallySizedBox, OverflowBox,
    Placeholder, SizedBox, Widget,
    internal::{Key, TreeError, WidgetTree},
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const BOX: u64 = 1;
const CHILD: u64 = 2;
const OVERFLOW: u64 = 3;

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn bounds(tree: &WidgetTree, key: u64) -> Rect {
    let element = tree.element_with_key(&Key::from(key)).unwrap();
    tree.element_bounds(element).unwrap()
}

// widgets/fractionally_sized_box_test.dart:53, with a physical alignment control
// from :32 and a retained direction change.
#[test]
fn fractional_logical_alignment_follows_the_ambient_direction_in_place() {
    for (alignment, ltr_x, rtl_x) in [
        (
            AlignmentGeometry::Directional(AlignmentDirectional::TOP_END),
            400.,
            0.,
        ),
        (
            AlignmentGeometry::Physical(Alignment::TOP_RIGHT),
            400.,
            400.,
        ),
    ] {
        let child = Widget::from(Placeholder::new()).with_key(CHILD);
        let inner = Widget::from(
            FractionallySizedBox::builder()
                .alignment(alignment)
                .width_factor(0.5)
                .height_factor(0.5)
                .child(child)
                .build(),
        )
        .with_key(BOX);
        let scene = |direction| Directionality::new(direction, inner.clone()).into();
        let mut tree = WidgetTree::new();
        let root = tree.mount(scene(TextDirection::Rtl)).unwrap();
        tree.layout(Constraints::tight(VIEW)).unwrap();
        let id = tree.element_with_key(&Key::from(BOX)).unwrap();
        let render = tree.render_id(id);
        for (direction, x) in [
            (TextDirection::Rtl, rtl_x),
            (TextDirection::Ltr, ltr_x),
            (TextDirection::Rtl, rtl_x),
        ] {
            tree.update(root, scene(direction)).unwrap();
            tree.layout(Constraints::tight(VIEW)).unwrap();
            assert_eq!(tree.render_id(id), render);
            let placed = bounds(&tree, CHILD);
            assert_eq!(placed, rect(x, 0., 400., 300.));
            let center =
                placed.origin + Offset::new(placed.size.width / 2., placed.size.height / 2.);
            assert_eq!(center, Offset::new(x + 200., 150.));
        }
    }
}

// widgets/fractionally_sized_box_test.dart:74, including the actual Container.
#[test]
fn overflow_logical_end_aligns_a_fractional_child_in_both_directions() {
    let content = Widget::from(Container::new()).with_key(CHILD);
    let box_ = FractionallySizedBox::new(content)
        .width_factor(0.5)
        .height_factor(0.25);
    let overflow = Widget::from(
        OverflowBox::new(Center::new(box_))
            .min_width(0.)
            .max_width(100.)
            .min_height(0.)
            .max_height(100.)
            .alignment(AlignmentDirectional::TOP_END),
    )
    .with_key(OVERFLOW);
    let scene = |direction| Directionality::new(direction, overflow.clone()).into();
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(TextDirection::Rtl)).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    let element = tree.element_with_key(&Key::from(OVERFLOW)).unwrap();
    let render = tree.render_id(element);
    for (direction, x) in [(TextDirection::Rtl, 25.), (TextDirection::Ltr, 725.)] {
        tree.update(root, scene(direction)).unwrap();
        tree.layout(Constraints::tight(VIEW)).unwrap();
        assert_eq!(bounds(&tree, CHILD), rect(x, 37.5, 50., 25.));
        assert_eq!(tree.render_id(element), render);
    }
}

// widgets/fractionally_sized_box_test.dart:100, with the same placeholder.
#[test]
fn fractional_with_placeholder_lays_out_and_paints_at_zero_area() {
    let box_ = Widget::from(FractionallySizedBox::new(Placeholder::new())).with_key(BOX);
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, Center::new(box_)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    let _ = tree.paint();
    assert_eq!(bounds(&tree, BOX).size, Size::ZERO);
}

#[test]
fn childless_fractional_box_reserves_its_factor_without_resolving_alignment() {
    let make = |with_child| {
        let mut box_ = FractionallySizedBox::empty()
            .alignment(AlignmentDirectional::TOP_END)
            .width_factor(0.5)
            .height_factor(0.25);
        if with_child {
            box_ = box_.child(SizedBox::square(1.));
        }
        Center::new(Widget::from(box_).with_key(BOX)).into()
    };
    // Absence of a child means direction is not needed. With a physical
    // alignment there is never a direction requirement.
    let mut tree = WidgetTree::new();
    let root = tree.mount(make(false)).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    let element = tree.element_with_key(&Key::from(BOX)).unwrap();
    assert_eq!(bounds(&tree, BOX), rect(200., 225., 400., 150.));
    assert!(tree.children(element).unwrap().is_empty());
    assert!(matches!(
        tree.update(root, make(true)),
        Err(TreeError::MissingDirectionality {
            widget: "FractionallySizedBox",
            ..
        })
    ));

    let mut other = WidgetTree::new();
    let overflow = OverflowBox::new(SizedBox::shrink()).alignment(AlignmentDirectional::TOP_END);
    assert!(matches!(
        other.mount(overflow.into()),
        Err(TreeError::MissingDirectionality {
            widget: "OverflowBox",
            ..
        })
    ));
}
