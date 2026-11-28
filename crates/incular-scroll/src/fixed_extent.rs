use std::ops::Range;

/// A constant-time index for a sliver whose children have one configured
/// main-axis extent. The query surface keeps double-precision coordinates;
/// materialized widgets can still use native single-precision geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedExtentSliverMetrics {
    child_count: usize,
    item_extent: f64,
}

impl FixedExtentSliverMetrics {
    /// Invalid or negative extents are treated as zero, as on the existing
    /// fixed-extent list constructor.
    #[must_use]
    pub fn new(child_count: usize, item_extent: f64) -> Self {
        Self {
            child_count,
            item_extent: if item_extent.is_finite() {
                item_extent.max(0.)
            } else {
                0.
            },
        }
    }

    #[must_use]
    pub const fn item_extent(self) -> f64 {
        self.item_extent
    }

    /// The logical offset is useful before a child exists; the index is
    /// intentionally not clamped to the current child count.
    #[must_use]
    pub fn index_to_layout_offset(self, index: usize) -> f64 {
        index as f64 * self.item_extent
    }

    #[must_use]
    pub fn max_scroll_extent(self) -> f64 {
        self.index_to_layout_offset(self.child_count)
    }

    /// First child overlapping the beginning of a nonnegative scroll interval.
    #[must_use]
    pub fn min_child_index(self, offset: f64) -> usize {
        if self.item_extent == 0. {
            return 0;
        }
        let fraction = offset / self.item_extent;
        snap_double_index(fraction, self.item_extent).floor() as usize
    }

    /// Last index before the end of a half-open interval. The public query
    /// returns zero even for an interval ending at zero.
    #[must_use]
    pub fn max_child_index(self, offset: f64) -> usize {
        if self.item_extent == 0. {
            return 0;
        }
        let fraction = offset / self.item_extent - 1.;
        snap_double_index(fraction, self.item_extent).ceil() as usize
    }

    /// Materialization uses single-precision layout coordinates, with one
    /// representable step at a boundary instead of a fixed double epsilon.
    /// Unlike the direct index query, an empty cache window can return no slots.
    #[doc(hidden)]
    #[must_use]
    pub fn cache_range_f32(self, start: f32, end: f32) -> Range<usize> {
        let (first, past_last) = if self.item_extent > 0. {
            (
                self.layout_ratio(start).floor() as usize,
                self.layout_ratio(end).ceil() as usize,
            )
        } else {
            // The first zero-sized child can lay out without visiting other
            // children that all share its offset.
            (0, 1)
        };
        let first = first.min(self.child_count);
        first..past_last.min(self.child_count).max(first)
    }

    fn layout_ratio(self, offset: f32) -> f64 {
        let coordinate = f64::from(offset);
        let ratio = coordinate / self.item_extent;
        let nearest = ratio.round();
        let tolerance = f64::from(offset.next_up()) - coordinate;
        if (coordinate - nearest * self.item_extent).abs() <= tolerance {
            nearest
        } else {
            ratio
        }
    }
}

fn snap_double_index(fraction: f64, item_extent: f64) -> f64 {
    // Flutter's precisionErrorTolerance is measured in pixels, not items.
    const PIXEL_TOLERANCE: f64 = 1e-10;
    let nearest = fraction.round();
    if (fraction * item_extent - nearest * item_extent).abs() < PIXEL_TOLERANCE {
        nearest
    } else {
        fraction
    }
}
