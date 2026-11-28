//! Fixed-extent slivers own an exact half-open paint/cache interval.

use incular_config::{Axis, Constraints};
use incular_core::{Color, Size};
use incular_rendering::PaintCommand;
use incular_runtime::Runtime;
use incular_scroll::SliverConstraints;
use incular_widgets::{
    CustomScrollView, Directionality, ScrollController, SizedBox, Sliver, SliverFillViewport,
    SliverFixedExtentList, TextDirection, Widget,
};

#[test]
fn viewport_sized_pages_mount_and_paint_only_the_page_at_the_boundary() {
    let ctl = ScrollController::new();
    let sliver: Box<dyn Sliver> = Box::new(SliverFillViewport::new((0u64..3).map(|i| {
        Widget::box_(Size::new(400., 100.), Color::rgba(51, i as u8, 91, 255)).with_key(i)
    })));
    let view = CustomScrollView::new([sliver])
        .cache_extent(0.)
        .controller(ctl.clone());
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, view).into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));

    for (pixels, index) in [
        (0., 0),
        (600., 1),
        // Use a representable f32 predecessor before visiting the exact edge.
        (1200_f32.next_down(), 2),
        (1199.999999999998_f64 as f32, 2),
    ] {
        ctl.jump_to(pixels);
        let (list, _) = runtime.run_frame(bounds).unwrap();
        assert_eq!(ctl.offset(), pixels);
        let mounted = (0u64..3)
            .filter(|key| runtime.tree().element_with_key(&(*key).into()).is_some())
            .collect::<Vec<_>>();
        assert_eq!(mounted, [index], "children mounted at {pixels}");
        let painted = list
            .commands()
            .iter()
            .filter_map(|c| match c {
                PaintCommand::Rect { color, .. } if color.red == 51 && color.blue == 91 => {
                    Some(u64::from(color.green))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(painted, [index], "children painted at {pixels}");
    }
}

#[test]
fn fixed_extent_layout_excludes_the_exact_cache_endpoint() {
    for (extent, end, expected_last) in [
        (0., 1234., 0),
        (600., 600., 0),
        (600., 601., 1),
        (600., 2700., 4),
        (414., 2484., 5),
        (600., 600_f32.next_up(), 0),
        (600., 600_f32.next_up().next_up(), 1),
        (411.42857, 411.42857_f32 * 6., 5),
    ] {
        let list = SliverFixedExtentList::new(12, extent, |_| SizedBox::shrink());
        let layout = list
            .create_render_sliver(&ScrollController::new(), Axis::Vertical, false)
            .perform_layout(SliverConstraints::new(
                Axis::Vertical,
                false,
                0.,
                0.,
                0.,
                end,
                800.,
                end,
                end,
                0.,
            ));
        assert_eq!(layout.geometry.scroll_extent, extent * 12.);
        let slots = layout
            .children
            .iter()
            .map(|c| c.semantic_index.unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            slots,
            (0..=expected_last).collect::<Vec<_>>(),
            "extent={extent}, end={end}"
        );
        for child in layout.children {
            assert_eq!(child.extent, extent);
            assert_eq!(child.offset, child.semantic_index.unwrap() as f32 * extent);
        }
    }
}

#[test]
fn viewport_sized_page_layout_uses_the_same_ulp_boundary_as_fixed_lists() {
    let descriptor = SliverFillViewport::new((0..3).map(|_| SizedBox::shrink()));
    let mut sliver =
        descriptor.create_render_sliver(&ScrollController::new(), Axis::Vertical, false);
    let children = |layout: incular_widgets::internal::SliverLayout| {
        layout
            .children
            .into_iter()
            .map(|child| child.semantic_index.unwrap())
            .collect::<Vec<_>>()
    };
    for (offset, extent, expected) in [
        (1200_f32.next_down(), 600., vec![2]),
        (0., 600_f32.next_up(), vec![0]),
        (0., 600_f32.next_up().next_up(), vec![0, 1]),
    ] {
        let layout = sliver.perform_layout(SliverConstraints::new(
            Axis::Vertical,
            false,
            offset,
            0.,
            0.,
            extent,
            800.,
            600.,
            extent,
            0.,
        ));
        assert_eq!(
            children(layout),
            expected,
            "offset={offset}, cache extent={extent}"
        );
    }
}

#[test]
fn layout_uses_configured_fixed_or_viewport_extent_for_offsets_and_content() {
    for (is_fill, offset, viewport, item_extent, count, first, total) in [
        (true, 0., 600., 600., 3, 0, 1800.),
        (true, 1200., 600., 600., 3, 2, 1800.),
        (false, 0., 600., 30., 3, 0, 90.),
        (false, 45., 600., 30., 3, 1, 90.),
    ] {
        let descriptor: Box<dyn Sliver> = if is_fill {
            Box::new(SliverFillViewport::new(
                (0..count).map(|_| SizedBox::shrink()),
            ))
        } else {
            Box::new(SliverFixedExtentList::new(count, item_extent, |_| {
                SizedBox::shrink()
            }))
        };
        let layout = descriptor
            .create_render_sliver(&ScrollController::new(), Axis::Vertical, false)
            .perform_layout(SliverConstraints::new(
                Axis::Vertical,
                false,
                offset,
                0.,
                0.,
                viewport,
                800.,
                viewport,
                viewport,
                0.,
            ));
        assert_eq!(layout.geometry.scroll_extent, total);
        assert_eq!(layout.children[0].semantic_index, Some(first));
        assert_eq!(layout.children[0].offset, first as f32 * item_extent);
        assert_eq!(layout.children[0].extent, item_extent);
    }
}

#[test]
fn fixed_extent_child_query_keeps_flutter_double_precision_at_the_edges() {
    use incular_scroll::FixedExtentSliverMetrics as Metrics;
    const GENERIC: f64 = 600.;
    const SCREEN: f64 = 411.42857142857144;
    for (end, extent, last) in [
        (1234., 0., 0),
        (0., GENERIC, 0),
        (600., GENERIC, 0),
        (601., GENERIC, 1),
        (GENERIC + 1e-10, GENERIC, 1),
        (2700., GENERIC, 4),
        (414. * 6., 414., 5),
        (SCREEN * 6. + 1e-11, SCREEN, 5),
        (GENERIC + 1e-11, GENERIC, 0),
    ] {
        assert_eq!(
            Metrics::new(12, extent).max_child_index(end),
            last,
            "extent={extent}, end={end}"
        );
    }
    let metrics = Metrics::new(3, GENERIC);
    assert_eq!(metrics.min_child_index(1199.999999999998), 2);
    assert_eq!(metrics.min_child_index(1199.9999999), 1);
}

#[test]
fn descriptors_expose_configured_index_offset_and_total_without_a_caller_extent() {
    let fill = SliverFillViewport::new((0..3).map(|_| SizedBox::shrink()));
    let fixed = SliverFixedExtentList::new(3, 30., |_| SizedBox::shrink());
    for (metrics, scroll, expected) in [
        (fill.extent_metrics(600.), 1200., (600., 2, 1, 6000., 1800.)),
        (fill.extent_metrics(600.), 0., (600., 0, 0, 6000., 1800.)),
        (fixed.extent_metrics(), 45., (30., 1, 1, 300., 90.)),
        (fixed.extent_metrics(), 0., (30., 0, 0, 300., 90.)),
    ] {
        assert_eq!(
            (
                metrics.item_extent(),
                metrics.min_child_index(scroll),
                metrics.max_child_index(scroll),
                metrics.index_to_layout_offset(10),
                metrics.max_scroll_extent()
            ),
            expected
        );
    }

    let fractional = SliverFillViewport::builder(3, |_| SizedBox::shrink())
        .viewport_fraction(0.5)
        .fallback_extent(222.);
    let regular = fractional.extent_metrics(600.);
    assert_eq!(
        (regular.item_extent(), regular.max_scroll_extent()),
        (300., 900.)
    );
    let unbounded = fractional.extent_metrics(f32::INFINITY);
    assert_eq!(
        (unbounded.item_extent(), unbounded.max_scroll_extent()),
        (111., 333.)
    );
}
