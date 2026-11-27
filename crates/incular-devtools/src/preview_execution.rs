//! Startup and shutdown gate for applying changes to a running preview host.

use std::{
    future::Future,
    sync::{Arc, Mutex},
};

use tokio::sync::Notify;

/// An operation for the already launched preview application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PreviewExecutionKind {
    /// Refreshes code or the published preview set without a full restart.
    HotReload,
    /// Reinitializes the running preview with its updated dependencies or state.
    HotRestart,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Status {
    #[default]
    WaitingForConnection,
    Connected,
    Stopped,
}

#[derive(Default)]
struct State {
    status: Status,
    pending: Option<PreviewExecutionKind>,
}

#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    changed: Notify,
}

/// Producer handle for source updates, explicit restart and host lifecycle.
#[derive(Clone)]
pub struct PreviewExecutionRequests(Arc<Shared>);

impl PreviewExecutionRequests {
    /// Records the operation unless this preview session has stopped.
    /// A queued restart subsumes reload requests, not an in-flight reload.
    pub fn request(&self, kind: PreviewExecutionKind) -> bool {
        let mut state = self.0.state.lock().expect("preview execution state");
        if state.status == Status::Stopped {
            return false;
        }
        let pending = Some(state.pending.map_or(kind, |queued| queued.max(kind)));
        let changed = pending != state.pending;
        state.pending = pending;
        let ready = state.status == Status::Connected;
        drop(state);
        if changed && ready {
            self.0.changed.notify_one();
        }
        true
    }

    /// Releases any queued operation after the debug connection is usable.
    /// Repeating or delivering this signal after shutdown has no effect.
    pub fn connected(&self) -> bool {
        let mut state = self.0.state.lock().expect("preview execution state");
        if state.status != Status::WaitingForConnection {
            return false;
        }
        state.status = Status::Connected;
        let pending = state.pending.is_some();
        drop(state);
        if pending {
            self.0.changed.notify_one();
        }
        true
    }

    /// Drops queued operations and rejects new ones. An operation already
    /// handed to the host retains its normal completion behavior.
    pub fn stop(&self) {
        let mut state = self.0.state.lock().expect("preview execution state");
        state.status = Status::Stopped;
        state.pending = None;
        drop(state);
        self.0.changed.notify_one();
    }
}

/// The single consumer for one preview session. Use its producer handle
/// from the watcher, debug connection and host shutdown callbacks.
pub struct PreviewExecution {
    requests: PreviewExecutionRequests,
}

impl Default for PreviewExecution {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewExecution {
    /// Creates a session that waits for a debugger or live connection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            requests: PreviewExecutionRequests(Arc::new(Shared::default())),
        }
    }

    /// Clones the producer side for independent host and watcher callbacks.
    #[must_use]
    pub fn requests(&self) -> PreviewExecutionRequests {
        self.requests.clone()
    }

    /// Waits for one eligible operation, then waits for its host result.
    /// Using one mutable consumer prevents overlapping operations. When the
    /// host reports an error, callers can report it and continue the loop.
    /// Returns `None` when the session stops instead of calling the host.
    pub async fn perform_next<F: Future>(
        &mut self,
        perform: impl FnOnce(PreviewExecutionKind) -> F,
    ) -> Option<F::Output> {
        let shared = &self.requests.0;
        let kind = loop {
            let pending = {
                let mut state = shared.state.lock().expect("preview execution state");
                match state.status {
                    Status::Stopped => return None,
                    Status::Connected => state.pending.take(),
                    Status::WaitingForConnection => None,
                }
            };
            if let Some(kind) = pending {
                break kind;
            }
            shared.changed.notified().await;
        };
        Some(perform(kind).await)
    }
}

impl Drop for PreviewExecution {
    fn drop(&mut self) {
        self.requests.stop();
    }
}
