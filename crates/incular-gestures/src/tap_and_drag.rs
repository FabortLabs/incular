//! Flutter's `BaseTapAndDragGestureRecognizer` and its
//! `TapAndHorizontalDragGestureRecognizer` and `TapAndPanGestureRecognizer`
//! (`gestures/tap_and_drag.dart`): taps counted into a series, and drags
//! that start from them.

use std::{collections::HashMap, rc::Rc, time::Duration};

use incular_core::{
    DragStartBehavior, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, Transform, time::Instant,
};

use crate::arena_member::{
    ArenaLink, ArenaMember, arena_recognizer, compute_hit_slop, compute_pan_slop, distance,
};
use crate::constants::{DOUBLE_TAP_SLOP, DOUBLE_TAP_TIMEOUT, PRESS_TIMEOUT};
use crate::{GestureDisposition, PointerDeviceKind, RawPointerEvent, Velocity};

/// Flutter `TapDragDownDetails`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapDragDownDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    pub kind: PointerDeviceKind,
    pub consecutive_tap_count: u32,
}

/// Flutter `TapDragUpDetails`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapDragUpDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    pub kind: PointerDeviceKind,
    pub consecutive_tap_count: u32,
}

/// Flutter `TapDragStartDetails`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapDragStartDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    pub source_time: Option<Instant>,
    pub kind: PointerDeviceKind,
    pub consecutive_tap_count: u32,
}

/// Flutter `TapDragUpdateDetails`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapDragUpdateDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    pub source_time: Option<Instant>,
    pub delta: Offset,
    pub kind: PointerDeviceKind,
    pub offset_from_origin: Offset,
    pub local_offset_from_origin: Offset,
    pub consecutive_tap_count: u32,
}

/// Flutter `TapDragEndDetails`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapDragEndDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    pub velocity: Velocity,
    pub primary_velocity: Option<f32>,
    pub consecutive_tap_count: u32,
}

/// Flutter `_DragState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragState {
    Ready,
    Possible,
    Accepted,
}

/// The two concrete recognizers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragAxis {
    Horizontal,
    Pan,
}

/// A (global, local) position pair, Flutter's `OffsetPair`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct OffsetPair {
    global: Offset,
    local: Offset,
}

impl OffsetPair {
    fn of(event: &RawPointerEvent) -> Self {
        Self {
            global: event.position,
            local: event.local_position,
        }
    }
}

type Callback<T> = Option<Rc<dyn Fn(T)>>;

/// Flutter's `BaseTapAndDragGestureRecognizer`, as
/// [`Self::horizontal_drag`] (`TapAndHorizontalDragGestureRecognizer`) or
/// [`Self::pan`] (`TapAndPanGestureRecognizer`).
pub struct TapAndDragGestureRecognizer {
    axis: DragAxis,
    supported_devices: Option<Vec<PointerDeviceKind>>,
    drag_start_behavior: DragStartBehavior,
    drag_update_throttle_frequency: Option<Duration>,
    max_consecutive_tap: Option<u32>,
    eager_victory_on_drag: bool,
    local_transform: Option<Transform>,
    on_tap_down: Callback<TapDragDownDetails>,
    on_tap_up: Callback<TapDragUpDetails>,
    on_drag_start: Callback<TapDragStartDetails>,
    on_drag_update: Callback<TapDragUpdateDetails>,
    on_drag_end: Callback<TapDragEndDetails>,
    on_cancel: Option<Rc<dyn Fn()>>,
    on_tap_track_start: Option<Rc<dyn Fn()>>,
    on_tap_track_reset: Option<Rc<dyn Fn()>>,

    // `_TapStatusTrackerMixin`.
    down: Option<RawPointerEvent>,
    up: Option<RawPointerEvent>,
    consecutive_tap_count: u32,
    origin_position: Option<OffsetPair>,
    previous_buttons: Option<u32>,
    /// The consecutive-tap timer's deadline; it stays set after firing until
    /// it is stopped, as Flutter's inactive timer does.
    consecutive_tap_timer: Option<Instant>,
    last_tap_offset: Option<Offset>,

