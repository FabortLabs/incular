use std::time::{Duration, Instant};

use incular_core::{Offset, PointerPhase};

use crate::details::{
    DragCallbacks, DragDownDetails, DragEndDetails, DragStartDetails, DragUpdateDetails,
    GestureAction, GestureCallbacks, GestureDecision, LongPressEndDetails,
    LongPressMoveUpdateDetails, LongPressStartDetails, PointerDeviceKind, PointerEvent,
    TapDownDetails, TapUpDetails, Velocity,
};
use crate::raw::GestureRecognizer;
use crate::velocity::{VelocityEstimate, VelocityTracker};

/// Flutter kTouchSlop: hit tolerance for touch, stylus, trackpad and unknown kinds.
const TOUCH_HIT_SLOP: f32 = 18.;
/// Flutter kPrecisePointerHitSlop: hit tolerance for mouse pointers.
const PRECISE_POINTER_HIT_SLOP: f32 = 1.;
/// Flutter kPrecisePointerPanSlop: pan acceptance distance for mouse pointers.
const PRECISE_POINTER_PAN_SLOP: f32 = 2.;
/// Flutter kMinFlingVelocity: releases slower than this are not flings.
const MIN_FLING_VELOCITY: f32 = 50.;
/// Flutter kMaxFlingVelocity: fling velocities are clamped to this magnitude.
const MAX_FLING_VELOCITY: f32 = 8000.;

/// Flutter computeHitSlop: tap tolerance and single-axis drag acceptance distance.
fn compute_hit_slop(kind: PointerDeviceKind) -> f32 {
    match kind {
        PointerDeviceKind::Mouse => PRECISE_POINTER_HIT_SLOP,
        PointerDeviceKind::Touch
        | PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Trackpad
        | PointerDeviceKind::Unknown => TOUCH_HIT_SLOP,
    }
}

/// Flutter computePanSlop: pan acceptance distance.
fn compute_pan_slop(kind: PointerDeviceKind) -> f32 {
    match kind {
        PointerDeviceKind::Mouse => PRECISE_POINTER_PAN_SLOP,
        PointerDeviceKind::Touch
        | PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Trackpad
        | PointerDeviceKind::Unknown => PointerGestureRecognizer::PAN_SLOP,
    }
}

impl GestureCallbacks {
    /// Whether this callback set contributes an exclusive single-pointer
    /// recognizer to a gesture arena. Scale is intentionally separate so a
    /// pending pinch can coexist until one recognizer claims the stream.
    #[must_use]
    pub fn has_pointer_recognizer(&self) -> bool {
        self.on_tap.is_some()
            || self.on_tap_down.is_some()
            || self.on_tap_up.is_some()
            || self.on_tap_cancel.is_some()
            || self.on_double_tap.is_some()
            || self.on_double_tap_down.is_some()
            || self.on_double_tap_cancel.is_some()
            || self.on_long_press.is_some()
            || self.on_long_press_start.is_some()
            || self.on_long_press_move_update.is_some()
            || self.on_long_press_up.is_some()
            || self.on_long_press_end.is_some()
            || self.on_pan_down.is_some()
            || self.on_pan_start.is_some()
            || self.on_pan_update.is_some()
            || self.on_pan_end.is_some()
            || self.on_pan_cancel.is_some()
            || self.on_horizontal_drag_down.is_some()
            || self.on_horizontal_drag_start.is_some()
            || self.on_horizontal_drag_update.is_some()
            || self.on_horizontal_drag_end.is_some()
            || self.on_horizontal_drag_cancel.is_some()
            || self.on_vertical_drag_down.is_some()
            || self.on_vertical_drag_start.is_some()
            || self.on_vertical_drag_update.is_some()
            || self.on_vertical_drag_end.is_some()
            || self.on_vertical_drag_cancel.is_some()
    }
}

/// A reusable recognizer for tap, double-tap, long-press, and pan.
pub struct PointerGestureRecognizer {
    callbacks: GestureCallbacks,
    kind: PointerDeviceKind,
    down: Option<PointerEvent>,
    last_tap: Option<Instant>,
    pan_started: bool,
    pan_action: Option<PanAction>,
    long_press_started: bool,
    velocity: VelocityTracker,
    tap_moved_beyond_slop: bool,
    drag: DragProgress,
}

