//! An independently registered double tap. A released first press can hold
//! its arena while the recognizer tracks an entirely different pointer.

use std::{
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    rc::{Rc, Weak},
};

use incular_core::{PRIMARY_POINTER_BUTTON, PointerPhase, time::Instant};

use crate::{
    GestureArenaClient, GestureArenaKey, GestureArenaManager, GestureArenaRegistration,
    GestureDisposition, PointerDeviceKind, RawPointerEvent, TapDownDetails,
    arena_member::distance,
    constants::{DOUBLE_TAP_MIN_TIME, DOUBLE_TAP_SLOP, DOUBLE_TAP_TIMEOUT, DOUBLE_TAP_TOUCH_SLOP},
};

type Handler = Rc<dyn Fn()>;
type DownHandler = Rc<dyn Fn(TapDownDetails)>;
type ButtonFilter = Rc<dyn Fn(u32) -> bool>;

#[derive(Default)]
struct State {
    first: Option<FirstTap>,
    active: Vec<Tracker>,
}

struct FirstTap {
    tracker: Tracker,
    released_at: Instant,
}

struct Tracker {
    key: GestureArenaKey,
    down: RawPointerEvent,
    entry: GestureArenaRegistration,
}

struct Shared {
    state: RefCell<State>,
    callbacks: RefCell<Callbacks>,
}

#[derive(Default)]
struct Callbacks {
    down: Option<DownHandler>,
    tap: Option<Handler>,
    cancel: Option<Handler>,
}

impl Shared {
    fn reset(&self, notify: bool) {
        let (first, active) = {
            let mut state = self.state.borrow_mut();
            (state.first.take(), std::mem::take(&mut state.active))
        };
        let cancellation = catch_unwind(AssertUnwindSafe(|| {
            if notify && first.is_some() && !active.is_empty() {
                self.notify_cancel();
            }
        }));
        Self::release(first);
        for tracker in active {
            tracker.entry.resolve(GestureDisposition::Rejected);
        }
        if let Err(panic) = cancellation {
            resume_unwind(panic);
        }
    }

    fn release(first: Option<FirstTap>) {
        if let Some(first) = first {
            first.tracker.entry.resolve(GestureDisposition::Rejected);
            first.tracker.entry.release();
        }
    }

    fn notify_cancel(&self) {
        let callback = self.callbacks.borrow().cancel.clone();
        if let Some(callback) = callback {
            callback();
        }
    }

    fn reject(&self, key: GestureArenaKey) {
        let (rejected, first, cancel) = {
            let mut state = self.state.borrow_mut();
            if state
                .first
                .as_ref()
                .is_some_and(|first| first.tracker.key == key)
            {
                drop(state);
                self.reset(true);
                return;
            }
            let Some(index) = state.active.iter().position(|tracker| tracker.key == key) else {
                return;
            };
            let tracker = state.active.remove(index);
            let cancel = state.first.is_some();
            let first = if state.active.is_empty() {
                state.first.take()
            } else {
                None
            };
            (tracker, first, cancel)
        };
        rejected.entry.resolve(GestureDisposition::Rejected);
        let cancellation = catch_unwind(AssertUnwindSafe(|| {
            if cancel {
                self.notify_cancel();
            }
        }));
        Self::release(first);
        if let Err(panic) = cancellation {
            resume_unwind(panic);
        }
    }
}

struct Client(Weak<Shared>);

impl GestureArenaClient for Client {
    fn accept_gesture(&self, _: GestureArenaKey) {}

    fn reject_gesture(&self, key: GestureArenaKey) {
        if let Some(shared) = self.0.upgrade() {
            shared.reject(key);
        }
    }
}

/// A standalone double-tap recognizer for a managed gesture arena.
///
/// Call add_pointer before closing each pointer's arena, route for subsequent
/// pointer events, and advance_to at the timer deadline reported by deadline.
pub struct DoubleTapGestureRecognizer {
    shared: Rc<Shared>,
    client: Rc<Client>,
    filter: Option<ButtonFilter>,
    devices: Option<Vec<PointerDeviceKind>>,
}

impl Default for DoubleTapGestureRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl DoubleTapGestureRecognizer {
    #[must_use]
    pub fn new() -> Self {
        let shared = Rc::new(Shared {
            state: RefCell::new(State::default()),
            callbacks: RefCell::new(Callbacks::default()),
        });
        let client = Rc::new(Client(Rc::downgrade(&shared)));
        Self {
            shared,
            client,
            filter: None,
            devices: None,
        }
    }

    #[must_use]
    pub fn allowed_buttons_filter(mut self, filter: impl Fn(u32) -> bool + 'static) -> Self {
        self.filter = Some(Rc::new(filter));
        self
    }

    #[must_use]
    pub fn supported_devices(
        mut self,
        devices: impl IntoIterator<Item = PointerDeviceKind>,
    ) -> Self {
        self.devices = Some(devices.into_iter().collect());
        self
    }

