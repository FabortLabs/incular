//! Flutter's multi-drag recognizers (`gestures/multidrag.dart`): every
//! pointer drags on its own, each in its own arena.

use std::{collections::BTreeMap, rc::Rc, time::Duration};

use incular_core::{Offset, PRIMARY_POINTER_BUTTON, PointerPhase, time::Instant};

use crate::arena_member::{ArenaLink, ArenaMember, arena_recognizer, distance};
use crate::constants::LONG_PRESS_TIMEOUT;
use crate::slop::compute_hit_slop;
use crate::velocity::VelocityTracker;
use crate::{
    DragEndDetails, DragUpdateDetails, GestureDisposition, PointerDeviceKind, RawPointerEvent,
    Velocity,
};
use incular_core::DeviceGestureSettings;

/// Flutter's `Drag`: the client a multi-drag start returns, which receives
/// that pointer's updates. Every method defaults to doing nothing.
pub trait Drag {
    fn update(&mut self, _details: DragUpdateDetails) {}
    fn end(&mut self, _details: DragEndDetails) {}
    fn cancel(&mut self) {}
}

/// Flutter `GestureMultiDragStartCallback`: the drag client for a pointer
/// that starts dragging at the given position, or `None` to ignore it.
pub type MultiDragStartCallback = Rc<dyn Fn(Offset) -> Option<Box<dyn Drag>>>;

/// Which `MultiDragPointerState` the recognizer creates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MultiDragKind {
    Immediate,
    Horizontal,
    Vertical,
    Delayed(Duration),
}

/// Timer deadline, followed by creation order when deadlines tie.
type DragTimer = (Instant, u64);

/// Flutter `MultiDragPointerState`.
struct PointerState {
    initial_position: Offset,
    kind: PointerDeviceKind,
    velocity: VelocityTracker,
    client: Option<Box<dyn Drag>>,
    pending_delta: Option<Offset>,
    last_pending_event_time: Option<Instant>,
    has_entry: bool,
    /// `_DelayedPointerState._timer`'s deadline.
    timer: Option<DragTimer>,
    /// `_DelayedPointerState._starter`: accepted while the delay runs.
    starter: bool,
}

/// Flutter's `MultiDragGestureRecognizer` family. Construct one with
/// [`Self::immediate`], [`Self::horizontal`], [`Self::vertical`] or
/// [`Self::delayed`].
pub struct MultiDragGestureRecognizer {
    kind: MultiDragKind,
    supported_devices: Option<Vec<PointerDeviceKind>>,
    /// Flutter `GestureRecognizer.gestureSettings`: platform slop overrides.
    gesture_settings: DeviceGestureSettings,
    allowed_buttons: Option<Rc<dyn Fn(u32) -> bool>>,
    on_start: Option<MultiDragStartCallback>,
    pointers: BTreeMap<u64, PointerState>,
    timers: BTreeMap<DragTimer, u64>,
    timer_sequence: u64,
    link: ArenaLink,
}

impl MultiDragGestureRecognizer {
    fn with_kind(kind: MultiDragKind) -> Self {
        Self {
            kind,
            supported_devices: None,
            gesture_settings: DeviceGestureSettings::default(),
            // `_defaultButtonAcceptBehavior`: the primary button only.
            allowed_buttons: None,
            on_start: None,
            pointers: BTreeMap::new(),
            timers: BTreeMap::new(),
            timer_sequence: 0,
            link: ArenaLink::default(),
        }
    }

    /// Flutter `ImmediateMultiDragGestureRecognizer`: a pointer drags once
    /// it moves past the hit slop in any direction.
    #[must_use]
    pub fn immediate() -> Self {
        Self::with_kind(MultiDragKind::Immediate)
    }

    /// Flutter `HorizontalMultiDragGestureRecognizer`.
    #[must_use]
    pub fn horizontal() -> Self {
        Self::with_kind(MultiDragKind::Horizontal)
    }

    /// Flutter `VerticalMultiDragGestureRecognizer`.
    #[must_use]
    pub fn vertical() -> Self {
        Self::with_kind(MultiDragKind::Vertical)
    }

    /// Flutter `DelayedMultiDragGestureRecognizer`: a pointer drags once it
    /// has stayed within the hit slop for `delay`, `kLongPressTimeout` by
    /// default.
    #[must_use]
    pub fn delayed(delay: Option<Duration>) -> Self {
        Self::with_kind(MultiDragKind::Delayed(delay.unwrap_or(LONG_PRESS_TIMEOUT)))
    }

