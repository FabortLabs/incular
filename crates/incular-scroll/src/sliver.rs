//! Renderer-independent sliver protocol values.
//!
//! A sliver is laid out in the viewport's main axis rather than receiving a
//! normal finite box. These values mirror the information exchanged by
//! Flutter's `RenderViewport` and `RenderSliver` without making the scroll
//! crate depend on widget or renderer types.

use std::hash::{Hash, Hasher};

use incular_config::{Axis, AxisDirection, EdgeInsets, TextDirection};
use incular_core::{finite_non_negative, finite_or_zero};

/// Direction in which a sliver grows from its viewport center, independent
/// of the viewport's physical axis direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SliverGrowthDirection {
    #[default]
    Forward,
    Reverse,
}

impl SliverGrowthDirection {
    #[must_use]
    pub const fn reversed(self) -> Self {
        match self {
            Self::Forward => Self::Reverse,
            Self::Reverse => Self::Forward,
        }
    }
}

/// Resolves logical growth into the viewport's physical axis.
#[must_use]
pub const fn apply_growth_direction_to_axis_direction(
    axis: AxisDirection,
    growth: SliverGrowthDirection,
) -> AxisDirection {
    match growth {
        SliverGrowthDirection::Forward => axis,
        SliverGrowthDirection::Reverse => match axis {
            AxisDirection::Down => AxisDirection::Up,
            AxisDirection::Up => AxisDirection::Down,
            AxisDirection::Right => AxisDirection::Left,
            AxisDirection::Left => AxisDirection::Right,
        },
    }
}

/// The constraints supplied by a viewport to one sliver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliverConstraints {
    pub axis: Axis,
    pub reverse: bool,
    pub growth_direction: SliverGrowthDirection,
    /// Ambient direction for slivers with logical edges, independent of the physical axis.
    pub text_direction: Option<TextDirection>,
    /// Ambient safe padding after consumption by ancestor safe-area slivers.
    pub safe_insets: EdgeInsets,
    pub scroll_offset: f32,
    pub preceding_scroll_extent: f32,
    pub overlap: f32,
    pub remaining_paint_extent: f32,
    pub cross_axis_extent: f32,
    pub viewport_main_axis_extent: f32,
    pub remaining_cache_extent: f32,
    pub cache_origin: f32,
}

impl Hash for SliverConstraints {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.axis.hash(state);
        self.reverse.hash(state);
        self.growth_direction.hash(state);
        self.text_direction.hash(state);
        self.safe_insets.hash(state);
        for value in [
            self.scroll_offset,
            self.preceding_scroll_extent,
            self.overlap,
            self.remaining_paint_extent,
            self.cross_axis_extent,
            self.viewport_main_axis_extent,
            self.remaining_cache_extent,
            self.cache_origin,
        ] {
            (if value == 0. { 0 } else { value.to_bits() }).hash(state);
        }
    }
}

impl SliverConstraints {
    /// Creates normalized constraints for one sliver.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        axis: Axis,
        reverse: bool,
        scroll_offset: f32,
        preceding_scroll_extent: f32,
        overlap: f32,
        remaining_paint_extent: f32,
        cross_axis_extent: f32,
        viewport_main_axis_extent: f32,
        remaining_cache_extent: f32,
        cache_origin: f32,
    ) -> Self {
        Self {
            axis,
            reverse,
            growth_direction: SliverGrowthDirection::Forward,
            text_direction: None,
            safe_insets: EdgeInsets::ZERO,
            scroll_offset: finite_non_negative(scroll_offset),
            preceding_scroll_extent: finite_non_negative(preceding_scroll_extent),
            overlap: finite_or_zero(overlap),
            remaining_paint_extent: finite_non_negative(remaining_paint_extent),
            cross_axis_extent: finite_non_negative(cross_axis_extent),
            viewport_main_axis_extent: finite_non_negative(viewport_main_axis_extent),
            remaining_cache_extent: finite_non_negative(remaining_cache_extent),
            cache_origin: finite_or_zero(cache_origin),
        }
    }

    #[must_use]
    pub const fn axis_direction(self) -> AxisDirection {
        match (self.axis, self.reverse) {
            (Axis::Vertical, false) => AxisDirection::Down,
            (Axis::Vertical, true) => AxisDirection::Up,
            (Axis::Horizontal, false) => AxisDirection::Right,
            (Axis::Horizontal, true) => AxisDirection::Left,
        }
    }

    #[must_use]
    pub const fn with_axis_direction(mut self, direction: AxisDirection) -> Self {
        self.axis = direction.axis();
        self.reverse = direction.is_reversed();
        self
    }

    /// Expresses growth relative to the canonical physical down/right axes.
    #[must_use]
    pub const fn normalized_growth_direction(self) -> SliverGrowthDirection {
        if self.reverse {
            self.growth_direction.reversed()
        } else {
            self.growth_direction
        }
    }

    /// Returns the visible interval in this sliver's local coordinates.
    #[must_use]
    pub fn visible_interval(self, scroll_extent: f32) -> (f32, f32) {
        let extent = finite_non_negative(scroll_extent);
        let start = self.scroll_offset.min(extent);
        let end = (start + self.remaining_paint_extent).min(extent);
        (start, end.max(start))
    }
}

/// The result returned by a sliver after layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliverGeometry {
    pub scroll_extent: f32,
    pub paint_extent: f32,
    pub layout_extent: f32,
    pub max_paint_extent: f32,
    /// Main-axis space a pinned sliver can permanently cover at the
    /// viewport's leading edge. A merely floating sliver reports zero.
    pub max_scroll_obstruction_extent: f32,
    pub hit_test_extent: f32,
    pub paint_origin: f32,
    pub cache_extent: f32,
    pub visible: bool,
    pub has_visual_overflow: bool,
    /// A sliver can request a correction after a measurement changes the
    /// content before the current anchor.
    pub scroll_offset_correction: Option<f32>,
}

