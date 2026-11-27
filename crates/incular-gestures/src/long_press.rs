//! Standalone long-press recognition with separate registration and routing.

use incular_core::{
    DeviceGestureSettings, PRIMARY_POINTER_BUTTON, PointerPhase, SECONDARY_POINTER_BUTTON,
    TERTIARY_POINTER_BUTTON, Transform, time::Instant,
};
use std::time::Duration;

use crate::{
    GestureCallbacks, GestureDecision, GestureDisposition, GestureRecognizer,
    LongPressStartDetails, PointerGestureRecognizer, RawPointerEvent, arena_member::ArenaLink,
};

/// The direct long-press recognizer. Register a down event, settle its arena,
/// then route pointer events and elapsed deadlines separately. Closing an
/// arena is not a long-press acceptance: only the long-press deadline is.
///
/// This uses the same movement, button and release rules as the retained
/// pointer recognizer.
pub struct LongPressGestureRecognizer {
    callbacks: GestureCallbacks,
    duration: Duration,
    gesture_settings: DeviceGestureSettings,
    local_transform: Option<Transform>,
    contact: Option<Contact>,
    arena: ArenaLink,
}

struct Contact {
    pointer: u64,
    deadline: Option<Instant>,
    routed_down: bool,
    started: bool,
    award: ArenaAward,
    engine: PointerGestureRecognizer,
}

enum ArenaAward {
    Standalone,
    Pending(Option<LongPressStartDetails>),
    Accepted,
}

impl LongPressGestureRecognizer {
    #[must_use]
    pub fn new(callbacks: GestureCallbacks) -> Self {
        Self {
            callbacks,
            duration: PointerGestureRecognizer::LONG_PRESS_TIMEOUT,
            gesture_settings: DeviceGestureSettings::default(),
            local_transform: None,
            contact: None,
            arena: ArenaLink::default(),
        }
    }

    /// Overrides the default long-press deadline of 500 ms.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Sends resolutions to a host arena instead of acting as the sole
    /// member. The resolver may award the request synchronously or return
    /// `None` and call [`Self::accept_gesture`] or [`Self::reject_gesture`]
    /// after the recognizer returns. Configure it before adding a pointer.
    #[must_use]
    pub fn with_external_arena(
        mut self,
        resolver: impl FnMut(u64, GestureDisposition) -> Option<GestureDisposition> + 'static,
    ) -> Self {
        self.arena.set_external(Box::new(resolver));
        self
    }

    pub fn set_gesture_settings(&mut self, settings: DeviceGestureSettings) {
        self.gesture_settings = settings;
        if let Some(contact) = &mut self.contact {
            contact.engine.set_gesture_settings(settings);
        }
    }

    pub fn set_local_transform(&mut self, global_to_local: Transform) {
        self.local_transform = Some(global_to_local);
        if let Some(contact) = &mut self.contact {
            contact.engine.set_local_transform(global_to_local);
        }
    }

    /// Joins the pointer's arena without firing a down or start callback.
    #[must_use]
    pub fn add_pointer(&mut self, down: RawPointerEvent) -> bool {
        if down.phase != PointerPhase::Down
            || self.contact.is_some()
            || !self.callbacks.allows_pointer_device(down.kind)
            || !matches!(
                down.buttons,
                PRIMARY_POINTER_BUTTON | SECONDARY_POINTER_BUTTON | TERTIARY_POINTER_BUTTON
            )
        {
            return false;
        }
        let Some(button) = self.callbacks.for_button_mask(down.buttons) else {
            return false;
        };
        let callbacks = GestureCallbacks {
            on_long_press_down: button.on_long_press_down,
            on_long_press_cancel: button.on_long_press_cancel,
            on_long_press: button.on_long_press,
            on_long_press_start: button.on_long_press_start,
            on_long_press_move_update: button.on_long_press_move_update,
            on_long_press_up: button.on_long_press_up,
            on_long_press_end: button.on_long_press_end,
            ..Default::default()
        };
        if callbacks.on_long_press_down.is_none()
            && callbacks.on_long_press_cancel.is_none()
            && callbacks.on_long_press.is_none()
            && callbacks.on_long_press_start.is_none()
            && callbacks.on_long_press_move_update.is_none()
            && callbacks.on_long_press_up.is_none()
            && callbacks.on_long_press_end.is_none()
        {
            return false;
        }
        let mut engine = PointerGestureRecognizer::new(callbacks);
        let award = if self.arena.has_external() {
            engine = engine.with_external_long_press_deadline();
            ArenaAward::Pending(None)
        } else {
            engine = engine.with_long_press_duration(self.duration);
            ArenaAward::Standalone
        };
        engine.set_gesture_settings(self.gesture_settings);
        if let Some(transform) = self.local_transform {
            engine.set_local_transform(transform);
        }
        self.arena.add(down.pointer);
        self.contact = Some(Contact {
            pointer: down.pointer,
            deadline: down.time.checked_add(self.duration),
            routed_down: false,
            started: false,
            award,
            engine,
        });
        true
    }

    /// Closes the standalone (sole-member) arena without starting a hold.
    pub fn close_arena(&mut self, pointer: u64) -> bool {
        self.arena.close(pointer).is_some()
    }