/// Flutter `DragGestureRecognizer` bookkeeping for the active pointer.
#[derive(Clone, Copy)]
struct DragProgress {
    /// `_pendingDragOffset`: movement accumulated before the drag is accepted.
    pending: Offset,
    /// `_lastPendingEventTimestamp`.
    pending_time: Option<Instant>,
    /// `_globalDistanceMoved` of the vertical drag recognizer (signed sum).
    moved_vertical: f32,
    /// `_globalDistanceMoved` of the horizontal drag recognizer (signed sum).
    moved_horizontal: f32,
    /// `_globalDistanceMoved` of the pan recognizer (path length).
    moved_pan: f32,
    /// `_lastPosition`: where the previous move event left the pointer.
    last_position: Offset,
}

impl DragProgress {
    fn at(position: Offset) -> Self {
        Self {
            pending: Offset::ZERO,
            pending_time: None,
            moved_vertical: 0.,
            moved_horizontal: 0.,
            moved_pan: 0.,
            last_position: position,
        }
    }
}

/// Legacy direct-use gesture detector.
///
/// The retained widget path uses [`PointerGestureRecognizer`] directly. This
/// small owning wrapper keeps the old convenience API concrete while
/// `GestureRecognizer` itself is now a real object-safe trait for raw factory
/// erasure.
pub struct GestureDetector {
    recognizer: PointerGestureRecognizer,
}

