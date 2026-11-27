//! Flutter's primary-pointer recognizer lifecycle, using the explicit arena
//! and pointer router. The host owns add/close/route/sweep order.

use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::{Rc, Weak},
    time::Duration,
};

use incular_core::{DeviceGestureSettings, PointerPhase, Transform, time::Instant};

use crate::{
    GestureArenaClient, GestureArenaKey, GestureArenaManager, GestureArenaRegistration,
    GestureDisposition, OffsetPair, PointerRouteId, PointerRouter, RawPointerEvent,
    constants::TOUCH_SLOP,
};

/// The observable state of a one-sequence primary-pointer recognizer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GestureRecognizerState {
    #[default]
    Ready,
    Possible,
    Defunct,
}

/// Global movement allowed before or after winning the arena.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GestureSlopTolerance {
    /// Uses the current device settings, or 18 logical pixels.
    #[default]
    DeviceDefault,
    Pixels(f32),
    Unlimited,
}

impl GestureSlopTolerance {
    fn resolve(self, settings: DeviceGestureSettings) -> Option<f32> {
        match self {
            Self::DeviceDefault => Some(settings.touch_slop.unwrap_or(TOUCH_SLOP)),
            Self::Pixels(value) => Some(value),
            Self::Unlimited => None,
        }
    }
}

/// Configuration of the generic base recognizer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PrimaryPointerGestureOptions {
    pub pre_accept: GestureSlopTolerance,
    pub post_accept: GestureSlopTolerance,
    pub deadline: Option<Duration>,
}

/// Custom recognition logic. Returning a disposition resolves all of the
/// current sequence's arena memberships; `None` leaves them undecided.
pub trait PrimaryPointerGestureHandler {
    /// A short name for diagnostic output.
    fn debug_description(&self) -> &'static str {
        "primary pointer"
    }

    /// The developer-facing class name used in allocation events.
    fn debug_class_name(&self) -> &'static str {
        "PrimaryPointerGestureRecognizer"
    }

    fn accepts_pointer(&self, _down: RawPointerEvent) -> bool {
        true
    }
    fn handle_primary_pointer(&self, event: RawPointerEvent) -> Option<GestureDisposition>;
    fn did_exceed_deadline(&self, _down: RawPointerEvent) -> Option<GestureDisposition> {
        None
    }
    fn did_resolve(&self, _key: GestureArenaKey, _disposition: GestureDisposition) {}
}

/// Recognizes one primary pointer while retaining additional pointer
/// routes until their sequence ends. The host supplies the per-view router
/// and drives its normal arena and frame clocks. Callbacks run without a
/// borrowed state so they can register or remove routes of their own.
pub struct PrimaryPointerGestureRecognizer {
    inner: Rc<Inner>,
    #[cfg(debug_assertions)]
    allocation: Cell<Option<incular_core::memory::allocations::ObjectLifetime>>,
    #[cfg(debug_assertions)]
    debug_owner: Option<String>,
}

struct Inner {
    router: PointerRouter,
    arena: GestureArenaManager,
    view: u64,
    options: PrimaryPointerGestureOptions,
    settings: Cell<DeviceGestureSettings>,
    handler: Box<dyn PrimaryPointerGestureHandler>,
    state: RefCell<State>,
}

#[derive(Default)]
struct State {
    phase: GestureRecognizerState,
    primary: Option<(GestureArenaKey, u64)>,
    initial: Option<OffsetPair>,
    accepted: bool,
    deadline: Option<(Instant, RawPointerEvent)>,
    entries: Vec<Tracked>,
    next_token: u64,
    disposed: bool,
}

struct Tracked {
    key: GestureArenaKey,
    token: u64,
    route: Option<PointerRouteId>,
    registration: Option<GestureArenaRegistration>,
    awaiting_decision: bool,
}

struct Client {
    owner: Weak<Inner>,
    token: u64,
}

impl GestureArenaClient for Client {
    fn accept_gesture(&self, key: GestureArenaKey) {
        if let Some(owner) = self.owner.upgrade() {
            owner.notify(key, self.token, GestureDisposition::Accepted);
        }
    }

    fn reject_gesture(&self, key: GestureArenaKey) {
        if let Some(owner) = self.owner.upgrade() {
            owner.notify(key, self.token, GestureDisposition::Rejected);
        }
    }
}

impl fmt::Debug for PrimaryPointerGestureRecognizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut details = f.debug_struct(self.inner.handler.debug_class_name());
        details
            .field("description", &self.inner.handler.debug_description())
            .field("state", &self.state())
            .field("primary_pointer", &self.primary_pointer())
            .field("initial_position", &self.initial_position());
        #[cfg(debug_assertions)]
        if let Some(owner) = &self.debug_owner {
            details.field("debug_owner", owner);
        }
        details.finish()
    }
}

