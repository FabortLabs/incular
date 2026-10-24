//! Flutter's `MultiTapGestureRecognizer` and `SerialTapGestureRecognizer`
//! (`gestures/multitap.dart`).

use std::{collections::BTreeMap, rc::Rc, time::Duration};

use incular_core::{Offset, PointerPhase, time::Instant};

use crate::arena_member::{ArenaLink, ArenaMember, arena_recognizer, compute_hit_slop, distance};
use crate::constants::{
    DOUBLE_TAP_MIN_TIME, DOUBLE_TAP_SLOP, DOUBLE_TAP_TIMEOUT, DOUBLE_TAP_TOUCH_SLOP,
};
use crate::{GestureDisposition, PointerDeviceKind, RawPointerEvent, TapDownDetails, TapUpDetails};

fn device_allowed(supported: Option<&Vec<PointerDeviceKind>>, kind: PointerDeviceKind) -> bool {
    supported.is_none_or(|kinds| kinds.contains(&kind))
}

/// Flutter `_TapGesture`: one pointer's tap.
struct TapGesture {
    initial: Offset,
    kind: PointerDeviceKind,
    tracking: bool,
    won_arena: bool,
    /// The long-tap timer's deadline.
    long_tap: Option<Instant>,
    last: (Offset, Offset),
    last_position_final: Option<(Offset, Offset)>,
}

type MultiTapDownCallback = Rc<dyn Fn(u64, TapDownDetails)>;
type MultiTapUpCallback = Rc<dyn Fn(u64, TapUpDetails)>;
type MultiTapCallback = Rc<dyn Fn(u64)>;

/// Flutter's `MultiTapGestureRecognizer`: every pointer taps on its own, each
/// in its own arena.
pub struct MultiTapGestureRecognizer {
    long_tap_delay: Duration,
    supported_devices: Option<Vec<PointerDeviceKind>>,
    on_tap_down: Option<MultiTapDownCallback>,
    on_tap_up: Option<MultiTapUpCallback>,
    on_tap: Option<MultiTapCallback>,
    on_tap_cancel: Option<MultiTapCallback>,
    on_long_tap_down: Option<MultiTapDownCallback>,
    gestures: BTreeMap<u64, TapGesture>,
    link: ArenaLink,
}