/// An inconsistency returned by a custom sliver's layout.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum SliverGeometryValidationError {
    #[error("sliver {field} must be nonnegative (got {value})")]
    NegativeExtent { field: &'static str, value: f32 },
    #[error("sliver layoutExtent {layout} exceeds paintExtent {paint}")]
    LayoutExceedsPaint { layout: f32, paint: f32 },
    #[error("sliver maxPaintExtent {maximum} is less than paintExtent {paint}")]
    MaxPaintTooSmall { maximum: f32, paint: f32 },
    #[error("sliver scrollOffsetCorrection must be nonzero when supplied")]
    ZeroCorrection,
}

impl Default for SliverGeometry {
    fn default() -> Self {
        Self::ZERO
    }
}

impl SliverGeometry {
    /// An empty geometry result.
    pub const ZERO: Self = Self {
        scroll_extent: 0.,
        paint_extent: 0.,
        layout_extent: 0.,
        max_paint_extent: 0.,
        max_scroll_obstruction_extent: 0.,
        hit_test_extent: 0.,
        paint_origin: 0.,
        cache_extent: 0.,
        visible: false,
        has_visual_overflow: false,
        scroll_offset_correction: None,
    };

    /// Builds ordinary flowing geometry for a sliver with `scroll_extent`.
    #[must_use]
    pub fn from_scroll_extent(constraints: SliverConstraints, scroll_extent: f32) -> Self {
        let scroll_extent = finite_non_negative(scroll_extent);
        let visible_start = constraints.scroll_offset.min(scroll_extent);
        let paint_extent = (scroll_extent - visible_start)
            .min(constraints.remaining_paint_extent)
            .max(0.);
        let layout_extent = paint_extent.min(scroll_extent).max(0.);
        let cache_start = (constraints.scroll_offset + constraints.cache_origin).max(0.);
        let cache_extent = (scroll_extent - cache_start)
            .min(constraints.remaining_cache_extent.max(0.))
            .max(0.);
        Self {
            scroll_extent,
            paint_extent,
            layout_extent,
            max_paint_extent: scroll_extent,
            max_scroll_obstruction_extent: 0.,
            hit_test_extent: paint_extent,
            paint_origin: 0.,
            cache_extent,
            visible: paint_extent > 0.,
            has_visual_overflow: visible_start > 0.
                || scroll_extent > constraints.remaining_paint_extent,
            scroll_offset_correction: None,
        }
    }

    /// Checks the same extent relationships as Flutter's debug validator
    /// without normalizing the inputs or panicking.
    pub fn validate(self) -> Result<(), SliverGeometryValidationError> {
        use SliverGeometryValidationError::*;
        for (field, value) in [
            ("scrollExtent", self.scroll_extent),
            ("paintExtent", self.paint_extent),
            ("layoutExtent", self.layout_extent),
            ("cacheExtent", self.cache_extent),
        ] {
            if value < 0. || value.is_nan() {
                return Err(NegativeExtent { field, value });
            }
        }
        if self.layout_extent > self.paint_extent {
            return Err(LayoutExceedsPaint {
                layout: self.layout_extent,
                paint: self.paint_extent,
            });
        }
        const PRECISION_ERROR_TOLERANCE: f64 = 1e-10;
        if f64::from(self.paint_extent) - f64::from(self.max_paint_extent)
            > PRECISION_ERROR_TOLERANCE
        {
            return Err(MaxPaintTooSmall {
                maximum: self.max_paint_extent,
                paint: self.paint_extent,
            });
        }
        if self.hit_test_extent < 0. || self.hit_test_extent.is_nan() {
            return Err(NegativeExtent {
                field: "hitTestExtent",
                value: self.hit_test_extent,
            });
        }
        if self.scroll_offset_correction == Some(0.) {
            return Err(ZeroCorrection);
        }
        Ok(())
    }

    /// Returns a copy with bounded, internally consistent values.
    #[must_use]
    pub fn normalized(self) -> Self {
        // A negative/non-finite scroll extent means the producer did not
        // return usable geometry. Treat the whole result as empty instead of
        // allowing an unrelated max-paint value to keep a phantom sliver
        // alive.
        if !self.scroll_extent.is_finite() || self.scroll_extent < 0. {
            return Self {
                scroll_offset_correction: self
                    .scroll_offset_correction
                    .filter(|value| value.is_finite()),
                ..Self::ZERO
            };
        }
        let scroll_extent = finite_non_negative(self.scroll_extent);
        let max_paint_extent = finite_non_negative(self.max_paint_extent).max(scroll_extent);
        let paint_extent = finite_non_negative(self.paint_extent).min(max_paint_extent);
        let layout_extent = finite_non_negative(self.layout_extent).min(paint_extent);
        Self {
            scroll_extent,
            paint_extent,
            layout_extent,
            max_paint_extent,
            max_scroll_obstruction_extent: finite_non_negative(self.max_scroll_obstruction_extent),
            hit_test_extent: finite_non_negative(self.hit_test_extent),
            paint_origin: finite_or_zero(self.paint_origin),
            cache_extent: finite_non_negative(self.cache_extent),
            visible: self.visible && paint_extent > 0.,
            has_visual_overflow: self.has_visual_overflow,
            scroll_offset_correction: self
                .scroll_offset_correction
                .filter(|value| value.is_finite()),
        }
    }
}
