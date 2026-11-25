use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    ballistic::{FlingSimulation, NestedMap},
    controller::{ScrollController, ScrollState},
    physics::{ScrollDelta, ScrollPhysics},
};

/// General nested scrolling coordinator. Controllers are ordered innermost to
/// outermost. A clamped child consumes only what it can, and the precise
/// leftover propagates once to its parent.
#[derive(Clone, Debug, Default)]
pub struct NestedScrollCoordinator {
    controllers: Vec<ScrollController>,
    active: Option<usize>,
}

impl NestedScrollCoordinator {
    #[must_use]
    pub fn new(innermost_first: impl IntoIterator<Item = ScrollController>) -> Self {
        Self {
            controllers: innermost_first.into_iter().collect(),
            active: None,
        }
    }

    #[must_use]
    pub fn active_index(&self) -> Option<usize> {
        self.active
    }

    pub fn cancel(&mut self) {
        self.active = None;
    }

    /// Transfers a wheel delta without duplication.
    pub fn apply_delta(&mut self, delta: f32) -> ScrollDelta {
        let mut remaining = delta;
        let mut total = 0.;
        self.active = None;
        for (index, controller) in self.controllers.iter().enumerate() {
            let result = ScrollPhysics::clamping().apply_delta(
                controller.offset(),
                remaining,
                0.,
                controller.max_offset(),
            );
            if result.consumed != 0. {
                controller.jump_to(result.position);
                total += result.consumed;
                self.active = Some(index);
            }
            remaining = result.unconsumed;
            if remaining == 0. {
                break;
            }
        }
        ScrollDelta {
            position: total,
            consumed: total,
            unconsumed: remaining,
            accepted: total != 0.,
            ..ScrollDelta::default()
        }
    }

    /// Touch-drag entry point. The sign convention is the same logical
    /// content-offset convention as wheel input.
    pub fn apply_drag_delta(&mut self, delta: f32) -> ScrollDelta {
        self.apply_delta(delta)
    }

    /// Momentum entry point. The coordinator keeps the receiving viewport as
    /// `active_index` until the caller settles or cancels that sequence.
    pub fn apply_momentum_delta(&mut self, delta: f32) -> ScrollDelta {
        self.apply_delta(delta)
    }
}

/// Takes over the user-driven motion of a [`ScrollController`]: the drag
/// updates and the ballistic run after a release. This is the seam Flutter
/// fills with custom `ScrollPosition`s (`_NestedScrollPosition`,
/// `_DraggableScrollableSheetScrollPosition`). Installed with
/// [`ScrollController::set_drive_delegate`]; the viewport that owns the
/// gesture calls [`ScrollController::apply_user_drag`] and
/// [`ScrollController::delegated_ballistic`], which reach the delegate.
pub trait ScrollDriveDelegate {
    /// Applies one drag update of `delta` logical pixels (positive moves the
    /// offset forward) for `controller`, whose viewport resolved `physics`.
    /// The result's `unconsumed` part continues to enclosing viewports.
    fn apply_user_drag(
        &self,
        controller: &ScrollController,
        physics: ScrollPhysics,
        delta: f32,
    ) -> ScrollDelta;

    /// The ballistic runs that replace `controller`'s own release ballistic
    /// at `velocity` (logical pixels per second), each a controller and the
    /// simulation it follows in its own offset space, starting at its
    /// current offset. `None` keeps the ordinary ballistic.
    fn ballistic(
        &self,
        controller: &ScrollController,
        physics: ScrollPhysics,
        velocity: f32,
    ) -> Option<Vec<(ScrollController, FlingSimulation)>> {
        let _ = (controller, physics, velocity);
        None
    }

    /// The `NestedScrollView` coordinator behind this delegate, if any.
    #[doc(hidden)]
    fn nested_coordinator(&self) -> Option<NestedScrollViewCoordinator> {
        None
    }
}

/// A weak handle to a controller, so coordinators stored in a controller's
/// own state never keep it alive.
#[derive(Clone)]
struct WeakController {
    state: Weak<RefCell<ScrollState>>,
    changes: incular_core::reactivity::DependencySource,
}

