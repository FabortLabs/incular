use incular_core::time::Instant;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use incular_core::{
    DragStartBehavior, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, SECONDARY_POINTER_BUTTON,
    TERTIARY_POINTER_BUTTON,
};

use crate::details::{
    DragCallbacks, DragDownDetails, DragEndDetails, DragStartDetails, DragUpdateDetails,
    GestureAction, GestureCallbacks, GestureDecision, LongPressDownDetails, LongPressEndDetails,
    LongPressMoveUpdateDetails, LongPressStartDetails, PointerDeviceKind, PointerEvent,
    TapDownDetails, TapMoveDetails, TapUpDetails, Velocity,
};
use crate::raw::GestureRecognizer;
use crate::tap_callback_error::{TapCallbackKind, TapCancelReason, invoke as invoke_tap_callback};
use crate::velocity::{VelocityEstimate, VelocityTracker, VelocityTrackerKind};

use crate::constants::{
    DOUBLE_TAP_MIN_TIME, DOUBLE_TAP_SLOP, DOUBLE_TAP_TOUCH_SLOP, MAX_FLING_VELOCITY,
    MIN_FLING_VELOCITY, PAN_SLOP, PRESS_TIMEOUT,
};
use crate::slop::{compute_hit_slop, compute_pan_slop};
use incular_core::DeviceGestureSettings;

impl GestureCallbacks {
    #[doc(hidden)]
    #[must_use]
    pub fn has_tap_callbacks(&self) -> bool {
        self.on_tap.is_some()
            || self.on_tap_move.is_some()
            || self.on_tap_down.is_some()
            || self.on_tap_up.is_some()
            || self.on_tap_cancel.is_some()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn has_double_tap_callbacks(&self) -> bool {
        self.on_double_tap.is_some()
            || self.on_double_tap_down.is_some()
            || self.on_double_tap_cancel.is_some()
    }

    /// Whether this callback set contributes an exclusive single-pointer
    /// recognizer to a gesture arena. Scale is intentionally separate so a
    /// pending pinch can coexist until one recognizer claims the stream.
    #[must_use]
    pub fn has_pointer_recognizer(&self) -> bool {
        self.has_tap_callbacks()
            || self.has_double_tap_callbacks()
            || self.on_long_press_down.is_some()
            || self.on_long_press_cancel.is_some()
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

/// How far a tap may move from its original position.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TapSlopTolerance {
    /// Use the view's touch slop, defaulting to 18 logical pixels.
    #[default]
    Default,
    /// Maximum global distance in logical pixels.
    Distance(f32),
    /// Do not reject a tap based on movement.
    Unlimited,
}

impl TapSlopTolerance {
    fn exceeded(self, distance: f32, default: f32) -> bool {
        match self {
            Self::Default => distance > default,
            Self::Distance(tolerance) => distance > tolerance,
            Self::Unlimited => false,
        }
    }
}

/// Tap details held after pointer-up until the tap wins the arena.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct PendingTapRelease {
    down: Option<TapDownDetails>,
    up: TapUpDetails,
    down_announced: bool,
}

impl PendingTapRelease {
    /// Reports the remaining detail callbacks in Flutter's order.
    pub fn dispatch(self, callbacks: &GestureCallbacks) {
        if let Some(details) = self.down
            && let Some(callback) = &callbacks.on_tap_down
        {
            invoke_tap_callback(TapCallbackKind::Down, || callback(details));
        }
        if let Some(callback) = &callbacks.on_tap_up {
            invoke_tap_callback(TapCallbackKind::Up, || callback(self.up));
        }
    }

    /// Reports cancellation only if the tap already announced its down.
    pub fn cancel(self, callbacks: &GestureCallbacks, reason: TapCancelReason) {
        if self.down_announced
            && let Some(callback) = &callbacks.on_tap_cancel
        {
            invoke_tap_callback(TapCallbackKind::Cancel(reason), || callback());
        }
    }
}

/// A reusable recognizer for tap, double-tap, long-press, and pan.
pub struct PointerGestureRecognizer {
    callbacks: GestureCallbacks,
    allowed_buttons: Option<Rc<dyn Fn(u32) -> bool>>,
    kind: PointerDeviceKind,
    down: Option<PointerEvent>,
    down_buttons: Option<u32>,
    last_tap: Option<Instant>,
    /// Flutter `_firstTap`: where and when the first tap of a possible double
    /// tap went down.
    last_tap_down: Option<(Instant, Offset, u32)>,
    /// The current press is the second tap of a double tap.
    double_tap_second: Cell<bool>,
    /// The current press is too far from the first tap to join its double
    /// tap (`kDoubleTapSlop`), so the first tap stays pending without it.
    double_tap_ignored: bool,
    /// The pointer moved beyond `kDoubleTapTouchSlop` during the press.
    double_tap_moved: bool,
    /// `onTapDown` of the current press, deferred until `kPressTimeout`
    /// elapses or the tap is accepted.
    pending_tap_down: Option<TapDownDetails>,
    pending_tap_release: Cell<Option<PendingTapRelease>>,
    /// Flutter's `_sentTapDown`: `onTapCancel` follows only a reported
    /// `onTapDown`.
    tap_down_sent: bool,
    tap_accepted: bool,
    pre_accept_tap_slop: TapSlopTolerance,
    post_accept_tap_slop: TapSlopTolerance,
    /// Global-to-local transform of the receiving widget; `None` reports
    /// local values equal to global ones.
    local_transform: Option<incular_core::Transform>,
    /// Physical length of one unit of local x/y movement. Compute this
    /// once at routing rather than inverting or taking norms on each move.
    world_axis_scale: [f64; 2],
    pan_started: bool,
    pan_action: Option<PanAction>,
    long_press_started: bool,
    long_press_duration: Option<Duration>,
    velocity: VelocityTracker,
    tap_rejected: bool,
    drag: DragProgress,
    /// `GestureRecognizer.minFlingVelocity`, `maxFlingVelocity` and
    /// `minFlingDistance` overrides (a scrollable's physics set them).
    fling: FlingLimits,
    /// With `DragStartBehavior.down`, the update Flutter's `_checkDrag`
    /// reports right after `onStart` for the movement that won the arena.
    start_update: Option<GestureAction>,
    /// Pans accept like Flutter's `ImmediateMultiDragGestureRecognizer`.
    immediate_drag: bool,
    /// Flutter `DelayedMultiDragGestureRecognizer.delay`: the drag starts once
    /// the press has lasted this long, provided it stayed within slop.
    delayed_drag: Option<Duration>,
    /// A delayed drag moved beyond slop before its delay and lost the arena.
    drag_rejected: bool,
    /// The unstarted drag callbacks are owed at most one cancellation.
    drag_cancel_pending: bool,
    /// The arena resolved this press for the recognizer before any movement
    /// (Flutter closes the arena after the hit test, and a sole member wins by
    /// default). Its drag then starts without waiting for the slop.
    accepted_on_press: bool,
    /// The first-tap record shared by the recognizers of successive presses on
    /// one widget (Flutter keeps `_firstTap` on a recognizer for its lifetime).
    tap_memory: Option<Rc<Cell<TapMemory>>>,
    /// This contact may still deliver a single long-press cancellation.
    long_press_pending: bool,
    /// Movement beyond slop before the long-press deadline rejected the long
    /// press (Flutter `LongPressGestureRecognizer` resolves rejected), so the
    /// deadline no longer starts it.
    long_press_rejected: bool,
    /// Flutter `GestureRecognizer.gestureSettings`: the platform slops a
    /// `GestureDetector` takes from `MediaQuery`.
    gesture_settings: DeviceGestureSettings,
}

/// The first tap of a possible double tap, carried from one press to the
/// next through [`PointerGestureRecognizer::set_tap_memory`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TapMemory {
    last_tap: Option<Instant>,
    last_tap_down: Option<(Instant, Offset, u32)>,
}

