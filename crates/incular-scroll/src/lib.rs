//! Widget-independent scroll state, physics, and scrollbar geometry.

mod activity;
mod animation_completion;
mod attachment;
mod ballistic;
mod controller;
mod coordinator;
mod extent_index;
mod fixed_extent;
mod metrics;
mod notifications;
mod physics;
mod restoration;
mod scrollbar;
mod sliver;

pub use activity::{
    ActivityId, ActivityOrigin, DragStartBehavior, OwnedActivity, OwnedDrive,
    ScrollViewKeyboardDismissBehavior,
};
pub use animation_completion::{ScrollAnimationCompletion, ScrollAnimationSignal};
pub use attachment::{
    AttachmentConflict, MetricAttachment, MetricOwner, MetricWriteError, ViewportMetricsUpdate,
    ViewportPositionHandoff,
};
pub use ballistic::{
    ClampingScrollSimulation, FlingSimulation, FlingSimulationKind, ScrollSpring, ScrollTolerance,
};
pub use controller::{ScrollAnimation, ScrollController, page_for_pixels, pixels_for_page};
pub use coordinator::{NestedScrollCoordinator, NestedScrollViewCoordinator, ScrollDriveDelegate};
pub use extent_index::{ExtentIndexMetrics, MeasuredExtentIndex};
pub use fixed_extent::FixedExtentSliverMetrics;
pub use metrics::{PageMetrics, PageMetricsUpdate, ScrollMetrics};
pub use notifications::{
    ScrollNotification, ScrollNotificationSubscription, ScrollNotificationType,
};
pub use physics::{
    BoundaryPhysics, ClampingScrollPhysics, ScrollDecelerationRate, ScrollDelta, ScrollPhysics,
    ScrollSpringStep, Scrollability, SnapPhysics,
};
pub use scrollbar::{
    ScrollbarGeometry, ScrollbarStyle, scrollbar_geometry, scrollbar_geometry_for_axis,
};
pub use sliver::{SliverConstraints, SliverGeometry, SliverGeometryValidationError, SliverGrowthDirection, apply_growth_direction_to_axis_direction};
