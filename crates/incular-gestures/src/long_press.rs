//! Standalone long-press recognition with separate registration and routing.

use incular_core::{
    DeviceGestureSettings, PRIMARY_POINTER_BUTTON, PointerPhase, SECONDARY_POINTER_BUTTON,
    TERTIARY_POINTER_BUTTON, Transform, time::Instant,
};
use std::time::Duration;

use crate::{
    GestureCallbacks, GestureDecision, GestureDisposition, GestureRecognizer,
    PointerGestureRecognizer, RawPointerEvent, arena_member::ArenaLink,
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
    routed_down: bool,
    started: bool,
    engine: PointerGestureRecognizer,
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
        let mut engine =
            PointerGestureRecognizer::new(callbacks).with_long_press_duration(self.duration);
        engine.set_gesture_settings(self.gesture_settings);
        if let Some(transform) = self.local_transform {
            engine.set_local_transform(transform);
        }
        self.arena.add(down.pointer);
        self.contact = Some(Contact {
            pointer: down.pointer,
            routed_down: false,
            started: false,
            engine,
        });
        true
    }

    /// Closes the standalone (sole-member) arena without starting a hold.
    pub fn close_arena(&mut self, pointer: u64) -> bool {
        self.arena.close(pointer).is_some()
    }

    /// Removes a rejected member and cancels only an already-routed press.
    pub fn reject_gesture(&mut self, pointer: u64) {
        if self
            .contact
            .as_ref()
            .is_some_and(|contact| contact.pointer == pointer)
            && let Some(mut contact) = self.contact.take()
        {
            contact.engine.reject();
            let _ = self.arena.resolve(pointer, GestureDisposition::Rejected);
        }
    }

    /// Delivers a deadline separately from routing input.
    pub fn poll(&mut self, now: Instant) {
        let Some(contact) = self.contact.as_mut().filter(|contact| contact.routed_down) else {
            return;
        };
        if let GestureDecision::Accept(action) = contact.engine.poll(now) {
            contact.started = true;
            contact.engine.dispatch(action);
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
        if terminal || !contact.started && !contact.engine.has_pending_deadline() {
            self.contact = None;
            let _ = self.arena.resolve(pointer, GestureDisposition::Rejected);
        }
    }

    /// Abandons tracking without synthesizing a release or cancellation.
    pub fn dispose(&mut self) {
        self.contact = None;
        self.arena = ArenaLink::default();
    }
}
