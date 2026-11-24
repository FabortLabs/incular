//! Flutter 3.50.0 layout regressions from
//! audits/flutter-3.50.0-parity/focused-comparisons.md.

use incular_config::{Alignment, Constraints, MainAxisSize};
use incular_core::{Rect, Size};
use incular_widgets::{
    Align, ConstrainedBox, Expanded, Row, SizedBox, Table, TextDirection, Widget, Wrap,
    internal::{Key, WidgetTree},
};

fn sized(width: f32, height: f32, key: u64) -> Widget {
    Widget::from(SizedBox::new().width(width).height(height)).with_key(key)
}

fn loose_scene(subject: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(Align::new(Alignment::TOP_LEFT, subject).into())
        .expect("mount fixture");
    tree.layout(Constraints::tight(Size::new(100., 30.)))
        .expect("layout fixture");
    tree
}

fn bounds(tree: &WidgetTree, key: u64) -> Rect {
    let id = tree.element_with_key(&Key::from(key)).expect("fixture key");
    tree.element_bounds(id).expect("fixture bounds")
}

#[test]
fn min_row_keeps_bounded_expanded_allocations() {
    let row = Row::new([
        Expanded::new(sized(10., 10., 1)),
        Expanded::new(sized(10., 10., 2)),
    ])
    .main_axis_size(MainAxisSize::Min)
    .text_direction(TextDirection::Ltr);
    let tree = loose_scene(Widget::from(row).with_key(3_u64));
    let left = bounds(&tree, 1);
    let right = bounds(&tree, 2);
    assert_eq!(
        (bounds(&tree, 3).size.width, right.origin.x - left.origin.x),
        (100., 50.),
    );
}

#[test]
fn default_table_flex_divides_bounded_width_without_using_intrinsic_widths() {
    let tree = loose_scene(Table::new(2, [sized(80., 10., 1), sized(80., 10., 2)]));
    let left = bounds(&tree, 1);
    let right = bounds(&tree, 2);
    assert_eq!(
        (
            left.size.width,
            right.size.width,
            right.origin.x - left.origin.x
        ),
        (50., 50., 50.),
    );
}

#[test]
fn wrap_keeps_children_in_the_same_run_at_a_fractional_boundary() {
    let tree = loose_scene(ConstrainedBox::new(
        Constraints::new(0., 20.4, 0., f32::INFINITY),
        Wrap::new([sized(10.1, 10., 1), sized(10.3, 10., 2)]).text_direction(TextDirection::Ltr),
    ));
    let left = bounds(&tree, 1);
    let right = bounds(&tree, 2);
    assert_eq!((left.size.width, right.size.width), (10.1, 10.3));
    assert_eq!(right.origin.y - left.origin.y, 0.);
}
