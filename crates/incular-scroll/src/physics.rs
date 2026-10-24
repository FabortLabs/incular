use crate::ballistic::{FlingSimulation, ScrollSpring, ScrollTolerance};
use crate::{
    attachment::{MetricWriteError, ViewportMetricsUpdate},
    controller::{ScrollController, ScrollState},
    notifications::ScrollNotificationType,
    restoration::{ScrollRestoration, persist_scroll_offset},
};

/// Flutter `kTouchSlop`: the default `ScrollPhysics.minFlingDistance`.
const TOUCH_SLOP: f32 = 18.;
/// Flutter `kMinFlingVelocity`.
const MIN_FLING_VELOCITY: f32 = 50.;
/// Flutter `kMaxFlingVelocity`.
const MAX_FLING_VELOCITY: f32 = 8000.;

/// Flutter `BouncingScrollPhysics.applyPhysicsToUserOffset`, taking and
/// returning a scroll-offset delta. Flutter's version works on the finger's
/// delta, which is the negation: the offset grows as the finger moves back.
fn bouncing_user_offset(
    rate: ScrollDecelerationRate,
    pixels: f32,
    offset: f32,
    min: f32,
    max: f32,
    viewport: f32,
) -> f32 {
    if offset == 0. || (pixels >= min && pixels <= max) {
        return offset;
    }
    -bouncing_finger_offset(rate, pixels, -offset, min, max, viewport)
}

fn bouncing_finger_offset(
    rate: ScrollDecelerationRate,
    pixels: f32,
    offset: f32,
    min: f32,
    max: f32,
    viewport: f32,
) -> f32 {
    let past_start = (min - pixels).max(0.);
    let past_end = (pixels - max).max(0.);
    let past = past_start.max(past_end);
    let easing = (past_start > 0. && offset < 0.) || (past_end > 0. && offset > 0.);
    let fraction = |overscroll: f32| {
        if viewport > 0. {
            overscroll / viewport
        } else {
            0.
        }
    };
    // `frictionFactor`: starts at 0.52 (0.26 when fast) and hardens as more
    // of the viewport is dragged past the edge.
    let friction_factor = |overscroll_fraction: f32| {
        (1. - overscroll_fraction).powi(2)
            * match rate {
                ScrollDecelerationRate::Fast => 0.26,
                ScrollDecelerationRate::Normal => 0.52,
            }
    };
    // Apply less resistance when easing the overscroll than when tensioning.
    let friction = if easing {
        friction_factor(fraction(past - offset.abs()))
    } else {
        friction_factor(fraction(past))
    };
    let direction = offset.signum();
    if easing && rate == ScrollDecelerationRate::Fast {
        return direction * offset.abs();
    }
    direction * apply_friction(past, offset.abs(), friction)
}

/// Flutter `BouncingScrollPhysics._applyFriction`.
fn apply_friction(extent_outside: f32, abs_delta: f32, gamma: f32) -> f32 {
    let mut total = 0.;
    let mut remaining = abs_delta;
    if extent_outside > 0. {
        let delta_to_limit = extent_outside / gamma;
        if remaining < delta_to_limit {
            return remaining * gamma;
        }
        total += extent_outside;
        remaining -= delta_to_limit;
    }
    total + remaining
}

/// Snapshot carried from the locked extent commit to the unlocked
/// notification phase, so every publication path dispatches from the
/// same committed values.
pub(crate) struct ExtentPublication {
    pub(crate) persistence: Option<(Option<ScrollRestoration>, f32)>,
    pub(crate) content: f32,
    pub(crate) viewport: f32,
    pub(crate) old_content: f32,
    pub(crate) old_viewport: f32,
    pub(crate) old_offset: f32,
}

/// The default clamping policy used by native desktop/mobile views.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClampingScrollPhysics;
impl ClampingScrollPhysics {
    #[must_use]
    pub fn apply(self, current: f32, delta: f32, min: f32, max: f32) -> f32 {
        (current + delta).clamp(min.min(max), max.max(min))
    }
}

/// Whether a policy participates in a gesture when there is no scroll range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scrollability {
    /// Accept a gesture only when a position can move.
    #[default]
    WhenScrollable,
    /// Accept a gesture even for a zero-length range (useful for pull effects).
    Always,
    /// Never consume a gesture.
    Never,
}

/// Flutter `ScrollDecelerationRate`: how quickly a bouncing scroll slows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollDecelerationRate {
    /// The iOS-like default.
    #[default]
    Normal,
    /// Faster deceleration with harder overscroll resistance.
    Fast,
}

