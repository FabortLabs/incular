//! Retained, reentrancy-safe routes for platform-neutral pointer events.

use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::{Rc, Weak},
};

use incular_core::{IdGenerator, Transform};

use crate::RawPointerEvent;

/// A registration in one [PointerRouter]. Tokens from other routers do not
/// affect this router's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PointerRouteId {
    owner: u64,
    ordinal: u64,
}

/// An event view for a route, preserving the global coordinates and metadata
/// in `event` and providing local coordinates for the route's transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoutedPointerEvent {
    pub event: RawPointerEvent,
    pub transform: Option<Transform>,
}

/// A callback panicked while handling a routed pointer event. Subsequent
/// routes still run under Rust's unwinding panic mode.
pub struct PointerRouteError {
    pub router: WeakPointerRouter,
    pub route: PointerRouteId,
    pub event: RoutedPointerEvent,
    pub panic: Box<dyn Any + Send + 'static>,
}

impl PointerRouteError {
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| self.panic.downcast_ref::<&str>().copied())
    }
}

impl fmt::Debug for PointerRouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PointerRouteError")
            .field("router", &self.router)
            .field("route", &self.route)
            .field("event", &self.event)
            .field("message", &self.message())
            .finish_non_exhaustive()
    }
}

pub type PointerRouteErrorHandler = Rc<dyn Fn(PointerRouteError)>;

struct Route {
    id: PointerRouteId,
    active: Cell<bool>,
    transform: Option<Transform>,
    callback: Box<dyn Fn(RoutedPointerEvent)>,
}

type Routes = Rc<Vec<Rc<Route>>>;

#[derive(Default)]
struct RoutingState {
    pointers: HashMap<u64, Routes>,
    global: Routes,
    next_route: u64,
    on_error: Option<PointerRouteErrorHandler>,
}

/// Routes specific pointer streams before global routes, each in registration
/// order. A clone shares registrations. Additions during dispatch only apply
/// to later events; removals affect the current event immediately.
#[derive(Clone)]
pub struct PointerRouter {
    id: u64,
    state: Rc<RefCell<RoutingState>>,
}

/// A weak router reference, useful in route callbacks that need to
/// mutate their own registrations without keeping the router alive.
#[derive(Clone)]
pub struct WeakPointerRouter {
    id: u64,
    state: Weak<RefCell<RoutingState>>,
}

impl fmt::Debug for WeakPointerRouter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PointerRouter")
            .field("id", &self.id)
            .finish()
    }
}

impl WeakPointerRouter {
    #[must_use]
    pub fn upgrade(&self) -> Option<PointerRouter> {
        Some(PointerRouter {
            id: self.id,
            state: self.state.upgrade()?,
        })
    }
}

impl fmt::Debug for PointerRouter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PointerRouter")
            .field("id", &self.id)
            .finish()
    }
}

impl Default for PointerRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl PointerRouter {
    #[must_use]
    pub fn new() -> Self {
        static NEXT: IdGenerator = IdGenerator::new();
        Self {
            id: NEXT.next(),
            state: Rc::new(RefCell::new(RoutingState::default())),
        }
    }

    #[must_use]
    pub fn downgrade(&self) -> WeakPointerRouter {
        WeakPointerRouter {
            id: self.id,
            state: Rc::downgrade(&self.state),
        }
    }

    /// Registers a callback for one pointer. The optional transform maps
    /// global coordinates into the recipient's local coordinates.
    pub fn add_route(
        &self,
        pointer: u64,
        transform: Option<Transform>,
        callback: impl Fn(RoutedPointerEvent) + 'static,
    ) -> PointerRouteId {
        self.add(Some(pointer), transform, callback)
    }

    /// Registers a callback for all routed pointer events.
    pub fn add_global_route(
        &self,
        transform: Option<Transform>,
        callback: impl Fn(RoutedPointerEvent) + 'static,
    ) -> PointerRouteId {
        self.add(None, transform, callback)
    }

    fn add(
        &self,
        pointer: Option<u64>,
        transform: Option<Transform>,
        callback: impl Fn(RoutedPointerEvent) + 'static,
    ) -> PointerRouteId {
        let mut state = self.state.borrow_mut();
        state.next_route = state
            .next_route
            .checked_add(1)
            .expect("pointer route identity space exhausted");
        let id = PointerRouteId {
            owner: self.id,
            ordinal: state.next_route,
        };
        let route = Rc::new(Route {
            id,
            active: Cell::new(true),
            transform,
            callback: Box::new(callback),
        });
        let routes = match pointer {
            Some(pointer) => state.pointers.entry(pointer).or_default(),
            None => &mut state.global,
        };
        Rc::make_mut(routes).push(route);
        id
    }

    /// Removes a pointer route, returning false for an unknown registration.
    pub fn remove_route(&self, pointer: u64, id: PointerRouteId) -> bool {
        let removed = {
            let mut state = self.state.borrow_mut();
            let Some(routes) = state.pointers.get_mut(&pointer) else {
                return false;
            };
            let removed = Self::remove(routes, id);
            if routes.is_empty() {
                state.pointers.remove(&pointer);
            }
            removed
        };
        removed.is_some()
    }

    /// Removes a global route, returning false for an unknown registration.
    pub fn remove_global_route(&self, id: PointerRouteId) -> bool {
        let removed = { Self::remove(&mut self.state.borrow_mut().global, id) };
        removed.is_some()
    }

    fn remove(routes: &mut Routes, id: PointerRouteId) -> Option<Rc<Route>> {
        let index = routes.iter().position(|route| route.id == id)?;
        let removed = Rc::make_mut(routes).remove(index);
        removed.active.set(false);
        // Caller drops the closure after releasing the mutable router borrow.
        Some(removed)
    }

    #[must_use]
    pub fn global_route_count(&self) -> usize {
        self.state.borrow().global.len()
    }

    pub fn set_error_handler(&self, handler: Option<PointerRouteErrorHandler>) {
        let previous = std::mem::replace(&mut self.state.borrow_mut().on_error, handler);
        drop(previous);
    }

    /// Delivers the event to every currently registered route. Snapshots are
    /// cheap shared references; the route arrays only copy on mutation
    /// while another dispatch still uses them.
    pub fn route(&self, event: RawPointerEvent) {
        let (local, global) = {
            let state = self.state.borrow();
            (
                state.pointers.get(&event.pointer).cloned(),
                state.global.clone(),
            )
        };
        let event = event.in_global_coordinates();
        for route in local
            .iter()
            .flat_map(|routes| routes.iter())
            .chain(global.iter())
        {
            if !route.active.get() {
                continue;
            }
            let routed = RoutedPointerEvent {
                event: event.transformed(route.transform),
                transform: route.transform,
            };
            if let Err(panic) = catch_unwind(AssertUnwindSafe(|| (route.callback)(routed))) {
                let handler = self.state.borrow().on_error.clone();
                if let Some(handler) = handler {
                    let error = PointerRouteError {
                        router: self.downgrade(),
                        route: route.id,
                        event: routed,
                        panic,
                    };
                    // A diagnostic observer must not halt input dispatch.
                    let _ = catch_unwind(AssertUnwindSafe(|| handler(error)));
                }
            }
        }
    }
}
