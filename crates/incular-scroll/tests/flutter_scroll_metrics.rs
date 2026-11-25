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

#[test]
fn both_edges_and_overscroll_limits_match_flutter() {
    let actual = [-120., -20., 0., 35., 200., 220., 320.].map(|pixels| {
        let metrics = metrics_at(pixels);
        (
            metrics.at_edge(),
            metrics.out_of_range(),
            metrics.extent_inside(),
        )
    });
    assert_eq!(
        actual,
        [
            (false, true, 0.),
            (false, true, 80.),
            (true, false, 100.),
            (false, false, 100.),
            (true, false, 100.),
            (false, true, 80.),
            (false, true, 0.),
        ]
    );

    let short = ScrollMetrics {
        max_scroll_extent: 0.,
        ..metrics_at(0.)
    };
    assert_eq!((short.at_edge(), short.extent_inside()), (true, 100.));
    let beyond = ScrollMetrics {
        pixels: 20.,
        ..short
    };
    assert_eq!((beyond.at_edge(), beyond.extent_inside()), (false, 80.));
}
