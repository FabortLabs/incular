use std::rc::Rc;

use incular_core::time::Instant;
use incular_core::{DeviceGestureSettings, DragStartBehavior, Offset, PointerPhase, Transform};

use crate::constants::{MAX_FLING_VELOCITY, MIN_FLING_VELOCITY};
use crate::details::{PointerDeviceKind, PointerEvent, RawPointerEvent, Velocity};
use crate::slop::{compute_pan_slop, compute_scale_slop};
use crate::velocity::VelocityTracker;

/// Metadata for scale start events (Flutter `ScaleStartDetails`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleStartDetails {
    pub focal_point: Offset,
    /// [`Self::focal_point`] in the recognizing widget's coordinates.
    pub local_focal_point: Offset,
    pub pointer_count: usize,
    /// Timestamp of the event that started the scale (`sourceTimeStamp`).
    pub source_time: Option<Instant>,
    /// The device kind of the first contact.
    pub kind: Option<PointerDeviceKind>,
}

/// The current geometry of a scale interaction (Flutter
/// `ScaleUpdateDetails`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleUpdateDetails {
    pub focal_point: Offset,
    /// [`Self::focal_point`] in the recognizing widget's coordinates.
    pub local_focal_point: Offset,
    /// The span relative to the start: the mean distance of the contacts
    /// from their focal point.
    pub scale: f32,
    /// [`Self::scale`] from the horizontal distances alone.
    pub horizontal_scale: f32,
    /// [`Self::scale`] from the vertical distances alone.
    pub vertical_scale: f32,
    /// Clockwise rotation in radians of the line between the first two
    /// contacts since the start.
    pub rotation: f32,
    /// Focal point movement in the recognizer's local coordinates.
    pub focal_point_delta: Offset,
    pub pointer_count: usize,
    /// Timestamp of the event (`sourceTimeStamp`).
    pub source_time: Option<Instant>,
}

/// Metadata for scale end events (Flutter `ScaleEndDetails`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleEndDetails {
    /// The velocity of the contact that lifted, when it was a fling.
    pub velocity: Velocity,
    /// The rate the scale was changing at, per second; `-1.0` when unknown.
    pub scale_velocity: f32,
    pub pointer_count: usize,
}

impl ScaleUpdateDetails {
    #[must_use]
    pub const fn new(focal_point: Offset, scale: f32) -> Self {
        Self {
            focal_point,
            local_focal_point: focal_point,
            scale,
            horizontal_scale: scale,
            vertical_scale: scale,
            rotation: 0.0,
            focal_point_delta: Offset::ZERO,
            pointer_count: 2,
            source_time: None,
        }
    }
}

/// Pointer IDs are unique within a device at Incular ingress, not
/// necessarily across devices reporting to the same window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ContactId {
    device: u64,
    pointer: u64,
}

impl From<&RawPointerEvent> for ContactId {
    fn from(event: &RawPointerEvent) -> Self {
        Self {
            device: event.device,
            pointer: event.pointer,
        }
    }
}

/// Flutter `_LineBetweenPointers`: the first two contacts in arrival order.
#[derive(Clone, Copy, Debug, PartialEq)]
struct LineBetweenPointers {
    start_id: ContactId,
    start: Offset,
    end_id: ContactId,
    end: Offset,
}

/// One tracked contact (`_pointerLocations`, `_velocityTrackers` and the
/// device kind) in `_pointerQueue` order.
struct Contact {
    id: ContactId,
    position: Offset,
    kind: PointerDeviceKind,
    velocity: VelocityTracker,
}

/// Phase of the absolute pointer pan/zoom protocol.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerPanZoomPhase {
    Start,
    Update {
        /// Translation relative to the gesture's start, in global coordinates.
        pan: Offset,
        /// Multiplicative scale relative to the gesture's start (initially 1).
        scale: f32,
        /// Rotation in radians relative to the gesture's start.
        rotation: f32,
    },
    End,
}

/// A trackpad gesture stream. Unlike a platform delta, each update describes
/// the full motion since Start; do not sum successive updates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerPanZoomEvent {
    pub device: u64,
    pub pointer: u64,
    pub position: Offset,
    pub phase: PointerPanZoomPhase,
    pub time: Instant,
    pub synthesized: bool,
}

