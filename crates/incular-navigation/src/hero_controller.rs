//! Flutter's `HeroController`: flies heroes between page routes when a
//! navigator's top route changes.

use std::{cell::RefCell, rc::Rc};

use incular_core::Rect;
use incular_widgets::internal::HeroFlights;
use incular_widgets::{
    AnimationController, AnimationStatus, CreateRectTween, HeroFlightDirection, HeroRectTween,
    Widget,
};

use super::{NavigationEvent, Navigator, NavigatorObserver, Route, RoutePresentation};

/// Flutter's `HeroController`: when a navigator's top route changes between
/// two pages, the heroes with a tag in both fly from one to the other along
/// the route animation, in [`Self::overlay`] above the routes.
///
/// Incular removes a popped route at once, which Flutter does for a route
/// whose reverse transition has no duration, so heroes fly on pushes only.
/// There are no user-gesture route transitions, so
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
        let observer = navigator.observe(move |event| {
            // Pushes, replacements and page reconciliation all add routes.
            for route in routes.routes() {
                route.heroes().activate();
            }
            if let NavigationEvent::ActiveRouteChanged {
                previous: Some(previous),
                current: Some(current),
            } = event
            {
                controller.did_change_top(&current, &previous);
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

    /// `didChangeTop`.
    fn did_change_top(&self, top: &Route, previous_top: &Route) {
        self.maybe_start_hero_transition(previous_top, top);
    }

    /// `_maybeStartHeroTransition`.
    fn maybe_start_hero_transition(&self, from: &Route, to: &Route) {
        let is_page = |route: &Route| matches!(route.presentation, RoutePresentation::Page { .. });
        if from.id == to.id || !is_page(from) || !is_page(to) {
            return;
        }
        // A route without a transition has a completed animation.
        let (new_animation, new_status, new_value) = route_animation(to);
        let (old_animation, old_status, old_value) = route_animation(from);
        let direction = match (old_status, new_status) {
            (AnimationStatus::Reverse, _) => Some(HeroFlightDirection::Pop),
            (_, AnimationStatus::Forward) => Some(HeroFlightDirection::Push),
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
