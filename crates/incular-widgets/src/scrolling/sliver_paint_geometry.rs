//! Coordinate conversion shared by painted sliver decorations and clips.

use incular_core::{Rect, Size};
use incular_scroll::{SliverConstraints, SliverGeometry, SliverGrowthDirection};

/// Full sliver paint bounds, including its cached offscreen extent, in the
/// physical coordinate space of its render sliver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SliverPaintBounds {
    pub(super) rect: Rect,
    /// Distance from the maximum rectangle's leading edge to its visible
    /// paint origin, in logical scroll direction. Pinning can cap this.
    logical_leading: f32,
}

impl SliverPaintBounds {
    pub fn new(
        constraints: SliverConstraints,
        geometry: SliverGeometry,
        reported_cross_extent: Option<f32>,
    ) -> Option<Self> {
        let main = if geometry.max_paint_extent.is_infinite() {
            constraints.scroll_offset + geometry.cache_extent + constraints.cache_origin
        } else {
            geometry.max_paint_extent
        };
        if main <= 0. || !main.is_finite() {
            return None;
        }
        let cross = reported_cross_extent.unwrap_or(constraints.cross_axis_extent);
        let size = constraints.axis.size(main, cross);
        let logical_leading = constraints.scroll_offset.clamp(
            0.,
            (geometry.scroll_extent - geometry.max_scroll_obstruction_extent).max(0.),
        );
        let physical_origin = if is_reversed(constraints) {
            // Match RenderSliver.getMaxPaintRect: the physical reverse axis
            // starts at the far edge of the *currently painted* extent.
            geometry.paint_extent + logical_leading - main
        } else {
            -logical_leading
        };
        Some(Self {
            rect: Rect::from_origin_size(constraints.axis.offset(physical_origin, 0.), size),
            logical_leading,
        })
    }

    pub fn size(&self) -> Size {
        self.rect.size
    }

    pub fn content_offset(&self, constraints: SliverConstraints) -> f32 {
        constraints.scroll_offset - self.logical_leading
    }
}

