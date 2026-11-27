//! Flutter's `ForcePressGestureRecognizer` (`gestures/force_press.dart`).

use std::rc::Rc;

use incular_core::{Offset, PointerPhase, PointerSampleMetadata, time::Instant};

use crate::arena_member::{ArenaLink, ArenaMember, arena_recognizer};
use crate::slop::compute_hit_slop;
use crate::{GestureDisposition, PointerDeviceKind, RawPointerEvent};
use incular_core::DeviceGestureSettings;

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

/// An unnormalized pressure sample for a force recognizer fed directly by
/// a device. Keep the raw value and range so custom interpolators can use
/// device units. A backend can additionally supply normalized force for
/// the default interpolation, independently of this calibration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForcePressureSample {
    pub min: f32,
    pub max: f32,
    pub value: f32,
}

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
/// A custom interpolator receives device units when calibration is
/// available. The default path preserves a backend's normalized force
/// (which may include angle correction) and falls back to raw calibration.
/// A device with neither is excluded from the arena. Hosts can also use
/// [`Self::handle_device_sample`] or its arena-managed counterparts.
pub struct ForcePressGestureRecognizer {
    start_pressure: f32,
    peak_pressure: f32,
    interpolation: ForceInterpolation,
    custom_interpolation: bool,
    supported_devices: Option<Vec<PointerDeviceKind>>,
    /// Flutter `GestureRecognizer.gestureSettings`: platform slop overrides.
    gesture_settings: DeviceGestureSettings,
    on_start: Option<ForcePressCallback>,
    on_update: Option<ForcePressCallback>,
    on_peak: Option<ForcePressCallback>,
    on_end: Option<ForcePressCallback>,
    state: ForceState,
    last_position: (Offset, Offset),
    last_pressure: f32,
    /// Whether the arena accepted this recognizer for the current press.
    accepted: bool,
    /// The event that started the press while its acceptance was still
    /// pending in an external arena: its peak and update follow `onStart`.
    deferred: Option<ForcePressDetails>,
    tracked: Vec<u64>,
    entries: Vec<u64>,
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
            custom_interpolation: false,
            supported_devices: None,
            gesture_settings: DeviceGestureSettings::default(),
            on_start: None,
            on_update: None,
            on_peak: None,
            on_end: None,
            state: ForceState::Ready,
            last_position: (Offset::ZERO, Offset::ZERO),
            last_pressure: 0.0,
            accepted: false,
            deferred: None,
            tracked: Vec::new(),
            entries: Vec::new(),
            link: ArenaLink::default(),
        }
    }

    /// Sets the start and peak thresholds; `peak` must exceed `start`.
    ///
    /// # Panics
    ///
    /// Panics if `peak` does not exceed `start`; use [`Self::maybe_pressures`]
    /// for untrusted input.
    #[must_use]
    pub fn pressures(self, start: f32, peak: f32) -> Self {
        self.maybe_pressures(start, peak)
            .expect("peak pressure exceeds start pressure")
    }

    /// [`Self::pressures`] returning `None` instead of panicking when `peak`
    /// does not exceed `start`.
    #[must_use]
    pub fn maybe_pressures(mut self, start: f32, peak: f32) -> Option<Self> {
        // Written positively so NaN is rejected too.
        if peak > start {
            self.start_pressure = start;
            self.peak_pressure = peak;
            Some(self)
        } else {
            None
        }
    }

    #[must_use]
    pub fn interpolation(mut self, interpolation: impl Fn(f32, f32, f32) -> f32 + 'static) -> Self {
        self.interpolation = Rc::new(interpolation);
        self.custom_interpolation = true;
        self
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

    /// Whether a platform sample can participate in force recognition.
    #[must_use]
    pub fn supports_pressure_sample(sample: PointerSampleMetadata) -> bool {
        sample.pressure.is_some() || calibrated_pressure(sample).is_some()
    }

    /// Flutter `addPointer` for a press.
    pub fn add_pointer(&mut self, event: RawPointerEvent) {
        self.add_pressure_pointer(event, Self::supports_pressure_sample(event.sample));
    }

    /// Adds a press with the physical device's raw range. A raw maximum
    /// of 1 or less is Flutter's marker for a device without force sensing.
    pub fn add_pointer_with_device_pressure(
        &mut self,
        event: RawPointerEvent,
        sample: ForcePressureSample,
    ) {
        self.add_pressure_pointer(event, sample.max > 1.0 || sample.max.is_nan());
    }

    fn add_pressure_pointer(&mut self, event: RawPointerEvent, senses_pressure: bool) {
        let allowed = self
            .supported_devices
            .as_ref()
            .is_none_or(|kinds| kinds.contains(&event.kind));
        if !allowed {
            return;
        }
        // `addAllowedPointer`: a device without pressure sensing does not
        // take part.
        if !senses_pressure {
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
        let physical = calibrated_pressure(event.sample);
        let portable = event.sample.pressure.map(|pressure| ForcePressureSample {
            min: 0.0,
            max: 1.0,
            value: pressure.get(),
        });
        let pressure = if self.custom_interpolation {
            physical.or(portable)
        } else {
            portable.or(physical)
        };
        self.handle_pressure_event(
            event,
            pressure.unwrap_or(ForcePressureSample {
                min: 0.0,
                max: 1.0,
                value: f32::NAN,
            }),
        );
    }

    /// Routes an event with raw device pressure in an external arena.
    /// The interpolator receives exactly this sample's range and value.
    pub fn handle_event_with_device_pressure(
        &mut self,
        event: RawPointerEvent,
        sample: ForcePressureSample,
    ) {
        self.handle_pressure_event(event, sample);
    }

    /// Drives a raw device sample with this recognizer alone in the arena.
    pub fn handle_device_sample(&mut self, event: RawPointerEvent, sample: ForcePressureSample) {
        self.drive_with(
            event,
            |this, event| this.add_pointer_with_device_pressure(event, sample),
            |this, event| this.handle_event_with_device_pressure(event, sample),
        );
    }

    fn handle_pressure_event(&mut self, event: RawPointerEvent, sample: ForcePressureSample) {
        let event = self.link.with_deltas(event);
        if !self.tracked.contains(&event.pointer) {
            return;
        }
        if matches!(event.phase, PointerPhase::Move | PointerPhase::Down) {
            let pressure = (self.interpolation)(sample.min, sample.max, sample.value);
            self.last_position = (event.position, event.local_position);
            self.last_pressure = pressure;
            if self.state == ForceState::Possible {
                if pressure > self.start_pressure {
                    self.state = ForceState::Started;
                    self.resolve(GestureDisposition::Accepted);
                } else if event.delta.x * event.delta.x + event.delta.y * event.delta.y
                    > compute_hit_slop(event.kind, self.gesture_settings)
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
            if self.state == ForceState::Started
                && !self.accepted
                && event.phase != PointerPhase::Down
            {
                // After the press, Flutter's closed arena accepts inside
                // `resolve`, so `onStart` precedes this event's peak and
                // update. An external arena decides later: replay them after
                // the acceptance. (On the press the arena is still open, and
                // Flutter reports them before `onStart`, as here.)
                self.deferred = Some(details);
            } else {
                self.report_peak_and_update(details);
            }
        }
        if matches!(event.phase, PointerPhase::Up | PointerPhase::Cancel) {
            self.stop_tracking_pointer(event.pointer);
        }
    }

    /// Flutter `acceptGesture`.
    pub fn accept_gesture(&mut self, _pointer: u64) {
        self.accepted = true;
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
        if let Some(details) = self.deferred.take() {
            self.report_peak_and_update(details);
        }
    }

    /// Flutter `rejectGesture`.
    pub fn reject_gesture(&mut self, pointer: u64) {
        self.deferred = None;
        self.stop_tracking_pointer(pointer);
        self.did_stop_tracking_last_pointer();
    }

    /// Flutter `handleEvent`'s `onForcePressPeak` and `onForcePressUpdate`
    /// for one event of a started press.
    fn report_peak_and_update(&mut self, details: ForcePressDetails) {
        let pressure = details.pressure;
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
        // Settling never pushes to `entries`, so the buffer can be reused.
        let mut entries = std::mem::take(&mut self.entries);
        for &pointer in &entries {
            self.settle(pointer, disposition);
        }
        entries.clear();
        self.entries = entries;
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
        self.accepted = false;
        self.deferred = None;
    }
}

fn calibrated_pressure(sample: PointerSampleMetadata) -> Option<ForcePressureSample> {
    let contact = sample.contact;
    let value = contact.raw_pressure?;
    (contact.pressure_max.is_finite()
        && contact.pressure_min.is_finite()
        && contact.pressure_max > contact.pressure_min.max(1.0))
    .then_some(ForcePressureSample {
        min: contact.pressure_min,
        max: contact.pressure_max,
        value,
    })
}

arena_recognizer!(ForcePressGestureRecognizer);
