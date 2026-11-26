//! Ordered, disposable UI-thread notifications with owned registrations.

use crate::reactivity::DependencySource;
use std::{
    any::Any,
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    rc::{Rc, Weak},
};

type Callback = Rc<dyn Fn() + 'static>;

struct Listener {
    id: u64,
    callback: Callback,
}

#[derive(Default)]
struct State {
    listeners: Vec<Option<Listener>>,
    tracked: Option<DependencySource>,
    live_listeners: usize,
    depth: usize,
    next_id: u64,
    disposed: bool,
    #[cfg(debug_assertions)]
    observed: bool,
    #[cfg(debug_assertions)]
    allocation: Option<crate::memory::allocations::ObjectLifetime>,
}

impl State {
    fn assert_alive(&self) {
        assert!(
            !self.disposed,
            "A ChangeNotifier was used after being disposed.\nOnce you have called dispose() on a ChangeNotifier, it can no longer be used."
        );
    }

    fn remove(&mut self, id: u64) -> Option<Listener> {
        let index = self
            .listeners
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|listener| listener.id == id))?;
        self.live_listeners -= 1;
        if self.depth == 0 {
            self.listeners.remove(index)
        } else {
            // Keep indexes stable for every active notification.
            self.listeners[index].take()
        }
    }
}

/// Flutter's `ChangeNotifier` lifecycle with Rust-owned listener registrations.
/// Clones refer to the same notifier; disposing any clone closes them all.
#[derive(Clone, Default)]
pub struct ChangeNotifier(Rc<RefCell<State>>);

impl ChangeNotifier {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a callback in insertion order. Dropping the subscription
    /// removes just this registration, even after disposal.
    ///
    /// # Panics
    /// Panics if the notifier has been disposed.
    pub fn add_listener(&self, callback: impl Fn() + 'static) -> ChangeNotifierSubscription {
        #[cfg(debug_assertions)]
        self.observe_first_listener();
        let mut state = self.0.borrow_mut();
        state.assert_alive();
        let id = state.next_id;
        state.next_id = id
            .checked_add(1)
            .expect("ChangeNotifier listener IDs exhausted");
        state.listeners.push(Some(Listener {
            id,
            callback: Rc::new(callback),
        }));
        state.live_listeners += 1;
        ChangeNotifierSubscription {
            state: Rc::downgrade(&self.0),
            id,
        }
    }

    #[cfg(debug_assertions)]
    fn observe_first_listener(&self) {
        {
            let mut state = self.0.borrow_mut();
            state.assert_alive();
            if std::mem::replace(&mut state.observed, true) {
                return;
            }
        }
        // Allocation observers can themselves use or dispose this notifier.
        // Do not hold its state borrow while invoking them.
        let lifetime =
            crate::memory::allocations::ObjectLifetime::new("foundation", "ChangeNotifier");
        let mut state = self.0.borrow_mut();
        if !state.disposed {
            state.allocation = Some(lifetime);
        }
    }

    /// Whether at least one registration is active. After disposal this is false.
    #[must_use]
    pub fn has_listeners(&self) -> bool {
        let state = self.0.borrow();
        state.live_listeners != 0
            || state
                .tracked
                .as_ref()
                .is_some_and(|source| source.subscriber_count() != 0)
    }

    /// Makes the current build depend on notifications from this notifier.
    /// Tracking is independent of an application's listener token.
    pub fn track(&self) {
        let source = {
            let mut state = self.0.borrow_mut();
            state.assert_alive();
            state
                .tracked
                .get_or_insert_with(DependencySource::default)
                .clone()
        };
        source.track();
    }

    /// Debug check matching Flutter's `debugAssertNotDisposed`. In release
    /// builds the explicit check is a no-op and returns true.
    #[must_use]
    pub fn debug_assert_not_disposed(&self) -> bool {
        if cfg!(debug_assertions) {
            self.0.borrow().assert_alive();
        }
        true
    }

    /// Visits listeners in order. Registrations added by a callback are not
    /// part of the current dispatch; a nested dispatch sees its own list.
    ///
    /// If a callback panics, other live listeners still run. In unwinding
    /// builds the first panic is resumed after dispatch and cleanup.
    ///
    /// # Panics
    /// Panics if the notifier is disposed, or if a listener panicked.
    pub fn notify_listeners(&self) {
        if let Some(payload) = self.notify_listeners_collecting_panics().into_iter().next() {
            resume_unwind(payload);
        }
    }

    /// Delivers the same notification while collecting failure payloads
    /// in dispatch order instead of resuming unwinding afterward. An
    /// embedder can pass these to its UI error reporter; normal delivery
    /// and deferred listener cleanup still finish.
    ///
    /// # Panics
    /// Panics if the notifier has already been disposed. Callback panics
    /// are recoverable only in unwinding builds.
    pub fn notify_listeners_collecting_panics(&self) -> Vec<Box<dyn Any + Send + 'static>> {
        let (end, tracked) = {
            let mut state = self.0.borrow_mut();
            state.assert_alive();
            if state.live_listeners == 0
                && state
                    .tracked
                    .as_ref()
                    .is_none_or(|source| source.subscriber_count() == 0)
            {
                return Vec::new();
            }
            state.depth += 1;
            (state.listeners.len(), state.tracked.clone())
        };
        let guard = Notification(&self.0);
        let mut failures = Vec::new();
        for index in 0..end {
            let callback = self.0.borrow().listeners[index]
                .as_ref()
                .map(|listener| listener.callback.clone());
            if let Some(callback) = callback
                && let Err(payload) = catch_unwind(AssertUnwindSafe(|| callback()))
            {
                failures.push(payload);
            }
        }
        if let Some(tracked) = tracked {
            failures.extend(tracked.notify_collecting_panics());
        }
        drop(guard);
        failures
    }

    /// Closes the notifier and releases all callbacks immediately. It is
    /// invalid to dispose twice or while a notification is in progress.
    ///
    /// # Panics
    /// Panics after disposal or during notification.
    pub fn dispose(&self) {
        let listeners = {
            let mut state = self.0.borrow_mut();
            state.assert_alive();
            assert_eq!(
                state.depth, 0,
                "Cannot dispose a ChangeNotifier during notification."
            );
            state.disposed = true;
            state.live_listeners = 0;
            (
                std::mem::take(&mut state.listeners),
                state.tracked.take(),
                #[cfg(debug_assertions)]
                state.allocation.take(),
            )
        };
        // Captures can own other subscriptions on this notifier.
        drop(listeners);
    }
}

struct Notification<'a>(&'a RefCell<State>);

impl Drop for Notification<'_> {
    fn drop(&mut self) {
        let mut state = self.0.borrow_mut();
        state.depth -= 1;
        if state.depth == 0 && state.live_listeners != state.listeners.len() {
            state.listeners.retain(Option::is_some);
        }
    }
}

/// Owns one ChangeNotifier registration, not the notifier or its callback.
#[must_use = "keep the subscription alive to receive notifications"]
pub struct ChangeNotifierSubscription {
    state: Weak<RefCell<State>>,
    id: u64,
}

impl Drop for ChangeNotifierSubscription {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            let removed = state.borrow_mut().remove(self.id);
            drop(removed);
        }
    }
}