impl WeakController {
    fn new(controller: &ScrollController) -> Self {
        Self {
            state: Rc::downgrade(&controller.state),
            changes: controller.changes.clone(),
        }
    }

    fn upgrade(&self) -> Option<ScrollController> {
        Some(ScrollController::from_shared_position(
            self.state.upgrade()?,
            self.changes.clone(),
        ))
    }

    fn is(&self, controller: &ScrollController) -> bool {
        std::ptr::eq(self.state.as_ptr(), Rc::as_ptr(&controller.state))
    }
}

#[derive(Default)]
struct NestedState {
    outer: Option<WeakController>,
    inner: Option<WeakController>,
    float_header_slivers: bool,
}

/// Flutter `_NestedScrollCoordinator`: one scroll position made of an outer
/// position (the header slivers) and an inner one (the body's scroll view).
///
/// A drag forward first collapses the header (outer) and then scrolls the
/// body (inner); a drag back first returns the body to its start and then
/// expands the header, unless the header floats. A release flings both
/// positions along one simulation over their combined extents, so a fling
/// crosses from one position to the other.
#[derive(Clone, Default)]
pub struct NestedScrollViewCoordinator {
    state: Rc<RefCell<NestedState>>,
}

impl std::fmt::Debug for NestedScrollViewCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NestedScrollViewCoordinator")
            .field("outer", &self.outer())
            .field("inner", &self.inner())
            .finish()
    }
}

impl PartialEq for NestedScrollViewCoordinator {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl NestedScrollViewCoordinator {
    #[must_use]
    pub fn new(float_header_slivers: bool) -> Self {
        Self {
            state: Rc::new(RefCell::new(NestedState {
                float_header_slivers,
                ..NestedState::default()
            })),
        }
    }

    /// Flutter `NestedScrollView.floatHeaderSlivers`.
    #[must_use]
    pub fn float_header_slivers(&self) -> bool {
        self.state.borrow().float_header_slivers
    }

    pub fn set_float_header_slivers(&self, float: bool) {
        self.state.borrow_mut().float_header_slivers = float;
    }

    /// The outer (header) position.
    #[must_use]
    pub fn outer(&self) -> Option<ScrollController> {
        self.state.borrow().outer.as_ref()?.upgrade()
    }

    /// The inner (body) position, once a drag over the body bound it.
    #[must_use]
    pub fn inner(&self) -> Option<ScrollController> {
        self.state.borrow().inner.as_ref()?.upgrade()
    }

    /// Makes `outer` this view's outer position, releasing a previous one.
    pub fn attach_outer(&self, outer: &ScrollController) {
        let previous = self.outer();
        if previous.as_ref() == Some(outer) {
            return;
        }
        if let Some(previous) = previous {
            self.release(&previous);
            previous.state.borrow_mut().nested_outer = None;
        }
        if let Some(other) = outer.nested_outer_coordinator()
            && other != *self
        {
            other.release(outer);
        }
        self.state.borrow_mut().outer = Some(WeakController::new(outer));
        let mut state = outer.state.borrow_mut();
        state.nested_outer = Some(self.clone());
        state.drive_delegate = Some(Rc::new(self.clone()));
    }

    /// Makes `inner` this view's inner position (Flutter's inner
    /// `PrimaryScrollController`). A controller another delegate drives
    /// (for example a draggable sheet's list) keeps that delegate.
    pub fn bind_inner(&self, inner: &ScrollController) {
        if self
            .state
            .borrow()
            .inner
            .as_ref()
            .is_some_and(|current| current.is(inner))
        {
            return;
        }
        if inner.state.borrow().drive_delegate.is_some() {
            return;
        }
        if let Some(previous) = self.inner() {
            self.release(&previous);
        }
        self.state.borrow_mut().inner = Some(WeakController::new(inner));
        inner.state.borrow_mut().drive_delegate = Some(Rc::new(self.clone()));
    }

