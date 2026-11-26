use super::*;
use crate::{
    GestureArenaKey, GestureArenaSource, PointerPanZoomEvent, PointerPanZoomPhase, RawPointerEvent,
};

/// The movement a mono-drag recognizer accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragAxis {
    Horizontal,
    Vertical,
    Pan,
}

pub type TrackpadDragAxis = DragAxis;
pub type TrackpadDragRecognizer = MonodragGestureRecognizer;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Stream {
    device: u64,
    pointer: u64,
    absolute: bool,
}

impl Stream {
    fn from_arena(key: GestureArenaKey) -> Option<Self> {
        match key.source {
            GestureArenaSource::Pointer { device, pointer } => Some(Self {
                device,
                pointer,
                absolute: false,
            }),
            GestureArenaSource::PointerPanZoom { device, pointer } => Some(Self {
                device,
                pointer,
                absolute: true,
            }),
            GestureArenaSource::Trackpad { .. } => None,
        }
    }
}

struct Contact {
    stream: Stream,
    kind: PointerDeviceKind,
    previous: Offset,
    velocity: VelocityTracker,
    accepted: bool,
}

/// One drag tenure shared by physical pointers and absolute trackpad streams.
/// Each contact joins its own arena. An already accepted drag claims new
/// contacts immediately, processes the latest accepted contact's moves, and
/// ends only when the last contact leaves.
pub struct MonodragGestureRecognizer {
    recognizer: PointerGestureRecognizer,
    axis: PanAction,
    contacts: Vec<Contact>,
    first: Option<PointerEvent>,
    active: Option<Stream>,
    observed: Option<Stream>,
    wants_acceptance: bool,
    on_down: Option<Rc<dyn Fn(DragDownDetails)>>,
    on_cancel: Option<Rc<dyn Fn()>>,
}

impl MonodragGestureRecognizer {
    #[must_use]
    pub fn new(axis: DragAxis, callbacks: DragCallbacks) -> Self {
        let (axis, callbacks) = match axis {
            DragAxis::Horizontal => (
                PanAction::Horizontal,
                GestureCallbacks {
                    on_horizontal_drag_start: callbacks.on_start,
                    on_horizontal_drag_update: callbacks.on_update,
                    on_horizontal_drag_end: callbacks.on_end,
                    ..GestureCallbacks::default()
                },
            ),
            DragAxis::Vertical => (
                PanAction::Vertical,
                GestureCallbacks {
                    on_vertical_drag_start: callbacks.on_start,
                    on_vertical_drag_update: callbacks.on_update,
                    on_vertical_drag_end: callbacks.on_end,
                    ..GestureCallbacks::default()
                },
            ),
            DragAxis::Pan => (
                PanAction::Pan,
                GestureCallbacks {
                    on_pan_start: callbacks.on_start,
                    on_pan_update: callbacks.on_update,
                    on_pan_end: callbacks.on_end,
                    ..GestureCallbacks::default()
                },
            ),
        };
        Self {
            recognizer: PointerGestureRecognizer::new(callbacks),
            axis,
            contacts: Vec::new(),
            first: None,
            active: None,
            observed: None,
            wants_acceptance: false,
            on_down: None,
            on_cancel: None,
        }
    }

    #[must_use]
    pub fn with_drag_start_behavior(mut self, behavior: DragStartBehavior) -> Self {
        self.recognizer.callbacks.drag_start_behavior = behavior;
        self
    }

    /// Sets the callbacks for the first contact and for losing before acceptance.
    #[must_use]
    pub fn with_pending_callbacks(
        mut self,
        on_down: Option<Rc<dyn Fn(DragDownDetails)>>,
        on_cancel: Option<Rc<dyn Fn()>>,
    ) -> Self {
        self.on_down = on_down;
        self.on_cancel = on_cancel;
        self
    }

    pub fn set_local_transform(&mut self, transform: incular_core::Transform) {
        self.recognizer.set_local_transform(transform);
    }

    pub fn set_gesture_settings(&mut self, settings: DeviceGestureSettings) {
        self.recognizer.set_gesture_settings(settings);
    }

