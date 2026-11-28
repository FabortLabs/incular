//! Physical pointer packets at the embedder boundary. Already-logical
//! native events continue to use the ordinary raw pointer path.

use incular_core::{
    Matrix4, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPanZoomEvent,
    PointerPanZoomPhase, PointerPhase, PointerSampleMetadata, time::Instant,
};

use crate::{OffsetPair, RawPointerEvent, details::project_motion};

/// The change in a physical input record. A contact move and a hover
/// are distinct even when both happen to carry an empty button mask.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PointerDataChange {
    #[default]
    Cancel,
    Add,
    Remove,
    Hover,
    Down,
    Move,
    Up,
    PanZoomStart,
    PanZoomUpdate {
        physical_pan: Offset,
        physical_pan_delta: Offset,
        scale: f32,
        rotation: f32,
    },
    PanZoomEnd,
}

/// A signal takes precedence over the accompanying change.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PointerDataSignal {
    #[default]
    None,
    Scroll(Offset),
    ScrollInertiaCancel,
    Scale(f32),
    Unknown,
}

/// One engine-style record, before conversion to framework pixels.
/// Contact radii in `sample.contact` are physical pixels here. As in
/// Flutter's input protocol, pressure, distance, size and angles are
/// independent of the view's device pixel ratio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerData {
    pub view_id: u64,
    pub device: u64,
    pub pointer: u64,
    pub embedder_id: u64,
    pub platform_data: i64,
    pub change: PointerDataChange,
    pub signal: PointerDataSignal,
    pub kind: PointerDeviceKind,
    pub buttons: u32,
    pub physical_position: Offset,
    pub physical_delta: Offset,
    pub sample: PointerSampleMetadata,
    pub synthesized: bool,
    pub time: Instant,
}

impl PointerData {
    /// Construct a record with engine defaults (touch, no signal, no buttons).
    #[must_use]
    pub const fn new(change: PointerDataChange, time: Instant) -> Self {
        Self {
            view_id: 0,
            device: 0,
            pointer: 0,
            embedder_id: 0,
            platform_data: 0,
            change,
            signal: PointerDataSignal::None,
            kind: PointerDeviceKind::Touch,
            buttons: 0,
            physical_position: Offset::ZERO,
            physical_delta: Offset::ZERO,
            sample: PointerSampleMetadata::EMPTY,
            synthesized: false,
            time,
        }
    }
}

/// The contact form before it is mapped to the coarser retained pointer phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvertedPointerChange {
    Add,
    Remove,
    Hover,
    Down,
    Move,
    Up,
    Cancel,
}

/// A framework event in logical pixels. Scroll can be wrapped in a
/// [`crate::PointerSignalEvent`] if the embedder supplies a response callback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConvertedPointerKind {
    Contact {
        change: ConvertedPointerChange,
        event: RawPointerEvent,
    },
    Scroll(RawPointerEvent),
    ScrollInertiaCancel(RawPointerEvent),
    Scale {
        event: RawPointerEvent,
        scale: f32,
    },
    PanZoom {
        event: PointerPanZoomEvent,
        pan_delta: Offset,
    },
}

/// View-scoped framework input, with the original embedder identity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConvertedPointerEvent {
    pub view_id: u64,
    pub embedder_id: u64,
    /// Opaque platform payload. Only contact motion carries it.
    pub platform_data: i64,
    pub kind: ConvertedPointerKind,
}

impl ConvertedPointerEvent {
    /// Borrows this event in the coordinate space of a receiving target. A
    /// projection does not replace the source or copy its contact metadata.
    #[must_use]
    pub const fn projection<'a>(
        &'a self,
        transform: Option<&'a Matrix4>,
    ) -> PointerEventProjection<'a> {
        PointerEventProjection::new(self, transform)
    }
}

/// Local and global coordinates for one point and the motion that reached it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerMotionCoordinates {
    pub position: OffsetPair,
    pub delta: OffsetPair,
}

/// A borrowed view of a pointer event in another coordinate space. Multiple
/// listeners can project the same source without allocating or composing
/// their matrices. The source can be a converted packet or a raw sample.
#[derive(Clone, Copy, Debug)]
pub struct PointerEventProjection<'a, Event = ConvertedPointerEvent> {
    source: &'a Event,
    transform: Option<&'a Matrix4>,
}

impl<'a, Event> PointerEventProjection<'a, Event> {
    pub(crate) const fn new(source: &'a Event, transform: Option<&'a Matrix4>) -> Self {
        Self { source, transform }
    }