    /// Detaches both positions.
    pub fn detach(&self) {
        if let Some(inner) = self.inner() {
            self.release(&inner);
        }
        if let Some(outer) = self.outer() {
            self.release(&outer);
            outer.state.borrow_mut().nested_outer = None;
        }
        let mut state = self.state.borrow_mut();
        state.inner = None;
        state.outer = None;
    }

    /// Removes this coordinator's delegate from `controller`, leaving any
    /// other delegate in place.
    fn release(&self, controller: &ScrollController) {
        let mut state = controller.state.borrow_mut();
        let ours = state
            .drive_delegate
            .as_ref()
            .and_then(|delegate| delegate.nested_coordinator())
            .is_some_and(|coordinator| coordinator == *self);
        if ours {
            state.drive_delegate = None;
        }
    }

    fn positions(&self) -> Option<(ScrollController, Option<ScrollController>)> {
        Some((self.outer()?, self.inner()))
    }
}

/// Flutter `_NestedScrollPosition.applyClampedDragUpdate`, in logical offset
/// units: moves by `delta` without entering an overscroll and returns the
/// part that was not used.
fn apply_clamped_drag(controller: &ScrollController, delta: f32) -> f32 {
    let (pixels, max_extent) = {
        let state = controller.state.borrow();
        (state.offset, state.max_offset)
    };
    let min = if delta > 0. {
        f32::NEG_INFINITY
    } else {
        0f32.min(pixels)
    };
    let max = if delta < 0. {
        f32::INFINITY
    } else if pixels < 0. {
        0.
    } else {
        max_extent.max(pixels)
    };
    let target = pixels + delta;
    let value = target.clamp(min, max);
    if value == pixels {
        return delta;
    }
    controller.commit_user_offset(value);
    if value == target { 0. } else { target - value }
}

fn physics_of(position: &ScrollController, fallback: ScrollPhysics) -> ScrollPhysics {
    position.state.borrow().last_physics.unwrap_or(fallback)
}

impl ScrollDriveDelegate for NestedScrollViewCoordinator {
    /// Flutter `_NestedScrollCoordinator.applyUserOffset`.
    fn apply_user_drag(
        &self,
        controller: &ScrollController,
        physics: ScrollPhysics,
        delta: f32,
    ) -> ScrollDelta {
        let Some((outer, inner)) = self.positions() else {
            return controller.apply_physics(physics, delta);
        };
        if delta == 0. || !delta.is_finite() {
            return ScrollDelta {
                position: controller.offset(),
                ..ScrollDelta::default()
            };
        }
        match inner {
            None => {
                outer.apply_physics(physics_of(&outer, physics), delta);
            }
            Some(inner) if delta > 0. => {
                // Dragging up: any inner overscroll first, then the header,
                // then the body.
                let mut outer_delta = delta;
                if inner.offset() < 0. {
                    outer_delta = outer_delta.min(apply_clamped_drag(&inner, delta));
                }
                if outer_delta.abs() > 1e-10 {
                    let inner_delta = apply_clamped_drag(&outer, outer_delta);
                    if inner_delta != 0. {
                        inner.apply_physics(physics_of(&inner, physics), inner_delta);
                    }
                }
            }
            Some(inner) => {
                // Dragging down: the body back to its start first (after a
                // floating header), then the header.
                let mut inner_delta = delta;
                if self.float_header_slivers() {
                    inner_delta = apply_clamped_drag(&outer, delta);
                }
                if inner_delta != 0. {
                    let overscroll = apply_clamped_drag(&inner, inner_delta);
                    let mut outer_delta = overscroll.min(0.);
                    if outer_delta != 0. {
                        outer_delta -= apply_clamped_drag(&outer, outer_delta);
                    }
                    let remaining = overscroll - outer_delta;
                    if remaining < 0. {
                        inner.apply_physics(physics_of(&inner, physics), remaining);
                    }
                }
            }
        }
        // The coordinator owns the whole delta: nothing continues outward.
        ScrollDelta {
            position: controller.offset(),
            consumed: delta,
            unconsumed: 0.,
            overscroll: 0.,
            accepted: true,
        }
    }

