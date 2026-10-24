//! Flutter's `ForcePressGestureRecognizer` (`gestures/force_press.dart`).

use std::{collections::HashMap, rc::Rc};

use incular_core::{Offset, PointerPhase, time::Instant};

use crate::arena_member::{ArenaLink, ArenaMember, arena_recognizer, compute_hit_slop};
use crate::{GestureDisposition, PointerDeviceKind, RawPointerEvent};

/// Flutter `ForcePressDetails`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForcePressDetails {
    pub global_position: Offset,
    pub local_position: Offset,
    /// The interpolated pressure, from 0 to 1.
    pub pressure: f32,
}

/// Flutter `GestureForceInterpolation`: maps `(pressure_min, pressure_max,
/// pressure)` to the 0–1 range the thresholds compare against.
pub type ForceInterpolation = Rc<dyn Fn(f32, f32, f32) -> f32>;

type ForcePressCallback = Rc<dyn Fn(ForcePressDetails)>;

/// Flutter `_ForceState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ForceState {
    Ready,
    Possible,
    Accepted,
    Started,
    Peaked,
}

/// Flutter's `ForcePressGestureRecognizer`: a press whose pressure passes
/// `start_pressure` starts the gesture and one past `peak_pressure` peaks
/// it.
///
/// Incular's native adapters normalize a sensing device's pressure over its
/// own range, so the interpolation receives a minimum of 0 and a maximum
/// of 1; a device without pressure sensing reports none, which Flutter
/// detects as `pressureMax <= 1.0`, and the recognizer stays out of its
/// arena.
pub struct ForcePressGestureRecognizer {
    start_pressure: f32,
    peak_pressure: f32,
    interpolation: ForceInterpolation,
    supported_devices: Option<Vec<PointerDeviceKind>>,
    on_start: Option<ForcePressCallback>,
    on_update: Option<ForcePressCallback>,
    on_peak: Option<ForcePressCallback>,
    on_end: Option<ForcePressCallback>,
    state: ForceState,
    last_position: (Offset, Offset),
    last_pressure: f32,
    tracked: Vec<u64>,
    entries: Vec<u64>,
    kinds: HashMap<u64, PointerDeviceKind>,
    link: ArenaLink,
}

impl Default for ForcePressGestureRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl ForcePressGestureRecognizer {
    /// Starts at pressure 0.4 and peaks at 0.85, interpolating linearly.
    #[must_use]
    pub fn new() -> Self {
        Self {
            start_pressure: 0.4,
            peak_pressure: 0.85,
            interpolation: Rc::new(Self::inverse_lerp),
            supported_devices: None,
            on_start: None,
            on_update: None,
            on_peak: None,
            on_end: None,
            state: ForceState::Ready,
            last_position: (Offset::ZERO, Offset::ZERO),
            last_pressure: 0.0,
            tracked: Vec::new(),
            entries: Vec::new(),
            kinds: HashMap::new(),
            link: ArenaLink::default(),
        }
    }

    /// Sets the start and peak thresholds; `peak` must exceed `start`.
    #[must_use]
    pub fn pressures(mut self, start: f32, peak: f32) -> Self {
        assert!(peak > start, "the peak pressure exceeds the start pressure");
        self.start_pressure = start;
        self.peak_pressure = peak;
        self
    }

    #[must_use]
    pub fn interpolation(mut self, interpolation: impl Fn(f32, f32, f32) -> f32 + 'static) -> Self {
        self.interpolation = Rc::new(interpolation);
        self
    }

    /// Restricts the device kinds this recognizer accepts. All by default.
    #[must_use]
    pub fn supported_devices(mut self, kinds: impl IntoIterator<Item = PointerDeviceKind>) -> Self {
        self.supported_devices = Some(kinds.into_iter().collect());
        self
    }

