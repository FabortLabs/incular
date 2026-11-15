use incular_config::Axis;
use incular_scroll::{SliverConstraints, SliverGeometry};

#[test]
fn geometry_tracks_visible_and_total_extent() {
    let constraints = SliverConstraints::new(
        Axis::Vertical,
        false,
        25.,
        0.,
        0.,
        100.,
        320.,
        100.,
        150.,
        0.,
    );
    let geometry = SliverGeometry::from_scroll_extent(constraints, 400.);
    assert_eq!(geometry.scroll_extent, 400.);
    assert_eq!(geometry.paint_extent, 100.);
    assert_eq!(geometry.layout_extent, 100.);
    assert!(geometry.visible);
    assert!(geometry.has_visual_overflow);
}

#[test]
fn normalization_prevents_invalid_geometry() {
    let geometry = SliverGeometry {
        scroll_extent: -1.,
        paint_extent: 100.,
        layout_extent: 90.,
        max_paint_extent: 20.,
        hit_test_extent: -3.,
        paint_origin: f32::NAN,
        cache_extent: f32::INFINITY,
        visible: true,
        has_visual_overflow: false,
        scroll_offset_correction: Some(f32::NAN),
    }
    .normalized();
    assert_eq!(geometry.scroll_extent, 0.);
    assert_eq!(geometry.paint_extent, 0.);
    assert_eq!(geometry.layout_extent, 0.);
    assert_eq!(geometry.max_paint_extent, 0.);
    assert_eq!(geometry.hit_test_extent, 0.);
    assert_eq!(geometry.scroll_offset_correction, None);
}

/// Flutter's child range (rendering/sliver_list.dart:51-53): with 50 px rows,
/// a 600 px viewport and a 250 px cache, the row that ends exactly at the
/// leading cache edge is built and the row that starts exactly at the trailing
/// cache edge is not. The plain `materialized_range` keeps its own edges.
#[test]
fn flutter_child_range_includes_leading_edge_row_and_excludes_trailing_edge_row() {
    let index = incular_scroll::MeasuredExtentIndex::new(60, 50.);
    assert_eq!(index.flutter_child_range(0., 850., 0.), 0..17);
    assert_eq!(index.flutter_child_range(750., 850., 0.), 14..32);
    assert_eq!(index.materialized_range(750., 850., 0.), 15..33);
}
