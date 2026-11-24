//! Flutter 3.50.0 retained Flex and baseline layout counterparts.

use incular_config::{Axis, Constraints, CrossAxisAlignment, MainAxisAlignment, VerticalDirection};
use incular_core::{Offset, Rect, Size};
use incular_widgets::{
    Baseline, Center, Directionality, Flex, Placeholder, SizedBox, TextDirection, Widget,
    internal::{Key, WidgetTree},
};

fn squares(count: u64) -> impl Iterator<Item = Widget> {
    (1..=count).map(|key| Widget::from(SizedBox::square(100.)).with_key(key))
}

fn bounds(tree: &WidgetTree, key: u64) -> Rect {
    let element = tree.element_with_key(&Key::from(key)).expect("fixture key");
    tree.element_bounds(element).expect("fixture bounds")
}

// rendering/flex_test.dart:494, "MainAxisAlignment.spaceBetween with RenderFlex.spacing".
#[test]
fn flex_space_between_with_spacing_relayouts_when_axis_changes() {
    let scene = |axis| {
        Flex::new(axis, squares(3))
            .text_direction(TextDirection::Ltr)
            .main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .spacing(14.)
            .into()
    };
    let frame = Constraints::loose(Size::new(500., 400.));
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(Axis::Horizontal)).expect("mount");
    tree.layout(frame).expect("horizontal layout");
    for (key, x) in [(1, 0.), (2, 200.), (3, 400.)] {
        let child = bounds(&tree, key);
        assert_eq!((child.origin.x, child.size.width), (x, 100.));
    }
    tree.update(root, scene(Axis::Vertical))
        .expect("change axis");
    tree.layout(frame).expect("vertical layout");
    for (key, y) in [(1, 0.), (2, 150.), (3, 300.)] {
        let child = bounds(&tree, key);
        assert_eq!((child.origin.y, child.size.height), (y, 100.));
    }
}

// rendering/flex_test.dart:914, "children with no baselines are top-aligned".
// Neither child exposes a baseline, so an explicit alphabetic selector cannot
// affect the offsets; the Incular Flex widget does not offer that selector.
#[test]
fn flex_children_without_baselines_stay_at_top_in_both_vertical_directions() {
    let scene = |vertical_direction| {
        Flex::horizontal(squares(2))
            .text_direction(TextDirection::Ltr)
            .cross_axis_alignment(CrossAxisAlignment::Baseline)
            .vertical_direction(vertical_direction)
            .into()
    };
    let frame = Constraints::tight(Size::new(800., 600.));
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(VerticalDirection::Up)).expect("mount");
    tree.layout(frame).expect("bottom-up layout");
    for key in [1, 2] {
        assert_eq!(bounds(&tree, key).origin.y, 0.);
    }
    tree.update(root, scene(VerticalDirection::Down))
        .expect("change vertical direction");
    tree.layout(frame).expect("top-down layout");
    for key in [1, 2] {
        assert_eq!(bounds(&tree, key).origin.y, 0.);
    }
}

// rendering/flex_test.dart:1256, "Negative RenderFlex.spacing throws an exception".
// The equivalent retained constructor path goes through the Flex widget.
#[cfg(debug_assertions)]
#[test]
fn flex_rejects_negative_spacing_in_debug_instead_of_using_a_zero_gap() {
    let constructed = std::panic::catch_unwind(|| {
        Widget::from(
            Flex::horizontal(squares(2))
                .text_direction(TextDirection::Ltr)
                .spacing(-15.),
        )
    });
    if let Ok(scene) = constructed {
        let mut tree = WidgetTree::new();
        tree.mount(scene).expect("mount accepted scene");
        tree.layout(Constraints::loose(Size::new(500., 400.)))
            .expect("layout accepted scene");
        panic!(
            "negative spacing accepted, second child at x={}",
            bounds(&tree, 2).origin.x
        );
    }
}

// widgets/baseline_test.dart:115, "Baseline does not crash at zero area".
#[test]
fn baseline_placeholder_can_complete_a_zero_area_layout() {
    let baseline = Widget::from(Baseline::new(180., Placeholder::new())).with_key(1_u64);
    let scene = Directionality::new(TextDirection::Ltr, Center::new(baseline));
    let mut tree = WidgetTree::new();
    tree.mount(scene.into()).expect("mount");
    tree.layout(Constraints::tight(Size::ZERO))
        .expect("zero-area layout");
    assert_eq!(
        bounds(&tree, 1),
        Rect::from_origin_size(Offset::ZERO, Size::ZERO)
    );
}