impl PrimaryPointerGestureRecognizer {
    /// Builds a recognizer attached to one view's explicit arena and router.
    ///
    /// # Panics
    /// Panics for negative or NaN slop.
    #[must_use]
    pub fn new(
        view: u64,
        router: PointerRouter,
        arena: GestureArenaManager,
        options: PrimaryPointerGestureOptions,
        handler: impl PrimaryPointerGestureHandler + 'static,
    ) -> Self {
        for slop in [options.pre_accept, options.post_accept] {
            assert!(!matches!(slop, GestureSlopTolerance::Pixels(value) if !(value >= 0.)));
        }
        #[cfg(debug_assertions)]
        let allocation = Cell::new(Some(
            incular_core::memory::allocations::ObjectLifetime::new(
                "gestures",
                handler.debug_class_name(),
            ),
        ));
        Self {
            inner: Rc::new(Inner {
                router,
                arena,
                view,
                options,
                settings: Cell::new(DeviceGestureSettings::default()),
                handler: Box::new(handler),
                state: RefCell::new(State::default()),
            }),
            #[cfg(debug_assertions)]
            allocation,
            #[cfg(debug_assertions)]
            debug_owner: None,
        }
    }

    /// Labels this recognizer for diagnostic output. Drops the label in
    /// release builds rather than allocating it on the input path.
    #[must_use]
    pub fn with_debug_owner(self, owner: impl fmt::Display) -> Self {
        #[cfg(debug_assertions)]
        {
            let mut this = self;
            this.debug_owner = Some(owner.to_string());
            this
        }
        #[cfg(not(debug_assertions))]
        {
            drop(owner);
            self
        }
    }

    pub fn set_gesture_settings(&self, settings: DeviceGestureSettings) {
        self.inner.settings.set(settings);
    }

    #[must_use]
    pub fn pre_accept_slop_tolerance(&self) -> Option<f32> {
        self.inner
            .options
            .pre_accept
            .resolve(self.inner.settings.get())
    }

    #[must_use]
    pub fn post_accept_slop_tolerance(&self) -> Option<f32> {
        self.inner
            .options
            .post_accept
            .resolve(self.inner.settings.get())
    }

    #[must_use]
    pub fn state(&self) -> GestureRecognizerState {
        self.inner.state.borrow().phase
    }

    /// When ready again, still returns the most recently tracked primary ID.
    #[must_use]
    pub fn primary_pointer(&self) -> Option<u64> {
        self.inner
            .state
            .borrow()
            .primary
            .and_then(|(key, _)| key.pointer_id())
    }

    #[must_use]
    pub fn initial_position(&self) -> Option<OffsetPair> {
        self.inner.state.borrow().initial
    }

    /// Starts following a down event and enters its arena. This must run
    /// before the host closes registration for the pointer.
    ///
    /// # Panics
    /// Panics if the host has already closed the arena for this pointer.
    pub fn add_pointer(&self, down: RawPointerEvent, transform: Option<Transform>) -> bool {
        if down.phase != PointerPhase::Down || self.inner.state.borrow().disposed {
            return false;
        }
        let down = if transform.is_some() {
            down.transformed(transform)
        } else {
            down
        };
        if !self.inner.handler.accepts_pointer(down) {
            if !self.inner.state.borrow().accepted {
                self.resolve(GestureDisposition::Rejected);
            }
            return false;
        }
        let key = GestureArenaKey::pointer_device(self.inner.view, down.device, down.pointer);
        let token = {
            let mut state = self.inner.state.borrow_mut();
            if state.disposed || state.entries.iter().any(|entry| entry.key == key) {
                return false;
            }
            state.next_token = state
                .next_token
                .checked_add(1)
                .expect("primary pointer token exhausted");
            state.next_token
        };
        let owner = Rc::downgrade(&self.inner);
        let registration = self.inner.arena.add(
            key,
            Rc::new(Client {
                owner: owner.clone(),
                token,
            }),
        );
        let route = self
            .inner
            .router
            .add_route(down.pointer, transform, move |routed| {
                if routed.event.device == down.device
                    && let Some(owner) = owner.upgrade()
                {
                    owner.handle(key, token, routed.event);
                }
            });
        let mut state = self.inner.state.borrow_mut();
        if state.phase == GestureRecognizerState::Ready {
            state.phase = GestureRecognizerState::Possible;
            state.primary = Some((key, token));
            state.initial = Some(OffsetPair::from_event_position(&down));
            state.deadline = self.inner.options.deadline.and_then(|delay| {
                down.time
                    .checked_add(delay)
                    .map(|deadline| (deadline, down))
            });
            state.accepted = false;
        }
        state.entries.push(Tracked {
            key,
            token,
            route: Some(route),
            registration: Some(registration),
            awaiting_decision: true,
        });
        true
    }