    // `BaseTapAndDragGestureRecognizer`.
    past_slop_tolerance: bool,
    sent_tap_down: bool,
    won_arena_for_primary_pointer: bool,
    primary_pointer: Option<u64>,
    deadline_timer: Option<Instant>,
    drag_state: DragState,
    start: Option<RawPointerEvent>,
    initial_position: OffsetPair,
    current_position: OffsetPair,
    global_distance_moved: f32,
    global_distance_moved_all_axes: f32,
    last_drag_update_details: Option<TapDragUpdateDetails>,
    drag_update_throttle_timer: Option<Instant>,
    accepted_active_pointers: Vec<u64>,

    // `OneSequenceGestureRecognizer`.
    tracked: Vec<u64>,
    entries: Vec<u64>,
    kinds: HashMap<u64, PointerDeviceKind>,
    link: ArenaLink,
}

impl TapAndDragGestureRecognizer {
    fn with_axis(axis: DragAxis) -> Self {
        Self {
            axis,
            supported_devices: None,
            drag_start_behavior: DragStartBehavior::Start,
            drag_update_throttle_frequency: None,
            max_consecutive_tap: None,
            eager_victory_on_drag: true,
            local_transform: None,
            on_tap_down: None,
            on_tap_up: None,
            on_drag_start: None,
            on_drag_update: None,
            on_drag_end: None,
            on_cancel: None,
            on_tap_track_start: None,
            on_tap_track_reset: None,
            down: None,
            up: None,
            consecutive_tap_count: 0,
            origin_position: None,
            previous_buttons: None,
            consecutive_tap_timer: None,
            last_tap_offset: None,
            past_slop_tolerance: false,
            sent_tap_down: false,
            won_arena_for_primary_pointer: false,
            primary_pointer: None,
            deadline_timer: None,
            drag_state: DragState::Ready,
            start: None,
            initial_position: OffsetPair {
                global: Offset::ZERO,
                local: Offset::ZERO,
            },
            current_position: OffsetPair {
                global: Offset::ZERO,
                local: Offset::ZERO,
            },
            global_distance_moved: 0.0,
            global_distance_moved_all_axes: 0.0,
            last_drag_update_details: None,
            drag_update_throttle_timer: None,
            accepted_active_pointers: Vec::new(),
            tracked: Vec::new(),
            entries: Vec::new(),
            kinds: HashMap::new(),
            link: ArenaLink::default(),
        }
    }

    /// Flutter `TapAndHorizontalDragGestureRecognizer`: a drag starts once
    /// the horizontal travel passes the hit slop.
    #[must_use]
    pub fn horizontal_drag() -> Self {
        Self::with_axis(DragAxis::Horizontal)
    }

    /// Flutter `TapAndPanGestureRecognizer`: a drag starts once the travel
    /// passes the pan slop.
    #[must_use]
    pub fn pan() -> Self {
        Self::with_axis(DragAxis::Pan)
    }

    /// Restricts the device kinds this recognizer accepts. All by default.
    #[must_use]
    pub fn supported_devices(mut self, kinds: impl IntoIterator<Item = PointerDeviceKind>) -> Self {
        self.supported_devices = Some(kinds.into_iter().collect());
        self
    }

    /// Defaults to [`DragStartBehavior::Start`].
    #[must_use]
    pub fn drag_start_behavior(mut self, behavior: DragStartBehavior) -> Self {
        self.drag_start_behavior = behavior;
        self
    }

    /// Coalesces drag updates to at most one per `frequency`. Unthrottled
    /// by default.
    #[must_use]
    pub fn drag_update_throttle_frequency(mut self, frequency: Duration) -> Self {
        self.drag_update_throttle_frequency = Some(frequency);
        self
    }

