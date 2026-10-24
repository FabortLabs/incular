use std::rc::Rc;

use incular_core::time::Instant;
use incular_core::{DragStartBehavior, Offset, PointerPhase, Transform};

use crate::constants::{
    MAX_FLING_VELOCITY, MIN_FLING_VELOCITY, PAN_SLOP, PRECISE_POINTER_PAN_SLOP,
    PRECISE_POINTER_SCALE_SLOP, SCALE_SLOP,
};
use crate::details::{PointerDeviceKind, PointerEvent, RawPointerEvent, Velocity};
use crate::velocity::VelocityTracker;

/// Flutter `computeScaleSlop`: how far the span must change before a scale
/// is accepted.
#[must_use]
pub fn compute_scale_slop(kind: PointerDeviceKind) -> f32 {
    match kind {
        PointerDeviceKind::Mouse => PRECISE_POINTER_SCALE_SLOP,
        PointerDeviceKind::Touch
        | PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Trackpad
        | PointerDeviceKind::Unknown => SCALE_SLOP,
    }
}

/// Flutter `computePanSlop` as the scale recognizer uses it for focal-point
/// movement.
fn compute_pan_slop(kind: PointerDeviceKind) -> f32 {
    match kind {
        PointerDeviceKind::Mouse => PRECISE_POINTER_PAN_SLOP,
        PointerDeviceKind::Touch
        | PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Trackpad
        | PointerDeviceKind::Unknown => PAN_SLOP,
    }
}

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
    /// How far the focal point moved since the previous event.
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

/// Flutter `_LineBetweenPointers`: the first two contacts in arrival order.
#[derive(Clone, Copy, Debug, PartialEq)]
struct LineBetweenPointers {
    start_id: u64,
    start: Offset,
    end_id: u64,
    end: Offset,
}

/// One tracked contact (`_pointerLocations`, `_velocityTrackers` and the
/// device kind) in `_pointerQueue` order.
struct Contact {
    pointer: u64,
    position: Offset,
    kind: PointerDeviceKind,
    velocity: VelocityTracker,
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
    state: ScaleState,
    initial_focal_point: Offset,
    current_focal_point: Option<Offset>,
    /// `_delta`: focal point movement since the previous event.
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
    /// Maps global positions into the recognizing widget's coordinates
    /// (Flutter `PointerEvent.transform`).
    local_transform: Option<Transform>,
    drag_start_behavior: DragStartBehavior,
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
            state: ScaleState::Ready,
            initial_focal_point: Offset::ZERO,
            current_focal_point: None,
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
            local_transform: None,
            // Flutter `ScaleGestureRecognizer.dragStartBehavior` default.
            drag_start_behavior: DragStartBehavior::Down,
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