/// Overrides for the fling thresholds of the drag recognizers; `None` keeps
/// Flutter's defaults (`kMinFlingVelocity`, `kMaxFlingVelocity` and the
/// device kind's hit slop).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlingLimits {
    /// Flutter `minFlingVelocity`.
    pub min_velocity: Option<f32>,
    /// Flutter `maxFlingVelocity`.
    pub max_velocity: Option<f32>,
    /// Flutter `minFlingDistance`.
    pub min_distance: Option<f32>,
}

/// Flutter `DragGestureRecognizer` bookkeeping for the active pointer.
#[derive(Clone, Copy)]
struct DragProgress {
    /// `_pendingDragOffset`: movement accumulated before the drag is accepted.
    pending: Offset,
    pending_local: Offset,
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
            pending_local: Offset::ZERO,
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
    /// Flutter `kDoubleTapTimeout`.
    pub const DOUBLE_TAP_TIMEOUT: Duration = crate::constants::DOUBLE_TAP_TIMEOUT;
    /// Flutter `kLongPressTimeout`.
    pub const LONG_PRESS_TIMEOUT: Duration = crate::constants::LONG_PRESS_TIMEOUT;
    /// Flutter kPanSlop (touch-class pan acceptance distance).
    pub const PAN_SLOP: f32 = PAN_SLOP;
    /// Creates a recognizer whose device kind is unknown until a raw event
    /// reports it. Flutter treats unknown kinds with touch slops.
    #[must_use]
    pub fn new(callbacks: GestureCallbacks) -> Self {
        Self {
            callbacks,
            allowed_buttons: None,
            kind: PointerDeviceKind::Unknown,
            down: None,
            down_buttons: None,
            last_tap: None,
            last_tap_down: None,
            double_tap_second: Cell::new(false),
            double_tap_ignored: false,
            double_tap_moved: false,
            pending_tap_down: None,
            pending_tap_release: Cell::new(None),
            tap_down_sent: false,
            tap_accepted: false,
            pre_accept_tap_slop: TapSlopTolerance::Default,
            post_accept_tap_slop: TapSlopTolerance::Default,
            local_transform: None,
            world_axis_scale: [1., 1.],
            pan_started: false,
            pan_action: None,
            long_press_started: false,
            long_press_duration: Some(Self::LONG_PRESS_TIMEOUT),
            velocity: VelocityTracker::default(),
            tap_rejected: false,
            drag: DragProgress::at(Offset::ZERO),
            fling: FlingLimits::default(),
            start_update: None,
            immediate_drag: false,
            delayed_drag: None,
            drag_rejected: false,
            drag_cancel_pending: false,
            accepted_on_press: false,
            tap_memory: None,
            long_press_pending: false,
            long_press_rejected: false,
            gesture_settings: DeviceGestureSettings::default(),
        }
    }
    /// Flutter `GestureRecognizer.gestureSettings`: platform slop overrides
    /// for touch-class pointers. The default keeps `kTouchSlop` and
    /// `kPanSlop`; mice always use the precise-pointer slops.
    pub fn set_gesture_settings(&mut self, settings: DeviceGestureSettings) {
        self.gesture_settings = settings;
    }
    /// Filters new raw pointer presses by device category.
    pub fn set_supported_devices(&mut self, kinds: impl IntoIterator<Item = PointerDeviceKind>) {
        self.callbacks.set_supported_devices(kinds);
    }

    /// Restricts which initial button masks may enter this raw recognizer.
    /// Active presses keep their original admission. Legacy pointer events
    /// without a mask are treated as primary for this filter.
    pub fn set_allowed_buttons_filter(&mut self, filter: Option<Rc<dyn Fn(u32) -> bool>>) {
        self.allowed_buttons = filter;
    }

    fn allows_buttons(&self, buttons: u32) -> bool {
        self.allowed_buttons
            .as_ref()
            .is_none_or(|accept| accept(buttons))
    }

    fn forget_first_tap(&mut self) {
        self.last_tap = None;
        self.last_tap_down = None;
    }

    fn reject_double_tap_press(&mut self) {
        // Flutter resets a pending first tap even when a second press
        // is rejected by the device or allowed-buttons filter.
        self.cancel_double_tap();
        self.forget_first_tap();
        self.save_tap_memory();
    }

    // Only a press that qualified as the second tap reaches the cancellation
    // callback. Resolving the flag before invoking user code makes it one-shot.
    fn cancel_double_tap(&mut self) {
        if self.double_tap_second.replace(false) {
            self.double_tap_moved = true;
            self.forget_first_tap();
            self.save_tap_memory();
            if let Some(callback) = &self.callbacks.on_double_tap_cancel {
                callback();
            }
        }
    }

    fn save_tap_memory(&self) {
        if let Some(memory) = &self.tap_memory {
            memory.set(TapMemory {
                last_tap: self.last_tap,
                last_tap_down: self.last_tap_down,
            });
        }
    }

    /// Overrides the tap's maximum movement before and after it wins its
    /// arena. Other recognizers in this member still use their own slops.
    pub fn set_tap_slop_tolerances(
        &mut self,
        before_acceptance: TapSlopTolerance,
        after_acceptance: TapSlopTolerance,
    ) {
        for tolerance in [before_acceptance, after_acceptance] {
            debug_assert!(!matches!(tolerance, TapSlopTolerance::Distance(distance) if distance.is_nan() || distance < 0.));
        }
        self.pre_accept_tap_slop = before_acceptance;
        self.post_accept_tap_slop = after_acceptance;
    }

    /// The platform slop overrides in effect.
    #[must_use]
    pub fn gesture_settings(&self) -> DeviceGestureSettings {
        self.gesture_settings
    }