    /// Routes an absolute trackpad stream to this recognizer. A pending
    /// contender requests arbitration rather than exposing updates.
    pub fn observe(&mut self, event: PointerPanZoomEvent) -> bool {
        let stream = Stream {
            device: event.device,
            pointer: event.pointer,
            absolute: true,
        };
        self.observing(stream);
        match event.phase {
            PointerPanZoomPhase::Start => self.begin(
                stream,
                PointerDeviceKind::Trackpad,
                event.position,
                Offset::ZERO,
                event.time,
                event.synthesized,
            ),
            PointerPanZoomPhase::Update { pan, .. } => self.step(
                stream,
                event.position + pan,
                pan,
                event.time,
                event.synthesized,
            ),
            PointerPanZoomPhase::End => self.finish(stream, Some(event.time)),
        }
    }

    /// Routes a physical pointer to the same drag tenure. A normal drag
    /// admits the primary button only; changing the chord drops that contact.
    pub fn observe_raw(&mut self, event: RawPointerEvent) -> bool {
        let stream = Stream {
            device: event.device,
            pointer: event.pointer,
            absolute: false,
        };
        self.observing(stream);
        match event.phase {
            PointerPhase::Down if event.buttons == PRIMARY_POINTER_BUTTON => self.begin(
                stream,
                event.kind,
                event.position,
                event.position,
                event.time,
                event.synthesized,
            ),
            PointerPhase::Move if event.buttons == PRIMARY_POINTER_BUTTON => self.step(
                stream,
                event.position,
                event.position,
                event.time,
                event.synthesized,
            ),
            PointerPhase::Move | PointerPhase::Up | PointerPhase::Cancel => {
                self.finish(stream, Some(event.time))
            }
            _ => false,
        }
    }

    fn observing(&mut self, stream: Stream) {
        self.observed = Some(stream);
        self.wants_acceptance = false;
    }

    fn begin(
        &mut self,
        stream: Stream,
        kind: PointerDeviceKind,
        position: Offset,
        motion: Offset,
        time: Instant,
        synthesized: bool,
    ) -> bool {
        if self.contacts.iter().any(|contact| contact.stream == stream) {
            return false;
        }
        let inner = &mut self.recognizer;
        let mut velocity = VelocityTracker::default();
        if !synthesized {
            let sample = if stream.absolute {
                motion
            } else {
                inner.local(position)
            };
            velocity.add_position(time, sample);
        }
        if self.contacts.is_empty() {
            self.first = Some(PointerEvent {
                pointer: stream.pointer,
                position,
                phase: PointerPhase::Down,
                time,
            });
            inner.drag = DragProgress::at(position);
            inner.drag.pending_time = Some(time);
            if let Some(down) = &self.on_down {
                down(DragDownDetails {
                    global_position: position,
                    local_position: inner.local(position),
                });
            }
        }
        self.contacts.push(Contact {
            stream,
            kind,
            previous: motion,
            velocity,
            accepted: false,
        });
        self.wants_acceptance = inner.pan_started;
        true
    }