#[derive(Clone, Copy)]
enum PanAction {
    Pan,
    Horizontal,
    Vertical,
}
impl PointerGestureRecognizer {
    pub const DOUBLE_TAP_TIMEOUT: Duration = Duration::from_millis(300);
    pub const LONG_PRESS_TIMEOUT: Duration = Duration::from_millis(500);
    /// Flutter kPanSlop (touch-class pan acceptance distance).
    pub const PAN_SLOP: f32 = TOUCH_HIT_SLOP * 2.;
    /// Creates a recognizer whose device kind is unknown until a raw event
    /// reports it. Flutter treats unknown kinds with touch slops.
    #[must_use]
    pub fn new(callbacks: GestureCallbacks) -> Self {
        Self {
            callbacks,
            kind: PointerDeviceKind::Unknown,
            down: None,
            last_tap: None,
            pan_started: false,
            pan_action: None,
            long_press_started: false,
            velocity: VelocityTracker::default(),
            tap_moved_beyond_slop: false,
            drag: DragProgress::at(Offset::ZERO),
        }
    }
    fn has_horizontal_drag_callbacks(&self) -> bool {
        self.callbacks.on_horizontal_drag_down.is_some()
            || self.callbacks.on_horizontal_drag_start.is_some()
            || self.callbacks.on_horizontal_drag_update.is_some()
            || self.callbacks.on_horizontal_drag_end.is_some()
            || self.callbacks.on_horizontal_drag_cancel.is_some()
    }
    fn has_vertical_drag_callbacks(&self) -> bool {
        self.callbacks.on_vertical_drag_down.is_some()
            || self.callbacks.on_vertical_drag_start.is_some()
            || self.callbacks.on_vertical_drag_update.is_some()
            || self.callbacks.on_vertical_drag_end.is_some()
            || self.callbacks.on_vertical_drag_cancel.is_some()
    }
    fn has_pan_callbacks(&self) -> bool {
        self.callbacks.on_pan_down.is_some()
            || self.callbacks.on_pan_start.is_some()
            || self.callbacks.on_pan_update.is_some()
            || self.callbacks.on_pan_end.is_some()
            || self.callbacks.on_pan_cancel.is_some()
    }
    /// Flutter's `considerFling`: a release reports velocity only when it is a
    /// fling (speed above kMinFlingVelocity and displacement above
    /// computeHitSlop). Axis drags clamp their axis component and drop the
    /// other, pans clamp the magnitude. Anything else reports zero velocity with
    /// a primary velocity of 0 (Flutter 3.41 monodrag.dart `_checkEnd`).
    fn consider_fling(
        &self,
        action: PanAction,
        estimate: VelocityEstimate,
    ) -> (Velocity, Option<f32>) {
        let hit_slop = compute_hit_slop(self.kind);
        let velocity = estimate.pixels_per_second;
        let offset = estimate.offset;
        let not_a_fling = (Velocity::ZERO, Some(0.));
        match action {
            PanAction::Horizontal => {
                if velocity.x.abs() > MIN_FLING_VELOCITY && offset.x.abs() > hit_slop {
                    let dx = velocity.x.clamp(-MAX_FLING_VELOCITY, MAX_FLING_VELOCITY);
                    (Velocity::new(Offset::new(dx, 0.)), Some(dx))
                } else {
                    not_a_fling
                }
            }
            PanAction::Vertical => {
                if velocity.y.abs() > MIN_FLING_VELOCITY && offset.y.abs() > hit_slop {
                    let dy = velocity.y.clamp(-MAX_FLING_VELOCITY, MAX_FLING_VELOCITY);
                    (Velocity::new(Offset::new(0., dy)), Some(dy))
                } else {
                    not_a_fling
                }
            }
            PanAction::Pan => {
                let speed = velocity.x.hypot(velocity.y);
                let distance = offset.x.hypot(offset.y);
                if speed > MIN_FLING_VELOCITY && distance > hit_slop {
                    // Velocity.clampMagnitude(kMinFlingVelocity, kMaxFlingVelocity).
                    let magnitude = speed.clamp(MIN_FLING_VELOCITY, MAX_FLING_VELOCITY);
                    let scale = magnitude / speed;
                    (
                        Velocity::new(Offset::new(velocity.x * scale, velocity.y * scale)),
                        None,
                    )
                } else {
                    not_a_fling
                }
            }
        }
    }
    /// The drag recognizer that has moved far enough to accept the gesture.
    ///
    /// Flutter's `GestureDetector` registers the vertical, horizontal and pan
    /// recognizers in that order and the first to resolve wins the arena. Each
    /// measures its own `_globalDistanceMoved`: axis drags the signed sum along
    /// their axis (accepted beyond computeHitSlop) and pans the path length
    /// (accepted beyond computePanSlop), all with a strict comparison.
    fn drag_candidate(&self) -> Option<PanAction> {
        if self.has_vertical_drag_callbacks()
            && self.drag.moved_vertical.abs() > compute_hit_slop(self.kind)
        {
            Some(PanAction::Vertical)
        } else if self.has_horizontal_drag_callbacks()
            && self.drag.moved_horizontal.abs() > compute_hit_slop(self.kind)
        {
            Some(PanAction::Horizontal)
        } else if self.has_pan_callbacks()
            && self.drag.moved_pan.abs() > compute_pan_slop(self.kind)
        {
            Some(PanAction::Pan)
        } else {
            None
        }
    }
    /// An update for the accepted drag: the movement since the previous event,
    /// restricted to the drag axis for horizontal and vertical drags.
    fn drag_update(&self, action: PanAction, step: Offset, event: &PointerEvent) -> GestureAction {
        let (delta, primary_delta) = match action {
            PanAction::Pan => (step, None),
            PanAction::Horizontal => (Offset::new(step.x, 0.), Some(step.x)),
            PanAction::Vertical => (Offset::new(0., step.y), Some(step.y)),
        };
        let details = DragUpdateDetails {
            source_time: Some(event.time),
            delta,
            primary_delta,
            global_position: event.position,
            local_position: event.position,
            kind: self.kind,
        };
        match action {
            PanAction::Pan => GestureAction::Pan(details),
            PanAction::Horizontal => GestureAction::HorizontalDrag(details),
            PanAction::Vertical => GestureAction::VerticalDrag(details),
        }
    }
    /// The end of an accepted drag, shared by pointer-up and platform cancel
    /// (Flutter ends an accepted drag with `onEnd` in both cases).
    fn drag_end(&mut self, time: Instant) -> Option<GestureAction> {
        let action = self.pan_action.take()?;
        self.pan_started = false;
        let (velocity, primary_velocity) =
            self.consider_fling(action, self.velocity.estimate_at(time));
        let details = DragEndDetails {
            velocity,
            primary_velocity,
            global_position: self.drag.last_position,
            local_position: self.drag.last_position,
        };
        match action {
            PanAction::Pan => self
                .callbacks
                .on_pan_end
                .as_ref()
                .map(|_| GestureAction::PanEnd(details)),
            PanAction::Horizontal => self
                .callbacks
                .on_horizontal_drag_end
                .as_ref()
                .map(|_| GestureAction::HorizontalDragEnd(details)),
            PanAction::Vertical => self
                .callbacks
                .on_vertical_drag_end
                .as_ref()
                .map(|_| GestureAction::VerticalDragEnd(details)),
        }
    }
    /// Observes an event without invoking user callbacks. This lets a
    /// retained arena delay observable behavior until it has accepted this
    /// recognizer. Direct users should normally keep using [`Self::handle`].
    pub fn observe(&mut self, event: PointerEvent) -> GestureDecision {
        match event.phase {
            PointerPhase::Enter | PointerPhase::Exit => GestureDecision::Reject,
            PointerPhase::Down => {
                let is_double_tap = self.last_tap.is_some_and(|tap| {
                    event.time.saturating_duration_since(tap) <= Self::DOUBLE_TAP_TIMEOUT
                });
                if !is_double_tap {
                    self.last_tap = None;
                }
                self.down = Some(event);
                self.velocity = VelocityTracker::default();
                self.velocity.add_position(event.time, event.position);
                self.tap_moved_beyond_slop = false;
                self.pan_started = false;
                self.pan_action = None;
                self.drag = DragProgress::at(event.position);
                self.long_press_started = false;
                if let Some(callback) = &self.callbacks.on_tap_down {
                    callback(TapDownDetails {
                        global_position: event.position,
                        local_position: event.position,
                        kind: self.kind,
                    });
                }
                if is_double_tap && let Some(callback) = &self.callbacks.on_double_tap_down {
                    callback(TapDownDetails {
                        global_position: event.position,
                        local_position: event.position,
                        kind: self.kind,
                    });
                }
                let pan_down = DragDownDetails {
                    global_position: event.position,
                    local_position: event.position,
                };
                if let Some(callback) = &self.callbacks.on_pan_down {
                    callback(pan_down);
                }
                if let Some(callback) = &self.callbacks.on_horizontal_drag_down {
                    callback(pan_down);
                }
                if let Some(callback) = &self.callbacks.on_vertical_drag_down {
                    callback(pan_down);
                }
                GestureDecision::Pending
            }
            PointerPhase::Move => {
                let Some(down) = self.down else {
                    return GestureDecision::Reject;
                };
                let delta = event.position - down.position;
                self.velocity.add_position(event.time, event.position);
                if delta.x.hypot(delta.y) > compute_hit_slop(self.kind) {
                    self.tap_moved_beyond_slop = true;
                }
                if !self.pan_started
                    && !self.long_press_started
                    && event.time.saturating_duration_since(down.time) >= Self::LONG_PRESS_TIMEOUT
                {
                    self.long_press_started = true;
                    if let Some(callback) = &self.callbacks.on_long_press_start {
                        callback(LongPressStartDetails {
                            global_position: down.position,
                            local_position: down.position,
                        });
                    }
                }
                if self.long_press_started
                    && !self.pan_started
                    && let Some(callback) = &self.callbacks.on_long_press_move_update
                {
                    callback(LongPressMoveUpdateDetails {
                        global_position: event.position,
                        local_position: event.position,
                        offset_from_origin: delta,
                        local_offset_from_origin: delta,
                    });
                }
                let step = event.position - self.drag.last_position;
                self.drag.last_position = event.position;
                if self.pan_started
                    && let Some(action) = self.pan_action
                {
                    // Flutter reports every move after acceptance as the
                    // movement since the previous event.
                    return GestureDecision::Accept(self.drag_update(action, step, &event));
                }
                self.drag.pending = self.drag.pending + step;
                self.drag.pending_time = Some(event.time);
                self.drag.moved_vertical += step.y;
                self.drag.moved_horizontal += step.x;
                self.drag.moved_pan += step.x.hypot(step.y);
                if let Some(action) = self.drag_candidate() {
                    // DragStartBehavior.start: the drag begins where it was
                    // accepted and the movement that crossed the slop is not
                    // reported as an update.
                    let origin = down.position + self.drag.pending;
                    let start = DragStartDetails {
                        source_time: self.drag.pending_time,
                        global_position: origin,
                        local_position: origin,
                        kind: self.kind,
                    };
                    self.pan_action = Some(action);
                    self.pan_started = true;
                    self.drag.pending = Offset::ZERO;
                    self.drag.pending_time = None;
                    return GestureDecision::Accept(match action {
                        PanAction::Pan => GestureAction::PanStart(start),
                        PanAction::Horizontal => GestureAction::HorizontalDragStart(start),
                        PanAction::Vertical => GestureAction::VerticalDragStart(start),
                    });
                }
                let axis_callbacks = if delta.x.abs() >= delta.y.abs() {
                    self.has_horizontal_drag_callbacks()
                } else {
                    self.has_vertical_drag_callbacks()
                };
                if !axis_callbacks
                    && !self.has_pan_callbacks()
                    && delta.x.hypot(delta.y) > compute_pan_slop(self.kind)
                {
                    return GestureDecision::Reject;
                }
                GestureDecision::Pending
            }
            PointerPhase::Up => {
                let Some(down) = self.down.take() else {
                    return GestureDecision::Reject;
                };
                if self.pan_started {
                    return self
                        .drag_end(event.time)
                        .map_or(GestureDecision::Pending, GestureDecision::Accept);
                }
                let elapsed = event.time.saturating_duration_since(down.time);
                if elapsed >= Self::LONG_PRESS_TIMEOUT {
                    if !self.long_press_started {
                        self.long_press_started = true;
                        if let Some(callback) = &self.callbacks.on_long_press_start {
                            callback(LongPressStartDetails {
                                global_position: down.position,
                                local_position: down.position,
                            });
                        }
                    }
                    if let Some(callback) = &self.callbacks.on_long_press_up {
                        callback();
                    }
                    if let Some(callback) = &self.callbacks.on_long_press_end {
                        callback(LongPressEndDetails {
                            global_position: event.position,
                            local_position: event.position,
                            velocity: Velocity::new(
                                self.velocity.estimate_at(event.time).pixels_per_second,
                            ),
                        });
                    }
                    self.long_press_started = false;
                    if self.callbacks.on_long_press.is_some()
                        || self.callbacks.on_long_press_start.is_some()
                        || self.callbacks.on_long_press_move_update.is_some()
                        || self.callbacks.on_long_press_up.is_some()
                        || self.callbacks.on_long_press_end.is_some()
                    {
                        GestureDecision::Accept(GestureAction::LongPress)
                    } else {
                        GestureDecision::Reject
                    }
                } else if self.tap_moved_beyond_slop {
                    // Flutter cancels a tap once movement exceeds computeHitSlop,
                    // which also breaks any double-tap sequence.
                    self.last_tap = None;
                    GestureDecision::Reject
                } else if self.last_tap.is_some_and(|tap| {
                    event.time.saturating_duration_since(tap) <= Self::DOUBLE_TAP_TIMEOUT
                }) && self.callbacks.on_double_tap.is_some()
                {
                    if let Some(callback) = &self.callbacks.on_tap_up {
                        callback(TapUpDetails {
                            global_position: event.position,
                            local_position: event.position,
                            kind: self.kind,
                        });
                    }
                    self.last_tap = None;
                    GestureDecision::Accept(GestureAction::DoubleTap)
                } else if self.callbacks.on_tap.is_some()
                    || self.callbacks.on_tap_up.is_some()
                    || self.callbacks.on_tap_cancel.is_some()
                {
                    if let Some(callback) = &self.callbacks.on_tap_up {
                        callback(TapUpDetails {
                            global_position: event.position,
                            local_position: event.position,
                            kind: self.kind,
                        });
                    }
                    self.last_tap = Some(event.time);
                    GestureDecision::Accept(GestureAction::Tap)
                } else {
                    GestureDecision::Reject
                }
            }
            PointerPhase::Cancel => {
                if self.down.take().is_some() {
                    if self.pan_started {
                        // Flutter ends an accepted drag with onEnd when the
                        // pointer is cancelled; onCancel is for unaccepted ones.
                        self.long_press_started = false;
                        return self
                            .drag_end(event.time)
                            .map_or(GestureDecision::Cancelled, GestureDecision::Accept);
                    }
                    self.pan_started = false;
                    self.pan_action = None;
                    self.long_press_started = false;
                    GestureDecision::Cancelled
                } else {
                    GestureDecision::Reject
                }
            }
        }
    }
    /// Invokes a callback selected by [`Self::observe`] after the caller has
    /// accepted this recognizer.
    pub fn dispatch(&self, action: GestureAction) {
        match action {
            GestureAction::Tap => self.callbacks.on_tap.as_ref().map(|callback| callback()),
            GestureAction::DoubleTap => self
                .callbacks
                .on_double_tap
                .as_ref()
                .map(|callback| callback()),
            GestureAction::LongPress => self
                .callbacks
                .on_long_press
                .as_ref()
                .map(|callback| callback()),
            GestureAction::PanStart(details) => self
                .callbacks
                .on_pan_start
                .as_ref()
                .map(|callback| callback(details)),
            GestureAction::Pan(details) => self
                .callbacks
                .on_pan_update
                .as_ref()
                .map(|callback| callback(details)),
            GestureAction::PanEnd(details) => self
                .callbacks
                .on_pan_end
                .as_ref()
                .map(|callback| callback(details)),
            GestureAction::HorizontalDragStart(details) => self
                .callbacks
                .on_horizontal_drag_start
                .as_ref()
                .map(|callback| callback(details)),
            GestureAction::HorizontalDrag(details) => self
                .callbacks
                .on_horizontal_drag_update
                .as_ref()
                .map(|callback| callback(details)),
            GestureAction::HorizontalDragEnd(details) => self
                .callbacks
                .on_horizontal_drag_end
                .as_ref()
                .map(|callback| callback(details)),
            GestureAction::VerticalDragStart(details) => self
                .callbacks
                .on_vertical_drag_start
                .as_ref()
                .map(|callback| callback(details)),
            GestureAction::VerticalDrag(details) => self
                .callbacks
                .on_vertical_drag_update
                .as_ref()
                .map(|callback| callback(details)),
            GestureAction::VerticalDragEnd(details) => self
                .callbacks
                .on_vertical_drag_end
                .as_ref()
                .map(|callback| callback(details)),
        };
    }
    pub fn cancel(&self) {
        if let Some(callback) = &self.callbacks.on_tap_cancel {
            callback();
        }
        if let Some(callback) = &self.callbacks.on_double_tap_cancel {
            callback();
        }
        if let Some(callback) = &self.callbacks.on_pan_cancel {
            callback();
        }
        if let Some(callback) = &self.callbacks.on_horizontal_drag_cancel {
            callback();
        }
        if let Some(callback) = &self.callbacks.on_vertical_drag_cancel {
            callback();
        }
        if let Some(callback) = &self.callbacks.on_cancel {
            callback();
        }
    }
    pub fn handle(&mut self, event: PointerEvent) -> bool {
        match self.observe(event) {
            GestureDecision::Accept(action) => {
                self.dispatch(action);
                true
            }
            GestureDecision::Pending => self.down.is_some(),
            GestureDecision::Reject => false,
            GestureDecision::Cancelled => {
                self.cancel();
                true
            }
        }
    }
}

