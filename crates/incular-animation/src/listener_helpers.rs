//! Local value and status observers with a shared lazy parent lifecycle.

use std::{
    any::Any,
    cell::RefCell,
    hash::{Hash, Hasher},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

use incular_core::{HashedObserverList, ObserverList};

use crate::AnimationStatus;

pub type AnimationValueListener = Rc<dyn Fn()>;
pub type AnimationStatusListener = Rc<dyn Fn(AnimationStatus)>;

struct Identity<T: ?Sized>(Rc<T>);

impl<T: ?Sized> Clone for Identity<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized> PartialEq for Identity<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> Eq for Identity<T> {}

impl<T: ?Sized> Hash for Identity<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.0).cast::<()>().hash(state);
    }
}

/// A recoverable failure while calling a local animation listener.
pub type AnimationListenerPanic = Box<dyn Any + Send + 'static>;

/// Observer storage for an animation that only needs its parent while observed.
///
/// The first registration of either kind starts parent observation. Only
/// removing the last actual registration stops it. Local value callbacks
/// fire once per identity; status callbacks fire for each registration.
pub struct LazyAnimationListeners {
    values: RefCell<HashedObserverList<Identity<dyn Fn()>>>,
    statuses: RefCell<ObserverList<Identity<dyn Fn(AnimationStatus)>>>,
    start: Box<dyn Fn()>,
    stop: Box<dyn Fn()>,
}

impl LazyAnimationListeners {
    pub fn new(start: impl Fn() + 'static, stop: impl Fn() + 'static) -> Self {
        Self {
            values: RefCell::default(),
            statuses: RefCell::default(),
            start: Box::new(start),
            stop: Box::new(stop),
        }
    }

    #[must_use]
    pub fn is_listening(&self) -> bool {
        !self.values.borrow().is_empty() || !self.statuses.borrow().is_empty()
    }

    pub fn add_listener(&self, listener: AnimationValueListener) {
        self.start_if_idle();
        self.values.borrow_mut().add(Identity(listener));
    }

    pub fn remove_listener(&self, listener: &AnimationValueListener) -> bool {
        let removed = self.values.borrow_mut().remove(&Identity(listener.clone()));
        self.stop_if_idle(removed);
        removed
    }

    pub fn add_status_listener(&self, listener: AnimationStatusListener) {
        self.start_if_idle();
        self.statuses.borrow_mut().add(Identity(listener));
    }

    pub fn remove_status_listener(&self, listener: &AnimationStatusListener) -> bool {
        let removed = self
            .statuses
            .borrow_mut()
            .remove(&Identity(listener.clone()));
        self.stop_if_idle(removed);
        removed
    }

    fn start_if_idle(&self) {
        if !self.is_listening() {
            (self.start)();
        }
    }

    fn stop_if_idle(&self, removed: bool) {
        if removed && !self.is_listening() {
            (self.stop)();
        }
    }

    /// Calls the current listeners without holding registry borrows. Added
    /// listeners wait until the next dispatch; callbacks removed completely
    /// before their turn are skipped. The caller can report caught panics.
    #[must_use]
    pub fn notify_listeners(&self) -> Vec<AnimationListenerPanic> {
        let callbacks: Vec<_> = self.values.borrow().iter().cloned().collect();
        let mut errors = Vec::new();
        for callback in callbacks {
            let still_registered = self.values.borrow().contains(&callback);
            if still_registered
                && let Err(error) = catch_unwind(AssertUnwindSafe(|| (callback.0)()))
            {
                errors.push(error);
            }
        }
        errors
    }

    /// Calls the current status listeners and collects their failures.
    #[must_use]
    pub fn notify_status_listeners(&self, status: AnimationStatus) -> Vec<AnimationListenerPanic> {
        let callbacks: Vec<_> = self.statuses.borrow().iter().cloned().collect();
        let mut errors = Vec::new();
        for callback in callbacks {
            let still_registered = self.statuses.borrow().contains(&callback);
            if still_registered
                && let Err(error) = catch_unwind(AssertUnwindSafe(|| (callback.0)(status)))
            {
                errors.push(error);
            }
        }
        errors
    }
}