/// The default scroll-to-scale gain when converting a trackpad pan to zoom.
pub const DEFAULT_TRACKPAD_SCROLL_TO_SCALE_FACTOR: Offset = Offset::new(0.0, -1.0 / 200.0);

struct PanZoomContact {
    id: ContactId,
    position: Offset,
    pan: Offset,
    scale: f32,
    rotation: f32,
    velocity: VelocityTracker,
}

impl PanZoomContact {
    fn scale(&self, scroll_to_scale: Option<Offset>) -> f32 {
        scroll_to_scale.map_or(self.scale, |gain| {
            self.scale * (self.pan.x * gain.x + self.pan.y * gain.y).exp()
        })
    }
}

/// Flutter `_ScaleState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScaleState {
    /// No pointers are down.
    Ready,
    /// Pointers are down but the span and focal point are within slop.
    Possible,
    /// The scale won its arena but no update has started it yet.
    Accepted,
    /// `onStart` was delivered; moves report updates.
    Started,
}

/// A platform-neutral port of Flutter's `ScaleGestureRecognizer` for
/// pointer contacts.
///
/// The span is the average distance of the contacts from their focal point.
/// The gesture becomes acceptable once the span changes by more than
/// [`compute_scale_slop`] or the focal point moves by more than the pan slop
/// for the pointer kind of the current event. A single contact moving past
/// the pan slop is a one-pointer scale, as in Flutter.
pub struct ScaleGestureDetector {
    pointers: Vec<Contact>,
    pan_zooms: Vec<PanZoomContact>,
    initial_pan_zoom_scale: f32,
    initial_pan_zoom_rotation: f32,
    scroll_to_scale: Option<Offset>,
    state: ScaleState,
    initial_focal_point: Offset,
    current_focal_point: Option<Offset>,
    current_local_focal_point: Offset,
    /// `_delta`: local focal point movement since the previous event.
    focal_point_delta: Offset,
    initial_span: f32,
    current_span: f32,
    initial_horizontal_span: f32,
    current_horizontal_span: f32,
    initial_vertical_span: f32,
    current_vertical_span: f32,
    initial_line: Option<LineBetweenPointers>,
    current_line: Option<LineBetweenPointers>,
    /// `_initialEventTimestamp`.
    initial_event_time: Option<Instant>,
    /// Timestamp of the latest observed event.
    last_event_time: Option<Instant>,
    /// `_scaleVelocityTracker`, fed `(scale, 0)` while started.
    scale_velocity: Option<VelocityTracker>,
    /// The latest event asked the arena to accept this recognizer.
    wants_acceptance: bool,
    /// At least one tracked contact joined after the last arena claim.
    pending_contacts: bool,
    /// Maps global positions into the recognizing widget's coordinates
    /// (Flutter `PointerEvent.transform`).
    local_transform: Option<Transform>,
    drag_start_behavior: DragStartBehavior,
    /// Flutter `GestureRecognizer.gestureSettings`, used for the focal-point
    /// pan slop.
    gesture_settings: DeviceGestureSettings,
    supported_devices: Option<Box<[PointerDeviceKind]>>,
    on_update: Option<Rc<dyn Fn(ScaleUpdateDetails)>>,
    on_start: Option<Rc<dyn Fn(ScaleStartDetails)>>,
    on_end: Option<Rc<dyn Fn(ScaleEndDetails)>>,
}
impl ScaleGestureDetector {
    #[must_use]
    pub fn new(on_update: impl Fn(ScaleUpdateDetails) + 'static) -> Self {
        Self::with_callbacks(Some(Rc::new(on_update)), None, None)
    }

    /// Creates a scale recognizer with the complete Flutter-style callback
    /// lifecycle. `new` remains the compact update-only constructor.
    #[must_use]
    pub fn with_callbacks(
        on_update: Option<Rc<dyn Fn(ScaleUpdateDetails)>>,
        on_start: Option<Rc<dyn Fn(ScaleStartDetails)>>,
        on_end: Option<Rc<dyn Fn(ScaleEndDetails)>>,
    ) -> Self {
        Self {
            pointers: Vec::new(),
            pan_zooms: Vec::new(),
            initial_pan_zoom_scale: 1.0,
            initial_pan_zoom_rotation: 0.0,
            scroll_to_scale: None,
            state: ScaleState::Ready,
            initial_focal_point: Offset::ZERO,
            current_focal_point: None,
            current_local_focal_point: Offset::ZERO,
            focal_point_delta: Offset::ZERO,
            initial_span: 0.0,
            current_span: 0.0,
            initial_horizontal_span: 0.0,
            current_horizontal_span: 0.0,
            initial_vertical_span: 0.0,
            current_vertical_span: 0.0,
            initial_line: None,
            current_line: None,
            initial_event_time: None,
            last_event_time: None,
            scale_velocity: None,
            wants_acceptance: false,
            pending_contacts: false,
            local_transform: None,
            // Flutter `ScaleGestureRecognizer.dragStartBehavior` default.
            drag_start_behavior: DragStartBehavior::Down,
            gesture_settings: DeviceGestureSettings::default(),
            supported_devices: None,
            on_update,
            on_start,
            on_end,
        }
    }

