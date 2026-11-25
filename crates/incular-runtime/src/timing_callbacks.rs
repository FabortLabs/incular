//! Callback delivery for completed frame timing reports.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::{Rc, Weak},
};

use crate::FrameRecord;

type Callback = Rc<dyn Fn(&[FrameRecord])>;
type TimelineCallback = Rc<dyn Fn(FrameTimelineEvent)>;
type ErrorHandler = Rc<dyn Fn(&FrameTimingCallbackError)>;

#[derive(Default)]
struct State {
    next: u64,
    callbacks: BTreeMap<u64, Callback>,
    timeline_callbacks: BTreeMap<u64, TimelineCallback>,
    on_error: Option<ErrorHandler>,
}

/// Monotonic phase timestamps supplied by a host with native timing data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeFrameTiming {
    pub frame_number: u64,
    pub vsync_start_us: u64,
    pub build_start_us: u64,
    pub build_finish_us: u64,
    pub raster_start_us: u64,
    pub raster_finish_us: u64,
}

/// The scheduler diagnostic payload for one completed frame. Values are
/// microseconds; start_time is the host's monotonic build-start timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameTimelineEvent {
    pub number: u64,
    pub start_time: u64,
    pub elapsed: u64,
    pub build: u64,
    pub raster: u64,
    pub vsync_overhead: u64,
}

impl FrameTimelineEvent {
    pub const NAME: &'static str = "Incular.Frame";
}

impl NativeFrameTiming {
    /// Converts a host sample without inventing unavailable timestamps.
    /// A report with phases out of order is rejected.
    #[must_use]
    pub fn timeline_event(self) -> Option<FrameTimelineEvent> {
        if self.raster_start_us < self.build_finish_us {
            return None;
        }
        Some(FrameTimelineEvent {
            number: self.frame_number,
            start_time: self.build_start_us,
            elapsed: self.raster_finish_us.checked_sub(self.vsync_start_us)?,
            build: self.build_finish_us.checked_sub(self.build_start_us)?,
            raster: self.raster_finish_us.checked_sub(self.raster_start_us)?,
            vsync_overhead: self.build_start_us.checked_sub(self.vsync_start_us)?,
        })
    }
}

/// A panic raised by a frame timing callback, isolated from other observers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameTimingCallbackError {
    message: String,
}

impl std::fmt::Display for FrameTimingCallbackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for FrameTimingCallbackError {}

/// Local observers of runtime frame timings, dispatched in registration order.
#[derive(Clone, Default)]
pub struct FrameTimingCallbacks(Rc<RefCell<State>>);

impl FrameTimingCallbacks {
    /// Subscribe until the returned handle is dropped.
    #[must_use]
    pub fn add(&self, callback: impl Fn(&[FrameRecord]) + 'static) -> FrameTimingSubscription {
        let mut state = self.0.borrow_mut();
        let id = state.next;
        state.next = state
            .next
            .checked_add(1)
            .expect("frame timing callback ID exhausted");
        state.callbacks.insert(id, Rc::new(callback));
        FrameTimingSubscription {
            id,
            owner: Rc::downgrade(&self.0),
            timeline: false,
        }
    }

    /// Observes separately reported native timeline events. Recording an
    /// ordinary CPU/GPU summary does not fabricate these timestamps.
    #[must_use]
    pub fn add_timeline(
        &self,
        callback: impl Fn(FrameTimelineEvent) + 'static,
    ) -> FrameTimingSubscription {
        let mut state = self.0.borrow_mut();
        let id = state.next;
        state.next = state
            .next
            .checked_add(1)
            .expect("frame timing callback ID exhausted");
        state.timeline_callbacks.insert(id, Rc::new(callback));
        FrameTimingSubscription {
            id,
            owner: Rc::downgrade(&self.0),
            timeline: true,
        }
    }

    /// Observe panics raised by timing callbacks. Without an error handler,
    /// failures are reported via tracing.
    pub fn set_error_handler(&self, handler: impl Fn(&FrameTimingCallbackError) + 'static) {
        let previous = self.0.borrow_mut().on_error.replace(Rc::new(handler));
        drop(previous);
    }

    pub fn clear_error_handler(&self) {
        let previous = self.0.borrow_mut().on_error.take();
        drop(previous);
    }

    /// Deliver one host batch, including an empty report. Subscriptions added
    /// during dispatch wait until the next report. Callbacks removed before
    /// their turn are not invoked.
    pub fn report(&self, frames: &[FrameRecord]) {
        let ids: Vec<_> = self.0.borrow().callbacks.keys().copied().collect();
        for id in ids {
            let callback = self.0.borrow().callbacks.get(&id).cloned();
            let Some(callback) = callback else { continue };
            if let Err(payload) = catch_unwind(AssertUnwindSafe(|| callback(frames))) {
                self.report_error(payload);
            }
        }
    }
    /// Dispatches one timeline event for each valid host sample, in order.
    /// A new observer joins the next batch, not the current one.
    pub fn report_native_timeline(&self, timings: &[NativeFrameTiming]) {
        let ids: Vec<_> = self.0.borrow().timeline_callbacks.keys().copied().collect();
        for event in timings.iter().filter_map(|timing| timing.timeline_event()) {
            for &id in &ids {
                let callback = self.0.borrow().timeline_callbacks.get(&id).cloned();
                if let Some(callback) = callback
                    && let Err(payload) = catch_unwind(AssertUnwindSafe(|| callback(event)))
                {
                    self.report_error(payload);
                }
            }
        }
    }

    fn report_error(&self, payload: Box<dyn std::any::Any + Send>) {
        let failure = FrameTimingCallbackError {
            message: payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("frame timing callback panicked")
                .to_owned(),
        };
        let handler = self.0.borrow().on_error.clone();
        if let Some(handler) = handler {
            handler(&failure);
        } else {
            tracing::error!(target: "incular::frame", error = %failure, "frame timing observer");
        }
    }
}

/// Keeps a timing callback subscribed. Dropping it removes only that callback.
pub struct FrameTimingSubscription {
    id: u64,
    owner: Weak<RefCell<State>>,
    timeline: bool,
}

impl Drop for FrameTimingSubscription {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.upgrade() {
            if self.timeline {
                let callback = owner.borrow_mut().timeline_callbacks.remove(&self.id);
                drop(callback);
            } else {
                let callback = owner.borrow_mut().callbacks.remove(&self.id);
                drop(callback);
            }
        }
    }
}
