//! Exact geometry controls from Flutter's pinned GridView cross-axis tests.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_widgets::{
    Directionality, GridView, SliverGridDelegate, Widget,
    internal::{Key, WidgetTree},
};

fn compare(spacing: f32, cell: f32) {
    let target = Key::from("grid cross-axis target");
    let mut tree = WidgetTree::new();
    let scene = |direction| {
        Directionality::new(
            direction,
            GridView::new(
                SliverGridDelegate::fixed_cross_axis_count(4).cross_axis_spacing(spacing),
                [Widget::box_(Size::ZERO, Color::TRANSPARENT).with_key(target.clone())],
            ),
        )
        .into()
    };
    let root = tree.mount(scene(TextDirection::Ltr)).unwrap();
    let viewport = Constraints::tight(Size::new(800., 600.));
    tree.layout(viewport).unwrap();
    let first = tree.element_with_key(&target).unwrap();
    for (direction, x) in [
        (TextDirection::Ltr, 0.),
        (TextDirection::Rtl, 800. - cell),
        (TextDirection::Ltr, 0.),
    ] {
        tree.update(root, scene(direction)).unwrap();
        tree.layout(viewport).unwrap();
        assert_eq!(tree.element_with_key(&target), Some(first));
        assert_eq!(
            tree.element_bounds(first).unwrap(),
            Rect::from_origin_size(Offset::new(x, 0.), Size::new(cell, cell))
        );
    }
}

#[test]
fn vertical_grid_places_its_first_lane_at_the_ambient_leading_edge() {
    compare(0., 200.);
}

#[test]
fn each_cross_axis_gap_is_removed_before_dividing_the_cell_width() {
    compare(8., 194.);
}