impl GestureRecognizer for PointerGestureRecognizer {
    fn observe(&mut self, event: PointerEvent) -> GestureDecision {
        Self::observe(self, event)
    }

    fn dispatch(&mut self, action: GestureAction) {
        Self::dispatch(self, action);
    }

    fn observe_raw(&mut self, event: crate::details::RawPointerEvent) -> GestureDecision {
        self.kind = event.kind;
        Self::observe(self, event.legacy())
    }

    fn cancel(&mut self) {
        Self::cancel(self);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl GestureDetector {
    #[must_use]
    pub fn new(callbacks: GestureCallbacks) -> Self {
        Self {
            recognizer: PointerGestureRecognizer::new(callbacks),
        }
    }

    pub fn observe(&mut self, event: PointerEvent) -> GestureDecision {
        self.recognizer.observe(event)
    }

    pub fn dispatch(&mut self, action: GestureAction) {
        self.recognizer.dispatch(action);
    }

    pub fn cancel(&mut self) {
        self.recognizer.cancel();
    }

    pub fn handle(&mut self, event: PointerEvent) -> bool {
        self.recognizer.handle(event)
    }
}

/// A platform-neutral pan recognizer with start, update and end callbacks.
///
/// It is Flutter's `PanGestureRecognizer`: it accepts once the pointer's path
/// exceeds the pan slop for its device kind, starts at the accepted position
/// and reports incremental updates.
pub struct DragGestureDetector {
    recognizer: PointerGestureRecognizer,
}
impl DragGestureDetector {
    #[must_use]
    pub fn new(callbacks: DragCallbacks) -> Self {
        Self::with_kind(callbacks, PointerDeviceKind::Unknown)
    }
    /// Creates a detector whose slops follow the given pointer device kind.
    #[must_use]
    pub fn with_kind(callbacks: DragCallbacks, kind: PointerDeviceKind) -> Self {
        let mut recognizer = PointerGestureRecognizer::new(GestureCallbacks {
            on_pan_start: callbacks.on_start,
            on_pan_update: callbacks.on_update,
            on_pan_end: callbacks.on_end,
            ..GestureCallbacks::default()
        });
        recognizer.kind = kind;
        Self { recognizer }
    }
    pub fn handle(&mut self, event: PointerEvent) -> bool {
        match event.phase {
            PointerPhase::Enter | PointerPhase::Exit => false,
            _ => self.recognizer.handle(event),
        }
    }
}