    /// Sets the long-press deadline for this recognizer before routing a press.
    #[must_use]
    pub fn with_long_press_duration(mut self, duration: Duration) -> Self {
        self.long_press_duration = Some(duration);
        self
    }
    /// Makes the drag start only after the press has lasted `delay`, as
    /// Flutter's `DelayedMultiDragGestureRecognizer` does (a long-press
    /// reorder or `LongPressDraggable`). Movement beyond slop before the delay
    /// rejects the drag; `None` restores immediate starts.
    pub fn set_delayed_drag(&mut self, delay: Option<Duration>) {
        self.delayed_drag = delay;
    }
    /// Shares the first-tap record with the recognizers of later presses on the
    /// same widget, so a double tap can complete across two presses.
    pub fn set_tap_memory(&mut self, memory: Rc<Cell<TapMemory>>) {
        let stored = memory.get();
        self.last_tap = stored.last_tap;
        self.last_tap_down = stored.last_tap_down;
        self.tap_memory = Some(memory);
    }
    /// Whether the arena may resolve this press for the recognizer before any
    /// movement: a drag-only recognizer that starts on movement and waits for
    /// no deadline. Tap, long-press and delayed drags keep competing.
    #[must_use]
    pub fn can_accept_on_press(&self) -> bool {
        self.delayed_drag.is_none()
            && !self.has_tap_callbacks()
            && !self.has_long_press_callbacks()
            && self.callbacks.on_double_tap.is_none()
            && self.callbacks.on_double_tap_down.is_none()
            && self.callbacks.on_double_tap_cancel.is_none()
            && self.sole_drag_axis().is_some()
    }

    /// Combined pointer members may represent multiple Flutter recognizers.
    /// Only a tap without a local competitor can win at arena close.
    #[doc(hidden)]
    #[must_use]
    pub fn can_accept_tap_on_press(&self) -> bool {
        self.pending_tap_down.is_some()
            && self.delayed_drag.is_none()
            && !self.has_long_press_callbacks()
            && !self.has_pan_callbacks()
            && !self.has_horizontal_drag_callbacks()
            && !self.has_vertical_drag_callbacks()
            && self.callbacks.on_double_tap.is_none()
            && self.callbacks.on_double_tap_down.is_none()
            && self.callbacks.on_double_tap_cancel.is_none()
    }
    /// Sets Flutter's `onlyAcceptDragOnThreshold`. It controls when
    /// callbacks begin; it does not change arena eligibility.
    pub fn set_only_accept_drag_on_threshold(&mut self, value: bool) {
        self.callbacks.only_accept_drag_on_threshold = value;
    }

    /// Accepts the press for the arena's sole drag member (see
    /// [`Self::can_accept_on_press`]). By default it starts at the press
    /// and reports every later movement. An explicit threshold policy
    /// records the same arena win but withholds callbacks until slop.
    pub fn accept_on_press(&mut self) -> Option<GestureAction> {
        let down = self.down?;
        let action = self.sole_drag_axis()?;
        if self.pan_started || self.drag_rejected {
            return None;
        }
        self.accepted_on_press = true;
        if self.callbacks.only_accept_drag_on_threshold {
            return None;
        }
        Some(self.begin_drag(down, action, &down, DragStartBehavior::Down))
    }

    /// Accept a drag by default when the other arena members left during
    /// this move. Unlike an uncontested press, the default start position
    /// and time are taken from the move for `DragStartBehavior::Start`.
    #[doc(hidden)]
    pub fn accept_drag_after_competitors_leave(
        &mut self,
        event: &PointerEvent,
    ) -> Option<GestureAction> {
        if event.phase != PointerPhase::Move || !self.can_accept_on_press() {
            return None;
        }
        let down = self.down.filter(|down| down.pointer == event.pointer)?;
        let action = self.sole_drag_axis()?;
        if self.pan_started || self.drag_rejected {
            return None;
        }
        self.accepted_on_press = true;
        if self.callbacks.only_accept_drag_on_threshold {
            return None;
        }
        Some(self.begin_drag(down, action, event, self.callbacks.drag_start_behavior))
    }

    /// The timestamp of the last pending drag event, cleared on resolution.
    #[doc(hidden)]
    #[must_use]
    pub fn debug_last_pending_drag_timestamp(&self) -> Option<Instant> {
        self.drag.pending_time
    }

    /// Resolves the sole local drag after its sibling tap lost the
    /// press. The host must call this only when there is no competing
    /// member outside this combined recognizer.
    #[doc(hidden)]
    pub fn accept_remaining_drag_after_tap_loss(
        &mut self,
        event: &PointerEvent,
    ) -> Option<GestureAction> {
        if event.phase != PointerPhase::Move
            || self.pan_started
            || self.drag_rejected
            || self.delayed_drag.is_some()
            || self.callbacks.only_accept_drag_on_threshold
            || !self.has_tap_callbacks()
            || !self.tap_rejected
            || self.has_long_press_callbacks()
            || self.callbacks.on_double_tap.is_some()
            || self.callbacks.on_double_tap_down.is_some()
            || self.callbacks.on_double_tap_cancel.is_some()
        {
            return None;
        }
        let down = self.down.filter(|down| down.pointer == event.pointer)?;
        let action = self.sole_drag_axis()?;
        Some(self.begin_drag(down, action, event, self.callbacks.drag_start_behavior))
    }

    /// Makes pans accept as Flutter's `ImmediateMultiDragGestureRecognizer`
    /// does, once the displacement from the press exceeds the hit slop
    /// (`kTouchSlop`, or `kPrecisePointerHitSlop` for mice), rather than when
    /// the path length exceeds the pan slop.
    pub fn set_immediate_drag(&mut self, immediate: bool) {
        self.immediate_drag = immediate;
    }
    /// Chooses the velocity tracker for drag releases (Flutter
    /// `velocityTrackerBuilder`); a `Scrollable` takes it from its scroll
    /// behavior.
    pub fn set_velocity_tracker(&mut self, kind: VelocityTrackerKind) {
        self.velocity = VelocityTracker::new(kind);
    }
    /// Sets the fling thresholds, as a `Scrollable` does from its physics.
    pub fn set_fling_limits(&mut self, limits: FlingLimits) {
        self.fling = limits;
    }
    /// Takes the update that follows an accepted drag start under
    /// [`DragStartBehavior::Down`]. Callers that dispatch actions themselves
    /// deliver it right after the start action.
    pub fn take_start_update(&mut self) -> Option<GestureAction> {
        self.start_update.take()
    }
    /// Delivers the tap's arena acceptance for the tracked pointer.
    /// Flutter reports TapDown by the earlier of acceptance and the press
    /// timeout; this keeps a later slop rejection observable as TapCancel.
    /// Returns whether it newly delivered the down notification.
    pub fn accept_tap_on_arena_close(&mut self, pointer: u64) -> bool {
        if self.down.is_none_or(|down| down.pointer != pointer)
            || !self.has_tap_callbacks()
            || self.tap_rejected
        {
            return false;
        }
        self.tap_accepted = true;
        let announced = self.pending_tap_down.is_some();
        self.flush_tap_down();
        announced
    }

