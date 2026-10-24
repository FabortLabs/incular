//! Flutter `Dismissible` (`widgets/dismissible.dart`): a child that can be
//! dragged away, revealing a background, then collapses before reporting the
//! dismissal.

use std::{
    cell::Cell,
    rc::{Rc, Weak},
    time::Duration,
};

use incular_animation::{AnimationController, AnimationStatus, Curve};
use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::{DragStartBehavior, Offset};

use crate::gestures::{DragEndDetails, DragStartDetails, DragUpdateDetails};
use crate::{
    Align, ClipRect, FractionalTranslation, GestureCallbacks, GestureDetector, HitTestBehavior,
    Positioned, Stack, Widget,
};

/// `_kMinFlingVelocity`.
const MIN_FLING_VELOCITY: f32 = 700.0;
/// `_kMinFlingVelocityDelta`.
const MIN_FLING_VELOCITY_DELTA: f32 = 400.0;
/// `_kDismissThreshold`.
const DISMISS_THRESHOLD: f32 = 0.4;
/// `Dismissible.movementDuration` default.
const MOVEMENT_DURATION: Duration = Duration::from_millis(200);
/// `Dismissible.resizeDuration` default.
const RESIZE_DURATION: Duration = Duration::from_millis(300);

/// `_kResizeTimeCurve`: `Interval(0.4, 1.0, curve: Curves.ease)`.
fn resize_time_curve(t: f32) -> f32 {
    Curve::Ease.apply(((t - 0.4) / 0.6).clamp(0.0, 1.0))
}

/// The direction in which a [`Dismissible`] can be dismissed (Flutter
/// `DismissDirection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DismissDirection {
    /// Dragging either up or down.
    Vertical,
    /// Dragging either left or right.
    Horizontal,
    /// Dragging against the reading direction.
    EndToStart,
    /// Dragging in the reading direction.
    StartToEnd,
    /// Dragging up only.
    Up,
    /// Dragging down only.
    Down,
    /// Not dismissible.
    None,
}

impl DismissDirection {
    fn is_x_axis(self) -> bool {
        matches!(self, Self::Horizontal | Self::EndToStart | Self::StartToEnd)
    }
}

/// Flutter `DismissUpdateDetails`, reported while the child moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DismissUpdateDetails {
    /// The direction the child is being dragged towards.
    pub direction: DismissDirection,
    /// Whether the progress is past the direction's dismiss threshold.
    pub reached: bool,
    /// [`Self::reached`] at the previous update.
    pub previous_reached: bool,
    /// The fraction of the child's extent it has moved, `0.0..=1.0`.
    pub progress: f32,
}

/// A child that can be dismissed by dragging it in the given direction.
///
/// When the drag ends past the direction's threshold (a fraction of the
/// child's extent, `0.4` by default) or flings, the child slides out over
/// `movement_duration`, then collapses over `resize_duration` and
/// `on_dismissed` reports the direction. The owner then removes the widget.
///
/// The extent is the incoming maximum constraint along the drag axis (the
/// child's width in a vertical list). A fling slides the child out over the
/// remaining `movement_duration` rather than Flutter's spring simulation.
/// Animations start on the next frame, as Flutter's tickers do.
#[derive(Clone)]
pub struct Dismissible {
    child: Widget,
    background: Option<Widget>,
    secondary_background: Option<Widget>,
    confirm_dismiss: Option<Rc<dyn Fn(DismissDirection) -> bool>>,
    on_resize: Option<Rc<dyn Fn()>>,
    on_update: Option<Rc<dyn Fn(DismissUpdateDetails)>>,
    on_dismissed: Option<Rc<dyn Fn(DismissDirection)>>,
    direction: DismissDirection,
    resize_duration: Option<Duration>,
    dismiss_thresholds: Vec<(DismissDirection, f32)>,
    movement_duration: Duration,
    cross_axis_end_offset: f32,
    drag_start_behavior: DragStartBehavior,
    behavior: HitTestBehavior,
}