    /// Restricts the device kinds this recognizer accepts. All by default.
    #[must_use]
    pub fn supported_devices(mut self, kinds: impl IntoIterator<Item = PointerDeviceKind>) -> Self {
        self.supported_devices = Some(kinds.into_iter().collect());
        self
    }

    /// Flutter `GestureRecognizer.gestureSettings`: platform slop overrides
    /// for touch-class pointers (see [`crate::compute_hit_slop`]).
    #[must_use]
    pub fn gesture_settings(mut self, settings: DeviceGestureSettings) -> Self {
        self.gesture_settings = settings;
        self
    }

    /// Which pressed-button chords may start a drag. The primary button
    /// alone by default.
    #[must_use]
    pub fn allowed_buttons_filter(mut self, filter: impl Fn(u32) -> bool + 'static) -> Self {
        self.set_allowed_buttons_filter(Some(Rc::new(filter)));
        self
    }

    /// Replaces the button filter. `None` restores the primary-only default.
    pub fn set_allowed_buttons_filter(&mut self, filter: Option<Rc<dyn Fn(u32) -> bool>>) {
        self.allowed_buttons = filter;
    }

    #[must_use]
    pub fn on_start(
        mut self,
        callback: impl Fn(Offset) -> Option<Box<dyn Drag>> + 'static,
    ) -> Self {
        self.on_start = Some(Rc::new(callback));
        self
    }

    /// Flutter `addPointer` for a press.
    pub fn add_pointer(&mut self, event: RawPointerEvent) {
        let allowed = self
            .supported_devices
            .as_ref()
            .is_none_or(|kinds| kinds.contains(&event.kind))
            && self
                .allowed_buttons
                .as_ref()
                .map_or(event.buttons == PRIMARY_POINTER_BUTTON, |filter| {
                    filter(event.buttons)
                });
        if !allowed || self.pointers.contains_key(&event.pointer) {
            return;
        }
        let timer = match self.kind {
            MultiDragKind::Delayed(delay) => {
                let key = (event.time + delay, self.timer_sequence);
                self.timer_sequence += 1;
                self.timers.insert(key, event.pointer);
                Some(key)
            }
            MultiDragKind::Immediate | MultiDragKind::Horizontal | MultiDragKind::Vertical => None,
        };
        self.pointers.insert(
            event.pointer,
            PointerState {
                initial_position: event.position,
                kind: event.kind,
                velocity: VelocityTracker::default(),
                client: None,
                pending_delta: Some(Offset::ZERO),
                last_pending_event_time: None,
                has_entry: true,
                timer,
                starter: false,
            },
        );
        self.link.add(event.pointer);
    }

    /// `_handleEvent` for a routed event.
    pub fn handle_event(&mut self, event: RawPointerEvent) {
        let event = self.link.with_deltas(event);
        let pointer = event.pointer;
        let Some(state) = self.pointers.get_mut(&pointer) else {
            return;
        };
        match event.phase {
            PointerPhase::Move => {
                if !event.synthesized {
                    state.velocity.add_position(event.time, event.position);
                }
                if let Some(client) = state.client.as_mut() {
                    client.update(DragUpdateDetails {
                        source_time: Some(event.time),
                        delta: event.delta,
                        primary_delta: None,
                        global_position: event.position,
                        local_position: event.position,
                        kind: state.kind,
                    });
                } else if let Some(pending) = state.pending_delta {
                    state.pending_delta = Some(pending + event.delta);
                    state.last_pending_event_time = Some(event.time);
                    self.check_for_resolution_after_move(pointer);
                }
            }
            PointerPhase::Up => {
                if let Some(mut client) = state.client.take() {
                    let velocity = state.velocity.estimate_at(event.time).pixels_per_second;
                    client.end(DragEndDetails {
                        velocity: Velocity::new(velocity),
                        primary_velocity: None,
                        global_position: Offset::ZERO,
                        local_position: Offset::ZERO,
                    });
                } else {
                    state.pending_delta = None;
                    state.last_pending_event_time = None;
                }
                self.remove_state(pointer);
            }
            PointerPhase::Cancel => {
                if let Some(mut client) = state.client.take() {
                    client.cancel();
                } else {
                    state.pending_delta = None;
                    state.last_pending_event_time = None;
                }
                self.remove_state(pointer);
            }
            PointerPhase::Down | PointerPhase::Enter | PointerPhase::Exit => {}
        }
    }