/// Boundary behavior for the unified [`ScrollPhysics`] policy.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BoundaryPhysics {
    /// Clamp at the content bounds and return the unused delta to a parent
    /// coordinator.
    #[default]
    Clamping,
    /// Flutter `BouncingScrollPhysics`: the position may leave the content
    /// bounds under growing resistance and a spring returns it.
    Bouncing {
        /// Flutter `BouncingScrollPhysics.decelerationRate`.
        deceleration_rate: ScrollDecelerationRate,
    },
}

/// Optional deterministic settle target after wheel/drag momentum ends.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SnapPhysics {
    #[default]
    None,
    Page {
        extent: f32,
    },
    /// A page whose extent is the viewport's current main-axis dimension.
    /// This is the policy used by a Flutter-style `PageView` with static
    /// children; unlike a fixed page extent it remains correct after a window
    /// resize.
    PageViewport,
    FixedExtent {
        extent: f32,
    },
}

/// Result of one policy application. `unconsumed` is deliberately explicit so
/// nested viewports transfer a delta once, rather than applying it to every
/// ancestor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollDelta {
    pub position: f32,
    pub consumed: f32,
    pub unconsumed: f32,
    pub overscroll: f32,
    pub accepted: bool,
}

/// One monotonic-frame spring integration result for bouncing scroll.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollSpringStep {
    pub position: f32,
    pub velocity: f32,
    pub settled: bool,
}

/// Rust-native composable scroll policy. It replaces a hierarchy of widget
/// subclasses with independent scrollability, boundary, and snap choices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollPhysics {
    pub scrollability: Scrollability,
    pub boundary: BoundaryPhysics,
    pub snap: SnapPhysics,
    maintain_range: bool,
    /// Device pixel ratio that scales the tolerances (Flutter reads it from
    /// `ScrollMetrics.devicePixelRatio`).
    device_pixel_ratio: f32,
}

impl Default for ScrollPhysics {
    fn default() -> Self {
        Self::clamping()
    }
}

impl ScrollPhysics {
    #[must_use]
    pub const fn clamping() -> Self {
        Self {
            scrollability: Scrollability::WhenScrollable,
            boundary: BoundaryPhysics::Clamping,
            snap: SnapPhysics::None,
            maintain_range: false,
            device_pixel_ratio: 1.,
        }
    }

    #[must_use]
    pub const fn always_scrollable(mut self) -> Self {
        self.scrollability = Scrollability::Always;
        self
    }

    #[must_use]
    pub const fn never_scrollable(mut self) -> Self {
        self.scrollability = Scrollability::Never;
        self
    }

    /// Flutter `BouncingScrollPhysics()` with the default
    /// [`ScrollDecelerationRate::Normal`].
    #[must_use]
    pub const fn bouncing(mut self) -> Self {
        self.boundary = BoundaryPhysics::Bouncing {
            deceleration_rate: ScrollDecelerationRate::Normal,
        };
        self
    }

    /// Flutter `BouncingScrollPhysics(decelerationRate: ...)`.
    #[must_use]
    pub const fn bouncing_with(mut self, deceleration_rate: ScrollDecelerationRate) -> Self {
        self.boundary = BoundaryPhysics::Bouncing { deceleration_rate };
        self
    }

    /// Sets the device pixel ratio that scales [`tolerance`](Self::tolerance).
    /// Non-finite and non-positive ratios keep the previous value.
    #[must_use]
    pub const fn with_device_pixel_ratio(mut self, device_pixel_ratio: f32) -> Self {
        if device_pixel_ratio.is_finite() && device_pixel_ratio > 0. {
            self.device_pixel_ratio = device_pixel_ratio;
        }
        self
    }

    /// Flutter `ScrollPhysics.toleranceFor`: velocity `1 / (0.050 * dpr)` and
    /// distance `1 / dpr`.
    #[must_use]
    pub fn tolerance(&self) -> ScrollTolerance {
        ScrollTolerance::for_device_pixel_ratio(self.device_pixel_ratio)
    }

    /// Flutter `BouncingScrollPhysics.decelerationRate`; clamping physics
    /// report [`ScrollDecelerationRate::Normal`].
    #[must_use]
    pub const fn deceleration_rate(&self) -> ScrollDecelerationRate {
        match self.boundary {
            BoundaryPhysics::Bouncing { deceleration_rate } => deceleration_rate,
            BoundaryPhysics::Clamping => ScrollDecelerationRate::Normal,
        }
    }