    /// Reports `onTapDown` once, for the press in progress.
    fn flush_tap_down(&mut self) {
        if let Some(details) = self.pending_tap_down.take() {
            self.tap_down_sent = true;
            if let Some(callback) = &self.callbacks.on_tap_down {
                invoke_tap_callback(TapCallbackKind::Down, || callback(details));
            }
        }
    }
    /// Flutter's `_checkCancel`: a tap that loses after reporting
    /// `onTapDown` reports `onTapCancel`; one that never reported it stays
    /// silent.
    fn tap_cancel(&mut self, reason: TapCancelReason) {
        self.pending_tap_down = None;
        if let Some(release) = self.pending_tap_release.take() {
            release.cancel(&self.callbacks, reason);
        }
        if std::mem::take(&mut self.tap_down_sent)
            && let Some(callback) = &self.callbacks.on_tap_cancel
        {
            invoke_tap_callback(TapCallbackKind::Cancel(reason), || callback());
        }
    }

    fn tap_withdrawal_reason(&self, otherwise: TapCancelReason) -> TapCancelReason {
        if self.tap_accepted {
            TapCancelReason::Spontaneous
        } else {
            otherwise
        }
    }
    /// Sets the global-to-local transform for the details' local positions,
    /// local drag deltas and velocities (Flutter's event transform);
    /// retained adapters pass the widget's inverse world transform.
    pub fn set_local_transform(&mut self, transform: incular_core::Transform) {
        self.local_transform = Some(transform);
        self.world_axis_scale = if transform.is_translation() {
            [1., 1.]
        } else if let Some(world) = transform.inverse() {
            let [a, b, c, d, _, _] = world.coefficients();
            [a.hypot(b), c.hypot(d)]
        } else {
            [1., 1.]
        };
    }
    fn local(&self, point: Offset) -> Offset {
        self.local_transform
            .map_or(point, |transform| transform.transform_point(point))
    }
    fn local_delta(&self, delta: Offset) -> Offset {
        self.local_transform
            .map_or(delta, |transform| transform.transform_vector(delta))
    }
    fn global_delta(&self, delta: Offset) -> Offset {
        self.local_transform
            .filter(|transform| !transform.is_translation())
            .and_then(incular_core::Transform::inverse)
            .map_or(delta, |transform| transform.transform_vector(delta))
    }

    fn project_drag_delta(&self, action: PanAction, delta: Offset) -> (Offset, Option<f32>) {
        match action {
            PanAction::Pan => (delta, None),
            PanAction::Horizontal => (Offset::new(delta.x, 0.), Some(delta.x)),
            PanAction::Vertical => (Offset::new(0., delta.y), Some(delta.y)),
        }
    }
    fn has_tap_callbacks(&self) -> bool {
        self.callbacks.has_tap_callbacks()
    }
    /// The recognizer lost its gesture arena (Flutter `rejectGesture`): a
    /// tap that already reported `onTapDown` is cancelled.
    pub fn reject(&mut self) {
        self.cancel_double_tap();
        self.tap_rejected = true;
        self.tap_cancel(TapCancelReason::Forced);
        self.long_press_cancel();
        self.cancel_pending_drag();
        self.drag.pending_time = None;
        self.drag_rejected = true;
    }
    /// Fires the deadline-driven callbacks of the press in progress at
    /// `now`: `onTapDown` after `kPressTimeout` and the long-press start after
    /// `kLongPressTimeout`. A runtime calls this each frame while
    /// [`has_pending_deadline`](Self::has_pending_deadline) is true; events
    /// carry their own timestamps and apply the same deadlines.
    pub fn poll(&mut self, now: Instant) -> GestureDecision {
        let Some(down) = self.down else {
            return GestureDecision::Pending;
        };
        let held = now.saturating_duration_since(down.time);
        if held >= PRESS_TIMEOUT && !self.pan_started {
            self.flush_tap_down();
        }
        if let Some(delay) = self.delayed_drag
            && !self.pan_started
            && !self.drag_rejected
            && held >= delay
            && let Some(action) = self.start_delayed_drag(down, now)
        {
            return GestureDecision::Accept(action);
        }
        if let Some(duration) = self.long_press_duration
            && let Some(details) = self.due_long_press(now, duration)
        {
            return GestureDecision::Accept(self.begin_long_press(details));
        }
        GestureDecision::Pending
    }

    /// An external arena must award the deadline before the recognizer
    /// exposes a start, move, or end for that hold.
    pub(crate) fn with_external_long_press_deadline(mut self) -> Self {
        self.long_press_duration = None;
        self
    }

    pub(crate) fn has_pending_long_press(&self) -> bool {
        self.down.is_some()
            && !self.pan_started
            && !self.long_press_started
            && !self.long_press_rejected
            && self.has_long_press_callbacks()
    }

    pub(crate) fn due_long_press(
        &self,
        now: Instant,
        duration: Duration,
    ) -> Option<LongPressStartDetails> {
        let down = self.down?;
        if self.has_pending_long_press() && now.saturating_duration_since(down.time) >= duration {
            Some(LongPressStartDetails {
                global_position: down.position,
                local_position: self.local(down.position),
            })
        } else {
            None
        }
    }

    pub(crate) fn begin_long_press(&mut self, details: LongPressStartDetails) -> GestureAction {
        self.long_press_started = true;
        // The long press wins the arena; the tap loses.
        self.tap_cancel(TapCancelReason::Forced);
        GestureAction::LongPressStart(details)
    }