    /// Sets the point scale is measured from once the gesture is accepted.
    /// The bare recognizer defaults to [`DragStartBehavior::Down`]; Flutter's
    /// `GestureDetector` passes its own default, [`DragStartBehavior::Start`].
    #[must_use]
    pub fn with_drag_start_behavior(mut self, behavior: DragStartBehavior) -> Self {
        self.drag_start_behavior = behavior;
        self
    }

    /// Flutter `GestureRecognizer.gestureSettings`: platform slop overrides
    /// for the focal-point pan slop. The span's scale slop has no override.
    #[must_use]
    pub fn with_gesture_settings(mut self, settings: DeviceGestureSettings) -> Self {
        self.gesture_settings = settings;
        self
    }

    /// Replaces the platform slop overrides.
    pub fn set_gesture_settings(&mut self, settings: DeviceGestureSettings) {
        self.gesture_settings = settings;
    }

    #[must_use]
    pub const fn gesture_settings(&self) -> DeviceGestureSettings {
        self.gesture_settings
    }

    #[must_use]
    pub const fn drag_start_behavior(&self) -> DragStartBehavior {
        self.drag_start_behavior
    }

    /// Flutter's supportedDevices: only contacts of these kinds enter or
    /// reconfigure the recognition state. The default accepts all kinds.
    #[must_use]
    pub fn with_supported_devices(
        mut self,
        devices: impl IntoIterator<Item = PointerDeviceKind>,
    ) -> Self {
        self.supported_devices = Some(devices.into_iter().collect());
        self
    }

    fn allows_device(&self, kind: PointerDeviceKind) -> bool {
        self.supported_devices
            .as_ref()
            .is_none_or(|devices| devices.contains(&kind))
    }

    /// Enables scroll-to-scale with the given gain, or disables it with
    /// `None` (the default). When enabled the trackpad focal point stays at
    /// its reported position instead of following the pan.
    pub fn set_trackpad_scroll_to_scale(&mut self, gain: Option<Offset>) {
        self.scroll_to_scale = gain;
    }

    /// Number of currently tracked fingers. Each trackpad stream represents
    /// at least two fingers and contributes two to this count.
    #[must_use]
    pub fn pointer_count(&self) -> usize {
        self.pointers.len() + 2 * self.pan_zooms.len()
    }

    /// Sets the global-to-local transform used for the details' local
    /// focal points; retained adapters pass the widget's inverse transform.
    pub fn set_local_transform(&mut self, transform: Transform) {
        self.local_transform = Some(transform);
    }

    fn local(&self, point: Offset) -> Offset {
        self.local_transform
            .map_or(point, |transform| transform.transform_point(point))
    }

    /// Recognizes the event without an arena: an acceptable gesture is
    /// accepted at once.
    pub fn handle(&mut self, event: PointerEvent) -> bool {
        self.handle_raw(RawPointerEvent {
            kind: PointerDeviceKind::Unknown,
            ..RawPointerEvent::from(event)
        })
    }

    /// [`Self::handle`] for an event whose device kind is known.
    pub fn handle_raw(&mut self, event: RawPointerEvent) -> bool {
        if !self.allows_device(event.kind) {
            return false;
        }
        let handled = match event.phase {
            PointerPhase::Enter | PointerPhase::Exit => false,
            PointerPhase::Down => true,
            PointerPhase::Move | PointerPhase::Up | PointerPhase::Cancel => {
                self.tracks(ContactId::from(&event))
            }
        };
        let mut details = self.observe_raw(event);
        if self.wants_acceptance() {
            details = self.accept().or(details);
        }
        if let Some(details) = details {
            self.dispatch(details);
        }
        handled
    }

