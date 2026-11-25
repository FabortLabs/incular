/// A read-only snapshot of current scroll geometry and extents.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollMetrics {
    pub pixels: f32,
    pub min_scroll_extent: f32,
    pub max_scroll_extent: f32,
    pub viewport_dimension: f32,
    pub axis: incular_config::Axis,
    pub axis_direction: incular_config::AxisDirection,
    pub device_pixel_ratio: f32,
    /// The fraction used by a PageView, or `None` for ordinary scrolling.
    pub page_viewport_fraction: Option<f32>,
}

impl ScrollMetrics {
    /// Projects this snapshot into paged metrics if it came from a PageView.
    #[must_use]
    pub fn page_metrics(self) -> Option<PageMetrics> {
        self.page_viewport_fraction?;
        Some(PageMetrics(self))
    }

    #[must_use]
    pub fn extent_before(&self) -> f32 {
        (self.pixels - self.min_scroll_extent).max(0.0)
    }

    /// Viewport extent minus any portion beyond either scroll bound.
    #[must_use]
    pub fn extent_inside(&self) -> f32 {
        let viewport = self.viewport_dimension.max(0.);
        let before = (self.min_scroll_extent - self.pixels).clamp(0., viewport);
        let after = (self.pixels - self.max_scroll_extent).clamp(0., viewport);
        viewport - before - after
    }

    #[must_use]
    pub fn extent_after(&self) -> f32 {
        (self.max_scroll_extent - self.pixels).max(0.0)
    }

    #[must_use]
    pub fn extent_total(&self) -> f32 {
        self.max_scroll_extent - self.min_scroll_extent + self.viewport_dimension
    }

    /// Whether the position is exactly on a bound, not beyond it.
    #[must_use]
    pub fn at_edge(&self) -> bool {
        self.pixels == self.min_scroll_extent || self.pixels == self.max_scroll_extent
    }

    #[must_use]
    pub fn out_of_range(&self) -> bool {
        self.pixels < self.min_scroll_extent || self.pixels > self.max_scroll_extent
    }
}

/// An immutable snapshot received with a PageView scroll notification.
///
/// Unlike a live page controller, this snapshot uses the raw clamped offset:
/// it does not subtract the centering origin of fractions larger than one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageMetrics(ScrollMetrics);

/// Fields to replace in an existing [`PageMetrics`] snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PageMetricsUpdate {
    pub pixels: Option<f32>,
    pub min_scroll_extent: Option<f32>,
    pub max_scroll_extent: Option<f32>,
    pub viewport_dimension: Option<f32>,
    pub axis_direction: Option<incular_config::AxisDirection>,
    pub device_pixel_ratio: Option<f32>,
    pub viewport_fraction: Option<f32>,
}

impl PageMetrics {
    /// Creates a page snapshot with ordinary scroll metrics and the page fraction.
    #[must_use]
    pub fn new(mut metrics: ScrollMetrics, viewport_fraction: f32) -> Self {
        metrics.page_viewport_fraction = Some(viewport_fraction);
        Self(metrics)
    }

    /// Fraction of the viewport occupied by a page.
    #[must_use]
    pub fn viewport_fraction(&self) -> f32 {
        self.0
            .page_viewport_fraction
            .expect("PageMetrics always holds page geometry")
    }

    /// Fractional page described by these snapshot values.
    #[must_use]
    pub fn page(&self) -> f32 {
        let pixels = self
            .pixels
            .max(self.min_scroll_extent)
            .min(self.max_scroll_extent)
            .max(0.);
        pixels / (self.viewport_dimension * self.viewport_fraction()).max(1.)
    }

    /// Copies this snapshot, replacing only fields supplied in `update`.
    #[must_use]
    pub fn copy_with(self, update: PageMetricsUpdate) -> Self {
        let mut scroll = self.0;
        scroll.pixels = update.pixels.unwrap_or(scroll.pixels);
        scroll.min_scroll_extent = update.min_scroll_extent.unwrap_or(scroll.min_scroll_extent);
        scroll.max_scroll_extent = update.max_scroll_extent.unwrap_or(scroll.max_scroll_extent);
        scroll.viewport_dimension = update
            .viewport_dimension
            .unwrap_or(scroll.viewport_dimension);
        if let Some(direction) = update.axis_direction {
            scroll.axis_direction = direction;
            scroll.axis = direction.axis();
        }
        scroll.device_pixel_ratio = update
            .device_pixel_ratio
            .unwrap_or(scroll.device_pixel_ratio);
        Self::new(
            scroll,
            update.viewport_fraction.unwrap_or(self.viewport_fraction()),
        )
    }
}

impl std::ops::Deref for PageMetrics {
    type Target = ScrollMetrics;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

use crate::controller::ScrollController;

impl ScrollController {
    /// Returns the current metrics snapshot used by scroll notifications and
    /// accessibility adapters.
    #[must_use]
    pub fn metrics(&self) -> ScrollMetrics {
        let state = self.state.borrow();
        let (axis, reverse) = state
            .notification_context
            .unwrap_or((incular_config::Axis::Vertical, false));
        ScrollMetrics {
            pixels: state.offset,
            min_scroll_extent: state.min_offset,
            max_scroll_extent: state.max_offset,
            viewport_dimension: state.viewport_extent,
            axis,
            axis_direction: match (axis, reverse) {
                (incular_config::Axis::Horizontal, false) => incular_config::AxisDirection::Right,
                (incular_config::Axis::Horizontal, true) => incular_config::AxisDirection::Left,
                (incular_config::Axis::Vertical, false) => incular_config::AxisDirection::Down,
                (incular_config::Axis::Vertical, true) => incular_config::AxisDirection::Up,
            },
            device_pixel_ratio: 1.,
            page_viewport_fraction: state
                .last_physics
                .and_then(crate::ScrollPhysics::page_geometry_fraction),
        }
    }

    /// Declares the axis context for a controller no viewport owns, so
    /// headless hosts and tests can make notifications and metrics
    /// describe their interpreter. Refused while an attachment owns the
    /// controller — an owned context belongs to its owner's publications
    /// — without mutating anything. Attached viewports publish context
    /// through their lease instead; construction and configuration
    /// setters never touch shared state.
    pub fn try_set_metrics_context(
        &self,
        axis: incular_config::Axis,
        reverse: bool,
    ) -> Result<(), crate::attachment::MetricWriteError> {
        let mut state = self.state.borrow_mut();
        if let Some(live) = state.metric_attachment {
            return Err(crate::attachment::MetricWriteError::attached(live.tree));
        }
        state.notification_context = Some((axis, reverse));
        Ok(())
    }
}