    #[must_use]
    pub const fn drag_start_behavior(&self) -> DragStartBehavior {
        self.drag_start_behavior
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
        let handled = match event.phase {
            PointerPhase::Enter | PointerPhase::Exit => false,
            PointerPhase::Down => true,
            PointerPhase::Move | PointerPhase::Up | PointerPhase::Cancel => {
                self.tracks(event.pointer)
            }
        };
        let mut details = self.observe_raw(event);
        if self.wants_acceptance() {
            details = self.accept();
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
        self.last_event_time = Some(event.time);
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
                match self
                    .pointers
                    .iter_mut()
                    .find(|contact| contact.pointer == event.pointer)
                {
                    Some(contact) => contact.position = event.position,
                    None => self.pointers.push(Contact {
                        pointer: event.pointer,
                        position: event.position,
                        kind: event.kind,
                        velocity: VelocityTracker::default(),
                    }),
                }
                (true, true)
            }
            PointerPhase::Move => {
                let contact = self
                    .pointers
                    .iter_mut()
                    .find(|contact| contact.pointer == event.pointer)?;
                contact.velocity.add_position(event.time, event.position);
                contact.position = event.position;
                (false, true)
            }
            PointerPhase::Up | PointerPhase::Cancel => {
                let index = self
                    .pointers
                    .iter()
                    .position(|contact| contact.pointer == event.pointer)?;
                lifted = Some(self.pointers.remove(index));
                (true, false)
            }
        };
        self.update_lines();
        self.update();
        let mut details = None;
        if !configuration_changed || self.reconfigure(lifted.as_ref(), event.time) {
            details = self.advance_state_machine(should_start, event.kind, event.time);
        }
        if self.pointers.is_empty() {
            // Flutter `didStopTrackingLastPointer`.
            self.state = ScaleState::Ready;
            self.current_focal_point = None;
            self.initial_line = None;
            self.current_line = None;
            self.wants_acceptance = false;
        }
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
        self.wants_acceptance = false;
        match self.state {
            ScaleState::Possible => {
                self.state = ScaleState::Started;
                self.dispatch_start();
                if self.drag_start_behavior == DragStartBehavior::Start {
                    self.initial_focal_point = self.current_focal_point?;
                    self.initial_span = self.current_span;
                    self.initial_line = self.current_line;
                    self.initial_horizontal_span = self.current_horizontal_span;
                    self.initial_vertical_span = self.current_vertical_span;
                }
                let details = self.details(self.last_event_time);
                self.track_scale(details);
                Some(details)
            }
            ScaleState::Ready | ScaleState::Accepted | ScaleState::Started => None,
        }
    }

    /// Flutter `rejectGesture`: another recognizer won this pointer's arena,
    /// so the scale stops tracking it without reconfiguring.
    pub fn reject_pointer(&mut self, pointer: u64) {
        let Some(index) = self
            .pointers
            .iter()
            .position(|contact| contact.pointer == pointer)
        else {
            return;
        };
        self.pointers.remove(index);
        if self.pointers.is_empty() {
            self.state = ScaleState::Ready;
            self.current_focal_point = None;
            self.wants_acceptance = false;
        }
    }

    /// Delivers a previously observed update after the caller has resolved
    /// arbitration for the relevant pointer stream.
    pub fn dispatch(&self, details: ScaleUpdateDetails) {
        if let Some(callback) = &self.on_update {
            callback(details);
        }
    }

    fn tracks(&self, pointer: u64) -> bool {
        self.pointers
            .iter()
            .any(|contact| contact.pointer == pointer)
    }

    /// Flutter `_updateLines`: the line between the first two contacts.
    fn update_lines(&mut self) {
        let [first, second, ..] = self.pointers.as_slice() else {
            self.initial_line = self.current_line;
            return;
        };
        let line = LineBetweenPointers {
            start_id: first.pointer,
            start: first.position,
            end_id: second.pointer,
            end: second.position,
        };
        match self.initial_line {
            Some(initial)
                if initial.start_id == first.pointer && initial.end_id == second.pointer =>
            {
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
        let previous = self.current_focal_point;
        let count = self.pointers.len();
        let mut focal_point = Offset::ZERO;
        for contact in &self.pointers {
            focal_point = Offset::new(
                focal_point.x + contact.position.x,
                focal_point.y + contact.position.y,
            );
        }
        let divisor = count.max(1) as f32;
        let focal_point = Offset::new(focal_point.x / divisor, focal_point.y / divisor);
        self.current_focal_point = Some(focal_point);
        self.focal_point_delta = previous.map_or(Offset::ZERO, |previous| {
            Offset::new(focal_point.x - previous.x, focal_point.y - previous.y)
        });
        let (mut total, mut horizontal, mut vertical) = (0.0, 0.0, 0.0);
        for contact in &self.pointers {
            total += distance(focal_point, contact.position);
            horizontal += (focal_point.x - contact.position.x).abs();
            vertical += (focal_point.y - contact.position.y).abs();
        }
        let mean = |sum: f32| if count > 0 { sum / count as f32 } else { 0.0 };
        self.current_span = mean(total);
        self.current_horizontal_span = mean(horizontal);
        self.current_vertical_span = mean(vertical);
    }

    /// Flutter `_reconfigure`: a contact went down or up. Returns whether the
    /// state machine should still advance for this event.
    fn reconfigure(&mut self, lifted: Option<&Contact>, time: Instant) -> bool {
        if let Some(focal_point) = self.current_focal_point {
            self.initial_focal_point = focal_point;
        }
        self.initial_span = self.current_span;
        self.initial_line = self.current_line;
        self.initial_horizontal_span = self.current_horizontal_span;
        self.initial_vertical_span = self.current_vertical_span;
        if self.state == ScaleState::Started {
            if let Some(callback) = &self.on_end {
                let scale_velocity = self.scale_velocity.as_ref().map_or(-1.0, |tracker| {
                    tracker.estimate_at(time).pixels_per_second.x
                });
                // `_isFlingGesture`, then clamped to `kMaxFlingVelocity`.
                let velocity = lifted
                    .map(|contact| contact.velocity.estimate_at(time).pixels_per_second)
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
                    pointer_count: self.pointers.len(),
                });
            }
            self.state = ScaleState::Accepted;
            self.scale_velocity = Some(VelocityTracker::default());
            return false;
        }
        self.scale_velocity = Some(VelocityTracker::default());
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
            // Without trackpad pan-zoom contacts `_scaleFactor` equals
            // `_pointerScaleFactor`, so Flutter's third (1.05 ratio) test
            // never passes here.
            if span_delta > compute_scale_slop(kind) || focal_point_delta > compute_pan_slop(kind) {
                self.wants_acceptance = true;
            }
        } else {
            // `_state.index >= accepted`: the recognizer already owns the
            // stream.
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
                local_focal_point: self.local(focal_point),
                pointer_count: self.pointers.len(),
                source_time: self.initial_event_time,
                kind: self.pointers.first().map(|contact| contact.kind),
            });
        }
    }

    /// Flutter `_computeRotationFactor` without trackpad contacts.
    fn rotation(&self) -> f32 {
        let (Some(initial), Some(current)) = (self.initial_line, self.current_line) else {
            return 0.0;
        };
        let angle = |line: LineBetweenPointers| {
            (line.start.y - line.end.y).atan2(line.start.x - line.end.x)
        };
        angle(current) - angle(initial)
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
        ScaleUpdateDetails {
            focal_point,
            local_focal_point: self.local(focal_point),
            scale: ratio(self.current_span, self.initial_span),
            horizontal_scale: ratio(self.current_horizontal_span, self.initial_horizontal_span),
            vertical_scale: ratio(self.current_vertical_span, self.initial_vertical_span),
            rotation: self.rotation(),
            focal_point_delta: self.focal_point_delta,
            pointer_count: self.pointers.len(),
            source_time,
        }
    }
}

fn distance(first: Offset, second: Offset) -> f32 {
    (first.x - second.x).hypot(first.y - second.y)
}
