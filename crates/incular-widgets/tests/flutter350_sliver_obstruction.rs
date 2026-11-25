//! Exact Flutter 3.50 persistent-header max-scroll-obstruction geometry.
use incular_config::Axis;
use incular_scroll::{ScrollController, SliverConstraints};
use incular_widgets::{SizedBox, Sliver, SliverHeaderScrollBehavior, SliverPersistentHeader};

fn geometry(
    behavior: SliverHeaderScrollBehavior,
    scroll_offset: f32,
    paint_extent: f32,
) -> incular_scroll::SliverGeometry {
    let controller = ScrollController::new();
    let header = SliverPersistentHeader::with_delegate(100., 200., |_, _| {
        SizedBox::new().width(400.).height(100.)
    })
    .scroll_behavior(behavior);
    let mut render = header.create_render_sliver(&controller, Axis::Vertical, false);
    render
        .perform_layout(SliverConstraints::new(
            Axis::Vertical,
            false,
            scroll_offset,
            0.,
            0.,
            paint_extent,
            800.,
            600.,
            paint_extent,
            0.,
        ))
        .geometry
}

#[test]
fn floating_persistent_headers_do_not_permanently_obstruct_the_viewport() {
    assert_eq!(
        geometry(SliverHeaderScrollBehavior::Floating, 0., 600.).max_scroll_obstruction_extent,
        0.
    );
}

#[test]
fn floating_pinned_headers_obstruct_by_the_minimum_not_the_visible_extent() {
    for (offset, paint) in [(0., 600.), (75., 600.), (175., 50.), (240., 0.)] {
        let geometry = geometry(SliverHeaderScrollBehavior::FloatingPinned, offset, paint);
        assert_eq!(
            geometry.max_scroll_obstruction_extent, 100.,
            "offset {offset}, paint {paint}"
        );
        assert_eq!(geometry.normalized().max_scroll_obstruction_extent, 100.);
    }
}
