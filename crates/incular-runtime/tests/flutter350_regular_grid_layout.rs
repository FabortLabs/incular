use incular_config::{Axis, Constraints, TextDirection};
use incular_core::Size;
use incular_widgets::{SliverConstraints, SliverGridDelegate, SliverGridRegularTileLayout};

fn constraints(width: f32, axis: Axis) -> SliverConstraints {
    SliverConstraints::new(axis, false, 100., 0., 0., 0., width, 100., 0., 0.)
}

#[test]
fn sub_precision_main_axis_stride_keeps_the_first_tile() {
    // The original Dart fixture uses 1e300. Use a representable Rust
    // ratio that produces the same positive sub-tolerance stride.
    let layout = SliverGridDelegate::max_cross_axis_extent(500.)
        .child_aspect_ratio(1e20)
        .get_layout(constraints(500., Axis::Vertical));
    assert!(layout.main_axis_stride() > 0. && layout.main_axis_stride() < 1e-10);
    assert_eq!(layout.min_child_index_for_scroll_offset(1000.), 0);

    let control = SliverGridRegularTileLayout::new(3, 1e-8, 4., 1e-8, 4., false);
    assert_eq!(control.min_child_index_for_scroll_offset(1e-8), 3);
}

#[test]
fn row_boundaries_and_reversed_cross_axis_place_real_tiles() {
    let delegate = SliverGridDelegate::fixed_cross_axis_count(3)
        .main_axis_extent(20.)
        .main_axis_spacing(5.)
        .cross_axis_spacing(4.);
    let mut rtl = constraints(104., Axis::Vertical);
    rtl.text_direction = Some(TextDirection::Rtl);
    let layout = delegate.get_layout(rtl);

    assert_eq!(layout.cross_axis_count(), 3);
    assert_eq!(layout.min_child_index_for_scroll_offset(25.), 3);
    assert_eq!(layout.max_child_index_for_scroll_offset(25.), 2);
    assert_eq!(layout.max_child_index_for_scroll_offset(25.125), 5);
    assert_eq!(layout.max_child_index_for_scroll_offset(0.), 0);

    let third_in_second_row = layout.geometry_for_child_index(3);
    assert_eq!(third_in_second_row.scroll_offset, 25.);
    assert_eq!(third_in_second_row.cross_axis_offset, 72.);
    assert_eq!(third_in_second_row.trailing_scroll_offset(), 45.);
    assert_eq!(
        third_in_second_row.box_constraints(Axis::Vertical),
        Constraints::tight(Size::new(32., 20.))
    );
    assert_eq!(
        third_in_second_row.box_constraints(Axis::Horizontal),
        Constraints::tight(Size::new(20., 32.))
    );
    assert_eq!(layout.compute_max_scroll_offset(0), 0.);
    assert_eq!(layout.compute_max_scroll_offset(7), 70.);

    let normal = delegate.get_layout(constraints(104., Axis::Vertical));
    assert_eq!(normal.geometry_for_child_index(3).cross_axis_offset, 0.);
    assert_eq!(normal.geometry_for_child_index(5).cross_axis_offset, 72.);
}