    fn step(
        &mut self,
        stream: Stream,
        position: Offset,
        motion: Offset,
        time: Instant,
        synthesized: bool,
    ) -> bool {
        let Some(contact) = self
            .contacts
            .iter_mut()
            .find(|contact| contact.stream == stream)
        else {
            return false;
        };
        let inner = &mut self.recognizer;
        if !synthesized {
            let sample = if stream.absolute {
                motion
            } else {
                inner.local(position)
            };
            contact.velocity.add_position(time, sample);
        }
        let step = motion - contact.previous;
        contact.previous = motion;
        let local_step = inner.local_delta(step);
        if inner.pan_started {
            if !contact.accepted {
                self.wants_acceptance = true;
                return true;
            }
            if self.active == Some(stream) {
                inner.kind = contact.kind;
                inner.drag.last_position = position;
                let update = inner.drag_update_local(
                    self.axis,
                    local_step,
                    position,
                    inner.local(position),
                    time,
                );
                inner.dispatch(update);
            }
            return true;
        }
        inner.drag.last_position = position;
        inner.drag.pending = inner.drag.pending + step;
        inner.drag.pending_local = inner.drag.pending_local + local_step;
        inner.drag.pending_time = Some(time);
        let (distance, slop) = match self.axis {
            PanAction::Horizontal => (
                f64::from(inner.drag.pending_local.x.abs()) * inner.world_axis_scale[0],
                compute_hit_slop(contact.kind, inner.gesture_settings),
            ),
            PanAction::Vertical => (
                f64::from(inner.drag.pending_local.y.abs()) * inner.world_axis_scale[1],
                compute_hit_slop(contact.kind, inner.gesture_settings),
            ),
            PanAction::Pan => {
                inner.drag.moved_pan += step.x.hypot(step.y);
                (
                    f64::from(inner.drag.moved_pan),
                    compute_pan_slop(contact.kind, inner.gesture_settings),
                )
            }
        };
        self.wants_acceptance = distance > f64::from(slop);
        true
    }

    fn finish(&mut self, stream: Stream, release: Option<Instant>) -> bool {
        let Some(index) = self
            .contacts
            .iter()
            .position(|contact| contact.stream == stream)
        else {
            return false;
        };
        let contact = self.contacts.remove(index);
        if self.active == Some(stream) {
            self.active = self
                .contacts
                .iter()
                .find(|contact| contact.accepted)
                .map(|contact| contact.stream);
        }
        if !self.contacts.is_empty() {
            return true;
        }
        self.first = None;
        let inner = &mut self.recognizer;
        if !inner.pan_started {
            if let Some(cancel) = &self.on_cancel {
                cancel();
            }
        } else if let Some(time) = release {
            inner.kind = contact.kind;
            inner.velocity = contact.velocity;
            if let Some(end) = inner.drag_end(time) {
                inner.dispatch(end);
            }
        } else {
            inner.pan_started = false;
            inner.pan_action = None;
        }
        true
    }

    #[must_use]
    pub const fn wants_acceptance(&self) -> bool {
        self.wants_acceptance
    }

    /// Grants arena acceptance to the last routed contact. Hosts with a shared
    /// recognizer should use `accept_stream` for explicit stream ownership.
    pub fn accept(&mut self) {
        if let Some(stream) = self.observed {
            self.accept_contact(stream);
        }
    }

    /// Grants acceptance to a particular physical or absolute contact.
    pub fn accept_stream(&mut self, key: GestureArenaKey) {
        if let Some(stream) = Stream::from_arena(key) {
            self.accept_contact(stream);
        }
    }

    fn accept_contact(&mut self, stream: Stream) {
        self.wants_acceptance = false;
        let Some(contact) = self
            .contacts
            .iter_mut()
            .find(|contact| contact.stream == stream)
        else {
            return;
        };
        if contact.accepted {
            return;
        }
        contact.accepted = true;
        self.active = Some(stream);
        let inner = &mut self.recognizer;
        if !inner.pan_started
            && let Some(first) = self.first
        {
            inner.kind = contact.kind;
            let start = inner.begin_drag(
                first,
                self.axis,
                &first,
                inner.callbacks.drag_start_behavior,
            );
            inner.dispatch(start);
            if let Some(update) = inner.take_start_update() {
                inner.dispatch(update);
            }
        }
    }

    /// Drops the matching absolute trackpad stream after arena loss.
    pub fn reject(&mut self, device: u64, pointer: u64) {
        self.reject_contact(Stream {
            device,
            pointer,
            absolute: true,
        });
    }

    /// Drops the named contact without ending a drag still held by another.
    pub fn reject_stream(&mut self, key: GestureArenaKey) {
        if let Some(stream) = Stream::from_arena(key) {
            self.reject_contact(stream);
        }
    }

    fn reject_contact(&mut self, stream: Stream) {
        if self.observed == Some(stream) {
            self.wants_acceptance = false;
        }
        self.finish(stream, None);
    }
}
