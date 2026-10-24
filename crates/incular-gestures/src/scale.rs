use std::rc::Rc;

use incular_core::{DragStartBehavior, Offset, PointerPhase};

use crate::constants::{
    PAN_SLOP, PRECISE_POINTER_PAN_SLOP, PRECISE_POINTER_SCALE_SLOP, SCALE_SLOP,
};
use crate::details::{PointerDeviceKind, PointerEvent, RawPointerEvent};

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

/// Metadata for scale start events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleStartDetails {
    pub focal_point: Offset,
    pub pointer_count: usize,
}

/// The current geometry of a scale interaction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleUpdateDetails {
    pub focal_point: Offset,
    pub scale: f32,
    pub pointer_count: usize,
}

/// Metadata for scale end events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleEndDetails {
    pub pointer_count: usize,
}

impl ScaleUpdateDetails {
    #[must_use]
    pub const fn new(focal_point: Offset, scale: f32) -> Self {
        Self {
            focal_point,
            scale,
            pointer_count: 2,
        }
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
    /// `_pointerLocations` in `_pointerQueue` order.
    pointers: Vec<(u64, Offset)>,
    state: ScaleState,
    initial_focal_point: Offset,
    current_focal_point: Option<Offset>,
    initial_span: f32,
    current_span: f32,
    /// The latest event asked the arena to accept this recognizer.
    wants_acceptance: bool,
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
            initial_span: 0.0,
            current_span: 0.0,
            wants_acceptance: false,
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
        let (configuration_changed, should_start) = match event.phase {
            PointerPhase::Enter | PointerPhase::Exit => return None,
            PointerPhase::Down => {
                // Flutter `addAllowedPointer`.
                if self.state == ScaleState::Ready {
                    self.state = ScaleState::Possible;
                    self.initial_span = 0.0;
                    self.current_span = 0.0;
                }
                match self
                    .pointers
                    .iter_mut()
                    .find(|(id, _)| *id == event.pointer)
                {
                    Some((_, position)) => *position = event.position,
                    None => self.pointers.push((event.pointer, event.position)),
                }
                (true, true)
            }
            PointerPhase::Move => {
                let (_, position) = self
                    .pointers
                    .iter_mut()
                    .find(|(id, _)| *id == event.pointer)?;
                *position = event.position;
                (false, true)
            }
            PointerPhase::Up | PointerPhase::Cancel => {
                let index = self
                    .pointers
                    .iter()
                    .position(|(id, _)| *id == event.pointer)?;
                self.pointers.remove(index);
                (true, false)
            }
        };
        self.update();
        let mut details = None;
        if !configuration_changed || self.reconfigure() {
            details = self.advance_state_machine(should_start, event.kind);
        }
        if self.pointers.is_empty() {
            // Flutter `didStopTrackingLastPointer`.
            self.state = ScaleState::Ready;
            self.current_focal_point = None;
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
                }
                Some(self.details())
            }
            ScaleState::Ready | ScaleState::Accepted | ScaleState::Started => None,
        }
    }

    /// Flutter `rejectGesture`: another recognizer won this pointer's arena,
    /// so the scale stops tracking it without reconfiguring.
    pub fn reject_pointer(&mut self, pointer: u64) {
        let Some(index) = self.pointers.iter().position(|(id, _)| *id == pointer) else {
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
        self.pointers.iter().any(|(id, _)| *id == pointer)
    }

    /// Flutter `_update`: the focal point is the mean contact position and
    /// the span the mean distance of the contacts from it.
    fn update(&mut self) {
        let count = self.pointers.len();
        let mut focal_point = Offset::ZERO;
        for (_, position) in &self.pointers {
            focal_point = Offset::new(focal_point.x + position.x, focal_point.y + position.y);
        }
        let divisor = count.max(1) as f32;
        let focal_point = Offset::new(focal_point.x / divisor, focal_point.y / divisor);
        self.current_focal_point = Some(focal_point);
        let total_deviation: f32 = self
            .pointers
            .iter()
            .map(|(_, position)| distance(focal_point, *position))
            .sum();
        self.current_span = if count > 0 {
            total_deviation / count as f32
        } else {
            0.0
        };
    }

    /// Flutter `_reconfigure`: a contact went down or up. Returns whether the
    /// state machine should still advance for this event.
    fn reconfigure(&mut self) -> bool {
        if let Some(focal_point) = self.current_focal_point {
            self.initial_focal_point = focal_point;
        }
        self.initial_span = self.current_span;
        if self.state == ScaleState::Started {
            if let Some(callback) = &self.on_end {
                callback(ScaleEndDetails {
                    pointer_count: self.pointers.len(),
                });
            }
            self.state = ScaleState::Accepted;
            return false;
        }
        true
    }

    /// Flutter `_advanceStateMachine`.
    fn advance_state_machine(
        &mut self,
        should_start_if_accepted: bool,
        kind: PointerDeviceKind,
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
                self.state = ScaleState::Started;
                self.dispatch_start();
            }
            if self.state == ScaleState::Started {
                return Some(self.details());
            }
        }
        None
    }

    fn dispatch_start(&self) {
        if let (Some(callback), Some(focal_point)) = (&self.on_start, self.current_focal_point) {
            callback(ScaleStartDetails {
                focal_point,
                pointer_count: self.pointers.len(),
            });
        }
    }

    fn details(&self) -> ScaleUpdateDetails {
        // Flutter `_pointerScaleFactor`.
        let scale = if self.initial_span > 0.0 {
            self.current_span / self.initial_span
        } else {
            1.0
        };
        ScaleUpdateDetails {
            focal_point: self.current_focal_point.unwrap_or(Offset::ZERO),
            scale,
            pointer_count: self.pointers.len(),
        }
    }
}

fn distance(first: Offset, second: Offset) -> f32 {
    (first.x - second.x).hypot(first.y - second.y)
}
