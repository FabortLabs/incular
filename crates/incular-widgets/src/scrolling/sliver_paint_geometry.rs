//! Coordinate conversion shared by painted sliver decorations and clips.

use incular_core::{Rect, Size};
use incular_scroll::{SliverConstraints, SliverGeometry, SliverGrowthDirection};

/// Full sliver paint bounds, including its cached offscreen extent, in the
/// physical coordinate space of its render sliver.
pub(super) struct SliverPaintBounds {
    pub rect: Rect,
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

fn is_reversed(constraints: SliverConstraints) -> bool {
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
}