impl Dismissible {
    /// Creates a dismissible child reporting dismissals in `direction` to
    /// `on_dismissed`.
    #[must_use]
    pub fn new(
        direction: DismissDirection,
        child: impl Into<Widget>,
        on_dismissed: impl Fn(DismissDirection) + 'static,
    ) -> Self {
        Self {
            child: child.into(),
            background: None,
            secondary_background: None,
            confirm_dismiss: None,
            on_resize: None,
            on_update: None,
            on_dismissed: Some(Rc::new(on_dismissed)),
            direction,
            resize_duration: Some(RESIZE_DURATION),
            dismiss_thresholds: Vec::new(),
            movement_duration: MOVEMENT_DURATION,
            cross_axis_end_offset: 0.0,
            drag_start_behavior: DragStartBehavior::Start,
            behavior: HitTestBehavior::Opaque,
        }
    }

    /// Painted behind the child as it moves. With a secondary background,
    /// only for start-to-end and down drags.
    #[must_use]
    pub fn background(mut self, background: impl Into<Widget>) -> Self {
        self.background = Some(background.into());
        self
    }

    /// Painted behind the child for end-to-start and up drags.
    #[must_use]
    pub fn secondary_background(mut self, background: impl Into<Widget>) -> Self {
        self.secondary_background = Some(background.into());
        self
    }

    /// Decides whether a finished drag dismisses the child; returning false
    /// slides it back.
    #[must_use]
    pub fn confirm_dismiss(mut self, confirm: impl Fn(DismissDirection) -> bool + 'static) -> Self {
        self.confirm_dismiss = Some(Rc::new(confirm));
        self
    }

    /// Called on every frame of the collapse.
    #[must_use]
    pub fn on_resize(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_resize = Some(Rc::new(callback));
        self
    }

    /// Called whenever the child moves.
    #[must_use]
    pub fn on_update(mut self, callback: impl Fn(DismissUpdateDetails) + 'static) -> Self {
        self.on_update = Some(Rc::new(callback));
        self
    }

    /// The collapse duration; `None` reports the dismissal as soon as the
    /// child has slid out. Defaults to 300 ms.
    #[must_use]
    pub fn resize_duration(mut self, duration: Option<Duration>) -> Self {
        self.resize_duration = duration;
        self
    }

    /// The fraction of its extent the child must be dragged in a direction
    /// to dismiss it. Directions without an entry use `0.4`; a threshold of
    /// `1.0` or more never dismisses.
    #[must_use]
    pub fn dismiss_threshold(mut self, direction: DismissDirection, threshold: f32) -> Self {
        self.dismiss_thresholds
            .retain(|(entry, _)| *entry != direction);
        self.dismiss_thresholds.push((direction, threshold));
        self
    }

    /// How long the child takes to slide out or back. Defaults to 200 ms.
    #[must_use]
    pub fn movement_duration(mut self, duration: Duration) -> Self {
        self.movement_duration = duration;
        self
    }

    /// How far the child moves across the drag axis while dismissed, as a
    /// fraction of its extent. Defaults to `0.0`.
    #[must_use]
    pub fn cross_axis_end_offset(mut self, offset: f32) -> Self {
        self.cross_axis_end_offset = offset;
        self
    }

    /// Defaults to [`DragStartBehavior::Start`].
    #[must_use]
    pub fn drag_start_behavior(mut self, behavior: DragStartBehavior) -> Self {
        self.drag_start_behavior = behavior;
        self
    }

    /// Defaults to [`HitTestBehavior::Opaque`].
    #[must_use]
    pub fn behavior(mut self, behavior: HitTestBehavior) -> Self {
        self.behavior = behavior;
        self
    }
}

/// `_DismissibleState`.
struct DismissState {
    config: Dismissible,
    move_controller: AnimationController,
    resize_controller: Option<AnimationController>,
    drag_extent: Cell<f32>,
    drag_underway: Cell<bool>,
    threshold_reached: Cell<bool>,
    /// `_overallDragAxisExtent`, from the last layout.
    axis_extent: Cell<f32>,
    text_direction: Cell<TextDirection>,
    resizing: Cell<bool>,
    dismissed: Cell<bool>,
    revision: Rc<Cell<u64>>,
}

impl DismissState {
    fn rebuild(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
    }