    #[must_use]
    pub fn on_start(mut self, callback: impl Fn(ForcePressDetails) + 'static) -> Self {
        self.on_start = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_update(mut self, callback: impl Fn(ForcePressDetails) + 'static) -> Self {
        self.on_update = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_peak(mut self, callback: impl Fn(ForcePressDetails) + 'static) -> Self {
        self.on_peak = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_end(mut self, callback: impl Fn(ForcePressDetails) + 'static) -> Self {
        self.on_end = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub const fn start_pressure(&self) -> f32 {
        self.start_pressure
    }

    #[must_use]
    pub const fn peak_pressure(&self) -> f32 {
        self.peak_pressure
    }

    /// Flutter `_inverseLerp`, clamped to 0–1 unless it is NaN.
    #[must_use]
    pub fn inverse_lerp(min: f32, max: f32, t: f32) -> f32 {
        let value = (t - min) / (max - min);
        if value.is_nan() {
            value
        } else {
            value.clamp(0.0, 1.0)
        }
    }

    /// Flutter `addPointer` for a press.
    pub fn add_pointer(&mut self, event: RawPointerEvent) {
        self.kinds.insert(event.pointer, event.kind);
        let allowed = self
            .supported_devices
            .as_ref()
            .is_none_or(|kinds| kinds.contains(&event.kind));
        if !allowed {
            return;
        }
        // `addAllowedPointer`: a device without pressure sensing does not
        // take part.
        if event.sample.pressure.is_none() {
            self.resolve(GestureDisposition::Rejected);
            return;
        }
        self.start_tracking_pointer(event.pointer);
        if self.state == ForceState::Ready {
            self.state = ForceState::Possible;
            self.last_position = (event.position, event.local_position);
        }
    }

    /// Flutter `handleEvent` for a routed event.
    pub fn handle_event(&mut self, event: RawPointerEvent) {
        let event = self.link.with_deltas(event);
        if !self.tracked.contains(&event.pointer) {
            return;
        }
        if matches!(event.phase, PointerPhase::Move | PointerPhase::Down) {
            let pressure = (self.interpolation)(
                0.0,
                1.0,
                event
                    .sample
                    .pressure
                    .map_or(f32::NAN, |pressure| pressure.get()),
            );
            self.last_position = (event.position, event.local_position);
            self.last_pressure = pressure;
            if self.state == ForceState::Possible {
                if pressure > self.start_pressure {
                    self.state = ForceState::Started;
                    self.resolve(GestureDisposition::Accepted);
                } else if event.delta.x * event.delta.x + event.delta.y * event.delta.y
                    > compute_hit_slop(event.kind)
                {
                    // Flutter compares the squared delta with the slop.
                    self.resolve(GestureDisposition::Rejected);
                }
            }
            if pressure > self.start_pressure && self.state == ForceState::Accepted {
                self.state = ForceState::Started;
                if let Some(callback) = &self.on_start {
                    callback(ForcePressDetails {
                        global_position: self.last_position.0,
                        local_position: self.last_position.1,
                        pressure,
                    });
                }
            }
            let details = ForcePressDetails {
                global_position: event.position,
                local_position: event.local_position,
                pressure,
            };
            if let Some(callback) = &self.on_peak
                && pressure > self.peak_pressure
                && self.state == ForceState::Started
            {
                self.state = ForceState::Peaked;
                callback(details);
            }
            if let Some(callback) = &self.on_update
                && !pressure.is_nan()
                && matches!(self.state, ForceState::Started | ForceState::Peaked)
            {
                callback(details);
            }
        }
        if matches!(event.phase, PointerPhase::Up | PointerPhase::Cancel) {
            self.stop_tracking_pointer(event.pointer);
        }
    }

    /// Flutter `acceptGesture`.
    pub fn accept_gesture(&mut self, _pointer: u64) {
        if self.state == ForceState::Possible {
            self.state = ForceState::Accepted;
        }
        if let Some(callback) = &self.on_start
            && self.state == ForceState::Started
        {
            callback(ForcePressDetails {
                global_position: self.last_position.0,
                local_position: self.last_position.1,
                pressure: self.last_pressure,
            });
        }
    }

    /// Flutter `rejectGesture`.
    pub fn reject_gesture(&mut self, pointer: u64) {
        self.stop_tracking_pointer(pointer);
        self.did_stop_tracking_last_pointer();
    }

    /// The recognizer has no timers.
    pub fn poll(&mut self, _now: Instant) {}

    /// The recognizer has no timers.
    #[must_use]
    pub const fn next_deadline(&self) -> Option<Instant> {
        None
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
                self.did_stop_tracking_last_pointer();
            }
        }
    }

    /// `OneSequenceGestureRecognizer.resolve`: every tracked pointer.
    fn resolve(&mut self, disposition: GestureDisposition) {
        for pointer in std::mem::take(&mut self.entries) {
            self.settle(pointer, disposition);
        }
    }

    fn did_stop_tracking_last_pointer(&mut self) {
        let was_accepted = matches!(self.state, ForceState::Started | ForceState::Peaked);
        if self.state == ForceState::Possible {
            self.resolve(GestureDisposition::Rejected);
            return;
        }
        if was_accepted && let Some(callback) = &self.on_end {
            callback(ForcePressDetails {
                global_position: self.last_position.0,
                local_position: self.last_position.1,
                pressure: 0.0,
            });
        }
        self.state = ForceState::Ready;
    }
}

arena_recognizer!(ForcePressGestureRecognizer);