    /// Whether [`poll`](Self::poll) still has a deadline to fire for the press
    /// in progress.
    #[must_use]
    pub fn has_pending_deadline(&self) -> bool {
        self.down.is_some()
            && !self.pan_started
            && (self.pending_tap_down.is_some()
                || (self.long_press_duration.is_some() && self.has_pending_long_press())
                || (self.delayed_drag.is_some() && !self.drag_rejected))
    }
    /// A possible long press can be cancelled even without a down callback.
    fn long_press_cancel(&mut self) {
        if std::mem::take(&mut self.long_press_pending)
            && !self.long_press_started
            && let Some(callback) = &self.callbacks.on_long_press_cancel
        {
            callback();
        }
    }
    fn has_long_press_callbacks(&self) -> bool {
        self.callbacks.on_long_press_down.is_some()
            || self.callbacks.on_long_press_cancel.is_some()
            || self.callbacks.on_long_press.is_some()
            || self.callbacks.on_long_press_start.is_some()
            || self.callbacks.on_long_press_move_update.is_some()
            || self.callbacks.on_long_press_up.is_some()
            || self.callbacks.on_long_press_end.is_some()
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
        // `minFlingDistance` defaults to `computeHitSlop`; the velocity bounds
        // to `kMinFlingVelocity` and `kMaxFlingVelocity`.
        let hit_slop = self
            .fling
            .min_distance
            .unwrap_or_else(|| compute_hit_slop(self.kind, self.gesture_settings));
        let min_velocity = self.fling.min_velocity.unwrap_or(MIN_FLING_VELOCITY);
        let max_velocity = self
            .fling
            .max_velocity
            .unwrap_or(MAX_FLING_VELOCITY)
            .max(min_velocity);
        let velocity = estimate.pixels_per_second;
        let offset = estimate.offset;
        let not_a_fling = (Velocity::ZERO, Some(0.));
        match action {
            PanAction::Horizontal => {
                if velocity.x.abs() > min_velocity && offset.x.abs() > hit_slop {
                    let dx = velocity.x.max(-max_velocity).min(max_velocity);
                    (Velocity::new(Offset::new(dx, 0.)), Some(dx))
                } else {
                    not_a_fling
                }
            }
            PanAction::Vertical => {
                if velocity.y.abs() > min_velocity && offset.y.abs() > hit_slop {
                    let dy = velocity.y.max(-max_velocity).min(max_velocity);
                    (Velocity::new(Offset::new(0., dy)), Some(dy))
                } else {
                    not_a_fling
                }
            }
            PanAction::Pan => {
                let speed = velocity.x.hypot(velocity.y);
                let distance = offset.x.hypot(offset.y);
                if speed > min_velocity && distance > hit_slop {
                    // Velocity.clampMagnitude(minFlingVelocity, maxFlingVelocity).
                    let magnitude = speed.max(min_velocity).min(max_velocity);
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
        if self.drag_rejected {
            return None;
        }
        if self.accepted_on_press && !self.callbacks.only_accept_drag_on_threshold {
            // The arena resolved this press before any movement, so the first
            // movement starts the drag without a slop test.
            return if self.drag.pending == Offset::ZERO {
                None
            } else {
                self.mixed_axis_candidate()
            };
        }
        if self.has_vertical_drag_callbacks()
            && self.drag.moved_vertical.abs() > compute_hit_slop(self.kind, self.gesture_settings)
        {
            Some(PanAction::Vertical)
        } else if self.has_horizontal_drag_callbacks()
            && self.drag.moved_horizontal.abs() > compute_hit_slop(self.kind, self.gesture_settings)
        {
            Some(PanAction::Horizontal)
        } else if self.has_pan_callbacks()
            && if self.immediate_drag {
                self.drag.pending.x.hypot(self.drag.pending.y)
                    > compute_hit_slop(self.kind, self.gesture_settings)
            } else {
                self.drag.moved_pan.abs() > compute_pan_slop(self.kind, self.gesture_settings)
            }
        {
            Some(PanAction::Pan)
        } else {
            None
        }
    }
    /// The only drag kind the recognizer carries, when there is exactly one.
    fn sole_drag_axis(&self) -> Option<PanAction> {
        match (
            self.has_vertical_drag_callbacks(),
            self.has_horizontal_drag_callbacks(),
            self.has_pan_callbacks(),
        ) {
            (true, false, false) => Some(PanAction::Vertical),
            (false, true, false) => Some(PanAction::Horizontal),
            (false, false, true) => Some(PanAction::Pan),
            _ => None,
        }
    }
    /// The drag kind for a press the arena already resolved: the axis the
    /// movement favours among the callbacks present, else pan.
    fn mixed_axis_candidate(&self) -> Option<PanAction> {
        if self.has_vertical_drag_callbacks()
            && (!self.has_horizontal_drag_callbacks()
                || self.drag.moved_vertical.abs() >= self.drag.moved_horizontal.abs())
        {
            Some(PanAction::Vertical)
        } else if self.has_horizontal_drag_callbacks() {
            Some(PanAction::Horizontal)
        } else if self.has_pan_callbacks() {
            Some(PanAction::Pan)
        } else {
            None
        }
    }
    /// Starts the drag for `action` at the press position. `event` supplies
    /// the time and pointer for the update that follows a `Down` start.
    /// Flutter `_checkDrag`: with [`DragStartBehavior::Start`] the drag begins
    /// where it was accepted and the crossing movement is not an update; with
    /// `Down` it begins at the press and the pending movement (restricted to
    /// the axis) is the first update.
    fn begin_drag(
        &mut self,
        down: PointerEvent,
        action: PanAction,
        event: &PointerEvent,
        behavior: DragStartBehavior,
    ) -> GestureAction {
        let local_down = self.local(down.position);
        let (global_position, local_position) = match behavior {
            DragStartBehavior::Start => (
                down.position + self.drag.pending,
                local_down + self.drag.pending_local,
            ),
            DragStartBehavior::Down => {
                let (local_delta, _) = self.project_drag_delta(action, self.drag.pending_local);
                self.start_update = (local_delta != Offset::ZERO).then(|| {
                    self.drag_update_local(
                        action,
                        local_delta,
                        down.position + self.global_delta(local_delta),
                        local_down + local_delta,
                        self.drag.pending_time.unwrap_or(event.time),
                    )
                });
                (down.position, local_down)
            }
        };
        let start = DragStartDetails {
            source_time: self.drag.pending_time,
            global_position,
            local_position,
            kind: self.kind,
        };
        self.pan_action = Some(action);
        self.pan_started = true;
        self.drag_cancel_pending = false;
        // The drag wins the arena; the tap loses.
        self.tap_cancel(TapCancelReason::Forced);
        self.drag.pending = Offset::ZERO;
        self.drag.pending_local = Offset::ZERO;
        self.drag.pending_time = None;
        match action {
            PanAction::Pan => GestureAction::PanStart(start),
            PanAction::Horizontal => GestureAction::HorizontalDragStart(start),
            PanAction::Vertical => GestureAction::VerticalDragStart(start),
        }
    }
    /// Starts a delayed drag once its delay has passed, reporting the movement
    /// held within slop as the first update (Flutter `_delayPassed`, which
    /// starts at the press position).
    fn start_delayed_drag(&mut self, down: PointerEvent, time: Instant) -> Option<GestureAction> {
        let action = self.sole_drag_axis()?;
        let event = PointerEvent { time, ..down };
        Some(self.begin_drag(down, action, &event, DragStartBehavior::Down))
    }
    /// An update for the accepted drag: the movement since the previous event,
    /// restricted to the drag axis for horizontal and vertical drags.
    fn drag_update(&self, action: PanAction, step: Offset, event: &PointerEvent) -> GestureAction {
        self.drag_update_local(
            action,
            self.local_delta(step),
            event.position,
            self.local(event.position),
            event.time,
        )
    }

    fn drag_update_local(
        &self,
        action: PanAction,
        local_step: Offset,
        global_position: Offset,
        local_position: Offset,
        time: Instant,
    ) -> GestureAction {
        let (delta, primary_delta) = self.project_drag_delta(action, local_step);
        let details = DragUpdateDetails {
            source_time: Some(time),
            delta,
            primary_delta,
            global_position,
            local_position,
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
            local_position: self.local(self.drag.last_position),
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
        if event.phase == PointerPhase::Down {
            if !self.callbacks.allows_pointer_device(PointerDeviceKind::Unknown)
                || !self.allows_buttons(PRIMARY_POINTER_BUTTON)
            {
                self.reject_double_tap_press();
                return GestureDecision::Reject;
            }
            if self.down.is_none_or(|press| press.pointer == event.pointer) {
                self.kind = PointerDeviceKind::Unknown;
            }
        }
        self.observe_with_buttons(event, None, false)
    }

    fn observe_with_buttons(
        &mut self,
        event: PointerEvent,
        buttons: Option<u32>,
        synthesized: bool,
    ) -> GestureDecision {
        let mut routed = event;
        if self.down.is_none_or(|down| down.pointer == event.pointer) {
            if event.phase == PointerPhase::Down {
                self.down_buttons = buttons.filter(|&mask| mask != 0);
            } else if event.phase == PointerPhase::Move
                && self.down.is_some()
                && self
                    .down_buttons
                    .zip(buttons)
                    .is_some_and(|(down, now)| down != now)
            {
                // A tap belongs to the mask of the original press, even
                // when the pointer has not moved. A competing long press
                // or double tap keeps its own state in the combined member.
                let tap_only = self.has_tap_callbacks()
                    && !self.has_long_press_callbacks()
                    && self.callbacks.on_double_tap.is_none()
                    && self.callbacks.on_double_tap_down.is_none()
                    && self.callbacks.on_double_tap_cancel.is_none();
                if self.has_tap_callbacks() {
                    self.tap_rejected = true;
                    self.tap_cancel(self.tap_withdrawal_reason(TapCancelReason::Forced));
                }
                if !self.long_press_started
                    && (self.pan_started
                        || self.has_horizontal_drag_callbacks()
                        || self.has_vertical_drag_callbacks()
                        || self.has_pan_callbacks())
                {
                    // Flutter adds the sample to the velocity tracker, then
                    // gives up the pointer before considering slop or updating
                    // the drag position. The common Cancel path terminates a
                    // started drag, cancels a pending one and clears the press.
                    if !synthesized {
                        self.velocity
                            .add_position(event.time, self.local(event.position));
                    }
                    routed.phase = PointerPhase::Cancel;
                } else if !self.long_press_started
                    && !self.long_press_rejected
                    && self.has_long_press_callbacks()
                {
                    // An accepted long press keeps tracking; only a pending
                    // long press rejects the changed mask.
                    self.long_press_rejected = true;
                    self.long_press_cancel();
                } else if tap_only {
                    routed.phase = PointerPhase::Cancel;
                }
            }
        }
        let decision = self.observe_phase(routed, synthesized);
        self.save_tap_memory();
        decision
    }

    /// Deadline of a first release that can still form a double tap.
    /// A retained host can defer ordinary taps from the same arena until
    /// this decision, even if they belong to a different detector.
    #[must_use]
    pub fn first_tap_hold_deadline(&self) -> Option<Instant> {
        if self.callbacks.has_double_tap_callbacks()
            && !self.double_tap_ignored
            && !self.double_tap_second.get()
            && !self.double_tap_moved
        {
            self.last_tap?.checked_add(Self::DOUBLE_TAP_TIMEOUT)
        } else {
            None
        }
    }

    fn observe_phase(&mut self, event: PointerEvent, synthesized: bool) -> GestureDecision {
        // This recognizer has a single contact. A second pointer must not
        // replace that contact or complete/cancel its gesture on release.
        if self.down.is_some_and(|down| down.pointer != event.pointer) {
            return GestureDecision::Reject;
        }
        match event.phase {
            PointerPhase::Enter | PointerPhase::Exit => GestureDecision::Reject,
            PointerPhase::Down => {
                // Flutter `DoubleTapGestureRecognizer.addAllowedPointer`: the
                // first tap's release must be within `kDoubleTapTimeout`;
                // a press farther than `kDoubleTapSlop` from the first tap is
                // not part of the sequence (the first tap stays pending); one
                // earlier than `kDoubleTapMinTime` after the first press
                // restarts the sequence.
                let mut is_double_tap = false;
                let button = self.down_buttons.unwrap_or(PRIMARY_POINTER_BUTTON);
                // The default double-tap recognizer admits exactly the primary
                // mask; a custom filter can opt into other masks. This does not
                // prevent a combined member from handling a secondary single tap.
                self.double_tap_ignored =
                    !self.callbacks.has_double_tap_callbacks()
                        || (self.allowed_buttons.is_none() && button != PRIMARY_POINTER_BUTTON);
                self.double_tap_moved = false;
                if self.double_tap_ignored {
                    self.forget_first_tap();
                } else if let Some(tap) = self.last_tap {
                    if event.time.saturating_duration_since(tap) >= Self::DOUBLE_TAP_TIMEOUT {
                        self.forget_first_tap();
                    } else if let Some((first_down, first_position, first_button)) = self.last_tap_down {
                        let apart = event.position - first_position;
                        if apart.x.hypot(apart.y) > DOUBLE_TAP_SLOP {
                            self.double_tap_ignored = true;
                        } else if first_button != button
                            || event.time.saturating_duration_since(first_down) < DOUBLE_TAP_MIN_TIME
                        {
                            self.forget_first_tap();
                        } else {
                            is_double_tap = true;
                        }
                    }
                }
                self.double_tap_second.set(is_double_tap);
                self.down = Some(event);
                self.drag_cancel_pending = true;
                self.velocity.clear();
                if !synthesized {
                    self.velocity
                        .add_position(event.time, self.local(event.position));
                }
                self.tap_rejected = false;
                self.pan_started = false;
                self.pan_action = None;
                self.accepted_on_press = false;
                self.start_update = None;
                self.drag = DragProgress::at(event.position);
                self.drag.pending_time = Some(event.time);
                self.long_press_started = false;
                self.drag_rejected = false;
                // Flutter admits a long press only for an exact supported
                // single-button mask; a chord cannot emit Down or Cancel.
                // The combined recognizer can still consider other gestures.
                self.long_press_rejected = self.down_buttons.is_some_and(|buttons| {
                    !matches!(
                        buttons,
                        PRIMARY_POINTER_BUTTON | SECONDARY_POINTER_BUTTON | TERTIARY_POINTER_BUTTON
                    )
                });
                self.long_press_pending =
                    !self.long_press_rejected && self.has_long_press_callbacks();
                if !self.long_press_rejected
                    && let Some(callback) = &self.callbacks.on_long_press_down
                {
                    callback(LongPressDownDetails {
                        global_position: event.position,
                        local_position: self.local(event.position),
                    });
                }
                // Flutter's `TapGestureRecognizer` reports `onTapDown` only
                // after `kPressTimeout`, or when the tap is accepted first.
                self.pending_tap_release.take();
                self.tap_down_sent = false;
                self.tap_accepted = false;
                self.pending_tap_down = self.has_tap_callbacks().then_some(TapDownDetails {
                    global_position: event.position,
                    local_position: self.local(event.position),
                    kind: self.kind,
                });
                if is_double_tap && let Some(callback) = &self.callbacks.on_double_tap_down {
                    callback(TapDownDetails {
                        global_position: event.position,
                        local_position: self.local(event.position),
                        kind: self.kind,
                    });
                }
                let pan_down = DragDownDetails {
                    global_position: event.position,
                    local_position: self.local(event.position),
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
                if !synthesized {
                    self.velocity
                        .add_position(event.time, self.local(event.position));
                }
                let distance = delta.x.hypot(delta.y);
                if !self.long_press_started
                    && !self.long_press_rejected
                    && distance > compute_hit_slop(self.kind, self.gesture_settings)
                {
                    self.long_press_rejected = true;
                    self.long_press_cancel();
                }
                let tap_slop = if self.tap_accepted {
                    self.post_accept_tap_slop
                } else {
                    self.pre_accept_tap_slop
                };
                let default_tap_slop = self
                    .gesture_settings
                    .touch_slop
                    .unwrap_or(crate::constants::TOUCH_SLOP);
                if !self.tap_rejected
                    && self.has_tap_callbacks()
                    && tap_slop.exceeded(distance, default_tap_slop)
                {
                    self.tap_rejected = true;
                    // Past the press deadline Flutter had already reported
                    // `onTapDown`, so the rejection cancels the tap; before
                    // it, the tap never reported anything.
                    if event.time.saturating_duration_since(down.time) >= PRESS_TIMEOUT
                        && !self.pan_started
                    {
                        self.flush_tap_down();
                    }
                    self.tap_cancel(self.tap_withdrawal_reason(TapCancelReason::Forced));
                }
                if distance > DOUBLE_TAP_TOUCH_SLOP {
                    self.double_tap_moved = true;
                    self.cancel_double_tap();
                }
                if event.time.saturating_duration_since(down.time) >= PRESS_TIMEOUT
                    && !self.pan_started
                {
                    self.flush_tap_down();
                }
                if self.long_press_started
                    && !self.pan_started
                    && let Some(callback) = &self.callbacks.on_long_press_move_update
                {
                    callback(LongPressMoveUpdateDetails {
                        global_position: event.position,
                        local_position: self.local(event.position),
                        offset_from_origin: delta,
                        local_offset_from_origin: self.local(event.position)
                            - self.local(down.position),
                    });
                }
                if !self.pan_started
                    && !self.tap_rejected
                    && let Some(callback) = &self.callbacks.on_tap_move
                {
                    let details = TapMoveDetails {
                        global_position: event.position,
                        local_position: self.local(event.position),
                        delta: event.position - self.drag.last_position,
                        kind: self.kind,
                    };
                    invoke_tap_callback(TapCallbackKind::Move, || callback(details));
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
                let local_step = self.local_delta(step);
                self.drag.pending_local = self.drag.pending_local + local_step;
                self.drag.pending_time = Some(event.time);
                // Flutter first projects to the local drag axis, then
                // measures signed physical length in world units.
                self.drag.moved_vertical +=
                    (f64::from(local_step.y) * self.world_axis_scale[1]) as f32;
                self.drag.moved_horizontal +=
                    (f64::from(local_step.x) * self.world_axis_scale[0]) as f32;
                // A pan has no axis filter; projecting its full movement
                // back to world space yields the original physical step.
                self.drag.moved_pan += step.x.hypot(step.y);
                // A standalone deadline may also become due on a movement.
                if let Some(duration) = self.long_press_duration
                    && let Some(details) = self.due_long_press(event.time, duration)
                {
                    return GestureDecision::Accept(self.begin_long_press(details));
                }
                if let Some(delay) = self.delayed_drag {
                    // Flutter `DelayedMultiDragGestureRecognizer`: no drag
                    // before the delay; movement beyond slop before it loses.
                    if self.drag_rejected {
                        return GestureDecision::Pending;
                    }
                    if event.time.saturating_duration_since(down.time) >= delay {
                        return self
                            .start_delayed_drag(down, event.time)
                            .map_or(GestureDecision::Pending, GestureDecision::Accept);
                    }
                    if delta.x.hypot(delta.y) > compute_hit_slop(self.kind, self.gesture_settings) {
                        self.drag_rejected = true;
                        return GestureDecision::Reject;
                    }
                    return GestureDecision::Pending;
                }
                if let Some(action) = self.drag_candidate() {
                    // DragStartBehavior.start: the drag begins where it was
                    // accepted and the crossing movement is not an update.
                    // A press the arena resolved earlier starts at the press
                    // and reports this movement, like Flutter's accepted drag.
                    let behavior = if self.accepted_on_press
                        && !self.callbacks.only_accept_drag_on_threshold
                    {
                        DragStartBehavior::Down
                    } else {
                        self.callbacks.drag_start_behavior
                    };
                    return GestureDecision::Accept(
                        self.begin_drag(down, action, &event, behavior),
                    );
                }
                let axis_callbacks =
                    if self.drag.moved_horizontal.abs() >= self.drag.moved_vertical.abs() {
                        self.has_horizontal_drag_callbacks()
                    } else {
                        self.has_vertical_drag_callbacks()
                    };
                if !axis_callbacks
                    && !self.has_pan_callbacks()
                    && delta.x.hypot(delta.y) > compute_pan_slop(self.kind, self.gesture_settings)
                {
                    return GestureDecision::Reject;
                }
                GestureDecision::Pending
            }
            PointerPhase::Up => {
                let Some(down) = self.down.take() else {
                    return GestureDecision::Reject;
                };
                self.drag.pending_time = None;
                if self.pan_started {
                    self.cancel_double_tap();
                    return self
                        .drag_end(event.time)
                        .map_or(GestureDecision::Pending, GestureDecision::Accept);
                }
                // A tap can win within this same widget. Its unstarted
                // drag recognizers still report one cancellation on release.
                self.cancel_pending_drag();
                let elapsed = event.time.saturating_duration_since(down.time);
                if elapsed >= PRESS_TIMEOUT {
                    self.flush_tap_down();
                }
                let long_press_taken = (self.long_press_started
                    || self.long_press_duration.is_some_and(|duration| elapsed >= duration))
                    && self.has_long_press_callbacks()
                    && !self.long_press_rejected;
                if !long_press_taken {
                    self.long_press_cancel();
                }
                if long_press_taken {
                    self.cancel_double_tap();
                    self.tap_cancel(TapCancelReason::Forced);
                    let end = LongPressEndDetails {
                        global_position: event.position,
                        local_position: self.local(event.position),
                        velocity: Velocity::new(
                            self.velocity.estimate_at(event.time).pixels_per_second,
                        ),
                    };
                    self.long_press_pending = false;
                    if std::mem::take(&mut self.long_press_started) {
                        // Already accepted at the deadline or on a move.
                        self.dispatch_long_press_end(end);
                        GestureDecision::Pending
                    } else {
                        // A late release catches up both phases, but neither
                        // phase may report until this member wins the arena.
                        let start = LongPressStartDetails {
                            global_position: down.position,
                            local_position: self.local(down.position),
                        };
                        GestureDecision::Accept(GestureAction::LongPress { start, end })
                    }
                } else if self.double_tap_second.get() && !self.double_tap_moved
                {
                    // The second tap qualified when it went down (within
                    // `kDoubleTapTimeout` and `kDoubleTapSlop` of the first)
                    // and stayed within `kDoubleTapTouchSlop`.
                    self.tap_cancel(TapCancelReason::Forced);
                    self.last_tap = None;
                    self.last_tap_down = None;
                    GestureDecision::Accept(GestureAction::DoubleTap)
                } else if self.tap_rejected {
                    // A rejected tap cannot become a release or seed a new
                    // double-tap sequence.
                    self.last_tap = None;
                    self.last_tap_down = None;
                    self.pending_tap_down = None;
                    GestureDecision::Reject
                } else if self.has_tap_callbacks() {
                    // Releasing a pointer is not itself winning: a double
                    // recognizer may still be holding this tap's arena.
                    self.pending_tap_release.set(Some(PendingTapRelease {
                        down: self.pending_tap_down.take(),
                        up: TapUpDetails {
                            global_position: event.position,
                            local_position: self.local(event.position),
                            kind: self.kind,
                        },
                        down_announced: std::mem::take(&mut self.tap_down_sent),
                    }));
                    if !self.double_tap_ignored && !self.double_tap_moved {
                        self.last_tap = Some(event.time);
                        self.last_tap_down = Some((
                            down.time,
                            down.position,
                            self.down_buttons.unwrap_or(PRIMARY_POINTER_BUTTON),
                        ));
                    }
                    GestureDecision::Accept(GestureAction::Tap)
                } else {
                    // Flutter `DoubleTapGestureRecognizer` keeps the first tap
                    // pending for a second one even without `onTap`.
                    if !self.double_tap_ignored && !self.double_tap_moved {
                        self.last_tap = Some(event.time);
                        self.last_tap_down = Some((
                            down.time,
                            down.position,
                            self.down_buttons.unwrap_or(PRIMARY_POINTER_BUTTON),
                        ));
                    }
                    GestureDecision::Reject
                }
            }
            PointerPhase::Cancel => {
                self.cancel_double_tap();
                self.drag.pending_time = None;
                self.tap_cancel(self.tap_withdrawal_reason(TapCancelReason::Pointer));
                self.long_press_cancel();
                if self.down.take().is_some() {
                    if self.pan_started {
                        // Flutter ends an accepted drag with onEnd when the
                        // pointer is cancelled; onCancel is for unaccepted ones.
                        self.long_press_started = false;
                        return self
                            .drag_end(event.time)
                            .map_or(GestureDecision::Cancelled, GestureDecision::Accept);
                    }
                    self.cancel_pending_drag();
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

    /// Moves the pending TapDown/TapUp details to the retaining arena.
    #[doc(hidden)]
    pub fn take_pending_tap_release(&self) -> Option<PendingTapRelease> {
        self.pending_tap_release.take()
    }

    /// Emits details for a winning tap when its terminal callback is live.
    #[doc(hidden)]
    pub fn dispatch_pending_tap_details(&self) {
        if let Some(release) = self.take_pending_tap_release() {
            release.dispatch(&self.callbacks);
        }
    }

    fn dispatch_long_press_start(&self, details: LongPressStartDetails) {
        if let Some(callback) = &self.callbacks.on_long_press_start {
            callback(details);
        }
        if let Some(callback) = &self.callbacks.on_long_press {
            callback();
        }
    }

    fn dispatch_long_press_end(&self, details: LongPressEndDetails) {
        if let Some(callback) = &self.callbacks.on_long_press_end {
            callback(details);
        }
        if let Some(callback) = &self.callbacks.on_long_press_up {
            callback();
        }
    }

    /// Invokes a callback selected by [`Self::observe`] after the caller has
    /// accepted this recognizer.
    pub fn dispatch(&self, action: GestureAction) {
        match action {
            GestureAction::Tap => {
                self.dispatch_pending_tap_details();
                self.callbacks
                    .on_tap
                    .as_ref()
                    .map(|callback| invoke_tap_callback(TapCallbackKind::Tap, || callback()))
            }
            GestureAction::DoubleTap => {
                self.double_tap_second.set(false);
                self.callbacks.on_double_tap.as_ref().map(|callback| callback())
            }
            GestureAction::LongPress { start, end } => {
                self.dispatch_long_press_start(start);
                self.dispatch_long_press_end(end);
                None
            }
            GestureAction::LongPressStart(details) => {
                // Flutter `_checkLongPressStart`: `onLongPressStart`, then
                // `onLongPress`, when the deadline fires.
                self.dispatch_long_press_start(details);
                None
            }
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
            GestureAction::ArenaAccepted { .. } => None,
        };
    }
    fn cancel_pending_drag(&mut self) {
        if !std::mem::take(&mut self.drag_cancel_pending) {
            return;
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
    }

    pub fn cancel(&mut self) {
        self.tap_cancel(self.tap_withdrawal_reason(TapCancelReason::Forced));
        self.long_press_cancel();
        self.cancel_double_tap();
        self.cancel_pending_drag();
        if let Some(callback) = &self.callbacks.on_cancel {
            callback();
        }
    }
    pub fn handle(&mut self, event: PointerEvent) -> bool {
        match self.observe(event) {
            GestureDecision::Accept(action) => {
                self.dispatch(action);
                if let Some(update) = self.take_start_update() {
                    self.dispatch(update);
                }
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
        if event.phase == PointerPhase::Down {
            if !self.callbacks.allows_pointer_device(event.kind)
                || !self.allows_buttons(event.buttons)
            {
                self.reject_double_tap_press();
                return GestureDecision::Reject;
            }
            if self.down.is_none_or(|press| press.pointer == event.pointer) {
                self.kind = event.kind;
            }
        }
        self.observe_with_buttons(event.legacy(), Some(event.buttons), event.synthesized)
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

mod monodrag;
#[doc(hidden)]
pub use monodrag::DragBoundaryFrame;
pub use monodrag::{
    DragAxis, MonodragGestureRecognizer, MultitouchDragStrategy, TrackpadDragAxis, TrackpadDragRecognizer,
};