impl Default for MultiTapGestureRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiTapGestureRecognizer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            long_tap_delay: Duration::ZERO,
            supported_devices: None,
            on_tap_down: None,
            on_tap_up: None,
            on_tap: None,
            on_tap_cancel: None,
            on_long_tap_down: None,
            gestures: BTreeMap::new(),
            link: ArenaLink::default(),
        }
    }

    /// How long a pointer stays down before `on_long_tap_down`. Zero, the
    /// default, never reports a long tap.
    #[must_use]
    pub fn long_tap_delay(mut self, delay: Duration) -> Self {
        self.long_tap_delay = delay;
        self
    }

    /// Restricts the device kinds this recognizer accepts. All by default.
    #[must_use]
    pub fn supported_devices(mut self, kinds: impl IntoIterator<Item = PointerDeviceKind>) -> Self {
        self.supported_devices = Some(kinds.into_iter().collect());
        self
    }

    #[must_use]
    pub fn on_tap_down(mut self, callback: impl Fn(u64, TapDownDetails) + 'static) -> Self {
        self.on_tap_down = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_tap_up(mut self, callback: impl Fn(u64, TapUpDetails) + 'static) -> Self {
        self.on_tap_up = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_tap(mut self, callback: impl Fn(u64) + 'static) -> Self {
        self.on_tap = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_tap_cancel(mut self, callback: impl Fn(u64) + 'static) -> Self {
        self.on_tap_cancel = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_long_tap_down(mut self, callback: impl Fn(u64, TapDownDetails) + 'static) -> Self {
        self.on_long_tap_down = Some(Rc::new(callback));
        self
    }

    /// Flutter `addPointer` for a press.
    pub fn add_pointer(&mut self, event: RawPointerEvent) {
        if !device_allowed(self.supported_devices.as_ref(), event.kind) {
            return;
        }
        self.link.add(event.pointer);
        let long_tap =
            (self.long_tap_delay > Duration::ZERO).then(|| event.time + self.long_tap_delay);
        self.gestures.insert(
            event.pointer,
            TapGesture {
                initial: event.position,
                kind: event.kind,
                tracking: true,
                won_arena: false,
                long_tap,
                last: (event.position, event.local_position),
                last_position_final: None,
            },
        );
        if let Some(callback) = &self.on_tap_down {
            callback(
                event.pointer,
                TapDownDetails {
                    global_position: event.position,
                    local_position: event.local_position,
                    kind: event.kind,
                },
            );
        }
    }

    /// `_TapGesture.handleEvent` for a routed event.
    pub fn handle_event(&mut self, event: RawPointerEvent) {
        let event = self.link.with_deltas(event);
        let pointer = event.pointer;
        let Some(gesture) = self.gestures.get_mut(&pointer) else {
            return;
        };
        if !gesture.tracking {
            return;
        }
        match event.phase {
            PointerPhase::Move => {
                if distance(event.position - gesture.initial) > compute_hit_slop(event.kind) {
                    self.cancel(pointer);
                } else {
                    gesture.last = (event.position, event.local_position);
                }
            }
            PointerPhase::Cancel => self.cancel(pointer),
            PointerPhase::Up => {
                gesture.tracking = false;
                gesture.long_tap = None;
                gesture.last_position_final = Some((event.position, event.local_position));
                self.check(pointer);
            }
            PointerPhase::Down | PointerPhase::Enter | PointerPhase::Exit => {}
        }
    }

    /// Flutter `acceptGesture`.
    pub fn accept_gesture(&mut self, pointer: u64) {
        if let Some(gesture) = self.gestures.get_mut(&pointer) {
            gesture.won_arena = true;
            self.check(pointer);
        }
    }

    /// Flutter `rejectGesture`.
    pub fn reject_gesture(&mut self, pointer: u64) {
        let Some(gesture) = self.gestures.get_mut(&pointer) else {
            return;
        };
        gesture.tracking = false;
        gesture.long_tap = None;
        self.gestures.remove(&pointer);
        if let Some(callback) = &self.on_tap_cancel {
            callback(pointer);
        }
    }

    /// Fires the long-tap timers due by `now`.
    pub fn poll(&mut self, now: Instant) {
        let due: Vec<u64> = self
            .gestures
            .iter()
            .filter(|(_, gesture)| gesture.long_tap.is_some_and(|deadline| deadline <= now))
            .map(|(pointer, _)| *pointer)
            .collect();
        for pointer in due {
            let Some(gesture) = self.gestures.get_mut(&pointer) else {
                continue;
            };
            gesture.long_tap = None;
            let details = TapDownDetails {
                global_position: gesture.last.0,
                local_position: gesture.last.1,
                kind: gesture.kind,
            };
            if let Some(callback) = &self.on_long_tap_down {
                callback(pointer, details);
            }
        }
    }

    /// The earliest long-tap deadline still pending.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        self.gestures
            .values()
            .filter_map(|gesture| gesture.long_tap)
            .min()
    }

    /// `_TapGesture.cancel`.
    fn cancel(&mut self, pointer: u64) {
        let Some(gesture) = self.gestures.get(&pointer) else {
            return;
        };
        if gesture.won_arena {
            self.reject_gesture(pointer);
        } else {
            self.settle(pointer, GestureDisposition::Rejected);
        }
    }

    /// `_TapGesture._check`.
    fn check(&mut self, pointer: u64) {
        let Some(gesture) = self.gestures.get(&pointer) else {
            return;
        };
        let (true, Some((global, local))) = (gesture.won_arena, gesture.last_position_final) else {
            return;
        };
        let kind = gesture.kind;
        self.gestures.remove(&pointer);
        if let Some(callback) = &self.on_tap_up {
            callback(
                pointer,
                TapUpDetails {
                    global_position: global,
                    local_position: local,
                    kind,
                },
            );
        }
        if let Some(callback) = &self.on_tap {
            callback(pointer);
        }
    }
}

arena_recognizer!(MultiTapGestureRecognizer);

/// Flutter `SerialTapDownDetails`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SerialTapDownDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    pub kind: PointerDeviceKind,
    pub buttons: u32,
    /// The tap's number in the series, from 1.
    pub count: u32,
}

/// Flutter `SerialTapCancelDetails`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SerialTapCancelDetails {
    pub count: u32,
}