    /// Flutter `_NestedScrollCoordinator.goBallistic`: both positions follow
    /// one simulation over the combined extents.
    fn ballistic(
        &self,
        _controller: &ScrollController,
        physics: ScrollPhysics,
        velocity: f32,
    ) -> Option<Vec<(ScrollController, FlingSimulation)>> {
        let (outer, inner) = self.positions()?;
        let inner = inner?;
        if velocity == 0. || !velocity.is_finite() {
            return None;
        }
        let (outer_pixels, outer_min, outer_max) = (outer.offset(), 0., outer.max_offset());
        let (inner_pixels, inner_min, inner_max) = (inner.offset(), 0., inner.max_offset());
        // `_getMetrics`.
        let (pixels, min_range, max_range, correction, extra);
        if inner_pixels == inner_min {
            pixels = outer_pixels.clamp(outer_min, outer_max);
            min_range = outer_min;
            max_range = outer_max;
            correction = 0.;
            extra = 0.;
        } else {
            pixels = if inner_pixels < inner_min {
                inner_pixels - inner_min + outer_min
            } else {
                inner_pixels - inner_min + outer_max
            };
            if velocity > 0. && inner_pixels > inner_min {
                extra = outer_max - outer_pixels;
                min_range = pixels;
                max_range = pixels + extra;
                correction = outer_pixels - pixels;
            } else if velocity < 0. && inner_pixels < inner_min {
                extra = outer_pixels - outer_min;
                min_range = pixels - extra;
                max_range = pixels;
                correction = outer_pixels - pixels;
            } else {
                extra = if velocity > 0. {
                    outer_min - outer_pixels
                } else {
                    outer_pixels - (outer_max - outer_min)
                };
                min_range = outer_min;
                max_range = outer_max + extra;
                correction = 0.;
            }
        }
        let combined_max = outer_max + inner_max - inner_min + extra;
        let direction = velocity.signum();
        let mut runs = Vec::with_capacity(2);
        // The physics a press resolves carries the device pixel ratio the
        // tolerances scale with; the positions' own physics choose the
        // boundary behavior.
        let resolve = |position: &ScrollController| {
            let mut resolved = physics;
            resolved.boundary = physics_of(position, physics).boundary;
            resolved
        };
        if min_range != max_range
            && let Some(simulation) =
                resolve(&outer).ballistic_simulation(pixels, velocity, outer_min, combined_max)
        {
            runs.push((
                outer.clone(),
                simulation.mapped(NestedMap::Outer {
                    min_range,
                    max_range,
                    correction,
                    hold: outer_pixels,
                    direction,
                }),
            ));
        }
        if let Some(simulation) =
            resolve(&inner).ballistic_simulation(pixels, velocity, outer_min, combined_max)
        {
            runs.push((
                inner.clone(),
                simulation.mapped(NestedMap::Inner {
                    outer_min,
                    outer_max,
                    inner_min,
                    inner_max,
                    direction,
                }),
            ));
        }
        Some(runs)
    }

    fn nested_coordinator(&self) -> Option<NestedScrollViewCoordinator> {
        Some(self.clone())
    }
}

/// The ordinary ballistic of a position at `position` moving at `velocity`,
/// kept inside `[min, max]` when the physics clamp (the runs a delegate hands
/// over start from inside the extents and must not leave them).
pub(crate) fn clamped_ballistic(
    physics: ScrollPhysics,
    position: f32,
    velocity: f32,
    min: f32,
    max: f32,
) -> Option<FlingSimulation> {
    let simulation = physics.ballistic_simulation(position, velocity, min, max)?;
    if simulation.may_overscroll() {
        return Some(simulation);
    }
    Some(simulation.mapped(NestedMap::Inner {
        outer_min: min,
        outer_max: min,
        inner_min: min,
        inner_max: max,
        direction: velocity.signum(),
    }))
}

impl ScrollController {
    /// Installs (or with `None` removes) the delegate that takes over this
    /// controller's user drags and release ballistics.
    pub fn set_drive_delegate(&self, delegate: Option<Rc<dyn ScrollDriveDelegate>>) {
        self.state.borrow_mut().drive_delegate = delegate;
    }

