//! Optional single-device pointer resampling for frame-clock driven hosts.

use std::collections::VecDeque;

use incular_core::{Offset, PointerPhase, time::Instant};

use crate::RawPointerEvent;

/// Accumulates raw samples for one pointer stream and emits motion at
/// requested frame times. Enter and Exit are the platform's tracking
/// boundaries. The caller owns the sampling cadence and latency policy,
/// and routes scroll/trackpad signals separately.
///
/// Resampling changes positions, motion deltas and clocks, not pressure or
/// other hardware metadata. Multiple streams use separate instances.
#[derive(Default)]
pub struct PointerEventResampler {
    queued: VecDeque<RawPointerEvent>,
    previous: Option<RawPointerEvent>,
    next: Option<RawPointerEvent>,
    position: Offset,
    is_tracked: bool,
    is_down: bool,
    pointer: u64,
    buttons: u32,
}

impl PointerEventResampler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a raw sample in delivery order. Equal timestamps preserve that
    /// order, so a same-time press and release are never collapsed.
    pub fn add_event(&mut self, event: RawPointerEvent) {
        self.queued.push_back(event);
    }

    #[must_use]
    pub fn has_pending_events(&self) -> bool {
        !self.queued.is_empty()
    }

    #[must_use]
    pub const fn is_tracked(&self) -> bool {
        self.is_tracked
    }

    #[must_use]
    pub const fn is_down(&self) -> bool {
        self.is_down
    }

    /// Emits events at `at`, interpolating only between known samples.
    /// A supplied next frame permits an early Up/Exit before that frame,
    /// but cannot pull a future Down or Cancel forward.
    pub fn sample(
        &mut self,
        at: Instant,
        next_frame: Option<Instant>,
        mut emit: impl FnMut(RawPointerEvent),
    ) {
        self.prepare_samples(at);
        let position = self.position_at(at);
        let mut through = at;
        for event in &self.queued {
            if event.time <= at {
                continue;
            }
            if next_frame.is_none_or(|next| event.time >= next) {
                break;
            }
            match event.phase {
                PointerPhase::Up | PointerPhase::Exit => through = event.time,
                PointerPhase::Move => {}
                _ => break,
            }
        }
        while self
            .queued
            .front()
            .is_some_and(|event| event.time <= through)
        {
            let event = self.queued.pop_front().expect("checked queue front");
            let was_tracked = self.is_tracked;
            let was_down = self.is_down;
            let had_buttons = self.buttons;
            self.is_tracked = event.phase != PointerPhase::Exit;
            self.is_down = event.down;
            self.buttons = event.buttons;
            self.pointer = event.pointer;
            if self.is_tracked && !was_tracked {
                self.position = position;
            }
            if event.phase != PointerPhase::Move {
                self.emit_motion_if_changed(event, at, position, was_down, had_buttons, &mut emit);
                emit(at_position(event, at, position, Offset::ZERO));
            }
        }
        if self.is_tracked
            && let Some(next) = self.next
        {
            self.emit_motion_if_changed(next, at, position, self.is_down, self.buttons, &mut emit);
        }
    }

    /// Delivers pending input unchanged, then resets the resampler so the
    /// next tracking sequence cannot inherit this one's interpolation.
    pub fn stop(&mut self, mut emit: impl FnMut(RawPointerEvent)) {
        while let Some(event) = self.queued.pop_front() {
            emit(event);
        }
        self.previous = None;
        self.next = None;
        self.position = Offset::ZERO;
        self.is_tracked = false;
        self.is_down = false;
        self.pointer = 0;
        self.buttons = 0;
    }

    fn prepare_samples(&mut self, at: Instant) {
        for &event in &self.queued {
            if event.time <= at || self.previous.is_none() {
                self.previous = Some(event);
                self.next = Some(event);
            } else if self.next.is_some_and(|next| next.time < at) {
                self.next = Some(event);
                break;
            }
        }
    }

    fn position_at(&self, at: Instant) -> Offset {
        let Some(next) = self.next else {
            return Offset::ZERO;
        };
        let Some(previous) = self.previous else {
            return next.position;
        };
        if next.time <= at || next.time <= previous.time {
            return next.position;
        }
        let interval = next.time.duration_since(previous.time).as_secs_f64();
        let offset = if at >= previous.time {
            at.duration_since(previous.time).as_secs_f64()
        } else {
            -previous.time.duration_since(at).as_secs_f64()
        };
        let fraction = offset / interval;
        let blend = |a: f32, b: f32| {
            let a = f64::from(a);
            (a + (f64::from(b) - a) * fraction) as f32
        };
        Offset::new(
            blend(previous.position.x, next.position.x),
            blend(previous.position.y, next.position.y),
        )
    }

    fn emit_motion_if_changed(
        &mut self,
        origin: RawPointerEvent,
        at: Instant,
        position: Offset,
        down: bool,
        buttons: u32,
        emit: &mut impl FnMut(RawPointerEvent),
    ) {
        if position == self.position {
            return;
        }
        let delta = position - self.position;
        self.position = position;
        let event = RawPointerEvent {
            // Hover without a contact uses no contact identifier.
            pointer: if down { self.pointer } else { 0 },
            phase: PointerPhase::Move,
            down,
            buttons: if down { buttons } else { origin.buttons },
            button: None,
            ..origin
        };
        emit(at_position(event, at, position, delta));
    }
}

fn at_position(
    mut event: RawPointerEvent,
    at: Instant,
    position: Offset,
    delta: Offset,
) -> RawPointerEvent {
    event.time = at;
    event.position = position;
    event.local_position = position;
    event.delta = delta;
    event.local_delta = delta;
    event
}
