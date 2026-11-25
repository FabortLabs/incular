//! Debug-only framework object lifecycle events, separate from heap totals.
//! Events describe identities instead of retaining observed objects.

use std::{
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::{Rc, Weak},
};

pub const MEMORY_ALLOCATIONS_ENABLED: bool = cfg!(debug_assertions);

/// A stable identity for one object, independent of memory addresses.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ObjectId(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectEventKind {
    Created,
    Disposed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectEvent {
    pub id: ObjectId,
    pub library: &'static str,
    pub class_name: &'static str,
    pub kind: ObjectEventKind,
}

type Observer = Rc<dyn Fn(ObjectEvent)>;

#[derive(Default)]
struct State {
    observers: Vec<Option<(u64, Observer)>>,
    live: usize,
    dispatch_depth: usize,
    next_registration: u64,
}

thread_local! {
    static ALLOCATIONS: MemoryAllocations = MemoryAllocations::default();
}

/// The UI thread's registry of framework object event observers.
#[derive(Clone, Default)]
pub struct MemoryAllocations(Rc<RefCell<State>>);

impl MemoryAllocations {
    #[must_use]
    pub fn current() -> Self {
        ALLOCATIONS.with(Clone::clone)
    }

    #[must_use]
    pub fn has_listeners(&self) -> bool {
        MEMORY_ALLOCATIONS_ENABLED && self.0.borrow().live != 0
    }

    /// Registers in insertion order. A dropped subscription is inactive
    /// immediately, even if another observer drops it during delivery.
    pub fn add_listener(&self, listener: impl Fn(ObjectEvent) + 'static) -> AllocationSubscription {
        if !MEMORY_ALLOCATIONS_ENABLED {
            return AllocationSubscription {
                state: Weak::new(),
                registration: 0,
            };
        }
        let mut state = self.0.borrow_mut();
        let registration = state.next_registration;
        state.next_registration = registration
            .checked_add(1)
            .expect("too many allocation listeners");
        state
            .observers
            .push(Some((registration, Rc::new(listener))));
        state.live += 1;
        AllocationSubscription {
            state: Rc::downgrade(&self.0),
            registration,
        }
    }

    /// Delivers the event and returns the number of observers that panicked.
    /// Rust invokes the process panic hook for each failure before it is
    /// caught here; the remaining observers still receive the event.
    pub fn dispatch(&self, event: ObjectEvent) -> usize {
        if !self.has_listeners() {
            return 0;
        }
        let end = {
            let mut state = self.0.borrow_mut();
            state.dispatch_depth += 1;
            state.observers.len()
        };
        struct DispatchGuard<'a>(&'a RefCell<State>);
        impl Drop for DispatchGuard<'_> {
            fn drop(&mut self) {
                let mut state = self.0.borrow_mut();
                state.dispatch_depth -= 1;
                if state.dispatch_depth == 0 && state.observers.len() != state.live {
                    state.observers.retain(Option::is_some);
                }
            }
        }
        let _dispatch = DispatchGuard(&self.0);
        let mut failures = 0;
        for index in 0..end {
            let callback = self.0.borrow().observers[index]
                .as_ref()
                .map(|(_, cb)| cb.clone());
            if let Some(callback) = callback {
                failures +=
                    usize::from(catch_unwind(AssertUnwindSafe(|| callback(event))).is_err());
            }
        }
        failures
    }
}

/// Owns exactly one registration.
#[must_use = "dropping the subscription unregisters its callback"]
pub struct AllocationSubscription {
    state: Weak<RefCell<State>>,
    registration: u64,
}

impl Drop for AllocationSubscription {
    fn drop(&mut self) {
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let removed = {
            let mut state = state.borrow_mut();
            let Some(index) = state.observers.iter().position(|entry| {
                entry
                    .as_ref()
                    .is_some_and(|(id, _)| *id == self.registration)
            }) else {
                return;
            };
            state.live -= 1;
            if state.dispatch_depth == 0 {
                state.observers.remove(index)
            } else {
                state.observers[index].take()
            }
        };
        drop(removed);
    }
}

/// An instrumented owned lifetime. In release builds it occupies no space
/// and compiles down to no operations.
#[derive(Debug)]
pub struct ObjectLifetime {
    #[cfg(debug_assertions)]
    event: ObjectEvent,
}

impl ObjectLifetime {
    #[must_use]
    pub fn new(library: &'static str, class_name: &'static str) -> Self {
        #[cfg(debug_assertions)]
        {
            static IDS: crate::IdGenerator = crate::IdGenerator::new();
            let event = ObjectEvent {
                id: ObjectId(IDS.next()),
                library,
                class_name,
                kind: ObjectEventKind::Created,
            };
            let _ = ALLOCATIONS.try_with(|registry| registry.dispatch(event));
            Self { event }
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = (library, class_name);
            Self {}
        }
    }

    #[must_use]
    pub fn id(&self) -> Option<ObjectId> {
        #[cfg(debug_assertions)]
        {
            Some(self.event.id)
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }
}

impl Drop for ObjectLifetime {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        {
            let event = ObjectEvent {
                kind: ObjectEventKind::Disposed,
                ..self.event
            };
            let _ = ALLOCATIONS.try_with(|registry| registry.dispatch(event));
        }
    }
}
