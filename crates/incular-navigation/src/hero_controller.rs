//! Flutter's `HeroController`: flies heroes between page routes when a
//! navigator's top route changes.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_core::Rect;
use incular_widgets::internal::HeroFlights;
use incular_widgets::{
    AnimationController, AnimationStatus, CreateRectTween, HeroFlightDirection, HeroRectTween,
    Widget,
};

use super::{NavigationEvent, Navigator, NavigatorObserver, Route, RouteId, RoutePresentation};

/// Flutter's `HeroController`: when a navigator's top route changes between
/// two pages, the heroes with a tag in both fly from one to the other along
/// the route animation, in [`Self::overlay`] above the routes.
///
/// A popped route keeps its heroes mounted through its reverse animation, so
/// heroes fly back on pops as well. A popped route with no animation has no
/// reverse flight. There are no user-gesture route transitions, so
/// `Hero::transition_on_user_gestures` has nothing to apply to.
#[derive(Clone)]
pub struct HeroController {
    flights: HeroFlights,
    observer: Rc<RefCell<Option<NavigatorObserver>>>,
}

impl Default for HeroController {
    fn default() -> Self {
        Self::new()
    }
}

impl HeroController {
    #[must_use]
    pub fn new() -> Self {
        Self::with_flights(None)
    }

    /// Flutter's `HeroController(createRectTween:)`: the tween of flights
    /// whose destination hero has none.
    #[must_use]
    pub fn with_create_rect_tween(create: impl Fn(Rect, Rect) -> HeroRectTween + 'static) -> Self {
        let create: CreateRectTween = Rc::new(create);
        Self::with_flights(Some(create))
    }

    fn with_flights(create_rect_tween: Option<CreateRectTween>) -> Self {
        Self {
            flights: HeroFlights::new(create_rect_tween),
            observer: Rc::new(RefCell::new(None)),
        }
    }

    /// Observes `navigator`, replacing any navigator observed before.
    /// From then on its routes register their heroes; a route already
    /// mounted rebuilds its content once to do so.
    pub fn attach(&self, navigator: &Navigator) {
        for route in navigator.routes() {
            route.heroes().activate();
        }
        let controller = self.clone();
        let routes = navigator.clone();
        // A pop commits `Popped` immediately before its `ActiveRouteChanged`,
        // so the popped route is known when the top change arrives. Its
        // reverse animation starts on the next frame, after this event.
        let popping = Cell::new(None::<RouteId>);
        let removing = Cell::new(None::<RouteId>);
        let observer = navigator.observe(move |event| {
            // Pushes, replacements and page reconciliation all add routes.
            for route in routes.routes() {
                route.heroes().activate();
            }
            match event {
                NavigationEvent::Popped { route } => popping.set(Some(route.id)),
                NavigationEvent::Removed { route, .. } => removing.set(Some(route.id)),
                NavigationEvent::ActiveRouteChanged {
                    previous: Some(previous),
                    current: Some(current),
                } => {
                    let popped = popping.take() == Some(previous.id);
                    if removing.take() == Some(previous.id) {
                        // removeRoute does not run an outgoing animation.
                        return;
                    }
                    controller.did_change_top(&current, &previous, popped);
                }
                _ => {}
            }
        });
        *self.observer.borrow_mut() = Some(observer);
    }

    /// Stops observing the navigator.
    pub fn detach(&self) {
        self.observer.borrow_mut().take();
    }

    /// The layer the heroes fly in: place it above the navigator's routes,
    /// covering the same area.
    #[must_use]
    pub fn overlay(&self) -> Widget {
        self.flights.overlay()
    }

    /// `didChangeTop`, with `popped` for `didPop` (the previous top left the
    /// stack) rather than `didPush` or a replacement.
    fn did_change_top(&self, top: &Route, previous_top: &Route, popped: bool) {
        self.maybe_start_hero_transition(previous_top, top, popped);
    }

    /// `_maybeStartHeroTransition`.
    fn maybe_start_hero_transition(&self, from: &Route, to: &Route, popped: bool) {
        let is_page = |route: &Route| matches!(route.presentation, RoutePresentation::Page { .. });
        if from.id == to.id || !is_page(from) || !is_page(to) {
            return;
        }
        // A popped route without an animation has no reverse flight.
        if popped && from.transition.animation().is_none() {
            return;
        }
        // A route without a transition has a completed animation.
        let (new_animation, new_status, new_value) = route_animation(to);
        let (old_animation, old_status, old_value) = route_animation(from);
        let direction = match (old_status, new_status) {
            _ if popped => Some(HeroFlightDirection::Pop),
            (AnimationStatus::Reverse, _) => Some(HeroFlightDirection::Pop),
            (_, AnimationStatus::Forward) => Some(HeroFlightDirection::Push),
            // Incular's outlet starts a PageRouteBuilder animation on
            // its first frame, after this navigation event. The post-frame
            // flight callback checks that it actually began, so an initial,
            // added, or zero-duration route still does not fly.
            (_, AnimationStatus::Dismissed) if to.transition.route_animation().is_some() => {
                Some(HeroFlightDirection::Push)
            }
            _ => None,
        };
        match direction {
            Some(HeroFlightDirection::Pop) if old_value == 0. => return,
            Some(HeroFlightDirection::Push) if new_value == 1. => return,
            _ => {}
        }
        let animation = if direction == Some(HeroFlightDirection::Pop) {
            old_animation
        } else {
            new_animation
        };
        self.flights
            .transition(from.heroes(), to.heroes(), direction, animation);
    }
}

/// The route's animation with its status and value.
fn route_animation(route: &Route) -> (AnimationController, AnimationStatus, f32) {
    match route.transition.animation() {
        Some(animation) => {
            let (status, value) = (animation.status(), animation.value());
            (animation, status, value)
        }
        None => (
            AnimationController::default(),
            AnimationStatus::Completed,
            1.,
        ),
    }
}