    /// Flutter `acceptGesture`.
    pub fn accept_gesture(&mut self, pointer: u64) {
        let Some(state) = self.pointers.get_mut(&pointer) else {
            // An up can come before the accept.
            return;
        };
        let initial = state.initial_position;
        match self.kind {
            MultiDragKind::Delayed(_) if state.timer.is_some() => state.starter = true,
            _ => self.start_drag(initial, pointer),
        }
    }

    /// Flutter `rejectGesture`.
    pub fn reject_gesture(&mut self, pointer: u64) {
        if let Some(state) = self.pointers.get_mut(&pointer) {
            state.pending_delta = None;
            state.last_pending_event_time = None;
            state.has_entry = false;
            self.remove_state(pointer);
        }
    }

    /// Fires the delayed recognizer's timers due by `now`.
    pub fn poll(&mut self, now: Instant) {
        while self.next_deadline().is_some_and(|deadline| deadline <= now) {
            let (_, pointer) = self.timers.pop_first().expect("there is a due timer");
            // `_delayPassed`.
            let state = self
                .pointers
                .get_mut(&pointer)
                .expect("the due pointer is tracked");
            state.timer = None;
            if std::mem::take(&mut state.starter) {
                let initial = state.initial_position;
                self.start_drag(initial, pointer);
            } else {
                self.settle(pointer, GestureDisposition::Accepted);
            }
        }
    }

    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        self.timers
            .first_key_value()
            .map(|(&(deadline, _), _)| deadline)
    }

    /// `checkForResolutionAfterMove` of each pointer state.
    fn check_for_resolution_after_move(&mut self, pointer: u64) {
        let Some(state) = self.pointers.get_mut(&pointer) else {
            return;
        };
        let Some(pending) = state.pending_delta else {
            return;
        };
        let slop = compute_hit_slop(state.kind, self.gesture_settings);
        match self.kind {
            MultiDragKind::Immediate => {
                if distance(pending) > slop {
                    self.settle(pointer, GestureDisposition::Accepted);
                }
            }
            MultiDragKind::Horizontal => {
                if pending.x.abs() > slop {
                    self.settle(pointer, GestureDisposition::Accepted);
                }
            }
            MultiDragKind::Vertical => {
                if pending.y.abs() > slop {
                    self.settle(pointer, GestureDisposition::Accepted);
                }
            }
            MultiDragKind::Delayed(_) => {
                // Accepted, but moved too far before the delay: the drag
                // waits forever to start.
                if state.timer.is_none() {
                    return;
                }
                if distance(pending) > slop {
                    if let Some(timer) = state.timer.take() {
                        self.timers.remove(&timer);
                    }
                    self.settle(pointer, GestureDisposition::Rejected);
                }
            }
        }
    }

    /// `_startDrag`.
    fn start_drag(&mut self, initial: Offset, pointer: u64) {
        let drag = self
            .on_start
            .as_ref()
            .and_then(|on_start| on_start(initial));
        let Some(mut drag) = drag else {
            self.remove_state(pointer);
            return;
        };
        let Some(state) = self.pointers.get_mut(&pointer) else {
            return;
        };
        let details = DragUpdateDetails {
            source_time: state.last_pending_event_time,
            delta: state.pending_delta.unwrap_or(Offset::ZERO),
            primary_delta: None,
            global_position: state.initial_position,
            local_position: state.initial_position,
            kind: state.kind,
        };
        state.pending_delta = None;
        state.last_pending_event_time = None;
        drag.update(details);
        state.client = Some(drag);
    }

    /// `_removeState`: the state is disposed, rejecting its arena entry if
    /// it is still open.
    fn remove_state(&mut self, pointer: u64) {
        let Some(state) = self.pointers.remove(&pointer) else {
            return;
        };
        if let Some(timer) = state.timer {
            self.timers.remove(&timer);
        }
        if state.has_entry {
            self.settle(pointer, GestureDisposition::Rejected);
        }
    }
}

arena_recognizer!(MultiDragGestureRecognizer);