/// Flutter `SerialTapUpDetails`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SerialTapUpDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    pub kind: PointerDeviceKind,
    pub count: u32,
}

/// Flutter `_TapTracker`.
#[derive(Clone, Copy, Debug)]
struct TapTracker {
    pointer: u64,
    initial: Offset,
    buttons: u32,
    down_time: Instant,
    kind: PointerDeviceKind,
}

impl TapTracker {
    /// `hasElapsedMinTime`: the `kDoubleTapMinTime` countdown from the press
    /// has run out by `now`.
    fn has_elapsed_min_time(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.down_time) >= DOUBLE_TAP_MIN_TIME
    }
}

/// Flutter's `SerialTapGestureRecognizer`: reports every tap of a series
/// with its number in the series, each tap winning its own arena on
/// release.
pub struct SerialTapGestureRecognizer {
    supported_devices: Option<Vec<PointerDeviceKind>>,
    on_serial_tap_down: Option<Rc<dyn Fn(SerialTapDownDetails)>>,
    on_serial_tap_cancel: Option<Rc<dyn Fn(SerialTapCancelDetails)>>,
    on_serial_tap_up: Option<Rc<dyn Fn(SerialTapUpDetails)>>,
    /// The serial-tap timer's deadline.
    serial_tap_timer: Option<Instant>,
    completed_taps: Vec<TapTracker>,
    gesture_resolutions: BTreeMap<u64, GestureDisposition>,
    pending_tap: Option<TapTracker>,
    link: ArenaLink,
}