    /// Updates internal contact geometry without invoking the update
    /// callback. The legacy event has no device kind, which Flutter treats
    /// with touch slops. Retained adapters use this to wait until the scale
    /// recognizer wins its arena before exposing any scale updates.
    pub fn observe(&mut self, event: PointerEvent) -> Option<ScaleUpdateDetails> {
        self.observe_raw(RawPointerEvent {
            kind: PointerDeviceKind::Unknown,
            ..RawPointerEvent::from(event)
        })
    }

    /// Flutter `handleEvent` for one pointer contact. Returns the update to
    /// deliver while the scale is started. When the event moves the contacts
    /// past slop, [`Self::wants_acceptance`] turns true and the caller
    /// resolves its arena and calls [`Self::accept`].
    pub fn observe_raw(&mut self, event: RawPointerEvent) -> Option<ScaleUpdateDetails> {
        self.wants_acceptance = false;
        if !self.allows_device(event.kind) {
            return None;
        }
        let id = ContactId::from(&event);
        let mut lifted = None;
        let (configuration_changed, should_start) = match event.phase {
            PointerPhase::Enter | PointerPhase::Exit => return None,
            PointerPhase::Down => {
                // Flutter `addAllowedPointer`.
                self.initial_event_time = Some(event.time);
                if self.state == ScaleState::Ready {
                    self.state = ScaleState::Possible;
                    self.initial_span = 0.0;
                    self.current_span = 0.0;
                    self.initial_horizontal_span = 0.0;
                    self.current_horizontal_span = 0.0;
                    self.initial_vertical_span = 0.0;
                    self.current_vertical_span = 0.0;
                }
                match self.pointers.iter_mut().find(|contact| contact.id == id) {
                    Some(contact) => contact.position = event.position,
                    None => {
                        self.pending_contacts = true;
                        self.pointers.push(Contact {
                            id,
                            position: event.position,
                            kind: event.kind,
                            velocity: VelocityTracker::default(),
                        });
                    }
                }
                (true, true)
            }
            PointerPhase::Move => {
                let contact = self.pointers.iter_mut().find(|contact| contact.id == id)?;
                // Synthesized motion updates the contact geometry but is
                // not a physical velocity sample.
                if !event.synthesized {
                    contact.velocity.add_position(event.time, event.position);
                }
                contact.position = event.position;
                (false, true)
            }
            PointerPhase::Up | PointerPhase::Cancel => {
                let index = self.pointers.iter().position(|contact| contact.id == id)?;
                lifted = Some(self.pointers.remove(index));
                (true, false)
            }
        };
        self.finish_event(
            configuration_changed,
            should_start,
            lifted.as_ref().map(|contact| &contact.velocity),
            event.kind,
            event.time,
        )
    }

    /// Observes a routed absolute trackpad event. Arbitration follows the
    /// same `wants_acceptance` / `accept` contract as touch events. An arena
    /// that grants a sole member the stream when it closes should call
    /// `accept_without_update` after routing Start.
    pub fn observe_pan_zoom(&mut self, event: PointerPanZoomEvent) -> Option<ScaleUpdateDetails> {
        self.wants_acceptance = false;
        let kind = PointerDeviceKind::Trackpad;
        if !self.allows_device(kind) {
            return None;
        }
        let id = ContactId {
            device: event.device,
            pointer: event.pointer,
        };
        let index = self.pan_zooms.iter().position(|contact| contact.id == id);
        let mut lifted = None;
        let (configuration_changed, should_start) = match event.phase {
            PointerPanZoomPhase::Start => {
                if index.is_some() {
                    return None;
                }
                self.initial_event_time = Some(event.time);
                if self.state == ScaleState::Ready {
                    self.state = ScaleState::Possible;
                    self.initial_pan_zoom_scale = 1.0;
                    self.initial_pan_zoom_rotation = 0.0;
                }
                self.pending_contacts = true;
                self.pan_zooms.push(PanZoomContact {
                    id,
                    position: event.position,
                    pan: Offset::ZERO,
                    scale: 1.0,
                    rotation: 0.0,
                    velocity: VelocityTracker::default(),
                });
                (true, true)
            }
            PointerPanZoomPhase::Update {
                pan,
                scale,
                rotation,
            } => {
                let contact = &mut self.pan_zooms[index?];
                if !event.synthesized && self.scroll_to_scale.is_none() {
                    contact.velocity.add_position(event.time, pan);
                }
                contact.position = event.position;
                contact.pan = pan;
                contact.scale = scale;
                contact.rotation = rotation;
                (false, true)
            }
            PointerPanZoomPhase::End => {
                lifted = Some(self.pan_zooms.remove(index?));
                (true, false)
            }
        };
        self.finish_event(
            configuration_changed,
            should_start,
            lifted.as_ref().map(|contact| &contact.velocity),
            kind,
            event.time,
        )
    }