    #[must_use]
    pub fn on_double_tap_down(self, callback: impl Fn(TapDownDetails) + 'static) -> Self {
        self.shared.callbacks.borrow_mut().down = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_double_tap(self, callback: impl Fn() + 'static) -> Self {
        self.shared.callbacks.borrow_mut().tap = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_double_tap_cancel(self, callback: impl Fn() + 'static) -> Self {
        self.shared.callbacks.borrow_mut().cancel = Some(Rc::new(callback));
        self
    }

    /// Registers an allowed Down. Rejecting a mask on this route resets
    /// any held first tap without joining the newly opened arena.
    pub fn add_pointer(
        &self,
        arena: &GestureArenaManager,
        key: GestureArenaKey,
        event: RawPointerEvent,
    ) -> bool {
        if event.phase != PointerPhase::Down || key.pointer_id() != Some(event.pointer) {
            return false;
        }
        {
            let callbacks = self.shared.callbacks.borrow();
            if callbacks.down.is_none() && callbacks.tap.is_none() && callbacks.cancel.is_none() {
                return false;
            }
        }
        let allowed = self
            .filter
            .as_ref()
            .map_or(event.buttons == PRIMARY_POINTER_BUTTON, |filter| {
                filter(event.buttons)
            })
            && self
                .devices
                .as_ref()
                .is_none_or(|devices| devices.contains(&event.kind));
        if !allowed {
            self.shared.reset(true);
            return false;
        }

        self.advance_to(event.time);
        let match_kind = {
            let state = self.shared.state.borrow();
            if state.active.iter().any(|tracker| tracker.key == key)
                || state
                    .first
                    .as_ref()
                    .is_some_and(|first| first.tracker.key == key)
            {
                return false;
            }
            match &state.first {
                None => Match::Fresh,
                Some(first)
                    if distance(event.position - first.tracker.down.position) > DOUBLE_TAP_SLOP =>
                {
                    return false;
                }
                Some(first)
                    if first.tracker.down.buttons != event.buttons
                        || event
                            .time
                            .saturating_duration_since(first.tracker.down.time)
                            < DOUBLE_TAP_MIN_TIME =>
                {
                    Match::Restart
                }
                Some(_) => Match::Second,
            }
        };
        if match_kind == Match::Restart {
            self.shared.reset(true);
        }
        let entry = arena.add(key, self.client.clone());
        self.shared.state.borrow_mut().active.push(Tracker {
            key,
            down: event,
            entry,
        });
        let on_down = (match_kind == Match::Second)
            .then(|| self.shared.callbacks.borrow().down.clone())
            .flatten();
        if let Some(callback) = on_down {
            callback(TapDownDetails {
                global_position: event.position,
                local_position: event.local_position,
                kind: event.kind,
            });
        }
        true
    }

    /// Delivers a Move, Up, or Cancel to a previously registered pointer.
    pub fn route(&self, key: GestureArenaKey, event: RawPointerEvent) {
        if key.pointer_id() != Some(event.pointer) {
            return;
        }
        match event.phase {
            PointerPhase::Move => {
                let moved = self.shared.state.borrow().active.iter().any(|tracker| {
                    tracker.key == key
                        && distance(event.position - tracker.down.position) > DOUBLE_TAP_TOUCH_SLOP
                });
                if moved {
                    self.shared.reject(key);
                }
            }
            PointerPhase::Cancel => {
                let active = self
                    .shared
                    .state
                    .borrow()
                    .active
                    .iter()
                    .any(|tracker| tracker.key == key);
                if active {
                    self.shared.reject(key);
                }
            }
            PointerPhase::Up => self.release_press(key, event.time),
            _ => {}
        }
    }

    fn release_press(&self, key: GestureArenaKey, now: Instant) {
        let (pair, others) = {
            let mut state = self.shared.state.borrow_mut();
            let Some(index) = state.active.iter().position(|tracker| tracker.key == key) else {
                return;
            };
            let tracker = state.active.remove(index);
            let pair = if let Some(first) = state.first.take() {
                Some((first, tracker))
            } else {
                tracker.entry.hold();
                state.first = Some(FirstTap {
                    tracker,
                    released_at: now,
                });
                None
            };
            // Keep the buffer for the normal one-pointer case.
            let others = if state.active.is_empty() {
                Vec::new()
            } else {
                std::mem::take(&mut state.active)
            };
            (pair, others)
        };

        let notify = pair.is_some();
        if let Some((first, second)) = pair {
            first.tracker.entry.resolve(GestureDisposition::Accepted);
            first.tracker.entry.release();
            second.entry.resolve(GestureDisposition::Accepted);
        }
        for tracker in others {
            tracker.entry.resolve(GestureDisposition::Rejected);
        }
        if notify {
            let callback = self.shared.callbacks.borrow().tap.clone();
            if let Some(callback) = callback {
                callback();
            }
        }
    }

    /// The held first tap expires only while waiting for a second press.
    #[must_use]
    pub fn deadline(&self) -> Option<Instant> {
        let state = self.shared.state.borrow();
        if !state.active.is_empty() {
            return None;
        }
        state
            .first
            .as_ref()?
            .released_at
            .checked_add(DOUBLE_TAP_TIMEOUT)
    }

    /// Releases a timed-out first arena so a passive tap contender can win.
    pub fn advance_to(&self, now: Instant) {
        if self.deadline().is_some_and(|deadline| now >= deadline) {
            self.shared.reset(false);
        }
    }

    /// Cancels tracking and releases any held arena.
    pub fn reset(&self) {
        self.shared.reset(true);
    }
}

impl Drop for DoubleTapGestureRecognizer {
    fn drop(&mut self) {
        self.shared.reset(false);
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Match {
    Fresh,
    Restart,
    Second,
}