    /// The tap count at which a series starts over. Unbounded by default.
    #[must_use]
    pub fn max_consecutive_tap(mut self, max: u32) -> Self {
        self.max_consecutive_tap = Some(max);
        self
    }

    /// Whether a drag claims the arena as soon as it is detected. True by
    /// default; otherwise the recognizer waits to be the last one standing.
    #[must_use]
    pub fn eager_victory_on_drag(mut self, eager: bool) -> Self {
        self.eager_victory_on_drag = eager;
        self
    }

    /// Sets the global-to-local transform (Flutter's event transform), with
    /// which local drag deltas are measured in global distance.
    pub fn set_local_transform(&mut self, transform: Transform) {
        self.local_transform = Some(transform);
    }

    #[must_use]
    pub fn on_tap_down(mut self, callback: impl Fn(TapDragDownDetails) + 'static) -> Self {
        self.on_tap_down = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_tap_up(mut self, callback: impl Fn(TapDragUpDetails) + 'static) -> Self {
        self.on_tap_up = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_drag_start(mut self, callback: impl Fn(TapDragStartDetails) + 'static) -> Self {
        self.on_drag_start = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_drag_update(mut self, callback: impl Fn(TapDragUpdateDetails) + 'static) -> Self {
        self.on_drag_update = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_drag_end(mut self, callback: impl Fn(TapDragEndDetails) + 'static) -> Self {
        self.on_drag_end = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_cancel = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_tap_track_start(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_tap_track_start = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_tap_track_reset(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_tap_track_reset = Some(Rc::new(callback));
        self
    }

    /// The number of the tap in the current series; zero when none is
    /// tracked.
    #[must_use]
    pub const fn consecutive_tap_count(&self) -> u32 {
        self.consecutive_tap_count
    }

    /// Flutter `addPointer` for a press.
    pub fn add_pointer(&mut self, event: RawPointerEvent) {
        self.kinds.insert(event.pointer, event.kind);
        if self.is_pointer_allowed(&event) {
            self.add_allowed_pointer(event);
        } else if event.buttons != PRIMARY_POINTER_BUTTON && !self.won_arena_for_primary_pointer {
            // `handleNonAllowedPointer`.
            self.resolve(GestureDisposition::Rejected);
        }
    }

    fn is_pointer_allowed(&self, event: &RawPointerEvent) -> bool {
        match self.primary_pointer {
            None => {
                if event.buttons != PRIMARY_POINTER_BUTTON
                    || (self.on_tap_down.is_none()
                        && self.on_drag_start.is_none()
                        && self.on_drag_update.is_none()
                        && self.on_drag_end.is_none()
                        && self.on_tap_up.is_none()
                        && self.on_cancel.is_none())
                {
                    return false;
                }
            }
            Some(primary) => {
                if event.pointer != primary {
                    return false;
                }
            }
        }
        self.supported_devices
            .as_ref()
            .is_none_or(|kinds| kinds.contains(&event.kind))
    }

    fn add_allowed_pointer(&mut self, event: RawPointerEvent) {
        if self.drag_state != DragState::Ready {
            return;
        }
        // `_TapStatusTrackerMixin.addAllowedPointer`, after
        // `OneSequenceGestureRecognizer`'s.
        self.start_tracking_pointer(event.pointer);
        if self
            .consecutive_tap_timer
            .is_some_and(|deadline| deadline <= event.time)
        {
            self.tap_tracker_reset();
        }
        if self.max_consecutive_tap == Some(self.consecutive_tap_count) {
            self.tap_tracker_reset();
        }
        self.up = None;
        if self.down.is_some() && !self.represents_same_series(&event) {
            self.consecutive_tap_count = 1;
        } else {
            self.consecutive_tap_count += 1;
        }
        self.consecutive_tap_timer = None;
        self.down = Some(event);
        self.previous_buttons = Some(event.buttons);
        self.last_tap_offset = Some(event.position);
        self.origin_position = Some(OffsetPair::of(&event));
        if let Some(callback) = &self.on_tap_track_start {
            callback();
        }

        self.primary_pointer = Some(event.pointer);
        self.global_distance_moved = 0.0;
        self.global_distance_moved_all_axes = 0.0;
        self.drag_state = DragState::Possible;
        self.initial_position = OffsetPair::of(&event);
        self.current_position = self.initial_position;
        self.deadline_timer = Some(event.time + PRESS_TIMEOUT);
    }

    fn represents_same_series(&self, event: &RawPointerEvent) -> bool {
        self.consecutive_tap_timer.is_some()
            && self
                .last_tap_offset
                .is_some_and(|last| distance(event.position - last) <= DOUBLE_TAP_SLOP)
            && self.previous_buttons == Some(event.buttons)
    }

    /// Flutter `handleEvent` for a routed event.
    pub fn handle_event(&mut self, event: RawPointerEvent) {
        let event = self.link.with_deltas(event);
        if Some(event.pointer) != self.primary_pointer || !self.tracked.contains(&event.pointer) {
            return;
        }
        self.track_tap_status(&event);
        match event.phase {
            PointerPhase::Move => {
                let slop = compute_hit_slop(event.kind);
                self.past_slop_tolerance = self.past_slop_tolerance
                    || distance(event.position - self.initial_position.global) > slop;
                match self.drag_state {
                    DragState::Accepted => {
                        self.current_position = OffsetPair::of(&event);
                        self.check_drag_update(&event, None);
                    }
                    DragState::Possible => {
                        if self.start.is_none() {
                            self.check_drag(&event);
                        }
                        if let Some(start) = self.start
                            && self.won_arena_for_primary_pointer
                        {
                            self.drag_state = DragState::Accepted;
                            self.accept_drag(&start);
                        }
                    }
                    DragState::Ready => {}
                }
            }
            PointerPhase::Up => match self.drag_state {
                DragState::Possible => self.stop_tracking_pointer(event.pointer),
                DragState::Accepted => self.give_up_pointer(event.pointer),
                DragState::Ready => {}
            },
            PointerPhase::Cancel => {
                self.drag_state = DragState::Ready;
                self.give_up_pointer(event.pointer);
            }
            PointerPhase::Down | PointerPhase::Enter | PointerPhase::Exit => {}
        }
    }

    /// `_TapStatusTrackerMixin.handleEvent`.
    fn track_tap_status(&mut self, event: &RawPointerEvent) {
        match event.phase {
            PointerPhase::Move => {
                let slop = compute_hit_slop(event.kind);
                let past = self
                    .origin_position
                    .is_some_and(|origin| distance(event.position - origin.global) > slop);
                if past {
                    self.consecutive_tap_timer = None;
                    self.previous_buttons = None;
                    self.last_tap_offset = None;
                }
            }
            PointerPhase::Up => {
                self.up = Some(*event);
                if self.down.is_some() {
                    self.consecutive_tap_timer = Some(event.time + DOUBLE_TAP_TIMEOUT);
                }
            }
            PointerPhase::Cancel => self.tap_tracker_reset(),
            PointerPhase::Down | PointerPhase::Enter | PointerPhase::Exit => {}
        }
    }

    /// Flutter `acceptGesture`.
    pub fn accept_gesture(&mut self, pointer: u64) {
        if Some(pointer) != self.primary_pointer {
            return;
        }
        self.deadline_timer = None;
        self.accepted_active_pointers.push(pointer);
        if let Some(down) = self.down {
            self.check_tap_down(&down);
        }
        self.won_arena_for_primary_pointer = true;
        if let Some(start) = self.start {
            if !self.eager_victory_on_drag {
                self.drag_state = DragState::Accepted;
            }
            self.accept_drag(&start);
        }
        if let Some(up) = self.up {
            self.check_tap_up(&up);
        }
    }

    /// Flutter `rejectGesture`.
    pub fn reject_gesture(&mut self, pointer: u64) {
        if Some(pointer) != self.primary_pointer {
            return;
        }
        self.tap_tracker_reset();
        self.deadline_timer = None;
        self.give_up_pointer(pointer);
        self.reset_taps();
        self.reset_drag_update_throttle();
    }

    /// Fires the press deadline, the consecutive-tap timer and the drag
    /// update throttle when due by `now`.
    pub fn poll(&mut self, now: Instant) {
        if self.deadline_timer.is_some_and(|deadline| deadline <= now) {
            self.deadline_timer = None;
            // `_didExceedDeadline`.
            if let Some(down) = self.down {
                self.check_tap_down(&down);
                if self.consecutive_tap_count > 1 {
                    self.resolve(GestureDisposition::Accepted);
                }
            }
        }
        if self
            .drag_update_throttle_timer
            .is_some_and(|deadline| deadline <= now)
        {
            self.handle_drag_update_throttled();
        }
    }

    /// The earliest pending deadline. The consecutive-tap timer only marks
    /// the series stale, which the next press checks, so it is not one.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        [self.deadline_timer, self.drag_update_throttle_timer]
            .into_iter()
            .flatten()
            .min()
    }

    fn handle_drag_update_throttled(&mut self) {
        if let (Some(callback), Some(details)) =
            (&self.on_drag_update, self.last_drag_update_details)
        {
            callback(details);
        }
        self.drag_update_throttle_timer = None;
        self.last_drag_update_details = None;
    }

    /// `didStopTrackingLastPointer`.
    fn did_stop_tracking_last_pointer(&mut self, pointer: u64) {
        match self.drag_state {
            DragState::Ready => {
                self.check_cancel();
                self.resolve(GestureDisposition::Rejected);
            }
            DragState::Possible => {
                if self.past_slop_tolerance {
                    if self.won_arena_for_primary_pointer {
                        if let Some(down) = self.down {
                            if !self.remove_accepted_active_pointer(pointer) {
                                self.resolve_pointer(pointer, GestureDisposition::Rejected);
                            }
                            self.drag_state = DragState::Accepted;
                            self.accept_drag(&down);
                            self.check_drag_end();
                        }
                    } else {
                        self.check_cancel();
                        self.resolve(GestureDisposition::Rejected);
                    }
                } else if let Some(up) = self.up {
                    self.check_tap_up(&up);
                }
            }
            DragState::Accepted => self.check_drag_end(),
        }
        self.deadline_timer = None;
        self.start = None;
        self.drag_state = DragState::Ready;
        self.past_slop_tolerance = false;
    }

    /// Flutter `PointerEvent.transformDeltaViaPositions` from local to
    /// global.
    fn local_to_global_delta(&self, delta: Offset, end: Offset) -> Offset {
        match self.local_transform.and_then(Transform::inverse) {
            Some(inverse) => inverse.transform_point(end) - inverse.transform_point(end - delta),
            None => delta,
        }
    }

    fn delta_for_details(&self, delta: Offset) -> Offset {
        match self.axis {
            DragAxis::Horizontal => Offset::new(delta.x, 0.0),
            DragAxis::Pan => delta,
        }
    }

    fn primary_value(&self, value: Offset) -> Option<f32> {
        match self.axis {
            DragAxis::Horizontal => Some(value.x),
            DragAxis::Pan => None,
        }
    }

    fn has_sufficient_global_distance_to_accept(&self, kind: PointerDeviceKind) -> bool {
        let slop = match self.axis {
            DragAxis::Horizontal => compute_hit_slop(kind),
            DragAxis::Pan => compute_pan_slop(kind),
        };
        self.global_distance_moved.abs() > slop
    }

    fn accept_drag(&mut self, event: &RawPointerEvent) {
        if !self.won_arena_for_primary_pointer {
            return;
        }
        if self.drag_start_behavior == DragStartBehavior::Start {
            self.initial_position = OffsetPair {
                global: self.initial_position.global + event.delta,
                local: self.initial_position.local + event.local_delta,
            };
            self.current_position = self.initial_position;
        }
        self.check_drag_start(event);
        let local_delta = event.local_delta;
        if local_delta != Offset::ZERO {
            self.current_position = OffsetPair::of(event);
            let corrected_local = self.initial_position.local + local_delta;
            let global_delta = self.local_to_global_delta(local_delta, corrected_local);
            let corrected = OffsetPair {
                global: self.initial_position.global + global_delta,
                local: corrected_local,
            };
            self.check_drag_update(event, Some(corrected));
        }
    }

    fn check_drag(&mut self, event: &RawPointerEvent) {
        let moved_locally = self.delta_for_details(event.local_delta);
        let sign = self.primary_value(moved_locally).unwrap_or(1.0);
        // Dart's `double.sign`: zero stays zero.
        let sign = if sign > 0.0 {
            1.0
        } else if sign < 0.0 {
            -1.0
        } else {
            0.0
        };
        self.global_distance_moved +=
            distance(self.local_to_global_delta(moved_locally, event.local_position)) * sign;
        self.global_distance_moved_all_axes +=
            distance(self.local_to_global_delta(event.local_delta, event.local_position));
        if self.has_sufficient_global_distance_to_accept(event.kind)
            || (self.won_arena_for_primary_pointer
                && self.global_distance_moved_all_axes.abs() > compute_pan_slop(event.kind))
        {
            self.start = Some(*event);
            if self.eager_victory_on_drag {
                self.drag_state = DragState::Accepted;
                if !self.won_arena_for_primary_pointer {
                    self.resolve(GestureDisposition::Accepted);
                }
            }
        }
    }

    fn kind_for_pointer(&self, pointer: u64) -> PointerDeviceKind {
        self.kinds
            .get(&pointer)
            .copied()
            .unwrap_or(PointerDeviceKind::Unknown)
    }

    fn check_tap_down(&mut self, event: &RawPointerEvent) {
        if self.sent_tap_down {
            return;
        }
        let details = TapDragDownDetails {
            global_position: event.position,
            local_position: event.local_position,
            kind: self.kind_for_pointer(event.pointer),
            consecutive_tap_count: self.consecutive_tap_count,
        };
        if let Some(callback) = &self.on_tap_down {
            callback(details);
        }
        self.sent_tap_down = true;
    }

    fn check_tap_up(&mut self, event: &RawPointerEvent) {
        if !self.won_arena_for_primary_pointer {
            return;
        }
        let details = TapDragUpDetails {
            global_position: event.position,
            local_position: event.local_position,
            kind: event.kind,
            consecutive_tap_count: self.consecutive_tap_count,
        };
        if let Some(callback) = &self.on_tap_up {
            callback(details);
        }
        self.reset_taps();
        if !self.remove_accepted_active_pointer(event.pointer) {
            self.resolve_pointer(event.pointer, GestureDisposition::Rejected);
        }
    }

    fn check_drag_start(&mut self, event: &RawPointerEvent) {
        if let Some(callback) = &self.on_drag_start {
            callback(TapDragStartDetails {
                global_position: self.initial_position.global,
                local_position: self.initial_position.local,
                source_time: Some(event.time),
                kind: self.kind_for_pointer(event.pointer),
                consecutive_tap_count: self.consecutive_tap_count,
            });
        }
        self.start = None;
    }

    fn check_drag_update(&mut self, event: &RawPointerEvent, corrected: Option<OffsetPair>) {
        let global = corrected.map_or(event.position, |pair| pair.global);
        let local = corrected.map_or(event.local_position, |pair| pair.local);
        let details = TapDragUpdateDetails {
            global_position: global,
            local_position: local,
            source_time: Some(event.time),
            delta: event.local_delta,
            kind: self.kind_for_pointer(event.pointer),
            offset_from_origin: global - self.initial_position.global,
            local_offset_from_origin: local - self.initial_position.local,
            consecutive_tap_count: self.consecutive_tap_count,
        };
        if let Some(frequency) = self.drag_update_throttle_frequency {
            self.last_drag_update_details = Some(details);
            if self.drag_update_throttle_timer.is_none() {
                self.drag_update_throttle_timer = Some(event.time + frequency);
            }
        } else if let Some(callback) = &self.on_drag_update {
            callback(details);
        }
    }

    fn check_drag_end(&mut self) {
        if self.drag_update_throttle_timer.is_some() {
            self.handle_drag_update_throttled();
        }
        let details = TapDragEndDetails {
            global_position: self.current_position.global,
            local_position: self.current_position.local,
            velocity: Velocity::ZERO,
            primary_velocity: Some(0.0),
            consecutive_tap_count: self.consecutive_tap_count,
        };
        if let Some(callback) = &self.on_drag_end {
            callback(details);
        }
        self.reset_taps();
        self.reset_drag_update_throttle();
    }

    fn check_cancel(&mut self) {
        if !self.sent_tap_down {
            return;
        }
        if let Some(callback) = &self.on_cancel {
            callback();
        }
        self.reset_drag_update_throttle();
        self.reset_taps();
    }

    fn give_up_pointer(&mut self, pointer: u64) {
        self.stop_tracking_pointer(pointer);
        if !self.remove_accepted_active_pointer(pointer) {
            self.resolve_pointer(pointer, GestureDisposition::Rejected);
        }
    }

    fn remove_accepted_active_pointer(&mut self, pointer: u64) -> bool {
        match self
            .accepted_active_pointers
            .iter()
            .position(|p| *p == pointer)
        {
            Some(index) => {
                self.accepted_active_pointers.remove(index);
                true
            }
            None => false,
        }
    }

    fn reset_taps(&mut self) {
        self.sent_tap_down = false;
        self.won_arena_for_primary_pointer = false;
        self.primary_pointer = None;
    }

    fn reset_drag_update_throttle(&mut self) {
        if self.drag_update_throttle_frequency.is_none() {
            return;
        }
        self.last_drag_update_details = None;
        self.drag_update_throttle_timer = None;
    }

    fn tap_tracker_reset(&mut self) {
        self.consecutive_tap_timer = None;
        self.previous_buttons = None;
        self.origin_position = None;
        self.last_tap_offset = None;
        self.consecutive_tap_count = 0;
        self.down = None;
        self.up = None;
        if let Some(callback) = &self.on_tap_track_reset {
            callback();
        }
    }

    fn start_tracking_pointer(&mut self, pointer: u64) {
        if !self.tracked.contains(&pointer) {
            self.tracked.push(pointer);
        }
        if !self.entries.contains(&pointer) {
            self.entries.push(pointer);
        }
        self.link.add(pointer);
    }

    fn stop_tracking_pointer(&mut self, pointer: u64) {
        if let Some(index) = self.tracked.iter().position(|p| *p == pointer) {
            self.tracked.remove(index);
            if self.tracked.is_empty() {
                self.did_stop_tracking_last_pointer(pointer);
            }
        }
    }

    /// `OneSequenceGestureRecognizer.resolve`: every tracked pointer.
    fn resolve(&mut self, disposition: GestureDisposition) {
        for pointer in std::mem::take(&mut self.entries) {
            self.settle(pointer, disposition);
        }
    }

    /// `OneSequenceGestureRecognizer.resolvePointer`.
    fn resolve_pointer(&mut self, pointer: u64, disposition: GestureDisposition) {
        if let Some(index) = self.entries.iter().position(|p| *p == pointer) {
            self.entries.remove(index);
            self.settle(pointer, disposition);
        }
    }
}

arena_recognizer!(TapAndDragGestureRecognizer);