    /// The source event, with its global coordinates and untouched metadata.
    #[must_use]
    pub const fn source(self) -> &'a Event {
        self.source
    }

    /// The original event of a transformed view, or none in global space.
    #[must_use]
    pub fn original(self) -> Option<&'a Event> {
        self.transform.map(|_| self.source)
    }

    #[must_use]
    pub const fn transform(self) -> Option<&'a Matrix4> {
        self.transform
    }

    fn map_motion(&self, position: Offset, delta: Offset) -> PointerMotionCoordinates {
        let (local_position, local_delta) = self.transform.map_or((position, delta), |transform| {
            project_motion(*transform, position, delta)
        });
        PointerMotionCoordinates {
            position: OffsetPair::new(local_position, position),
            delta: OffsetPair::new(local_delta, delta),
        }
    }
}

impl PointerEventProjection<'_, RawPointerEvent> {
    /// Pointer position and movement in the original and target spaces.
    #[must_use]
    pub fn coordinates(self) -> PointerMotionCoordinates {
        self.map_motion(self.source.position, self.source.delta)
    }
}

impl PointerEventProjection<'_> {
    /// Pointer position and movement in both source and target coordinates.
    /// Wheel motion and cumulative pan are distinct from pointer movement.
    #[must_use]
    pub fn coordinates(self) -> PointerMotionCoordinates {
        let (position, delta) = match self.source.kind {
            ConvertedPointerKind::Contact { event, .. }
            | ConvertedPointerKind::Scroll(event)
            | ConvertedPointerKind::ScrollInertiaCancel(event)
            | ConvertedPointerKind::Scale { event, .. } => (event.position, event.delta),
            ConvertedPointerKind::PanZoom { event, .. } => (event.position, Offset::ZERO),
        };
        self.map_motion(position, delta)
    }

    /// Absolute pan and measured pan delta of a pan/zoom update in both
    /// spaces. Scale and rotation remain on the original event.
    #[must_use]
    pub fn pan_coordinates(self) -> Option<PointerMotionCoordinates> {
        match self.source.kind {
            ConvertedPointerKind::PanZoom {
                event:
                    PointerPanZoomEvent {
                        phase: PointerPanZoomPhase::Update { pan, .. },
                        ..
                    },
                pan_delta,
            } => Some(self.map_motion(pan, pan_delta)),
            _ => None,
        }
    }
}

/// Converts physical events without collecting or dispatching them.
pub struct PointerEventConverter;

impl PointerEventConverter {
    /// Expand records in order. The ratio is looked up when each record is
    /// pulled, never for unknown signals. Missing views, invalid ratios, and
    /// malformed scrolling deltas do not produce framework events.
    pub fn expand(
        data: impl IntoIterator<Item = PointerData>,
        mut device_pixel_ratio_for_view: impl FnMut(u64) -> Option<f64>,
    ) -> impl Iterator<Item = ConvertedPointerEvent> {
        data.into_iter().filter_map(move |datum| {
            if datum.signal == PointerDataSignal::Unknown {
                return None;
            }
            let ratio = device_pixel_ratio_for_view(datum.view_id)?;
            if !ratio.is_finite() || ratio <= 0.0 {
                return None;
            }
            convert(datum, ratio)
        })
    }
}

