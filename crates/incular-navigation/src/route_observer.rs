//! Flutter's `ModalRoute.of`, `RouteAware` and `RouteObserver`: widgets
//! inside a route find the route that presents them, and learn when routes
//! are pushed on top of it or popped off it.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::{Rc, Weak},
};

use incular_widgets::BuildContext;

use super::{NavigationEvent, Navigator, NavigatorObserver, Route, RouteId, RouteSettings};

/// The route presenting a widget, as Flutter's `ModalRoute.of` returns it.
///
/// Route outlets publish it around every route's content. The route value is
/// a snapshot of the route when it was mounted; the status queries read the
/// navigator, so they are always current.
#[derive(Clone)]
pub struct ModalRoute {
    route: Route,
    navigator: Navigator,
}

impl fmt::Debug for ModalRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModalRoute")
            .field("id", &self.route.id)
            .field("name", &self.route.name)
            .finish()
    }
}

impl ModalRoute {
    /// The scope value a route outlet publishes for `route`.
    #[must_use]
    pub fn new(route: Route, navigator: Navigator) -> Self {
        Self { route, navigator }
    }

    /// The route enclosing `context`, if any (Flutter's `ModalRoute.of`,
    /// which is `maybeOf` here because Incular never panics on lookups).
    #[must_use]
    pub fn of(context: &BuildContext<'_>) -> Option<Self> {
        context.find::<Self>()
    }

    /// Flutter's `ModalRoute.settingsOf`.
    #[must_use]
    pub fn settings_of(context: &BuildContext<'_>) -> Option<RouteSettings> {
        Self::of(context).map(|route| route.route.settings)
    }

    #[must_use]
    pub fn id(&self) -> RouteId {
        self.route.id
    }

    #[must_use]
    pub fn route(&self) -> &Route {
        &self.route
    }

    #[must_use]
    pub fn settings(&self) -> &RouteSettings {
        &self.route.settings
    }

    #[must_use]
    pub fn navigator(&self) -> &Navigator {
        &self.navigator
    }

    /// Flutter's `Route.isCurrent`: this route is the navigator's top route.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.navigator
            .current()
            .is_some_and(|current| current.id == self.route.id)
    }

    /// Flutter's `Route.isFirst`: this route is the navigator's bottom route.
    #[must_use]
    pub fn is_first(&self) -> bool {
        self.navigator
            .routes()
            .first()
            .is_some_and(|first| first.id == self.route.id)
    }

    /// Flutter's `Route.isActive`: this route is still on the navigator.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.navigator
            .routes()
            .iter()
            .any(|route| route.id == self.route.id)
    }

    /// Flutter's `ModalRoute.canPop`: the route is active and not the first.
    #[must_use]
    pub fn can_pop(&self) -> bool {
        self.is_active() && !self.is_first()
    }
}

/// Flutter's `RouteAware`: told about navigation relative to the route it
/// subscribed to with a [`RouteObserver`]. Every method defaults to doing
/// nothing.
pub trait RouteAware {
    /// The subscribed route was pushed, or the subscription was just made.
    fn did_push(&self) {}
    /// The subscribed route was popped.
    fn did_pop(&self) {}
    /// A route was pushed on top of the subscribed route.
    fn did_push_next(&self) {}
    /// The route on top of the subscribed route was popped, so it is shown.
    fn did_pop_next(&self) {}
}

type Callback = Option<Rc<dyn Fn()>>;

/// A [`RouteAware`] built from closures.
#[derive(Clone, Default)]
pub struct RouteAwareCallbacks {
    pub did_push: Callback,
    pub did_pop: Callback,
    pub did_push_next: Callback,
    pub did_pop_next: Callback,
}

impl fmt::Debug for RouteAwareCallbacks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouteAwareCallbacks")
            .finish_non_exhaustive()
    }
}