    fn finish_event(
        &mut self,
        configuration_changed: bool,
        should_start: bool,
        velocity: Option<&VelocityTracker>,
        kind: PointerDeviceKind,
        time: Instant,
    ) -> Option<ScaleUpdateDetails> {
        self.last_event_time = Some(time);
        self.update_lines();
        self.update();
        let mut details = None;
        if !configuration_changed || self.reconfigure(velocity, time) {
            details = self.advance_state_machine(should_start, kind, time);
        }
        self.reset_if_empty();
        details
    }

    /// Whether the latest observed event asks the arena to accept the scale
    /// (Flutter `resolve(GestureDisposition.accepted)`).
    #[must_use]
    pub const fn wants_acceptance(&self) -> bool {
        self.wants_acceptance
    }

    /// Whether the scale has started and reports updates.
    #[must_use]
    pub fn is_started(&self) -> bool {
        self.state == ScaleState::Started
    }

    /// Flutter `acceptGesture`: the arena gave this recognizer the pointer
    /// stream. A pending scale starts here and, with
    /// [`DragStartBehavior::Start`], measures later updates from the current
    /// contacts. Returns the update for the event that won the arena.
    pub fn accept(&mut self) -> Option<ScaleUpdateDetails> {
        if !self.accept_without_update() {
            return None;
        }
        let details = self.details(self.last_event_time);
        self.track_scale(details);
        Some(details)
    }

    /// Grants acceptance without synthesizing an update, as when a sole
    /// recognizer wins its arena immediately after the start event.
    pub fn accept_without_update(&mut self) -> bool {
        self.wants_acceptance = false;
        self.pending_contacts = false;
        if self.state != ScaleState::Possible {
            return false;
        }
        self.state = ScaleState::Started;
        self.dispatch_start();
        if self.drag_start_behavior == DragStartBehavior::Start {
            self.rebase();
        }
        true
    }

    /// Flutter `rejectGesture`: another recognizer won this pointer's arena.
    pub fn reject_pointer(&mut self, pointer: u64) {
        let id = self
            .pointers
            .iter()
            .find(|contact| contact.id.pointer == pointer)
            .map(|contact| contact.id)
            .or_else(|| {
                self.pan_zooms
                    .iter()
                    .find(|contact| contact.id.pointer == pointer)
                    .map(|contact| contact.id)
            });
        if let Some(id) = id {
            self.reject_device_pointer(id.device, id.pointer);
        }
    }

    /// Rejects only the contact from the specified device and pointer.
    pub fn reject_device_pointer(&mut self, device: u64, pointer: u64) {
        let id = ContactId { device, pointer };
        if let Some(index) = self.pointers.iter().position(|contact| contact.id == id) {
            self.pointers.remove(index);
        } else if let Some(index) = self.pan_zooms.iter().position(|contact| contact.id == id) {
            self.pan_zooms.remove(index);
        }
        self.reset_if_empty();
    }

    fn reset_if_empty(&mut self) {
        if self.pointer_count() == 0 {
            self.state = ScaleState::Ready;
            self.current_focal_point = None;
            self.initial_line = None;
            self.current_line = None;
            self.wants_acceptance = false;
            self.pending_contacts = false;
        }
    }

    /// Delivers a previously observed update after the caller has resolved
    /// arbitration for the relevant pointer stream.
    pub fn dispatch(&self, details: ScaleUpdateDetails) {
        if let Some(callback) = &self.on_update {
            callback(details);
        }
    }

    fn tracks(&self, id: ContactId) -> bool {
        self.pointers.iter().any(|contact| contact.id == id)
    }

