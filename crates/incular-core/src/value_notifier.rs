//! A shared reactive value with equality-filtered notifications.

use crate::reactivity::{ReactiveCell, Subscription, assert_mutation_allowed};
use std::rc::Rc;

/// Flutter's `ValueNotifier`: observes replacement of a value, not mutation
/// through interior mutability. An equal replacement does not notify.
pub struct ValueNotifier<T> {
    inner: Rc<ValueState<T>>,
}

struct ValueState<T> {
    cell: ReactiveCell<T>,
    #[cfg(debug_assertions)]
    _allocation: crate::memory::allocations::ObjectLifetime,
}

impl<T> Clone for ValueNotifier<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> ValueNotifier<T> {
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(ValueState {
                cell: ReactiveCell::new(value),
                #[cfg(debug_assertions)]
                _allocation: crate::memory::allocations::ObjectLifetime::new(
                    "foundation",
                    "ValueNotifier",
                ),
            }),
        }
    }

    /// Tracks this notifier as a dependency if a build is being recorded.
    pub fn track(&self) {
        self.inner.cell.source().track();
    }

    /// Reads a copy of the current value and tracks it in the active build.
    #[must_use]
    pub fn value(&self) -> T
    where
        T: Clone,
    {
        self.track();
        self.inner.cell.borrow().clone()
    }

    /// Calls `listener` synchronously when the value changes. Keep the
    /// returned subscription alive to continue receiving notifications.
    pub fn add_listener(&self, listener: impl Fn() + 'static) -> Subscription {
        self.inner.cell.source().subscribe((), listener)
    }

    /// Replaces the value and returns whether it changed. Listeners can
    /// safely read or update the value during notification.
    pub fn set_value(&self, value: T) -> bool
    where
        T: PartialEq,
    {
        if *self.inner.cell.borrow() == value {
            return false;
        }
        assert_mutation_allowed();
        let previous = std::mem::replace(&mut *self.inner.cell.borrow_mut(), value);
        drop(previous);
        self.inner.cell.source().notify();
        true
    }
}