    /// Flutter `ScrollPhysics.spring`: mass 0.5, stiffness 100, damping
    /// ratio 1.1, or mass 0.3, stiffness 75, ratio 1.3 for fast bouncing.
    #[must_use]
    pub fn spring(&self) -> ScrollSpring {
        match self.deceleration_rate() {
            ScrollDecelerationRate::Fast => ScrollSpring::with_damping_ratio(0.3, 75., 1.3),
            ScrollDecelerationRate::Normal => ScrollSpring::with_damping_ratio(0.5, 100., 1.1),
        }
    }

    #[must_use]
    pub const fn page_snapping(mut self, extent: f32) -> Self {
        self.snap = SnapPhysics::Page { extent };
        self
    }

    /// Enables viewport-sized page snapping.  This is the composable
    /// equivalent of Flutter's `PageScrollPhysics` when the page extent is
    /// supplied by the viewport rather than a fixed constructor argument.
    #[must_use]
    pub const fn page(mut self) -> Self {
        self.snap = SnapPhysics::PageViewport;
        self
    }

    #[must_use]
    pub const fn fixed_extent_snapping(mut self, extent: f32) -> Self {
        self.snap = SnapPhysics::FixedExtent { extent };
        self
    }

    /// Applies a raw logical delta. Bounds are normalized, non-finite input is
    /// rejected, and clamping returns exactly the unused portion for nesting.
    #[must_use]
    pub fn apply_delta(self, current: f32, delta: f32, min: f32, max: f32) -> ScrollDelta {
        self.apply_delta_in_viewport(current, delta, min, max, 0.)
    }

    /// [`apply_delta`](Self::apply_delta) for a viewport of known main-axis
    /// size. Bouncing physics scale the resistance to overscroll by the
    /// fraction of the viewport already scrolled past the edge, as Flutter's
    /// `BouncingScrollPhysics.applyPhysicsToUserOffset` does; a zero viewport
    /// counts as no overscroll fraction.
    #[must_use]
    pub fn apply_delta_in_viewport(
        self,
        current: f32,
        delta: f32,
        min: f32,
        max: f32,
        viewport: f32,
    ) -> ScrollDelta {
        if !current.is_finite() || !delta.is_finite() {
            return ScrollDelta {
                position: current,
                unconsumed: delta,
                ..ScrollDelta::default()
            };
        }
        if self.scrollability == Scrollability::Never {
            return ScrollDelta {
                position: current,
                unconsumed: delta,
                ..ScrollDelta::default()
            };
        }
        let (min, max) = (min.min(max), max.max(min));
        if min == max && self.scrollability == Scrollability::WhenScrollable {
            return ScrollDelta {
                position: current.clamp(min, max),
                unconsumed: delta,
                ..ScrollDelta::default()
            };
        }
        match self.boundary {
            BoundaryPhysics::Clamping => {
                let position = (current + delta).clamp(min, max);
                let consumed = position - current;
                ScrollDelta {
                    position,
                    consumed,
                    unconsumed: delta - consumed,
                    // `ClampingScrollPhysics.applyBoundaryConditions`: the
                    // part past an edge is reported as overscroll.
                    overscroll: delta - consumed,
                    accepted: consumed != 0. || self.scrollability == Scrollability::Always,
                }
            }
            BoundaryPhysics::Bouncing { deceleration_rate } => {
                let applied =
                    bouncing_user_offset(deceleration_rate, current, delta, min, max, viewport);
                let position = current + applied;
                let consumed = position - current;
                ScrollDelta {
                    position,
                    consumed,
                    // `applyBoundaryConditions` is 0 for bouncing physics:
                    // the position owns the whole sequence, so nothing is
                    // returned to a parent.
                    unconsumed: 0.,
                    overscroll: if position < min {
                        position - min
                    } else {
                        (position - max).max(0.)
                    },
                    accepted: true,
                }
            }
        }
    }

    /// Selects the deterministic page/fixed-item settle target from current
    /// position and velocity. Pages follow Flutter's `PageScrollPhysics`: a
    /// velocity beyond the tolerance velocity (20 px/s) moves half a page in
    /// its direction, then the page rounds to the nearest whole page. Fixed
    /// extents keep the fling-strength rule (above 120 px/s rounds away from
    /// rest, below -120 rounds back; slower movement picks the nearest item).
    #[must_use]
    pub fn snap_target(self, position: f32, velocity: f32, min: f32, max: f32) -> f32 {
        self.snap_target_for_extent(position, velocity, min, max, 0.)
    }

