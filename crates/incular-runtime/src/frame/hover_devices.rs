//! Last positioned sample per hover device, independent of touch input.

use incular_widgets::internal::RawPointerEvent;

#[derive(Clone, Copy)]
struct Sample {
    event: RawPointerEvent,
    recency: u64,
}

/// Keeps the common one-device case inline and iterates in connection
/// order. Recency is separate: the runtime exposes one window cursor.
#[derive(Default)]
pub(super) struct HoverDevices {
    first: Option<Sample>,
    rest: Vec<Sample>,
    recency: u64,
}

impl HoverDevices {
    pub(super) fn record(&mut self, event: RawPointerEvent) {
        let key = event.hover_device_identity();
        self.recency = self
            .recency
            .checked_add(1)
            .expect("hover sample order exhausted");
        let sample = Sample {
            event,
            recency: self.recency,
        };
        if let Some(previous) = self
            .first
            .iter_mut()
            .chain(&mut self.rest)
            .find(|previous| previous.event.hover_device_identity() == key)
        {
            *previous = sample;
        } else if self.first.is_none() {
            self.first = Some(sample);
        } else {
            self.rest.push(sample);
        }
    }

    /// Returns whether the device owned the single effective window cursor.
    pub(super) fn remove(&mut self, event: RawPointerEvent) -> bool {
        let key = event.hover_device_identity();
        let was_latest = self
            .latest()
            .is_some_and(|sample| sample.hover_device_identity() == key);
        if self
            .first
            .is_some_and(|sample| sample.event.hover_device_identity() == key)
        {
            self.first = (!self.rest.is_empty()).then(|| self.rest.remove(0));
        } else if let Some(index) = self
            .rest
            .iter()
            .position(|sample| sample.event.hover_device_identity() == key)
        {
            self.rest.remove(index);
        }
        was_latest
    }

    pub(super) fn latest(&self) -> Option<RawPointerEvent> {
        self.first
            .iter()
            .chain(&self.rest)
            .max_by_key(|sample| sample.recency)
            .map(|sample| sample.event)
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = RawPointerEvent> + '_ {
        self.first
            .iter()
            .chain(&self.rest)
            .map(|sample| sample.event)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.first.is_none()
    }
}