    /// Whether a [`ScrollDriveDelegate`] drives this controller.
    #[must_use]
    pub fn has_drive_delegate(&self) -> bool {
        self.state.borrow().drive_delegate.is_some()
    }

    /// The `NestedScrollView` this controller is the outer position of.
    #[must_use]
    pub fn nested_outer_coordinator(&self) -> Option<NestedScrollViewCoordinator> {
        self.state.borrow().nested_outer.clone()
    }

    /// Applies one user drag update: through the drive delegate when one is
    /// installed, otherwise [`apply_physics`](Self::apply_physics).
    pub fn apply_user_drag(&self, physics: ScrollPhysics, delta: f32) -> ScrollDelta {
        if !self.update_activity_drag_direction(delta) {
            return ScrollDelta {
                position: self.offset(),
                unconsumed: delta,
                ..ScrollDelta::default()
            };
        }
        let delegate = self.state.borrow().drive_delegate.clone();
        match delegate {
            Some(delegate) => delegate.apply_user_drag(self, physics, delta),
            None => self.apply_physics(physics, delta),
        }
    }

    /// The ballistic runs a drive delegate starts instead of this
    /// controller's own release ballistic. `None` without a delegate or when
    /// the delegate keeps the ordinary ballistic.
    #[must_use]
    pub fn delegated_ballistic(
        &self,
        physics: ScrollPhysics,
        velocity: f32,
    ) -> Option<Vec<(ScrollController, FlingSimulation)>> {
        let delegate = self.state.borrow().drive_delegate.clone()?;
        delegate.ballistic(self, physics, velocity)
    }

    /// Asks the owning viewport to start this position's ordinary ballistic
    /// at `velocity` on its next frame (Flutter `goBallistic`), as a
    /// delegate does when it hands a fling back to the list.
    pub fn request_ballistic(&self, velocity: f32, physics: ScrollPhysics) {
        if !velocity.is_finite() {
            return;
        }
        self.state.borrow_mut().pending_ballistic = Some((velocity, physics));
        self.changes.notify();
    }

    /// Whether a [`request_ballistic`](Self::request_ballistic) waits for the
    /// owning viewport.
    #[must_use]
    pub fn has_pending_ballistic(&self) -> bool {
        self.state.borrow().pending_ballistic.is_some()
    }

    /// Binds the scroll viewports under one press, innermost first: the
    /// innermost joins a `NestedScrollView` it is the body of, as Flutter's
    /// body scroll view does through the inner `PrimaryScrollController`.
    /// Returns the chain index of the outer position now coordinated with
    /// the innermost one: that viewport no longer moves on its own.
    pub fn bind_drag_chain<'a>(
        chain: impl IntoIterator<Item = &'a ScrollController>,
    ) -> Option<usize> {
        let mut chain = chain.into_iter();
        let first = chain.next()?;
        if first.nested_outer_coordinator().is_some() {
            return None;
        }
        let axis_of = |controller: &ScrollController| {
            controller
                .state
                .borrow()
                .notification_context
                .map(|(axis, _)| axis)
        };
        for (index, controller) in chain.enumerate() {
            if let Some(coordinator) = controller.nested_outer_coordinator() {
                if axis_of(first) != axis_of(controller) {
                    return None;
                }
                coordinator.bind_inner(first);
                return (coordinator.inner().as_ref() == Some(first)).then_some(index + 1);
            }
        }
        None
    }

    /// Moves to `value` for a user drag without physics: commits, persists
    /// in range, and reports an `Update`.
    pub(crate) fn commit_user_offset(&self, value: f32) {
        let (delta, persistence) = {
            let mut state = self.state.borrow_mut();
            let previous = state.offset;
            state.offset = value;
            state.pending_restored_offset = None;
            state.revision += 1;
            let in_range = (state.min_offset..=state.max_offset).contains(&value);
            (
                value - previous,
                in_range.then(|| state.restoration.clone()).flatten(),
            )
        };
        crate::restoration::persist_scroll_offset(persistence, value);
        self.dispatch_notification(
            crate::notifications::ScrollNotificationType::Update,
            delta,
            0.,
        );
    }
}
