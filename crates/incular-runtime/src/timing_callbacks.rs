//! Callback delivery for completed frame timing reports.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::{Rc, Weak},
};

use crate::FrameRecord;

type Callback = Rc<dyn Fn(&[FrameRecord])>;
type ErrorHandler = Rc<dyn Fn(&FrameTimingCallbackError)>;

#[derive(Default)]
struct State {
    next: u64,
    callbacks: BTreeMap<u64, Callback>,
    on_error: Option<ErrorHandler>,
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
    }
}

/// Keeps a timing callback subscribed. Dropping it removes only that callback.
pub struct FrameTimingSubscription {
    id: u64,
    owner: Weak<RefCell<State>>,
}

impl Drop for FrameTimingSubscription {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.upgrade() {
            let callback = owner.borrow_mut().callbacks.remove(&self.id);
            drop(callback);
        }
    }
}
