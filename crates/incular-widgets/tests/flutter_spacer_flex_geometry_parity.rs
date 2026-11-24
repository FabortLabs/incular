//! Flutter 3.50.0 widget Spacer and tight RTL Flex geometry counterparts.

use incular_config::{Axis, Constraints, MainAxisSize};
use incular_core::{Offset, Rect, Size};
use incular_widgets::{
    Column, Row, SizedBox, Spacer, TextDirection, UnconstrainedBox, Widget,
    internal::{Key, WidgetTree},
};

fn sized(width: f32, height: f32) -> Widget {
    SizedBox::new().width(width).height(height).into()
}

fn spacer(flex: u32, key: u64) -> Widget {
    Widget::from(Spacer::new().flex(flex)).with_key(key)
}

fn laid_out(scene: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(scene.into()).expect("mount");
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .expect("layout");
    tree
}

fn bounds(tree: &WidgetTree, key: u64) -> Rect {
    let element = tree.element_with_key(&Key::from(key)).expect("fixture key");
    tree.element_bounds(element).expect("fixture bounds")
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

// widgets/spacer_test.dart:9, first "Spacer takes up space." registration.
#[test]
fn spacer_in_tight_column_takes_remaining_height_but_not_width() {
    let column = Column::new([sized(10., 10.), spacer(1, 1), sized(10., 10.)]);
    assert_eq!(bounds(&laid_out(column), 1), rect(400., 10., 0., 580.));
}

// widgets/spacer_test.dart:24, "Spacer takes up space proportional to flex."
#[test]
fn weighted_rtl_spacers_distribute_remaining_width_without_cross_size() {
    let mut children = vec![sized(10., 10.)];
    for (index, flex) in [1, 1, 2, 4].into_iter().enumerate() {
        children.push(spacer(flex, index as u64 + 1));
        children.push(sized(10., 10.));
    }
    let tree = laid_out(Row::new(children).text_direction(TextDirection::Rtl));
    for (i, (left, width)) in [(696.25, 93.75), (592.5, 93.75), (395., 187.5), (10., 375.)]
        .into_iter()
        .enumerate()
    {
        let spacer = bounds(&tree, i as u64 + 1);
        assert_eq!((spacer.origin.x, spacer.size), (left, Size::new(width, 0.)),);
    }
}

// widgets/spacer_test.dart:60, second "Spacer takes up space." registration.
#[test]
fn spacer_retains_main_axis_fill_with_unbounded_cross_constraints() {
    let column = Column::new([sized(20., 10.), spacer(1, 1), sized(10., 10.)]);
    let view = UnconstrainedBox::new(Widget::from(column).with_key(2_u64))
        .constrained_axis(Axis::Vertical);
    let tree = laid_out(view);
    assert_eq!(bounds(&tree, 1), rect(400., 10., 0., 580.));
    assert_eq!(bounds(&tree, 2), rect(390., 0., 20., 600.));
}

// rendering/flex_test.dart:744, "MainAxisSize.min inside tightly constrained".
#[test]
fn tight_rtl_row_min_keeps_parent_width_and_places_children_at_the_right() {
    let children = (1..=3).map(|key| sized(100., 100.).with_key(key as u64));
    let tree = laid_out(
        Row::new(children)
            .main_axis_size(MainAxisSize::Min)
            .text_direction(TextDirection::Rtl),
    );
    for (key, x) in [(1_u64, 700.), (2, 600.), (3, 500.)] {
        assert_eq!(bounds(&tree, key), rect(x, 250., 100., 100.));
    }
}
