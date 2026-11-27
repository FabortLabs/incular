//! Independent platform and application demand for one window's semantics tree.

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
    listeners: RefCell<ListenerList>,
    dispatcher: UiDispatcher,
}

impl SemanticsActivation {
    pub(crate) fn new(platform: bool, dispatcher: UiDispatcher) -> Self {
        Self {
            platform: Cell::new(platform),
            handles: Cell::new(0),
            listeners: RefCell::default(),
            dispatcher,
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.platform.get() || self.handles.get() != 0
    }

    pub(crate) fn set_platform(&self, enabled: bool) {
        let before = self.enabled();
        self.platform.set(enabled);
        self.notify_if_changed(before);
    }

    pub(crate) fn acquire(self: &Rc<Self>) -> SemanticsHandle {
        let before = self.enabled();
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
        self.notify_if_changed(before);
        handle
    }

    pub(crate) fn listen(
        self: &Rc<Self>,
        listener: impl Fn(bool) + 'static,
        replay: bool,
    ) -> SemanticsEnabledSubscription {
        let listener: EnabledListener = Rc::new(listener);
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

    fn notify_if_changed(&self, before: bool) -> bool {
        let enabled = self.enabled();
        if before == enabled {
            return false;
        }
        let listeners = self.listeners.borrow().clone();
        for listener in listeners.iter() {
            listener(enabled);
        }
        true
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
        let before = activation.enabled();
        activation.handles.set(activation.handles.get() - 1);
        if activation.notify_if_changed(before) {
            // A handle can be dropped between native events or from a widget
            // callback. Wake the owner without borrowing its retained tree.
            activation
                .dispatcher
                .dispatch(crate::Runtime::sync_semantics_activation);
        }
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
