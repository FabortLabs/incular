//! Independent platform, view, and application demand for retained semantics.

use crate::UiDispatcher;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

type EnabledListener = Rc<dyn Fn(bool)>;
type ListenerList = Rc<Vec<EnabledListener>>;

pub(crate) struct SemanticsActivation {
    platform: Cell<bool>,
    handles: Cell<usize>,
    global: Cell<bool>,
    last_notified: Cell<bool>,
    listeners: RefCell<ListenerList>,
    dispatcher: UiDispatcher,
}

impl SemanticsActivation {
    pub(crate) fn new(platform: bool, dispatcher: UiDispatcher) -> Self {
        Self {
            platform: Cell::new(platform),
            handles: Cell::new(0),
            global: Cell::new(false),
            last_notified: Cell::new(platform),
            listeners: RefCell::default(),
            dispatcher,
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.platform.get() || self.handles.get() != 0 || self.global.get()
    }

    pub(crate) fn set_platform(&self, enabled: bool) {
        self.platform.set(enabled);
        self.notify_if_changed();
    }

    pub(crate) fn acquire(self: &Rc<Self>) -> SemanticsHandle {
        self.handles.set(
            self.handles
                .get()
                .checked_add(1)
                .expect("too many semantics handles"),
        );
        let handle = SemanticsHandle {
            activation: Rc::downgrade(self),
            #[cfg(debug_assertions)]
            _allocation: incular_core::memory::allocations::ObjectLifetime::new(
                "semantics",
                "SemanticsHandle",
            ),
        };
        self.notify_if_changed();
        handle
    }

    pub(crate) fn listen(
        self: &Rc<Self>,
        listener: impl Fn(bool) + 'static,
        replay: bool,
    ) -> SemanticsEnabledSubscription {
        let delivered = Cell::new((!replay).then(|| self.enabled()));
        let listener: EnabledListener = Rc::new(move |enabled| {
            if delivered.replace(Some(enabled)) != Some(enabled) {
                listener(enabled);
            }
        });
        Rc::make_mut(&mut self.listeners.borrow_mut()).push(listener.clone());
        let subscription = SemanticsEnabledSubscription {
            activation: Rc::downgrade(self),
            listener: listener.clone(),
        };
        if replay {
            listener(self.enabled());
        }
        subscription
    }

    fn notify_if_changed(&self) -> bool {
        let enabled = self.enabled();
        if self.last_notified.replace(enabled) == enabled {
            return false;
        }
        let listeners = self.listeners.borrow().clone();
        for listener in listeners.iter() {
            if self.last_notified.get() != enabled || self.enabled() != enabled {
                break; // A listener superseded this notification synchronously.
            }
            listener(enabled);
        }
        true
    }

    fn notify_and_wake(&self) {
        if self.notify_if_changed() {
            // A handle can be dropped between native events or from a widget
            // callback. Wake the owner without borrowing its retained tree.
            self.dispatcher
                .dispatch(crate::Runtime::sync_semantics_activation);
        }
    }
}

/// Application-level demand is opt-in. Until first requested, newly opened
/// windows do not allocate or register a per-view activation for this scope.
#[derive(Default)]
pub(crate) struct GlobalSemanticsActivation {
    tracking: Cell<bool>,
    handles: Cell<usize>,
    windows: RefCell<Vec<Weak<SemanticsActivation>>>,
}

impl GlobalSemanticsActivation {
    pub(crate) fn start_tracking(&self) -> bool {
        !self.tracking.replace(true)
    }

    pub(crate) fn is_tracking(&self) -> bool {
        self.tracking.get()
    }

    pub(crate) fn register(&self, view: &Rc<SemanticsActivation>) {
        let mut windows = self.windows.borrow_mut();
        // Windows can churn while a global handle lives for the whole app.
        // Prune before growth rather than scanning on each window opening.
        if windows.len() == windows.capacity() {
            windows.retain(|weak| weak.strong_count() != 0);
        }
        windows.push(Rc::downgrade(view));
        drop(windows);
        view.global.set(self.handles.get() != 0);
        view.notify_if_changed();
    }

    pub(crate) fn acquire(self: &Rc<Self>) -> GlobalSemanticsHandle {
        let count = self.handles.get();
        self.handles.set(
            count
                .checked_add(1)
                .expect("too many global semantics handles"),
        );
        if count == 0 {
            self.update_views(true);
        }
        GlobalSemanticsHandle {
            activation: Rc::downgrade(self),
        }
    }

    fn update_views(&self, enabled: bool) {
        // Update every effective state before calling user listeners. Snapshot
        // only weak references: a listener may close a not-yet-notified view.
        let views = {
            let mut tracked = self.windows.borrow_mut();
            tracked.retain(|weak| {
                if let Some(view) = weak.upgrade() {
                    view.global.set(enabled);
                    true
                } else {
                    false
                }
            });
            tracked.clone()
        };
        for view in views.into_iter().filter_map(|weak| weak.upgrade()) {
            view.notify_and_wake();
        }
    }
}

/// Keeps semantics enabled across all current and future windows of one
/// application until the last global handle drops. Local view handles and
/// native host demand remain independent.
#[must_use = "keep the handle while all application views need semantics"]
pub struct GlobalSemanticsHandle {
    activation: Weak<GlobalSemanticsActivation>,
}

impl Drop for GlobalSemanticsHandle {
    fn drop(&mut self) {
        if let Some(activation) = self.activation.upgrade() {
            let remaining = activation.handles.get() - 1;
            activation.handles.set(remaining);
            if remaining == 0 {
                activation.update_views(false);
            }
        }
    }
}

/// Keeps accessibility semantics enabled for one window. Dropping the last
/// application handle releases its request, without overriding the native
/// accessibility host's independent request.
#[must_use = "keep the handle while the application needs semantics"]
pub struct SemanticsHandle {
    activation: Weak<SemanticsActivation>,
    #[cfg(debug_assertions)]
    _allocation: incular_core::memory::allocations::ObjectLifetime,
}

impl Drop for SemanticsHandle {
    fn drop(&mut self) {
        let Some(activation) = self.activation.upgrade() else {
            return;
        };
        activation.handles.set(activation.handles.get() - 1);
        activation.notify_and_wake();
    }
}

/// Subscribes to changes in the effective, window-local semantics state.
/// Drop the subscription to remove its listener.
#[must_use = "keep the subscription to observe semantics state"]
pub struct SemanticsEnabledSubscription {
    activation: Weak<SemanticsActivation>,
    listener: EnabledListener,
}

impl Drop for SemanticsEnabledSubscription {
    fn drop(&mut self) {
        if let Some(activation) = self.activation.upgrade() {
            Rc::make_mut(&mut activation.listeners.borrow_mut())
                .retain(|listener| !Rc::ptr_eq(listener, &self.listener));
        }
    }
}