    /// Selects a snap target, resolving [`SnapPhysics::PageViewport`] against
    /// the supplied viewport extent.
    #[must_use]
    pub fn snap_target_for_extent(
        self,
        position: f32,
        velocity: f32,
        min: f32,
        max: f32,
        viewport_extent: f32,
    ) -> f32 {
        let extent = match self.snap {
            SnapPhysics::None => return position.clamp(min.min(max), max.max(min)),
            SnapPhysics::Page { extent } | SnapPhysics::FixedExtent { extent } => extent,
            SnapPhysics::PageViewport => viewport_extent,
        };
        if !extent.is_finite() || extent <= 0. {
            return position.clamp(min.min(max), max.max(min));
        }
        let unit = position / extent;
        let index = match self.snap {
            // Flutter `PageScrollPhysics._getTargetPixels` (page_view.dart:590).
            SnapPhysics::Page { .. } | SnapPhysics::PageViewport => {
                let tolerance_velocity = self.tolerance().velocity;
                let page = if velocity < -tolerance_velocity {
                    unit - 0.5
                } else if velocity > tolerance_velocity {
                    unit + 0.5
                } else {
                    unit
                };
                page.round()
            }
            // Flutter `FixedExtentScrollPhysics`: settle on the item nearest
            // to where the underlying ballistic simulation would naturally
            // stop (`_getItemFromOffset` over `x(double.infinity)`).
            SnapPhysics::FixedExtent { .. } | SnapPhysics::None => {
                let natural = self
                    .ballistic_simulation(position, velocity, min, max)
                    .map_or(position, |simulation| simulation.final_x());
                (natural.clamp(min.min(max), max.max(min)) / extent).round()
            }
        };
        (index * extent).clamp(min.min(max), max.max(min))
    }

    /// Returns the page/fixed-item step used for semantic scroll actions.
    /// Viewport pages resolve against the current viewport dimension.
    #[must_use]
    pub fn snap_extent(self, viewport_extent: f32) -> Option<f32> {
        let extent = match self.snap {
            SnapPhysics::Page { extent } | SnapPhysics::FixedExtent { extent } => extent,
            SnapPhysics::PageViewport => viewport_extent,
            SnapPhysics::None => return None,
        };
        (extent.is_finite() && extent > 0.).then_some(extent)
    }

    /// Advances a bounce spring by `elapsed_seconds`, the monotonic time since
    /// the spring started from `position` with `velocity`. This evaluates
    /// Flutter's closed-form `ScrollSpringSimulation` toward the nearest
    /// content bound, so the result never depends on how frames partition the
    /// motion. Runtime schedulers own when to call this.
    #[must_use]
    pub fn spring_step(
        self,
        position: f32,
        velocity: f32,
        elapsed_seconds: f32,
        min: f32,
        max: f32,
    ) -> ScrollSpringStep {
        let (low, high) = (min.min(max), max.max(min));
        let target = position.clamp(low, high);
        if !matches!(self.boundary, BoundaryPhysics::Bouncing { .. }) {
            return ScrollSpringStep {
                position: target,
                velocity: 0.,
                settled: true,
            };
        }
        let simulation = FlingSimulation::scroll_spring(
            self.spring(),
            position,
            target,
            velocity,
            self.tolerance(),
        );
        let elapsed = if elapsed_seconds.is_finite() {
            elapsed_seconds.max(0.)
        } else {
            0.
        };
        let settled = simulation.is_done(elapsed);
        ScrollSpringStep {
            position: if settled {
                target
            } else {
                simulation.x(elapsed)
            },
            velocity: if settled { 0. } else { simulation.dx(elapsed) },
            settled,
        }
    }

    #[must_use]
    pub fn parent(mut self, parent: Self) -> Self {
        if self.scrollability == Scrollability::WhenScrollable {
            self.scrollability = parent.scrollability;
        }
        if self.boundary == BoundaryPhysics::Clamping {
            self.boundary = parent.boundary;
        }
        if self.snap == SnapPhysics::None {
            self.snap = parent.snap;
        }
        self.maintain_range |= parent.maintain_range;
        self
    }

    #[must_use]
    pub fn then(self, next: Self) -> Self {
        next.parent(self)
    }

    #[must_use]
    pub const fn range_maintaining(mut self) -> Self {
        self.maintain_range = true;
        self
    }

    /// Whether this policy keeps a trailing-edge position when extents
    /// change.
    #[must_use]
    pub const fn is_range_maintaining(self) -> bool {
        self.maintain_range
    }