    /// Receives an arena award. Winning by default before the deadline
    /// does not cause an early long press.
    pub fn accept_gesture(&mut self, pointer: u64) {
        let Some(contact) = self
            .contact
            .as_mut()
            .filter(|contact| contact.pointer == pointer)
        else {
            return;
        };
        if let ArenaAward::Pending(due) = contact.award {
            contact.award = ArenaAward::Accepted;
            if let Some(details) = due {
                start_awarded_hold(contact, details);
            }
        }
    }

    /// Applies a host rejection without requesting a second resolution.
    pub fn reject_gesture(&mut self, pointer: u64) {
        if self
            .contact
            .as_ref()
            .is_some_and(|contact| contact.pointer == pointer)
            && let Some(mut contact) = self.contact.take()
        {
            contact.engine.reject();
            if !self.arena.has_external() {
                let _ = self.arena.resolve(pointer, GestureDisposition::Rejected);
            }
        }
    }

    /// Delivers a deadline separately from routing input.
    pub fn poll(&mut self, now: Instant) {
        let Some(contact) = self.contact.as_mut().filter(|contact| contact.routed_down) else {
            return;
        };
        if matches!(contact.award, ArenaAward::Pending(Some(_))) {
            return;
        }
        let Some(details) = contact.engine.due_long_press(now, self.duration) else {
            return;
        };
        if matches!(contact.award, ArenaAward::Pending(None)) {
            contact.award = ArenaAward::Pending(Some(details));
            let pointer = contact.pointer;
            match self.arena.resolve(pointer, GestureDisposition::Accepted) {
                Some(GestureDisposition::Accepted) => self.accept_gesture(pointer),
                Some(GestureDisposition::Rejected) => self.reject_gesture(pointer),
                _ => {}
            }
        } else {
            start_awarded_hold(contact, details);
        }
    }

    /// Routes a registered pointer. The actual down, not registration or
    /// closing the arena, delivers `onLongPressDown`.
    pub fn handle_event(&mut self, event: RawPointerEvent) {
        let Some(contact) = self
            .contact
            .as_ref()
            .filter(|contact| contact.pointer == event.pointer)
        else {
            return;
        };
        let down = event.phase == PointerPhase::Down;
        if down == contact.routed_down {
            return;
        }
        self.poll(event.time);
        let Some(contact) = self.contact.as_mut() else {
            return;
        };
        let pointer = event.pointer;
        let terminal = matches!(event.phase, PointerPhase::Up | PointerPhase::Cancel);
        contact.routed_down = true;
        let decision = contact.engine.observe_raw(event);
        match decision {
            GestureDecision::Accept(action) => {
                contact.started = true;
                contact.engine.dispatch(action);
            }
            GestureDecision::Cancelled => contact.engine.cancel(),
            _ => {}
        }
        if terminal || !contact.started && !contact.engine.has_pending_long_press() {
            let withdraw = !matches!(contact.award, ArenaAward::Accepted);
            self.contact = None;
            if withdraw {
                let _ = self.arena.resolve(pointer, GestureDisposition::Rejected);
            }
        }
    }

    /// Abandons tracking. A routed hold that is still pending is canceled;
    /// disposal never synthesizes a release for an accepted hold.
    pub fn dispose(&mut self) {
        if let Some(mut contact) = self.contact.take() {
            if contact.routed_down && !contact.started {
                contact.engine.reject();
            }
            if !matches!(contact.award, ArenaAward::Accepted) {
                let _ = self
                    .arena
                    .resolve(contact.pointer, GestureDisposition::Rejected);
            }
        }
        self.arena = ArenaLink::default();
    }
}

impl crate::ArenaRecognizer for LongPressGestureRecognizer {
    fn set_external_arena(&mut self, resolver: crate::ArenaResolver) {
        self.arena.set_external(resolver);
    }

    fn add_pointer(&mut self, event: RawPointerEvent) {
        if !LongPressGestureRecognizer::add_pointer(self, event) {
            let _ = self.arena.resolve(event.pointer, GestureDisposition::Rejected);
        }
    }

    fn handle_event(&mut self, event: RawPointerEvent) {
        LongPressGestureRecognizer::handle_event(self, event);
    }

    fn accept_gesture(&mut self, pointer: u64) {
        LongPressGestureRecognizer::accept_gesture(self, pointer);
    }

    fn reject_gesture(&mut self, pointer: u64) {
        // Flutter's routed long press emits cancel when it withdraws its
        // own contact (release, motion or pointer cancellation), not when
        // a different arena member wins. That loss only stops tracking.
        if self.contact.as_ref().is_some_and(|contact| contact.pointer == pointer) {
            self.contact = None;
        }
    }

    fn cancel_gesture(&mut self, pointer: u64) {
        LongPressGestureRecognizer::reject_gesture(self, pointer);
    }

    fn deadline_for_pointer(&self, pointer: u64) -> Option<Instant> {
        let contact = self.contact.as_ref().filter(|contact| contact.pointer == pointer)?;
        if contact.routed_down
            && !contact.started
            && !matches!(contact.award, ArenaAward::Pending(Some(_)))
            && contact.engine.has_pending_long_press()
        {
            contact.deadline
        } else {
            None
        }
    }

    fn poll(&mut self, now: Instant) {
        LongPressGestureRecognizer::poll(self, now);
    }
}

fn start_awarded_hold(contact: &mut Contact, details: LongPressStartDetails) {
    contact.started = true;
    let action = contact.engine.begin_long_press(details);
    contact.engine.dispatch(action);
}