impl RouteAwareCallbacks {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn on_push(mut self, callback: impl Fn() + 'static) -> Self {
        self.did_push = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_pop(mut self, callback: impl Fn() + 'static) -> Self {
        self.did_pop = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_push_next(mut self, callback: impl Fn() + 'static) -> Self {
        self.did_push_next = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_pop_next(mut self, callback: impl Fn() + 'static) -> Self {
        self.did_pop_next = Some(Rc::new(callback));
        self
    }
}

impl RouteAware for RouteAwareCallbacks {
    fn did_push(&self) {
        if let Some(callback) = &self.did_push {
            callback();
        }
    }

    fn did_pop(&self) {
        if let Some(callback) = &self.did_pop {
            callback();
        }
    }

    fn did_push_next(&self) {
        if let Some(callback) = &self.did_push_next {
            callback();
        }
    }

    fn did_pop_next(&self) {
        if let Some(callback) = &self.did_pop_next {
            callback();
        }
    }
}

#[derive(Default)]
struct ObserverInner {
    subscribers: RefCell<HashMap<RouteId, Vec<Weak<dyn RouteAware>>>>,
    navigators: RefCell<Vec<NavigatorObserver>>,
}

impl ObserverInner {
    fn subscribers_of(&self, route: RouteId) -> Vec<Rc<dyn RouteAware>> {
        let mut subscribers = self.subscribers.borrow_mut();
        let Some(list) = subscribers.get_mut(&route) else {
            return Vec::new();
        };
        list.retain(|aware| aware.strong_count() != 0);
        let live = list.iter().filter_map(Weak::upgrade).collect::<Vec<_>>();
        if list.is_empty() {
            subscribers.remove(&route);
        }
        live
    }
}

#[derive(Clone, Copy)]
enum PendingTransition {
    Push(RouteId),
    Pop(RouteId),
}

/// Flutter's `RouteObserver`: tells subscribed [`RouteAware`]s about
/// navigation relative to their routes, for every navigator it is attached
/// to.
///
/// Subscriptions hold the subscriber weakly: dropping the last `Rc` of a
/// subscriber unsubscribes it, the counterpart of calling `unsubscribe` in
/// Flutter's `dispose`.
#[derive(Clone, Default)]
pub struct RouteObserver {
    inner: Rc<ObserverInner>,
}

impl fmt::Debug for RouteObserver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouteObserver")
            .field("routes", &self.inner.subscribers.borrow().len())
            .finish()
    }
}

impl PartialEq for RouteObserver {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl RouteObserver {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Observes `navigator` (Flutter's `Navigator.observers`) for as long as
    /// this observer lives.
    pub fn attach(&self, navigator: &Navigator) {
        let inner = Rc::downgrade(&self.inner);
        let pending = Cell::new(None::<PendingTransition>);
        let token = navigator.observe(move |event| {
            let Some(inner) = inner.upgrade() else {
                return;
            };
            match event {
                NavigationEvent::Pushed { route } => {
                    pending.set(Some(PendingTransition::Push(route.id)));
                }
                NavigationEvent::Popped { route } => {
                    pending.set(Some(PendingTransition::Pop(route.id)));
                    for aware in inner.subscribers_of(route.id) {
                        aware.did_pop();
                    }
                }
                NavigationEvent::Replaced { .. } => pending.set(None),
                NavigationEvent::ActiveRouteChanged { previous, current } => {
                    match pending.take() {
                        // `didPush(route, previousRoute)`.
                        Some(PendingTransition::Push(pushed))
                            if current.as_ref().is_some_and(|route| route.id == pushed) =>
                        {
                            if let Some(previous) = previous {
                                for aware in inner.subscribers_of(previous.id) {
                                    aware.did_push_next();
                                }
                            }
                        }
                        // `didPop(route, previousRoute)`.
                        Some(PendingTransition::Pop(popped))
                            if previous.as_ref().is_some_and(|route| route.id == popped) =>
                        {
                            if let Some(current) = current {
                                for aware in inner.subscribers_of(current.id) {
                                    aware.did_pop_next();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
        self.inner.navigators.borrow_mut().push(token);
    }

    /// Flutter's `RouteObserver.subscribe`: `aware` hears about navigation
    /// relative to `route`, and is told [`RouteAware::did_push`] at once.
    pub fn subscribe(&self, aware: &Rc<dyn RouteAware>, route: RouteId) {
        let added = {
            let mut subscribers = self.inner.subscribers.borrow_mut();
            let list = subscribers.entry(route).or_default();
            let present = list
                .iter()
                .any(|existing| std::ptr::addr_eq(existing.as_ptr(), Rc::as_ptr(aware)));
            if !present {
                list.push(Rc::downgrade(aware));
            }
            !present
        };
        if added {
            aware.did_push();
        }
    }

    /// Flutter's `RouteObserver.unsubscribe`: removes `aware` from every
    /// route.
    pub fn unsubscribe(&self, aware: &Rc<dyn RouteAware>) {
        let mut subscribers = self.inner.subscribers.borrow_mut();
        for list in subscribers.values_mut() {
            list.retain(|existing| !std::ptr::addr_eq(existing.as_ptr(), Rc::as_ptr(aware)));
        }
        subscribers.retain(|_, list| !list.is_empty());
    }

    /// Whether `route` has live subscribers (Flutter's `debugObservingRoute`).
    #[must_use]
    pub fn is_observing(&self, route: RouteId) -> bool {
        !self.inner.subscribers_of(route).is_empty()
    }
}
