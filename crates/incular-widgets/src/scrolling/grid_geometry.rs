use incular_config::{Axis, Constraints};

/// Placement and size of one tile in sliver coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliverGridGeometry {
    pub scroll_offset: f32,
    pub cross_axis_offset: f32,
    pub main_axis_extent: f32,
    pub cross_axis_extent: f32,
}

impl SliverGridGeometry {
    #[must_use]
    pub fn trailing_scroll_offset(self) -> f32 {
        self.scroll_offset + self.main_axis_extent
    }

    /// Forces the tile's size without changing the axis of its parent.
    #[must_use]
    pub fn box_constraints(self, axis: Axis) -> Constraints {
        Constraints::tight(axis.size(self.main_axis_extent, self.cross_axis_extent))
    }
}

/// Equally sized and spaced sliver tiles (Flutter's
/// `SliverGridRegularTileLayout`). Scroll offsets are logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliverGridRegularTileLayout {
    cross_axis_count: usize,
    main_axis_stride: f32,
    cross_axis_stride: f32,
    child_main_axis_extent: f32,
    child_cross_axis_extent: f32,
    reverse_cross_axis: bool,
}

impl SliverGridRegularTileLayout {
    #[must_use]
    pub fn new(
        cross_axis_count: usize,
        main_axis_stride: f32,
        cross_axis_stride: f32,
        child_main_axis_extent: f32,
        child_cross_axis_extent: f32,
        reverse_cross_axis: bool,
    ) -> Self {
        debug_assert!(cross_axis_count > 0);
        debug_assert!(main_axis_stride >= 0. && cross_axis_stride >= 0.);
        debug_assert!(child_main_axis_extent >= 0. && child_cross_axis_extent >= 0.);
        Self {
            cross_axis_count: cross_axis_count.max(1),
            main_axis_stride,
            cross_axis_stride,
            child_main_axis_extent,
            child_cross_axis_extent,
            reverse_cross_axis,
        }
    }

    #[must_use]
    pub const fn cross_axis_count(self) -> usize {
        self.cross_axis_count
    }

    #[must_use]
    pub const fn main_axis_stride(self) -> f32 {
        self.main_axis_stride
    }

    /// The first tile in the row at a scroll offset. Flutter treats strides
    /// at or below precisionErrorTolerance as starting at tile zero.
    #[must_use]
    pub fn min_child_index_for_scroll_offset(self, offset: f32) -> usize {
        const PRECISION_ERROR_TOLERANCE: f64 = 1e-10;
        let stride = f64::from(self.main_axis_stride);
        if stride <= PRECISION_ERROR_TOLERANCE {
            return 0;
        }
        let row = (f64::from(offset) / stride).trunc() as usize;
        row.saturating_mul(self.cross_axis_count)
    }

    /// The inclusive trailing tile at a scroll offset. A zero stride or
    /// zero window still samples the leading tile, not a whole row.
    #[must_use]
    pub fn max_child_index_for_scroll_offset(self, offset: f32) -> usize {
        let stride = f64::from(self.main_axis_stride);
        if stride <= 0. {
            return 0;
        }
        let rows = (f64::from(offset) / stride).ceil() as usize;
        rows.saturating_mul(self.cross_axis_count).saturating_sub(1)
    }

    #[must_use]
    pub fn geometry_for_child_index(self, index: usize) -> SliverGridGeometry {
        let row = index / self.cross_axis_count;
        let column = index % self.cross_axis_count;
        let placed_column = if self.reverse_cross_axis {
            self.cross_axis_count - 1 - column
        } else {
            column
        };
        SliverGridGeometry {
            scroll_offset: row as f32 * self.main_axis_stride,
            cross_axis_offset: placed_column as f32 * self.cross_axis_stride,
            main_axis_extent: self.child_main_axis_extent,
            cross_axis_extent: self.child_cross_axis_extent,
        }
    }

    /// The total content extent, without spacing after the last row.
    #[must_use]
    pub fn compute_max_scroll_offset(self, child_count: usize) -> f32 {
        if child_count == 0 {
            return 0.;
        }
        self.geometry_for_child_index(child_count - 1)
            .trailing_scroll_offset()
    }
}