    /// `_extentToDirection`.
    fn extent_to_direction(&self, extent: f32) -> DismissDirection {
        if extent == 0.0 {
            return DismissDirection::None;
        }
        if self.config.direction.is_x_axis() {
            return match (self.text_direction.get(), extent < 0.0) {
                (TextDirection::Rtl, true) | (TextDirection::Ltr, false) => {
                    DismissDirection::StartToEnd
                }
                (TextDirection::Rtl | TextDirection::Ltr, _) => DismissDirection::EndToStart,
            };
        }
        if extent > 0.0 {
            DismissDirection::Down
        } else {
            DismissDirection::Up
        }
    }

    fn dismiss_direction(&self) -> DismissDirection {
        self.extent_to_direction(self.drag_extent.get())
    }

    fn dismiss_threshold(&self) -> f32 {
        let direction = self.dismiss_direction();
        self.config
            .dismiss_thresholds
            .iter()
            .find(|(entry, _)| *entry == direction)
            .map_or(DISMISS_THRESHOLD, |(_, threshold)| *threshold)
    }

    fn progress_for(&self, extent: f32) -> f32 {
        let axis_extent = self.axis_extent.get();
        if axis_extent > 0.0 {
            (extent.abs() / axis_extent).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// `_handleDragStart`.
    fn drag_start(&self, _details: DragStartDetails) {
        if self.resizing.get() || self.dismissed.get() {
            return;
        }
        self.drag_underway.set(true);
        if self.move_controller.is_active() {
            let extent = self.move_controller.value()
                * self.axis_extent.get()
                * self.drag_extent.get().signum();
            self.drag_extent.set(extent);
            self.move_controller.stop();
        } else {
            self.drag_extent.set(0.0);
            self.move_controller.set_value(0.0);
        }
        self.rebuild();
    }

    /// `_handleDragUpdate`.
    fn drag_update(&self, details: DragUpdateDetails) {
        if !self.drag_underway.get() || self.move_controller.is_active() {
            return;
        }
        let delta = details.primary_delta.unwrap_or(0.0);
        let extent = self.drag_extent.get();
        let ltr = self.text_direction.get() == TextDirection::Ltr;
        let next = match self.config.direction {
            DismissDirection::Horizontal | DismissDirection::Vertical => extent + delta,
            DismissDirection::Up if extent + delta < 0.0 => extent + delta,
            DismissDirection::Down if extent + delta > 0.0 => extent + delta,
            DismissDirection::EndToStart
                if (ltr && extent + delta < 0.0) || (!ltr && extent + delta > 0.0) =>
            {
                extent + delta
            }
            DismissDirection::StartToEnd
                if (ltr && extent + delta > 0.0) || (!ltr && extent + delta < 0.0) =>
            {
                extent + delta
            }
            DismissDirection::None => 0.0,
            DismissDirection::Up
            | DismissDirection::Down
            | DismissDirection::EndToStart
            | DismissDirection::StartToEnd => extent,
        };
        self.drag_extent.set(next);
        self.move_controller.set_value(self.progress_for(next));
        self.rebuild();
    }

    /// `_describeFlingGesture`: `Some(true)` flings out, `Some(false)` back.
    fn describe_fling(&self, details: &DragEndDetails) -> Option<bool> {
        if self.drag_extent.get() == 0.0 {
            return None;
        }
        let velocity = details.velocity.pixels_per_second;
        let (along, across) = if self.config.direction.is_x_axis() {
            (velocity.x, velocity.y)
        } else {
            (velocity.y, velocity.x)
        };
        if along.abs() - across.abs() < MIN_FLING_VELOCITY_DELTA || along.abs() < MIN_FLING_VELOCITY
        {
            return None;
        }
        Some(self.extent_to_direction(along) == self.dismiss_direction())
    }

    /// `_handleDragEnd`.
    fn drag_end(&self, details: DragEndDetails) {
        if !self.drag_underway.get() || self.move_controller.is_active() {
            return;
        }
        self.drag_underway.set(false);
        if self.move_controller.is_completed() {
            self.move_completed();
            return;
        }
        let along = if self.config.direction.is_x_axis() {
            details.velocity.pixels_per_second.x
        } else {
            details.velocity.pixels_per_second.y
        };
        match self.describe_fling(&details) {
            Some(true) => {
                if self.dismiss_threshold() >= 1.0 {
                    self.move_controller.reverse_on_next_tick();
                } else {
                    self.drag_extent.set(along.signum());
                    self.move_controller.forward_on_next_tick();
                }
            }
            Some(false) => {
                self.drag_extent.set(along.signum());
                self.move_controller.reverse_on_next_tick();
            }
            None => {
                if self.move_controller.status() != AnimationStatus::Dismissed {
                    if self.move_controller.value() > self.dismiss_threshold() {
                        self.move_controller.forward_on_next_tick();
                    } else {
                        self.move_controller.reverse_on_next_tick();
                    }
                }
            }
        }
        self.rebuild();
    }

    /// `_handleMoveCompleted`.
    fn move_completed(&self) {
        if self.dismiss_threshold() >= 1.0 {
            self.move_controller.reverse_on_next_tick();
            return;
        }
        let direction = self.dismiss_direction();
        let confirmed = self
            .config
            .confirm_dismiss
            .as_ref()
            .is_none_or(|confirm| confirm(direction));
        if !confirmed {
            self.move_controller.reverse_on_next_tick();
            return;
        }
        // `_startResizeAnimation`.
        match &self.resize_controller {
            None => self.finish(direction),
            Some(resize) => {
                self.resizing.set(true);
                resize.forward_on_next_tick();
            }
        }
        self.rebuild();
    }

    fn finish(&self, direction: DismissDirection) {
        if !self.dismissed.replace(true)
            && let Some(callback) = &self.config.on_dismissed
        {
            callback(direction);
        }
    }

    /// `_handleDismissUpdateValueChanged` and `_handleDismissStatusChanged`.
    fn move_changed(&self, value: f32) {
        if let Some(callback) = &self.config.on_update {
            let previous = self.threshold_reached.get();
            let reached = value > self.dismiss_threshold();
            self.threshold_reached.set(reached);
            callback(DismissUpdateDetails {
                direction: self.dismiss_direction(),
                reached,
                previous_reached: previous,
                progress: value,
            });
        }
        if self.move_controller.status() == AnimationStatus::Completed
            && !self.drag_underway.get()
            && !self.resizing.get()
            && !self.dismissed.get()
        {
            self.move_completed();
        }
    }

    /// `_handleResizeProgressChanged`.
    fn resize_changed(&self) {
        let Some(resize) = &self.resize_controller else {
            return;
        };
        if resize.status() == AnimationStatus::Completed {
            self.finish(self.dismiss_direction());
        } else if let Some(callback) = &self.config.on_resize {
            callback();
        }
    }

    /// `_DismissibleState.build`.
    fn build(&self, constraints: Constraints) -> Widget {
        let x_axis = self.config.direction.is_x_axis();
        let (max, min) = if x_axis {
            (constraints.max_width(), constraints.min_width())
        } else {
            (constraints.max_height(), constraints.min_height())
        };
        self.axis_extent
            .set(if max.is_finite() { max } else { min });
        let progress = self.move_controller.value();
        let sign = self.drag_extent.get().signum() * f32::from(self.drag_extent.get() != 0.0);
        let cross = self.config.cross_axis_end_offset * progress;
        let translation = if x_axis {
            Offset::new(sign * progress, cross)
        } else {
            Offset::new(cross, sign * progress)
        };
        let content: Widget =
            FractionalTranslation::new(translation, self.config.child.clone()).into();
        if self.resizing.get()
            && let Some(resize) = &self.resize_controller
        {
            // The child has slid out; the collapse shrinks the space it
            // leaves across the drag axis.
            let factor = 1.0 - resize_time_curve(resize.value());
            let align = Align::new(Alignment::CENTER, content);
            let align = if x_axis {
                align.height_factor(factor)
            } else {
                align.width_factor(factor)
            };
            return ClipRect::new(align).into();
        }
        let direction = self.dismiss_direction();
        let background = match &self.config.secondary_background {
            Some(secondary)
                if matches!(
                    direction,
                    DismissDirection::EndToStart | DismissDirection::Up
                ) =>
            {
                Some(secondary.clone())
            }
            _ => self.config.background.clone(),
        };
        let Some(background) = background.filter(|_| progress > 0.0) else {
            return content;
        };
        // `_DismissibleClipper`: only the region the child uncovered.
        let extent = self.axis_extent.get();
        let revealed = progress * extent;
        let (start, end) = if sign < 0.0 {
            (extent - revealed, extent)
        } else {
            (0.0, revealed)
        };
        let full = if x_axis {
            Positioned::builder()
                .left(-start)
                .width(extent)
                .top(0.0)
                .bottom(0.0)
                .child(background)
                .build()
        } else {
            Positioned::builder()
                .top(-start)
                .height(extent)
                .left(0.0)
                .right(0.0)
                .child(background)
                .build()
        };
        let clipped = ClipRect::new(Stack::new(vec![Widget::from(full)]));
        let region = if x_axis {
            Positioned::builder()
                .left(start)
                .width(end - start)
                .top(0.0)
                .bottom(0.0)
                .child(clipped)
                .build()
        } else {
            Positioned::builder()
                .top(start)
                .height(end - start)
                .left(0.0)
                .right(0.0)
                .child(clipped)
                .build()
        };
        Stack::new(vec![Widget::from(region), content]).into()
    }
}

impl From<Dismissible> for Widget {
    fn from(value: Dismissible) -> Self {
        let revision = Rc::new(Cell::new(0_u64));
        let move_controller = AnimationController::new(value.movement_duration);
        let resize_controller = value.resize_duration.map(AnimationController::new);
        let behavior = value.behavior;
        let drag_start_behavior = value.drag_start_behavior;
        let x_axis = value.direction.is_x_axis();
        let state = Rc::new(DismissState {
            config: value,
            move_controller: move_controller.clone(),
            resize_controller: resize_controller.clone(),
            drag_extent: Cell::new(0.0),
            drag_underway: Cell::new(false),
            threshold_reached: Cell::new(false),
            axis_extent: Cell::new(0.0),
            text_direction: Cell::new(TextDirection::Ltr),
            resizing: Cell::new(false),
            dismissed: Cell::new(false),
            revision: revision.clone(),
        });
        let weak: Weak<DismissState> = Rc::downgrade(&state);
        move_controller.add_listener({
            let weak = weak.clone();
            move |value| {
                if let Some(state) = weak.upgrade() {
                    state.move_changed(value);
                }
            }
        });
        if let Some(resize) = &resize_controller {
            resize.add_listener({
                let weak = weak.clone();
                move |_| {
                    if let Some(state) = weak.upgrade() {
                        state.resize_changed();
                    }
                }
            });
        }
        let content = Widget::stateful_layout_builder(revision.clone(), {
            let state = state.clone();
            move |context, constraints| {
                if let Some(direction) = context.text_direction() {
                    state.text_direction.set(direction);
                }
                state.build(constraints)
            }
        });
        let mut ticked = Widget::animation_ticker_with_revision(
            move_controller,
            false,
            revision.clone(),
            content,
        );
        if let Some(resize) = resize_controller {
            ticked = Widget::animation_ticker_with_revision(resize, false, revision, ticked);
        }
        let start = {
            let state = state.clone();
            Rc::new(move |details| state.drag_start(details))
        };
        let update = {
            let state = state.clone();
            Rc::new(move |details| state.drag_update(details))
        };
        let end = Rc::new(move |details| state.drag_end(details));
        let mut callbacks = GestureCallbacks {
            drag_start_behavior,
            ..GestureCallbacks::default()
        };
        if x_axis {
            callbacks.on_horizontal_drag_start = Some(start);
            callbacks.on_horizontal_drag_update = Some(update);
            callbacks.on_horizontal_drag_end = Some(end);
        } else {
            callbacks.on_vertical_drag_start = Some(start);
            callbacks.on_vertical_drag_update = Some(update);
            callbacks.on_vertical_drag_end = Some(end);
        }
        GestureDetector::new(ticked)
            .callbacks(callbacks)
            .behavior(behavior)
            .into()
    }
}