    /// Flutter `_updateLines`: the line between the first two contacts.
    fn update_lines(&mut self) {
        let [first, second, ..] = self.pointers.as_slice() else {
            self.initial_line = self.current_line;
            return;
        };
        let line = LineBetweenPointers {
            start_id: first.id,
            start: first.position,
            end_id: second.id,
            end: second.position,
        };
        match self.initial_line {
            Some(initial) if initial.start_id == first.id && initial.end_id == second.id => {
                self.current_line = Some(line);
            }
            _ => {
                self.initial_line = Some(line);
                self.current_line = Some(line);
            }
        }
    }

    /// Flutter `_update`: the focal point is the mean contact position and
    /// the spans the mean (horizontal, vertical) distances of the contacts
    /// from it.
    fn update(&mut self) {
        let previous_local = self
            .current_focal_point
            .map(|_| self.current_local_focal_point);
        let count = self.pointers.len();
        let mut focal_point = Offset::ZERO;
        for contact in &self.pointers {
            focal_point = Offset::new(
                focal_point.x + contact.position.x,
                focal_point.y + contact.position.y,
            );
        }
        let touch_divisor = count.max(1) as f32;
        let touch_focal = Offset::new(focal_point.x / touch_divisor, focal_point.y / touch_divisor);
        let focal_point = if self.pan_zooms.is_empty() {
            touch_focal
        } else {
            for trackpad in &self.pan_zooms {
                let trackpad_focal = if self.scroll_to_scale.is_some() {
                    trackpad.position
                } else {
                    trackpad.position + trackpad.pan
                };
                focal_point = focal_point + trackpad_focal;
            }
            let divisor = (count + self.pan_zooms.len()) as f32;
            Offset::new(focal_point.x / divisor, focal_point.y / divisor)
        };
        let local = self.local(focal_point);
        self.current_focal_point = Some(focal_point);
        self.current_local_focal_point = local;
        self.focal_point_delta = previous_local.map_or(Offset::ZERO, |previous| local - previous);
        let (mut total, mut horizontal, mut vertical) = (0.0, 0.0, 0.0);
        for contact in &self.pointers {
            total += distance(touch_focal, contact.position);
            horizontal += (touch_focal.x - contact.position.x).abs();
            vertical += (touch_focal.y - contact.position.y).abs();
        }
        let mean = |sum: f32| if count > 0 { sum / count as f32 } else { 0.0 };
        self.current_span = mean(total);
        self.current_horizontal_span = mean(horizontal);
        self.current_vertical_span = mean(vertical);
    }

    fn pan_zoom_scale(&self) -> f32 {
        self.pan_zooms
            .iter()
            .map(|contact| contact.scale(self.scroll_to_scale) / self.initial_pan_zoom_scale)
            .product()
    }

    fn rebase(&mut self) {
        if let Some(focal_point) = self.current_focal_point {
            self.initial_focal_point = focal_point;
        }
        self.initial_span = self.current_span;
        self.initial_line = self.current_line;
        self.initial_horizontal_span = self.current_horizontal_span;
        self.initial_vertical_span = self.current_vertical_span;
        self.initial_pan_zoom_scale = self.pan_zoom_scale();
        self.initial_pan_zoom_rotation =
            self.pan_zooms.iter().map(|contact| contact.rotation).sum();
    }

    /// Flutter `_reconfigure`: a contact went down or up. Returns whether the
    /// state machine should still advance for this event.
    fn reconfigure(&mut self, velocity: Option<&VelocityTracker>, time: Instant) -> bool {
        self.rebase();
        if self.state == ScaleState::Started {
            if let Some(callback) = &self.on_end {
                let scale_velocity = self.scale_velocity.as_ref().map_or(-1.0, |tracker| {
                    tracker.estimate_at(time).pixels_per_second.x
                });
                // `_isFlingGesture`, then clamped to `kMaxFlingVelocity`.
                let velocity = velocity
                    .map(|tracker| tracker.estimate_at(time).pixels_per_second)
                    .filter(|velocity| velocity.x.hypot(velocity.y) > MIN_FLING_VELOCITY)
                    .map_or(Velocity::ZERO, |velocity| {
                        let speed = velocity.x.hypot(velocity.y);
                        let scale = if speed > MAX_FLING_VELOCITY {
                            MAX_FLING_VELOCITY / speed
                        } else {
                            1.0
                        };
                        Velocity::new(Offset::new(velocity.x * scale, velocity.y * scale))
                    });
                callback(ScaleEndDetails {
                    velocity,
                    scale_velocity,
                    pointer_count: self.pointer_count(),
                });
            }
            self.state = ScaleState::Accepted;
            self.scale_velocity
                .get_or_insert_with(VelocityTracker::default)
                .clear();
            return false;
        }
        self.scale_velocity
            .get_or_insert_with(VelocityTracker::default)
            .clear();
        true
    }