impl Default for SerialTapGestureRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SerialTapGestureRecognizer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            supported_devices: None,
            on_serial_tap_down: None,
            on_serial_tap_cancel: None,
            on_serial_tap_up: None,
            serial_tap_timer: None,
            completed_taps: Vec::new(),
            gesture_resolutions: BTreeMap::new(),
            pending_tap: None,
            link: ArenaLink::default(),
        }
    }

    /// Restricts the device kinds this recognizer accepts. All by default.
    #[must_use]
    pub fn supported_devices(mut self, kinds: impl IntoIterator<Item = PointerDeviceKind>) -> Self {
        self.supported_devices = Some(kinds.into_iter().collect());
        self
    }

    #[must_use]
    pub fn on_serial_tap_down(mut self, callback: impl Fn(SerialTapDownDetails) + 'static) -> Self {
        self.on_serial_tap_down = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_serial_tap_cancel(
        mut self,
        callback: impl Fn(SerialTapCancelDetails) + 'static,
    ) -> Self {
        self.on_serial_tap_cancel = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_serial_tap_up(mut self, callback: impl Fn(SerialTapUpDetails) + 'static) -> Self {
        self.on_serial_tap_up = Some(Rc::new(callback));
        self
    }

    /// Whether a tap is in progress.
    #[must_use]
    pub const fn is_tracking_pointer(&self) -> bool {
        self.pending_tap.is_some()
    }

    /// Flutter `addPointer` for a press.
    pub fn add_pointer(&mut self, event: RawPointerEvent) {
        // `isPointerAllowed`.
        if (self.on_serial_tap_down.is_none()
            && self.on_serial_tap_cancel.is_none()
            && self.on_serial_tap_up.is_none())
            || !device_allowed(self.supported_devices.as_ref(), event.kind)
        {
            return;
        }
        let breaks_series = self
            .completed_taps
            .last()
            .is_some_and(|tap| !Self::represents_same_series(tap, &event));
        if breaks_series || self.pending_tap.is_some() {
            self.reset();
        }
        self.track_tap(event);
    }

    /// `_representsSameSeries`.
    fn represents_same_series(tap: &TapTracker, event: &RawPointerEvent) -> bool {
        tap.has_elapsed_min_time(event.time)
            && tap.buttons == event.buttons
            && distance(event.position - tap.initial) <= DOUBLE_TAP_SLOP
    }

    fn track_tap(&mut self, event: RawPointerEvent) {
        self.serial_tap_timer = None;
        if let Some(callback) = &self.on_serial_tap_down {
            callback(SerialTapDownDetails {
                global_position: event.position,
                local_position: event.local_position,
                kind: event.kind,
                buttons: event.buttons,
                count: self.count(),
            });
        }
        self.link.add(event.pointer);
        self.pending_tap = Some(TapTracker {
            pointer: event.pointer,
            initial: event.position,
            buttons: event.buttons,
            down_time: event.time,
            kind: event.kind,
        });
    }

    /// The number of the tap in progress.
    fn count(&self) -> u32 {
        u32::try_from(self.completed_taps.len()).map_or(u32::MAX, |count| count + 1)
    }

    /// `_handleEvent` for a routed event.
    pub fn handle_event(&mut self, event: RawPointerEvent) {
        let event = self.link.with_deltas(event);
        let Some(tracker) = self.pending_tap else {
            return;
        };
        if tracker.pointer != event.pointer {
            return;
        }
        match event.phase {
            PointerPhase::Up => self.register_tap(event, tracker),
            PointerPhase::Move => {
                if distance(event.position - tracker.initial) > DOUBLE_TAP_TOUCH_SLOP {
                    self.reset();
                }
            }
            PointerPhase::Cancel => self.reset(),
            PointerPhase::Down | PointerPhase::Enter | PointerPhase::Exit => {}
        }
    }

    /// Flutter `acceptGesture`.
    pub fn accept_gesture(&mut self, pointer: u64) {
        self.gesture_resolutions
            .insert(pointer, GestureDisposition::Accepted);
    }

    /// Flutter `rejectGesture`.
    pub fn reject_gesture(&mut self, pointer: u64) {
        self.gesture_resolutions
            .insert(pointer, GestureDisposition::Rejected);
        self.reset();
    }

    /// Fires the serial-tap timer when it is due by `now`: the series ends.
    pub fn poll(&mut self, now: Instant) {
        if self
            .serial_tap_timer
            .is_some_and(|deadline| deadline <= now)
        {
            self.serial_tap_timer = None;
            self.reset();
        }
    }

    #[must_use]
    pub const fn next_deadline(&self) -> Option<Instant> {
        self.serial_tap_timer
    }

    fn reject_pending_tap(&mut self) {
        let Some(tracker) = self.pending_tap.take() else {
            return;
        };
        // The resolution can re-enter `reset`, so the cancel count is taken
        // first.
        let count = self.count();
        if let Some(callback) = &self.on_serial_tap_cancel {
            callback(SerialTapCancelDetails { count });
        }
        if !self.gesture_resolutions.contains_key(&tracker.pointer) {
            self.settle(tracker.pointer, GestureDisposition::Rejected);
        }
    }

    fn reset(&mut self) {
        if self.pending_tap.is_some() {
            self.reject_pending_tap();
        }
        self.pending_tap = None;
        self.completed_taps.clear();
        self.gesture_resolutions.clear();
        self.serial_tap_timer = None;
    }

    fn register_tap(&mut self, event: RawPointerEvent, tracker: TapTracker) {
        if self.serial_tap_timer.is_none() {
            self.serial_tap_timer = Some(event.time + DOUBLE_TAP_TIMEOUT);
        }
        if !self.gesture_resolutions.contains_key(&event.pointer) {
            self.settle(event.pointer, GestureDisposition::Accepted);
        }
        self.pending_tap = None;
        if let Some(callback) = &self.on_serial_tap_up {
            callback(SerialTapUpDetails {
                global_position: event.position,
                local_position: event.local_position,
                kind: tracker.kind,
                count: self.count(),
            });
        }
        self.completed_taps.push(tracker);
    }
}

arena_recognizer!(SerialTapGestureRecognizer);
