//! Last positioned sample per hover device, independent of touch input.

use std::{cell::Cell, rc::Rc};

use incular_core::{ChangeNotifier, ChangeNotifierSubscription, PointerDeviceKind};
use incular_widgets::{MouseCursor, internal::RawPointerEvent};

/// A newly activated native cursor for one positioned hover device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseCursorUpdate {
    pub device: u64,
    pub pointer: u64,
    pub kind: PointerDeviceKind,
    pub cursor: MouseCursor,
}

#[derive(Clone, Copy)]
struct Sample {
    event: RawPointerEvent,
    recency: u64,
    cursor: Option<MouseCursor>,
}

/// Keeps the common one-device case inline and iterates in connection
/// order. Recency is separate: the runtime exposes one window cursor.
#[derive(Default)]
pub(super) struct HoverDevices {
    connection: Option<ConnectionListeners>,
    first: Option<Sample>,
    rest: Vec<Sample>,
    recency: u64,
}

struct ConnectionListeners {
    current: Rc<Cell<bool>>,
    changes: ChangeNotifier,
}

impl HoverDevices {
    pub(super) fn add_connection_listener(
        &mut self,
        callback: impl Fn(bool) + 'static,
    ) -> ChangeNotifierSubscription {
        let connected = !self.is_empty();
        let listeners = self.connection.get_or_insert_with(|| ConnectionListeners {
            current: Rc::new(Cell::new(connected)),
            changes: ChangeNotifier::new(),
        });
        let current = Rc::clone(&listeners.current);
        listeners
            .changes
            .add_listener(move || callback(current.get()))
    }

    pub(super) fn notify_connection_change(&self) {
        if let Some(listeners) = &self.connection {
            let connected = !self.is_empty();
            if listeners.current.replace(connected) != connected {
                listeners.changes.notify_listeners();
            }
        }
    }

    pub(super) fn record(&mut self, event: RawPointerEvent) {
        let key = event.hover_device_identity();
        self.recency = self
            .recency
            .checked_add(1)
            .expect("hover sample order exhausted");
        let sample = Sample {
            event,
            recency: self.recency,
            cursor: None,
        };
        if let Some(previous) = self
            .first
            .iter_mut()
            .chain(&mut self.rest)
            .find(|previous| previous.event.hover_device_identity() == key)
        {
            previous.event = event;
            previous.recency = sample.recency;
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

    pub(super) fn set_cursor(&mut self, event: RawPointerEvent, cursor: MouseCursor) -> bool {
        let key = event.hover_device_identity();
        let Some(sample) = self
            .first
            .iter_mut()
            .chain(&mut self.rest)
            .find(|sample| sample.event.hover_device_identity() == key)
        else {
            return false;
        };
        if sample.cursor == Some(cursor) {
            return false;
        }
        sample.cursor = Some(cursor);
        true
    }

    pub(super) fn latest_cursor(&self) -> Option<MouseCursor> {
        self.samples()
            .max_by_key(|sample| sample.recency)
            .and_then(|sample| sample.cursor)
    }

    pub(super) fn cursor_for(&self, device: u64, pointer: u64) -> Option<MouseCursor> {
        let key = (device, if device == 0 { pointer } else { 0 });
        self.samples()
            .find(|sample| sample.event.hover_device_identity() == key)
            .and_then(|sample| sample.cursor)
    }

    pub(super) fn sample_at(&self, index: usize) -> Option<RawPointerEvent> {
        (if index == 0 {
            self.first.as_ref()
        } else {
            self.rest.get(index - 1)
        })
        .map(|sample| sample.event)
    }

    fn samples(&self) -> impl Iterator<Item = &Sample> {
        self.first.iter().chain(&self.rest)
    }

    pub(super) fn latest(&self) -> Option<RawPointerEvent> {
        self.samples()
            .max_by_key(|sample| sample.recency)
            .map(|sample| sample.event)
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = RawPointerEvent> + '_ {
        self.samples().map(|sample| sample.event)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.first.is_none()
    }
}