pub(super) fn is_reversed(constraints: SliverConstraints) -> bool {
    constraints.reverse ^ (constraints.growth_direction == SliverGrowthDirection::Reverse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use incular_config::Axis;
    use incular_core::Offset;

    fn paint_bounds(
        axis: Axis,
        reverse: bool,
        growth: SliverGrowthDirection,
        geometry: SliverGeometry,
        cross: Option<f32>,
    ) -> SliverPaintBounds {
        let mut constraints =
            SliverConstraints::new(axis, reverse, 385., 0., 50., 600., 800., 600., 850., -50.);
        constraints.growth_direction = growth;
        SliverPaintBounds::new(constraints, geometry, cross).unwrap()
    }

    #[test]
    fn maximum_rect_is_relative_to_paint_origin_and_the_growing_axis() {
        let geometry = SliverGeometry {
            scroll_extent: 400.,
            paint_extent: 15.,
            max_paint_extent: 400.,
            ..SliverGeometry::ZERO
        };
        let vertical = paint_bounds(
            Axis::Vertical,
            false,
            SliverGrowthDirection::Forward,
            geometry,
            None,
        );
        assert_eq!(
            vertical.rect,
            Rect::from_origin_size(Offset::new(0., -385.), Size::new(800., 400.))
        );
        let backwards = paint_bounds(
            Axis::Horizontal,
            true,
            SliverGrowthDirection::Forward,
            geometry,
            Some(120.),
        );
        assert_eq!(
            backwards.rect,
            Rect::from_origin_size(Offset::ZERO, Size::new(400., 120.))
        );
        let double_reverse = paint_bounds(
            Axis::Horizontal,
            true,
            SliverGrowthDirection::Reverse,
            geometry,
            Some(120.),
        );
        assert_eq!(
            double_reverse.rect,
            Rect::from_origin_size(Offset::new(-385., 0.), Size::new(400., 120.))
        );
    }

    #[test]
    fn maximum_rect_stops_at_cache_end_or_pinned_obstruction() {
        let geometry = SliverGeometry {
            scroll_extent: 400.,
            paint_extent: 100.,
            max_paint_extent: 400.,
            max_scroll_obstruction_extent: 100.,
            ..SliverGeometry::ZERO
        };
        let pinned = paint_bounds(
            Axis::Vertical,
            false,
            SliverGrowthDirection::Forward,
            geometry,
            None,
        );
        assert_eq!(
            pinned.rect,
            Rect::from_origin_size(Offset::new(0., -300.), Size::new(800., 400.))
        );
        let open = paint_bounds(
            Axis::Vertical,
            false,
            SliverGrowthDirection::Forward,
            SliverGeometry {
                max_paint_extent: f32::INFINITY,
                cache_extent: 600.,
                ..geometry
            },
            None,
        );
        assert_eq!(open.size(), Size::new(800., 935.));
        assert_eq!(open.rect.origin, Offset::new(0., -300.));
    }

    #[test]
    fn exact_flutter_maximum_rects_use_physical_axes_and_reverse_growth() {
        use SliverGrowthDirection::{Forward, Reverse};
        let size = Size::new(100., 100.);
        for (axis, reverse, growth, scroll, origin) in [
            (Axis::Vertical, false, Forward, 0., Offset::ZERO),
            (Axis::Vertical, false, Reverse, 0., Offset::ZERO),
            (Axis::Vertical, false, Forward, 50., Offset::new(0., -50.)),
            (Axis::Vertical, true, Forward, 0., Offset::ZERO),
            (Axis::Vertical, true, Forward, 50., Offset::ZERO),
            (Axis::Horizontal, false, Forward, 0., Offset::ZERO),
            (Axis::Horizontal, false, Forward, 50., Offset::new(-50., 0.)),
            (Axis::Horizontal, true, Forward, 0., Offset::ZERO),
            (Axis::Horizontal, true, Forward, 50., Offset::ZERO),
        ] {
            let paint = 100. - scroll;
            let remaining = if scroll == 0. { 300. } else { 50. };
            let mut constraints = SliverConstraints::new(
                axis, reverse, scroll, 0., 0., remaining, 100., 100., 100., 0.,
            );
            constraints.growth_direction = growth;
            let geometry = SliverGeometry {
                scroll_extent: 100.,
                paint_extent: paint,
                max_paint_extent: 100.,
                ..SliverGeometry::ZERO
            };
            let actual = SliverPaintBounds::new(constraints, geometry, None)
                .unwrap()
                .rect;
            assert_eq!(
                actual,
                Rect::from_origin_size(origin, size),
                "{axis:?} reverse={reverse}, growth={growth:?}, offset={scroll}"
            );
        }
    }

    #[test]
    fn exact_flutter_maximum_rects_resolve_obstruction_cache_and_cross_axis_override() {
        let vertical = |scroll, remaining, cache, cache_origin| {
            SliverConstraints::new(
                Axis::Vertical,
                false,
                scroll,
                0.,
                0.,
                remaining,
                100.,
                100.,
                cache,
                cache_origin,
            )
        };
        let geometry = |scroll_extent, paint_extent, max_paint_extent| SliverGeometry {
            scroll_extent,
            paint_extent,
            max_paint_extent,
            ..SliverGeometry::ZERO
        };
        for (constraints, geometry, cross, expected) in [
            (
                vertical(95., 10., 100., 0.),
                SliverGeometry {
                    max_scroll_obstruction_extent: 10.,
                    ..geometry(100., 10., 100.)
                },
                None,
                Rect::from_origin_size(Offset::new(0., -90.), Size::new(100., 100.)),
            ),
            (
                vertical(50., 300., 200., -50.),
                SliverGeometry {
                    cache_extent: 150.,
                    ..geometry(f32::INFINITY, 100., f32::INFINITY)
                },
                None,
                Rect::from_origin_size(Offset::new(0., -50.), Size::new(100., 150.)),
            ),
            (
                vertical(0., 300., 100., 0.),
                geometry(100., 100., 100.),
                Some(50.),
                Rect::from_origin_size(Offset::ZERO, Size::new(50., 100.)),
            ),
            (
                vertical(0., 100., 100., 0.),
                geometry(200., 100., 150.),
                None,
                Rect::from_origin_size(Offset::ZERO, Size::new(100., 150.)),
            ),
        ] {
            assert_eq!(
                SliverPaintBounds::new(constraints, geometry, cross)
                    .unwrap()
                    .rect,
                expected
            );
        }
    }
}
