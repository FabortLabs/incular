//! Flutter 3.50.0 ScrollMetrics regression (C06 in the pinned parity audit).

use incular_scroll::ScrollMetrics;

fn metrics_at(pixels: f32) -> ScrollMetrics {
    ScrollMetrics {
        min_scroll_extent: 0.0,
        max_scroll_extent: 200.0,
        viewport_dimension: 100.0,
        device_pixel_ratio: 2.0,
        pixels,
        ..Default::default()
    }
}

#[test]
fn overscroll_is_not_at_edge_and_reduces_extent_inside() {
    let edge = metrics_at(0.0);
    assert!(edge.at_edge(), "the exact minimum is an edge");
    assert_eq!(edge.extent_inside(), 100.0);

    let overscroll = metrics_at(-20.0);
    assert_eq!(
        (overscroll.at_edge(), overscroll.extent_inside()),
        (false, 80.0),
    );
}