    pub fn resolve(&self, disposition: GestureDisposition) {
        self.inner.resolve(disposition);
    }

    /// Delivers a due deadline at most once; closing or resolving the
    /// recognizer first cancels it.
    pub fn advance_time(&self, now: Instant) {
        let due = {
            let mut state = self.inner.state.borrow_mut();
            state
                .deadline
                .filter(|(deadline, _)| now >= *deadline)
                .and_then(|_| state.deadline.take())
        };
        if let Some((_, down)) = due
            && let Some(result) = self.inner.handler.did_exceed_deadline(down)
        {
            self.resolve(result);
        }
    }

    pub fn dispose(&self) {
        self.inner.dispose();
        #[cfg(debug_assertions)]
        drop(self.allocation.take());
    }
}

impl Drop for PrimaryPointerGestureRecognizer {
    fn drop(&mut self) {
        self.dispose();
    }
}

impl Inner {
    fn resolve(&self, disposition: GestureDisposition) {
        if !matches!(
            disposition,
            GestureDisposition::Accepted | GestureDisposition::Rejected
        ) {
            return;
        }
        let entries: Vec<_> = self
            .state
            .borrow_mut()
            .entries
            .iter_mut()
            .filter_map(|entry| entry.registration.take())
            .collect();
        for entry in entries {
            entry.resolve(disposition);
        }
    }

    fn notify(&self, key: GestureArenaKey, token: u64, disposition: GestureDisposition) {
        {
            let mut state = self.state.borrow_mut();
            let Some(at) = state
                .entries
                .iter()
                .position(|entry| entry.key == key && entry.token == token)
            else {
                return;
            };
            let entry = &mut state.entries[at];
            entry.awaiting_decision = false;
            entry.registration = None;
            if entry.route.is_none() {
                state.entries.remove(at);
            }
            if state.primary == Some((key, token))
                && state.phase == GestureRecognizerState::Possible
            {
                state.deadline = None;
                if disposition == GestureDisposition::Accepted {
                    state.accepted = true;
                } else {
                    state.phase = GestureRecognizerState::Defunct;
                }
            }
        }
        self.handler.did_resolve(key, disposition);
    }

    fn handle(&self, key: GestureArenaKey, token: u64, event: RawPointerEvent) {
        struct CloseOnEnd<'a>(&'a Inner, GestureArenaKey, u64);
        impl Drop for CloseOnEnd<'_> {
            fn drop(&mut self) {
                self.0.stop_tracking(self.1, self.2);
            }
        }
        let _terminal_cleanup = matches!(event.phase, PointerPhase::Up | PointerPhase::Cancel)
            .then(|| CloseOnEnd(self, key, token));
        let primary = {
            let state = self.state.borrow();
            if state.phase == GestureRecognizerState::Possible
                && state.primary == Some((key, token))
            {
                state.initial.map(|position| (position, state.accepted))
            } else {
                None
            }
        };
        if let Some((initial, accepted)) = primary {
            let allowance = if accepted {
                self.options.post_accept
            } else {
                self.options.pre_accept
            };
            let delta = event.position - initial.global;
            if event.phase == PointerPhase::Move
                && allowance
                    .resolve(self.settings.get())
                    .is_some_and(|slop| delta.x.hypot(delta.y) > slop)
            {
                self.resolve(GestureDisposition::Rejected);
                self.stop_tracking(key, token);
            } else if let Some(disposition) = self.handler.handle_primary_pointer(event) {
                self.resolve(disposition);
            }
        }
    }

    fn stop_tracking(&self, key: GestureArenaKey, token: u64) {
        let removed = {
            let mut state = self.state.borrow_mut();
            let Some(at) = state
                .entries
                .iter()
                .position(|entry| entry.key == key && entry.token == token)
            else {
                return;
            };
            let route = state.entries[at].route.take();
            if !state.entries[at].awaiting_decision {
                state.entries.remove(at);
            }
            if state.entries.iter().all(|entry| entry.route.is_none()) {
                state.phase = GestureRecognizerState::Ready;
                state.initial = None;
                state.deadline = None;
                state.accepted = false;
            }
            route
        };
        if let Some(route) = removed
            && let Some(pointer) = key.pointer_id()
        {
            self.router.remove_route(pointer, route);
        }
    }

    fn dispose(&self) {
        let entries = {
            let mut state = self.state.borrow_mut();
            if state.disposed {
                return;
            }
            state.disposed = true;
            state.phase = GestureRecognizerState::Ready;
            state.initial = None;
            state.deadline = None;
            std::mem::take(&mut state.entries)
        };
        for entry in entries {
            if let Some(route) = entry.route
                && let Some(pointer) = entry.key.pointer_id()
            {
                self.router.remove_route(pointer, route);
            }
            if let Some(registration) = entry.registration {
                registration.resolve(GestureDisposition::Rejected);
            }
        }
    }
}