    #[must_use]
    pub const fn carousel(mut self, item_extent: f32) -> Self {
        self.snap = SnapPhysics::FixedExtent {
            extent: item_extent,
        };
        self
    }

    /// Flutter `ScrollPhysics.minFlingVelocity`: `kMinFlingVelocity` (50
    /// px/s), doubled by `BouncingScrollPhysics` to require a more deliberate
    /// fling.
    #[must_use]
    pub const fn min_fling_velocity(&self) -> f32 {
        match self.boundary {
            BoundaryPhysics::Bouncing { .. } => MIN_FLING_VELOCITY * 2.,
            BoundaryPhysics::Clamping => MIN_FLING_VELOCITY,
        }
    }

    /// Flutter `ScrollPhysics.maxFlingVelocity`: `kMaxFlingVelocity`, eight
    /// times that for fast bouncing physics.
    #[must_use]
    pub const fn max_fling_velocity(&self) -> f32 {
        match self.deceleration_rate() {
            ScrollDecelerationRate::Fast => MAX_FLING_VELOCITY * 8.,
            ScrollDecelerationRate::Normal => MAX_FLING_VELOCITY,
        }
    }

    /// Flutter `ScrollPhysics.minFlingDistance`: `kTouchSlop`.
    #[must_use]
    pub const fn min_fling_distance(&self) -> f32 {
        TOUCH_SLOP
    }

    /// Flutter `BouncingScrollPhysics.carriedMomentum`: how much of the
    /// velocity of a fling that was held is added to the next fling in the
    /// same direction. Clamping physics carry nothing.
    #[must_use]
    pub fn carried_momentum(&self, existing_velocity: f32) -> f32 {
        match self.boundary {
            BoundaryPhysics::Bouncing { .. } if existing_velocity.is_finite() => {
                existing_velocity.signum()
                    * (0.000_816 * existing_velocity.abs().powf(1.967)).min(40_000.)
            }
            _ => 0.,
        }
    }

    /// Flutter `ScrollPhysics.createBallisticSimulation` for a scroll at
    /// `position` inside `[min, max]` released with `velocity`, or `None`
    /// when the scroll goes idle.
    ///
    /// Bouncing physics run a `BouncingScrollSimulation` whenever the speed
    /// reaches the tolerance velocity or the position is beyond an extent:
    /// friction inside the extents and a spring at the one it crosses.
    /// Clamping physics spring back from beyond an extent, otherwise run a
    /// `ClampingScrollSimulation` unless the fling pushes into the extent
    /// the position already rests on.
    #[must_use]
    pub fn ballistic_simulation(
        &self,
        position: f32,
        velocity: f32,
        min: f32,
        max: f32,
    ) -> Option<FlingSimulation> {
        if !position.is_finite() || !velocity.is_finite() {
            return None;
        }
        let (leading, trailing) = (min.min(max), max.max(min));
        let tolerance = self.tolerance();
        match self.boundary {
            BoundaryPhysics::Bouncing { deceleration_rate } => {
                let out_of_range = position < leading || position > trailing;
                (velocity.abs() >= tolerance.velocity || out_of_range).then(|| {
                    FlingSimulation::bouncing(
                        self.spring(),
                        position,
                        velocity,
                        leading,
                        trailing,
                        match deceleration_rate {
                            ScrollDecelerationRate::Fast => 1400.,
                            ScrollDecelerationRate::Normal => 0.,
                        },
                        tolerance,
                    )
                })
            }
            BoundaryPhysics::Clamping => {
                if position > trailing || position < leading {
                    let end = if position > trailing {
                        trailing
                    } else {
                        leading
                    };
                    return Some(FlingSimulation::scroll_spring(
                        self.spring(),
                        position,
                        end,
                        velocity.min(0.),
                        tolerance,
                    ));
                }
                if velocity.abs() < tolerance.velocity
                    || (velocity > 0. && position >= trailing)
                    || (velocity < 0. && position <= leading)
                {
                    return None;
                }
                Some(FlingSimulation::clamping(position, velocity))
            }
        }
    }