    fn track_scale(&mut self, details: ScaleUpdateDetails) {
        if let (Some(tracker), Some(time)) = (self.scale_velocity.as_mut(), details.source_time) {
            tracker.add_position(time, Offset::new(details.scale, 0.0));
        }
    }

    /// Flutter `_advanceStateMachine`.
    fn advance_state_machine(
        &mut self,
        should_start_if_accepted: bool,
        kind: PointerDeviceKind,
        time: Instant,
    ) -> Option<ScaleUpdateDetails> {
        if self.state == ScaleState::Ready {
            self.state = ScaleState::Possible;
        }
        if self.state == ScaleState::Possible {
            let focal_point = self.current_focal_point?;
            let span_delta = (self.current_span - self.initial_span).abs();
            let focal_point_delta = distance(focal_point, self.initial_focal_point);
            let pan_zoom = self.pan_zoom_scale();
            if span_delta > compute_scale_slop(kind)
                || focal_point_delta > compute_pan_slop(kind, self.gesture_settings)
                || pan_zoom.max(pan_zoom.recip()) > 1.05
            {
                self.wants_acceptance = true;
            }
        } else {
            // Flutter resolves acceptance again when an accepted scale
            // has enrolled another contact. Ordinary moves with no new
            // contact need no additional arena walk.
            self.wants_acceptance = self.pending_contacts;
            if self.state == ScaleState::Accepted && should_start_if_accepted {
                self.initial_event_time = Some(time);
                self.state = ScaleState::Started;
                self.dispatch_start();
            }
            if self.state == ScaleState::Started {
                let details = self.details(Some(time));
                self.track_scale(details);
                return Some(details);
            }
        }
        None
    }

    fn dispatch_start(&self) {
        if let (Some(callback), Some(focal_point)) = (&self.on_start, self.current_focal_point) {
            callback(ScaleStartDetails {
                focal_point,
                local_focal_point: self.current_local_focal_point,
                pointer_count: self.pointer_count(),
                source_time: self.initial_event_time,
                kind: self
                    .pointers
                    .first()
                    .map(|contact| contact.kind)
                    .or_else(|| self.pan_zooms.first().map(|_| PointerDeviceKind::Trackpad)),
            });
        }
    }

    /// Flutter `_computeRotationFactor`.
    fn rotation(&self) -> f32 {
        let angle = |line: LineBetweenPointers| {
            (line.start.y - line.end.y).atan2(line.start.x - line.end.x)
        };
        let touch = self
            .initial_line
            .zip(self.current_line)
            .map_or(0.0, |(initial, current)| angle(current) - angle(initial));
        touch
            + self
                .pan_zooms
                .iter()
                .map(|contact| contact.rotation)
                .sum::<f32>()
            - self.initial_pan_zoom_rotation
    }

    fn details(&self, source_time: Option<Instant>) -> ScaleUpdateDetails {
        // Flutter `_pointerScaleFactor` and its horizontal/vertical forms.
        let ratio = |current: f32, initial: f32| {
            if initial > 0.0 {
                current / initial
            } else {
                1.0
            }
        };
        let focal_point = self.current_focal_point.unwrap_or(Offset::ZERO);
        let pan_zoom = self.pan_zoom_scale();
        ScaleUpdateDetails {
            focal_point,
            local_focal_point: self.current_local_focal_point,
            scale: ratio(self.current_span, self.initial_span) * pan_zoom,
            horizontal_scale: ratio(self.current_horizontal_span, self.initial_horizontal_span)
                * pan_zoom,
            vertical_scale: ratio(self.current_vertical_span, self.initial_vertical_span)
                * pan_zoom,
            rotation: self.rotation(),
            focal_point_delta: self.focal_point_delta,
            pointer_count: self.pointer_count(),
            source_time,
        }
    }
}

fn distance(first: Offset, second: Offset) -> f32 {
    (first.x - second.x).hypot(first.y - second.y)
}
