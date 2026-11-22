//! Flutter's `PredictiveBackRoute` (`widgets/routes.dart`): the current
//! route follows a system back gesture by setting its transition animation
//! to the gesture's progress, then pops on commit or animates back in on
//! cancel. The route transition follows the animation, so it tracks the
//! user's finger.

use std::rc::Rc;

use incular_widgets::{
    BackGestureObserver, BackGestureObserverRegistration, PredictiveBackEvent,
    add_shared_back_gesture_observer,
};

use crate::{Navigator, PopDecision, RouteId};

/// The gesture a navigator's current route follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BackGestureState {
    /// The route that started the gesture.
    pub(crate) route: Option<RouteId>,
    /// Flutter `NavigatorState.userGestureInProgress`.
    pub(crate) user_gesture: bool,
}

impl Navigator {
    /// Flutter `NavigatorState.userGestureInProgress`: whether a user
    /// gesture, such as a predictive back swipe, is driving a route
    /// transition.
    #[must_use]
    pub fn user_gesture_in_progress(&self) -> bool {
        self.back_gesture().user_gesture
    }

    /// Flutter `TransitionRoute.popGestureEnabled` for the current route:
    /// there is a route below it, it has a route animation to drive, and the
    /// pop guard would let it go.
    #[must_use]
    pub fn pop_gesture_enabled(&self) -> bool {
        let Some(route) = self.current() else {
            return false;
        };
        self.can_pop()
            && route.transition.route_animation().is_some()
            && self
                .pop_guard()
                .is_none_or(|guard| guard(&route) == PopDecision::Allow)
    }

    /// Flutter `TransitionRoute.handleStartBackGesture`: the current route's
    /// animation jumps to `progress` (1 shows the route, 0 has it gone) and
    /// a user gesture starts. Returns `false` when the current route has no
    /// route animation.
    pub fn handle_start_back_gesture(&self, progress: f32) -> bool {
        let Some(route) = self.current() else {
            return false;
        };
        let Some(animation) = route.transition.route_animation() else {
            return false;
        };
        let controller = animation.animation();
        controller.stop();
        controller.set_value(progress.clamp(0., 1.));
        self.set_back_gesture(BackGestureState {
            route: Some(route.id),
            user_gesture: true,
        });
        true
    }

    /// Flutter `handleUpdateBackGestureProgress`: follows the gesture, unless
    /// other navigation replaced the route that started it.
    pub fn handle_update_back_gesture_progress(&self, progress: f32) {
        if let Some(controller) = self.back_gesture_controller() {
            controller.set_value(progress.clamp(0., 1.));
        }
    }

    /// Flutter `handleCommitBackGesture`: pops the route. Its exit
    /// animation reverses from where the gesture left it. Flutter restarts
    /// that reverse from the upper bound because its Material predictive
    /// back transition switches to a separate commit animation; Incular's
    /// route transitions have none, so continuing from the gesture keeps the
    /// route where the finger left it.
    pub fn handle_commit_back_gesture(&self) {
        if self.back_gesture_controller().is_some() {
            let _ = self.pop();
        }
        self.end_back_gesture();
    }

    /// Flutter `handleCancelBackGesture`: the route animates back in.
    pub fn handle_cancel_back_gesture(&self) {
        if let Some(controller) = self.back_gesture_controller()
            && !controller.is_completed()
        {
            controller.forward_on_next_tick();
        }
        self.end_back_gesture();
    }

    /// Follows predictive back gestures from the platform (Flutter's
    /// `PredictiveBackPageTransitionsBuilder` observer): a swipe, not a
    /// button, on a route whose pop gesture is enabled starts with the
    /// route at `1 - progress`. The navigator is held weakly; dropping the
    /// registration stops observing.
    pub fn observe_back_gestures(&self) -> BackGestureObserverRegistration {
        add_shared_back_gesture_observer(Rc::new(NavigatorBackGestureObserver {
            navigator: self.downgrade(),
        }))
    }

    /// The current route's animation while it is the route that started
    /// the gesture (Flutter's `isCurrent` checks).
    fn back_gesture_controller(&self) -> Option<incular_widgets::AnimationController> {
        let started = self.back_gesture().route?;
        let route = self.current().filter(|route| route.id == started)?;
        Some(route.transition.route_animation()?.animation())
    }

    /// Flutter `didStopUserGesture`. Flutter waits for the settling
    /// animation's next status change; the gesture ends here at once.
    fn end_back_gesture(&self) {
        self.set_back_gesture(BackGestureState::default());
    }
}

struct NavigatorBackGestureObserver {
    navigator: crate::navigator::WeakNavigator,
}

impl BackGestureObserver for NavigatorBackGestureObserver {
    fn handle_start_back_gesture(&self, event: &PredictiveBackEvent) -> bool {
        let Some(navigator) = self.navigator.upgrade() else {
            return false;
        };
        // `PredictiveBackPageTransitionsBuilder`: `!backEvent.isButtonEvent
        // && _isEnabled`, with the route at `1 - backEvent.progress`.
        !event.is_button_event()
            && navigator.pop_gesture_enabled()
            && navigator.handle_start_back_gesture(1. - event.progress)
    }

    fn handle_update_back_gesture_progress(&self, event: &PredictiveBackEvent) {
        if let Some(navigator) = self.navigator.upgrade() {
            navigator.handle_update_back_gesture_progress(1. - event.progress);
        }
    }

    fn handle_commit_back_gesture(&self) {
        if let Some(navigator) = self.navigator.upgrade() {
            navigator.handle_commit_back_gesture();
        }
    }

    fn handle_cancel_back_gesture(&self) {
        if let Some(navigator) = self.navigator.upgrade() {
            navigator.handle_cancel_back_gesture();
        }
    }

    /// `WidgetsApp.didPopRoute`: `navigator.maybePop()`, which leaves the
    /// first route in place so the system back can leave the app.
    fn handle_pop_route(&self) -> bool {
        self.navigator.upgrade().is_some_and(|navigator| {
            navigator.can_pop()
                && matches!(
                    navigator.maybe_pop(),
                    crate::navigator::PopResult::Popped(_) | crate::navigator::PopResult::Blocked
                )
        })
    }
}