    /// Flutter `PageScrollPhysics.createBallisticSimulation`: a spring onto the
    /// page the release settles on (half a page further in the fling's
    /// direction past the tolerance velocity, then rounded), or the parent
    /// ballistics when out of range and not headed back in.
    #[must_use]
    pub fn page_ballistic_simulation(
        &self,
        position: f32,
        velocity: f32,
        min: f32,
        max: f32,
        page_extent: f32,
    ) -> Option<FlingSimulation> {
        let parent = self.ballistic_simulation(position, velocity, min, max);
        if !(page_extent.is_finite() && page_extent > 0.) || !position.is_finite() {
            return parent;
        }
        let (leading, trailing) = (min.min(max), max.max(min));
        if (velocity <= 0. && position <= leading) || (velocity >= 0. && position >= trailing) {
            return parent;
        }
        let target = self
            .snap_target_for_extent(position, velocity, leading, trailing, page_extent)
            .clamp(leading, trailing);
        (target != position).then(|| {
            FlingSimulation::scroll_spring(
                self.spring(),
                position,
                target,
                velocity,
                self.tolerance(),
            )
        })
    }

    /// The simulation a released drag runs, honoring the snap policy: pages
    /// and fixed extents settle on an item, anything else runs the ordinary
    /// [`ballistic_simulation`](Self::ballistic_simulation). `viewport` is the
    /// main-axis viewport size that [`SnapPhysics::PageViewport`] resolves
    /// against.
    #[must_use]
    pub fn release_simulation(
        &self,
        position: f32,
        velocity: f32,
        min: f32,
        max: f32,
        viewport: f32,
    ) -> Option<FlingSimulation> {
        match self.snap {
            SnapPhysics::None => self.ballistic_simulation(position, velocity, min, max),
            SnapPhysics::Page { extent } => {
                self.page_ballistic_simulation(position, velocity, min, max, extent)
            }
            SnapPhysics::PageViewport => {
                self.page_ballistic_simulation(position, velocity, min, max, viewport)
            }
            SnapPhysics::FixedExtent { extent } => {
                self.fixed_extent_ballistic_simulation(position, velocity, min, max, extent)
            }
        }
    }

    /// Flutter `FixedExtentScrollPhysics.createBallisticSimulation`: settle
    /// on the item nearest to where the underlying simulation would stop.
    #[must_use]
    pub fn fixed_extent_ballistic_simulation(
        &self,
        position: f32,
        velocity: f32,
        min: f32,
        max: f32,
        item_extent: f32,
    ) -> Option<FlingSimulation> {
        let parent = self.ballistic_simulation(position, velocity, min, max);
        if !(item_extent.is_finite() && item_extent > 0.) || !position.is_finite() {
            return parent;
        }
        let (leading, trailing) = (min.min(max), max.max(min));
        // Out of range and not headed back in: the parent ballistics put the
        // position back at the boundary.
        if (velocity <= 0. && position <= leading) || (velocity >= 0. && position >= trailing) {
            return parent;
        }
        // It would have ended at a scroll extent anyway.
        if let Some(natural) = &parent {
            let final_x = natural.final_x();
            if final_x == leading || final_x == trailing {
                return parent;
            }
        }
        let natural = parent.as_ref().map_or(position, FlingSimulation::final_x);
        let settling_item = (natural.clamp(leading, trailing) / item_extent).round();
        let settling_pixels = settling_item * item_extent;
        let tolerance = self.tolerance();
        // Already at rest where it intends to land.
        if velocity.abs() < tolerance.velocity
            && (settling_pixels - position).abs() < tolerance.distance
        {
            return None;
        }
        // Too slow to break past the current item: spring back onto it.
        let current_item = (position.clamp(leading, trailing) / item_extent).round();
        if settling_item == current_item {
            return Some(FlingSimulation::spring(
                self.spring(),
                position,
                settling_pixels,
                velocity,
                tolerance,
            ));
        }
        // A friction whose drag is tuned to land exactly on the item.
        Some(FlingSimulation::friction_through(
            position,
            settling_pixels,
            velocity,
            tolerance.velocity * velocity.signum(),
        ))
    }

    /// Flutter's ballistic simulation for a position that is not near an
    /// extent: [`ballistic_simulation`](Self::ballistic_simulation) over
    /// unbounded extents.
    #[must_use]
    pub fn fling_simulation(&self, position: f32, velocity: f32) -> Option<FlingSimulation> {
        self.ballistic_simulation(position, velocity, f32::NEG_INFINITY, f32::INFINITY)
    }

    /// Flutter `ScrollPhysics.dragStartDistanceMotionThreshold`: the distance a
    /// drag must travel from rest before it moves the content. Only
    /// `BouncingScrollPhysics` has one (3.5, `scroll_physics.dart:788`);
    /// clamping physics report `None` and transmit every drag offset.
    #[must_use]
    pub const fn drag_start_distance_motion_threshold(&self) -> Option<f32> {
        match self.boundary {
            BoundaryPhysics::Bouncing { .. } => Some(3.5),
            BoundaryPhysics::Clamping => None,
        }
    }
}

