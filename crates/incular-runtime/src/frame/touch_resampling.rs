//! Optional touch-only sampling at the retained runtime's frame clock.

use std::time::Duration;

use incular_core::{PointerDeviceKind, time::Instant};
use incular_widgets::internal::{PointerEventResampler, RawPointerEvent};

pub(super) const DEFAULT_DELAY: Duration = Duration::from_millis(38);
const INTERVAL: Duration = Duration::from_micros(16_667);

type Contact = (u64, u64);

#[derive(Default)]
pub(super) struct TouchResampling {
    pub(super) delay: Option<Duration>,
    pub(super) preferred_delay: Option<Duration>,
    last_sample: Option<Instant>,
    pub(super) deadline: Option<Instant>,
    streams: Vec<(Contact, PointerEventResampler)>,
    pub(super) ready: Vec<RawPointerEvent>,
}

impl TouchResampling {
    pub(super) fn enqueue(&mut self, event: RawPointerEvent) -> bool {
        if self.delay.is_none()
            || event.kind != PointerDeviceKind::Touch
            || event.scroll_delta != incular_core::Offset::ZERO
        {
            return false;
        }
        let contact = (event.device, event.pointer);
        if let Some((_, stream)) = self.streams.iter_mut().find(|(key, _)| *key == contact) {
            stream.add_event(event);
        } else {
            let mut stream = PointerEventResampler::new();
            stream.add_event(event);
            self.streams.push((contact, stream));
        }
        true
    }

    pub(super) fn sample(&mut self, frame_time: Instant) {
        let Some(delay) = self.delay else {
            return;
        };
        if let Some(at) = frame_time.checked_sub(delay)
            && self.last_sample.is_none_or(|previous| at >= previous)
        {
            self.last_sample = Some(at);
            let next = at.checked_add(INTERVAL);
            for (_, stream) in &mut self.streams {
                stream.sample(at, next, |event| self.ready.push(event));
            }
            self.streams
                .retain(|(_, stream)| stream.has_pending_events() || stream.is_down());
            if self.streams.is_empty() {
                self.last_sample = None;
            }
        }
        // An idle finger with no new data does not wake the host needlessly.
        self.deadline = self
            .streams
            .iter()
            .any(|(_, stream)| stream.has_pending_events())
            .then(|| frame_time.checked_add(INTERVAL))
            .flatten();
    }

    pub(super) fn due_tick(&self, now: Instant) -> Option<Instant> {
        let deadline = self.deadline.filter(|deadline| now >= *deadline)?;
        let elapsed = now.duration_since(deadline).as_micros();
        let period = INTERVAL.as_micros();
        let aligned = elapsed / period * period;
        let extra = u64::try_from(aligned).ok().map(Duration::from_micros);
        Some(
            extra
                .and_then(|extra| deadline.checked_add(extra))
                .unwrap_or(now),
        )
    }

    pub(super) fn stop(&mut self) {
        self.deadline = None;
        self.last_sample = None;
        for (_, mut stream) in self.streams.drain(..) {
            stream.stop(|event| self.ready.push(event));
        }
        // Flushing a multi-contact queue should keep original time order,
        // with stable ordering for equal timestamps within a contact.
        self.ready.sort_by_key(|event| event.time);
    }
}
