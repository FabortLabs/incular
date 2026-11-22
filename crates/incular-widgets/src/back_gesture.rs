//! Flutter's predictive back dispatch (`WidgetsBinding._handleStartBackGesture`
//! and its siblings in `widgets/binding.dart`): the observers that may
//! follow a system back gesture, and which of them took the current one.

use std::{cell::RefCell, rc::Rc};

use incular_platform::{BackGestureEvent, PredictiveBackEvent};

/// The back gesture part of Flutter's `WidgetsBindingObserver`. All methods
/// default to ignoring the gesture.
pub trait BackGestureObserver {
    /// Flutter `handleStartBackGesture`: whether this observer follows the
    /// gesture that `event` starts. Only observers that return `true` hear
    /// about its progress, commit and cancellation.
    fn handle_start_back_gesture(&self, event: &PredictiveBackEvent) -> bool {
        let _ = event;
        false
    }

    /// Flutter `handleUpdateBackGestureProgress`.
    fn handle_update_back_gesture_progress(&self, event: &PredictiveBackEvent) {
        let _ = event;
    }

    /// Flutter `handleCommitBackGesture`: the back happens.
    fn handle_commit_back_gesture(&self) {}

    /// Flutter `handleCancelBackGesture`: the back does not happen.
    fn handle_cancel_back_gesture(&self) {}

    /// Flutter `didPopRoute`: the system asked to go back (Android's back
    /// button, or a back gesture nobody followed). Returns whether this
    /// observer handled it, such as a navigator popping its top route.
    fn handle_pop_route(&self) -> bool {
        false
    }
}

type Observer = Rc<dyn BackGestureObserver>;

#[derive(Default)]
struct Registry {
    next: u64,
    observers: Vec<(u64, Observer)>,
    /// `_backGestureObservers`: the observers following the current gesture.
    active: Vec<(u64, Observer)>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::default();
}

/// Keeps an observer registered; dropping it removes the observer, which is
/// first told to cancel a gesture it is following.
#[must_use = "dropping the registration removes the observer"]
pub struct BackGestureObserverRegistration {
    id: u64,
}

impl Drop for BackGestureObserverRegistration {
    fn drop(&mut self) {
        let following = REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            registry.observers.retain(|(id, _)| *id != self.id);
            let index = registry.active.iter().position(|(id, _)| *id == self.id)?;
            Some(registry.active.remove(index).1)
        });
        // Flutter `removeObserver` cancels the gesture for an observer that
        // leaves in the middle of it.
        if let Some(observer) = following {
            observer.handle_cancel_back_gesture();
        }
    }
}

/// Flutter `WidgetsBinding.addObserver` for back gestures. Observers live on
/// the UI thread, in registration order.
pub fn add_back_gesture_observer(
    observer: impl BackGestureObserver + 'static,
) -> BackGestureObserverRegistration {
    add_shared_back_gesture_observer(Rc::new(observer))
}

/// [`add_back_gesture_observer`] for an observer the caller keeps a handle
/// to.
pub fn add_shared_back_gesture_observer(
    observer: Rc<dyn BackGestureObserver>,
) -> BackGestureObserverRegistration {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.next += 1;
        let id = registry.next;
        registry.observers.push((id, observer));
        BackGestureObserverRegistration { id }
    })
}

/// Flutter `WidgetsBinding.handlePopRoute`: asks the observers, newest
/// first, to handle a system back until one does. Returns `false` when none
/// did; the host then leaves the app, as Flutter's `SystemNavigator.pop`.
pub fn dispatch_pop_route() -> bool {
    let observers = REGISTRY.with(|registry| registry.borrow().observers.clone());
    observers
        .iter()
        .rev()
        .any(|(_, observer)| observer.handle_pop_route())
}

/// Delivers one stage of a back gesture to the observers, as Flutter's
/// binding does with the `flutter/backgesture` messages:
///
/// - `Start` clears the followers and asks every observer, in order; those
///   answering `true` follow the gesture. Returns whether any did.
/// - `Update`, `Commit` and `Cancel` go to the followers only. They return
///   whether there were any; a `Commit` nobody followed returns `false`, and
///   the caller then handles it as an ordinary back, as Flutter's
///   `handlePopRoute` fallback does.
pub fn dispatch_back_gesture(event: &BackGestureEvent) -> bool {
    match event {
        BackGestureEvent::Start(event) => {
            let observers = REGISTRY.with(|registry| {
                let mut registry = registry.borrow_mut();
                registry.active.clear();
                registry.observers.clone()
            });
            let mut following = Vec::new();
            for (id, observer) in observers {
                if observer.handle_start_back_gesture(event) {
                    following.push((id, observer));
                }
            }
            let any = !following.is_empty();
            REGISTRY.with(|registry| registry.borrow_mut().active = following);
            any
        }
        BackGestureEvent::Update(event) => {
            let active = REGISTRY.with(|registry| registry.borrow().active.clone());
            for (_, observer) in &active {
                observer.handle_update_back_gesture_progress(event);
            }
            !active.is_empty()
        }
        BackGestureEvent::Commit => {
            let active = REGISTRY.with(|registry| registry.borrow().active.clone());
            for (_, observer) in &active {
                observer.handle_commit_back_gesture();
            }
            !active.is_empty()
        }
        BackGestureEvent::Cancel => {
            let active = REGISTRY.with(|registry| registry.borrow().active.clone());
            for (_, observer) in &active {
                observer.handle_cancel_back_gesture();
            }
            !active.is_empty()
        }
    }
}