impl ScrollController {
    /// Applies one unified scroll policy step. Clamping is persisted normally;
    /// transient bounce positions stay in memory and return through
    /// [`ScrollPhysics::spring_step`] rather than being restored as a logical
    /// position.
    pub fn apply_physics(&self, physics: ScrollPhysics, delta: f32) -> ScrollDelta {
        let previous = self.offset();
        let result = physics.apply_delta_in_viewport(
            previous,
            delta,
            0.,
            self.max_offset(),
            self.viewport_extent(),
        );
        if result.position == previous {
            if result.overscroll != 0. {
                self.dispatch_notification(
                    ScrollNotificationType::Overscroll,
                    0.,
                    result.overscroll,
                );
            }
            return result;
        }
        let persistence = {
            let mut state = self.state.borrow_mut();
            state.offset = result.position;
            state.pending_restored_offset = None;
            state.revision += 1;
            matches!(physics.boundary, BoundaryPhysics::Clamping)
                .then(|| (state.restoration.clone(), result.position))
        };
        if let Some((restoration, offset)) = persistence {
            persist_scroll_offset(restoration, offset);
        }
        self.dispatch_notification(
            ScrollNotificationType::Update,
            result.position - previous,
            0.,
        );
        if result.overscroll != 0. {
            self.dispatch_notification(
                ScrollNotificationType::Overscroll,
                result.position - previous,
                result.overscroll,
            );
        }
        result
    }

    /// Stores a spring result produced with a monotonic frame duration.
    pub fn apply_spring_step(&self, step: ScrollSpringStep) -> bool {
        let previous = self.offset();
        if !step.position.is_finite() || step.position == previous {
            return false;
        }
        let mut state = self.state.borrow_mut();
        state.offset = step.position;
        state.revision += 1;
        drop(state);
        self.dispatch_notification(ScrollNotificationType::Update, step.position - previous, 0.);
        true
    }
    /// Updates content and viewport extents while no attachment owns the
    /// controller, failing without mutating anything otherwise. This is
    /// the unattached-publication entry: free controllers (headless
    /// models, tests, hosts driving their own position) publish here,
    /// while attached viewports publish through their
    /// [`MetricAttachment`](crate::MetricAttachment). A rejection
    /// preserves extents, offset, revision, ownership, and emits no
    /// notifications — authority is validated before any mutation,
    /// clamping, or dispatch.
    pub fn update_extents(&self, content: f32, viewport: f32) -> Result<(), MetricWriteError> {
        self.update_extents_with_physics(content, viewport, ScrollPhysics::default())
    }

    /// Updates content and viewport extents while applying the configured
    /// range-maintaining policy, subject to the same attachment rule as
    /// [`update_extents`](Self::update_extents): writes only while no
    /// attachment owns the controller.
    ///
    /// A plain extent update clamps a position when the new range becomes
    /// smaller.  [`ScrollPhysics::range_maintaining`] additionally keeps a
    /// position that was anchored to the old trailing edge anchored to the
    /// new trailing edge.  This is the important case for a list whose
    /// content grows, shrinks, or whose viewport is resized while the user is
    /// at the end.  The ordinary `update_extents` API remains available for
    /// callers that want simple clamping.
    pub fn update_extents_with_physics(
        &self,
        content: f32,
        viewport: f32,
        physics: ScrollPhysics,
    ) -> Result<(), MetricWriteError> {
        let publication = {
            let mut state = self.state.borrow_mut();
            if let Some(live) = state.metric_attachment {
                return Err(MetricWriteError::attached(live.tree));
            }
            Self::commit_extent_state(
                &mut state,
                ViewportMetricsUpdate::new(content, viewport, physics),
            )
        };
        self.finish_extent_publication(publication);
        Ok(())
    }