fn convert(datum: PointerData, ratio: f64) -> Option<ConvertedPointerEvent> {
    use ConvertedPointerChange as Change;
    use PointerDataChange as DataChange;
    use PointerDataSignal as Signal;

    let logical_length = |value: f32| (f64::from(value) / ratio) as f32;
    let logical = |point: Offset| Offset::new(logical_length(point.x), logical_length(point.y));
    let position = logical(datum.physical_position);
    let raw_event = |pointer, buttons, phase| {
        RawPointerEvent::new(
            pointer,
            datum.device,
            datum.kind,
            buttons,
            position,
            phase,
            datum.time,
        )
    };
    let mut platform_data = 0;
    let kind = match datum.signal {
        Signal::Unknown => return None,
        Signal::Scroll(physical_scroll) => {
            let scroll_delta = logical(physical_scroll);
            if !scroll_delta.x.is_finite() || !scroll_delta.y.is_finite() {
                return None;
            }
            ConvertedPointerKind::Scroll(RawPointerEvent {
                scroll_delta,
                local_scroll_delta: scroll_delta,
                ..raw_event(0, 0, PointerPhase::Move)
            })
        }
        Signal::ScrollInertiaCancel => {
            ConvertedPointerKind::ScrollInertiaCancel(raw_event(0, 0, PointerPhase::Move))
        }
        Signal::Scale(scale) => ConvertedPointerKind::Scale {
            event: raw_event(0, 0, PointerPhase::Move),
            scale,
        },
        Signal::None => match datum.change {
            DataChange::PanZoomStart
            | DataChange::PanZoomUpdate { .. }
            | DataChange::PanZoomEnd => {
                let (phase, pan_delta) = match datum.change {
                    DataChange::PanZoomStart => (PointerPanZoomPhase::Start, Offset::ZERO),
                    DataChange::PanZoomUpdate {
                        physical_pan,
                        physical_pan_delta,
                        scale,
                        rotation,
                    } => (
                        PointerPanZoomPhase::Update {
                            pan: logical(physical_pan),
                            scale,
                            rotation,
                        },
                        logical(physical_pan_delta),
                    ),
                    _ => (PointerPanZoomPhase::End, Offset::ZERO),
                };
                ConvertedPointerKind::PanZoom {
                    event: PointerPanZoomEvent {
                        device: datum.device,
                        pointer: datum.pointer,
                        position,
                        phase,
                        time: datum.time,
                        synthesized: datum.synthesized,
                    },
                    pan_delta,
                }
            }
            change => {
                // Trackpad movement is an aggregate pan/zoom or a signal,
                // not a down/move/up contact sequence. In particular, a bad
                // contact record must not cancel a valid hover or enter the
                // tap arena. Signals were selected before this branch.
                if datum.kind == PointerDeviceKind::Trackpad
                    && matches!(change, DataChange::Down | DataChange::Move | DataChange::Up)
                {
                    return None;
                }
                let (change, phase) = match change {
                    DataChange::Add => (Change::Add, PointerPhase::Enter),
                    DataChange::Remove => (Change::Remove, PointerPhase::Exit),
                    DataChange::Hover => (Change::Hover, PointerPhase::Move),
                    DataChange::Down => (Change::Down, PointerPhase::Down),
                    DataChange::Move => (Change::Move, PointerPhase::Move),
                    DataChange::Up => (Change::Up, PointerPhase::Up),
                    DataChange::Cancel => (Change::Cancel, PointerPhase::Cancel),
                    _ => unreachable!("pan/zoom changes have their own payload"),
                };
                let device_only = matches!(change, Change::Add | Change::Remove | Change::Hover);
                let boundary = matches!(change, Change::Add | Change::Remove);
                let moving = matches!(change, Change::Hover | Change::Move);
                let down = matches!(change, Change::Down | Change::Move);
                let buttons = if boundary {
                    0
                } else if down
                    && datum.buttons == 0
                    && !matches!(
                        datum.kind,
                        PointerDeviceKind::Mouse | PointerDeviceKind::Trackpad
                    )
                {
                    PRIMARY_POINTER_BUTTON
                } else {
                    datum.buttons
                };
                let mut event =
                    raw_event(if device_only { 0 } else { datum.pointer }, buttons, phase);
                event.down = down;
                event.synthesized = moving && datum.synthesized;
                if moving {
                    event.delta = logical(datum.physical_delta);
                    event.local_delta = event.delta;
                }
                let mut sample = datum.sample;
                let contact = &mut sample.contact;
                contact.radius_min = logical_length(contact.radius_min);
                contact.radius_max = logical_length(contact.radius_max);
                if boundary {
                    contact.size = 0.0;
                    contact.radius_major = 0.0;
                    contact.radius_minor = 0.0;
                } else {
                    contact.radius_major = logical_length(contact.radius_major);
                    contact.radius_minor = logical_length(contact.radius_minor);
                }
                if !matches!(change, Change::Down | Change::Move | Change::Up) {
                    sample.pressure = None;
                }
                if matches!(
                    change,
                    Change::Down | Change::Move | Change::Up | Change::Remove
                ) {
                    contact.distance = 0.0;
                }
                if change == Change::Remove {
                    contact.orientation = 0.0;
                    contact.tilt = 0.0;
                }
                if change == Change::Move {
                    platform_data = datum.platform_data;
                }
                event.sample = sample;
                ConvertedPointerKind::Contact { change, event }
            }
        },
    };
    Some(ConvertedPointerEvent {
        view_id: datum.view_id,
        embedder_id: datum.embedder_id,
        platform_data,
        kind,
    })
}
