//! Scoped performance preference requests on the scheduler thread.
//!
//! This coordinates preferences, not GPU adapter selection or Dart VM
//! tuning. The retained renderer and the host still own their policies.

use std::{cell::Cell, marker::PhantomData, rc::Rc};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PerformanceMode {
    #[default]
    Balanced,
    Latency,
    Throughput,
    Memory,
}

#[derive(Clone, Copy, Default)]
struct Requests {
    mode: Option<PerformanceMode>,
    count: usize,
}

thread_local! {
    static REQUESTS: Cell<Requests> = const { Cell::new(Requests { mode: None, count: 0 }) };
}

/// Keeps one preference active until it is dropped or disposed. Handles
/// cannot be sent to another scheduler thread.
#[must_use = "the request is active only while the handle is held"]
pub struct PerformanceModeRequestHandle {
    _thread: PhantomData<Rc<()>>,
    #[cfg(debug_assertions)]
    allocation: Option<incular_core::memory::allocations::ObjectLifetime>,
}

impl PerformanceModeRequestHandle {
    /// Releases this request exactly once; dropping the handle is equivalent.
    pub fn dispose(self) {}
}

impl Drop for PerformanceModeRequestHandle {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        drop(self.allocation.take());
        let _ = REQUESTS.try_with(|requests| {
            let mut state = requests.get();
            state.count = state
                .count
                .checked_sub(1)
                .expect("active performance request");
            if state.count == 0 {
                state.mode = None;
            }
            requests.set(state);
        });
    }
}

/// Acquires a scoped scheduler performance preference. Compatible requests
/// share it; a conflicting request is rejected without changing the state.
pub fn request_performance_mode(mode: PerformanceMode) -> Option<PerformanceModeRequestHandle> {
    let accepted = REQUESTS.with(|requests| {
        let mut state = requests.get();
        if state.mode.is_some_and(|current| current != mode) {
            return false;
        }
        state.mode = Some(mode);
        state.count = state
            .count
            .checked_add(1)
            .expect("too many performance requests");
        requests.set(state);
        true
    });
    accepted.then(|| PerformanceModeRequestHandle {
        _thread: PhantomData,
        #[cfg(debug_assertions)]
        allocation: Some(incular_core::memory::allocations::ObjectLifetime::new(
            "scheduler",
            "PerformanceModeRequestHandle",
        )),
    })
}

/// Reports an explicit request in debug builds, or None otherwise.
#[must_use]
pub fn debug_requested_performance_mode() -> Option<PerformanceMode> {
    if cfg!(debug_assertions) {
        REQUESTS.with(|requests| requests.get().mode)
    } else {
        None
    }
}

/// Checks for a leaked request in debug builds.
#[track_caller]
pub fn debug_assert_no_pending_performance_mode_requests(reason: &str) -> bool {
    debug_assert!(debug_requested_performance_mode().is_none(), "{reason}");
    true
}