    /// The single extent-update algorithm behind every publication path:
    /// attached, unattached-checked, and paired. Runs under the caller's
    /// state lock with authority already validated, so all paths share
    /// identical clamping, pending-request, revision, context, and
    /// restoration behavior.
    pub(crate) fn commit_extent_state(
        state: &mut ScrollState,
        update: ViewportMetricsUpdate,
    ) -> ExtentPublication {
        let old_offset = state.offset;
        let old_content = state.content_extent;
        let old_viewport = state.viewport_extent;
        let old_max_offset = state.max_offset;
        state.content_extent = update.content.max(0.);
        state.viewport_extent = update.viewport.max(0.);
        state.max_offset = (state.content_extent - state.viewport_extent).max(0.);
        if let Some((axis, reverse)) = update.context {
            state.notification_context = Some((axis, reverse));
        }
        let persistence = if let Some(requested) = state.pending_jump_offset {
            let next = requested.min(state.max_offset);
            if next != state.offset {
                state.offset = next;
                state.revision += 1;
            }
            if requested <= state.max_offset {
                state.pending_jump_offset = None;
            }
            None
        } else if let Some(restored) = state.pending_restored_offset {
            let next = restored.min(state.max_offset);
            if next != state.offset {
                state.offset = next;
                state.revision += 1;
            }
            // Keep a larger requested position pending while asynchronous
            // content grows, instead of overwriting the only snapshot
            // with a temporary short-content clamp.
            if restored <= state.max_offset {
                state.pending_restored_offset = None;
                if let Some(restoration) = state.restoration.clone() {
                    restoration.scope.note_restoration_outcome(1, 0);
                }
            }
            None
        } else {
            let next = if update.physics.is_range_maintaining()
                && (state.max_offset - old_max_offset).abs() > f32::EPSILON
                && old_max_offset > 0.
                && (old_offset - old_max_offset).abs() <= 0.001
            {
                // Preserve the logical trailing-edge anchor when the
                // range changes.  This handles both content mutation and
                // viewport resize without rebuilding the scroll view.
                state.max_offset
            } else if old_content == state.content_extent
                && old_viewport == state.viewport_extent
                && matches!(update.physics.boundary, BoundaryPhysics::Bouncing { .. })
            {
                // Retained layout republishes unchanged metrics during a
                // drag. Preserve its visual overscroll until settlement.
                old_offset
            } else {
                old_offset.clamp(0., state.max_offset)
            };
            if next != state.offset {
                state.offset = next;
                state.revision += 1;
                // Bouncing offsets are transient presentation state.
                (0. ..=state.max_offset)
                    .contains(&next)
                    .then(|| (state.restoration.clone(), next))
            } else {
                None
            }
        };
        ExtentPublication {
            persistence,
            content: update.content,
            viewport: update.viewport,
            old_content,
            old_viewport,
            old_offset,
        }
    }

    /// Post-commit side effects shared by every publication path:
    /// restoration persistence plus Metrics/Update notifications. Runs
    /// after the state lock drops, so listeners never observe a
    /// half-committed record.
    pub(crate) fn finish_extent_publication(&self, publication: ExtentPublication) {
        if let Some((restoration, offset)) = publication.persistence {
            persist_scroll_offset(restoration, offset);
        }
        let next_offset = self.offset();
        if (publication.content.max(0.) - publication.old_content).abs() > f32::EPSILON
            || (publication.viewport.max(0.) - publication.old_viewport).abs() > f32::EPSILON
        {
            self.dispatch_notification(ScrollNotificationType::Metrics, 0., 0.);
        }
        if (next_offset - publication.old_offset).abs() > f32::EPSILON {
            self.dispatch_notification(
                ScrollNotificationType::Update,
                next_offset - publication.old_offset,
                0.,
            );
        }
    }

    /// Settles this position according to a composed physics policy.
    ///
    /// Bouncing positions return to the nearest range boundary, while page or
    /// fixed-extent policies choose their deterministic snap target.  The
    /// operation is intentionally synchronous; the runtime can animate toward
    /// the returned target using its normal frame scheduler when desired.
    pub fn settle_physics(&self, physics: ScrollPhysics, velocity: f32) -> bool {
        let target = physics.snap_target_for_extent(
            self.offset(),
            velocity,
            0.,
            self.max_offset(),
            self.viewport_extent(),
        );
        self.jump_to(target)
    }

    /// Preserves the visible logical anchor after content is inserted or
    /// removed before the current viewport. `delta_before_viewport` is the
    /// change in content extent before the anchor (positive for insertion,
    /// negative for removal). This is intentionally an explicit mutation
    /// operation so virtualized models can call it without forcing a rebuild.
    pub fn adjust_for_content_change(
        &self,
        delta_before_viewport: f32,
        physics: ScrollPhysics,
    ) -> bool {
        if !physics.is_range_maintaining() || !delta_before_viewport.is_finite() {
            return false;
        }
        let current = self.offset();
        self.jump_to(current + delta_before_viewport)
    }
}
