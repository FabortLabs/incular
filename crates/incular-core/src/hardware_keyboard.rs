//! Pressed key and lock state over the portable native event stream.

use std::{
    any::Any,
    cell::RefCell,
    collections::{HashMap, HashSet},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::{Rc, Weak},
};

use crate::input::{Code, Key, KeyState, KeyboardEvent, Modifiers, NamedKey};

/// The keys and toggled locks observed from a stream of hardware events.
/// Physical positions own presses so a release still clears its press if
/// the platform changed the logical label while it was held.
#[derive(Clone, Debug, Default)]
pub struct HardwareKeyboardState {
    pressed: HashMap<Code, Key>,
    locks: Modifiers,
}

/// An event disagreed with the previous pressed-key state. Incular still
/// applies and routes that event rather than dropping it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IrregularKeyEvent {
    AlreadyPressed,
    NotPressed,
    MismatchedLogicalKey,
}

impl HardwareKeyboardState {
    /// Updates the state before forwarding an event to application handlers.
    pub fn observe(&mut self, event: &KeyboardEvent) {
        let _ = self.observe_checked(event);
    }

    /// Applies the event and reports any inconsistency in the prior state.
    /// A new press or repeat establishes the latest logical key, while a
    /// release always clears the physical position.
    #[must_use]
    pub fn observe_checked(&mut self, event: &KeyboardEvent) -> Option<IrregularKeyEvent> {
        let previous = self.pressed.get(&event.code);
        let irregularity = if event.state.is_down() && !event.repeat {
            previous
                .is_some()
                .then_some(IrregularKeyEvent::AlreadyPressed)
        } else {
            match previous {
                None => Some(IrregularKeyEvent::NotPressed),
                Some(key) if key != &event.key => Some(IrregularKeyEvent::MismatchedLogicalKey),
                Some(_) => None,
            }
        };

        match event.state {
            KeyState::Up => {
                self.pressed.remove(&event.code);
            }
            KeyState::Down => {
                if !event.repeat {
                    match &event.key {
                        Key::Named(NamedKey::CapsLock) => self.locks.toggle(Modifiers::CAPS_LOCK),
                        Key::Named(NamedKey::NumLock) => self.locks.toggle(Modifiers::NUM_LOCK),
                        Key::Named(NamedKey::ScrollLock) => {
                            self.locks.toggle(Modifiers::SCROLL_LOCK)
                        }
                        _ => {}
                    }
                }
                // Repeats and irregular presses can change the logical label.
                if previous != Some(&event.key) {
                    self.pressed.insert(event.code, event.key.clone());
                }
            }
        }
        irregularity
    }

    /// The physical positions currently pressed.
    #[must_use]
    pub fn physical_keys_pressed(&self) -> HashSet<Code> {
        self.pressed.keys().copied().collect()
    }

    /// The portable logical values currently pressed.
    #[must_use]
    pub fn logical_keys_pressed(&self) -> HashSet<Key> {
        self.pressed.values().cloned().collect()
    }

    /// Toggles observed in this event stream, independent of whether the
    /// corresponding lock key is still down.
    #[must_use]
    pub const fn lock_modes_enabled(&self) -> Modifiers {
        self.locks
    }

    #[must_use]
    pub fn is_physical_key_pressed(&self, code: Code) -> bool {
        self.pressed.contains_key(&code)
    }

    #[must_use]
    pub fn is_logical_key_pressed(&self, key: &Key) -> bool {
        self.pressed.values().any(|pressed| pressed == key)
    }
}

type Handler = Rc<dyn Fn(&KeyboardEvent) -> bool>;

#[derive(Default)]
struct HandlerRegistry {
    sequence: u64,
    handlers: Rc<Vec<(u64, Handler)>>,
}

/// Ordered hardware key handlers. Every registration is invoked even
/// after a handler accepts the event. Registration changes during one
/// dispatch take effect on the next event.
#[derive(Clone, Default)]
pub struct HardwareKeyHandlers {
    registry: Rc<RefCell<HandlerRegistry>>,
}

/// The combined handling result and original panic payloads. In
/// panic-abort builds a panicking handler cannot be recovered.
#[derive(Default)]
pub struct HardwareKeyDispatch {
    pub handled: bool,
    pub failures: Vec<Box<dyn Any + Send>>,
}

impl HardwareKeyHandlers {
    /// Adds one registration. Registering an equivalent handler twice
    /// creates two independently owned subscriptions.
    pub fn add(
        &self,
        handler: impl Fn(&KeyboardEvent) -> bool + 'static,
    ) -> HardwareKeySubscription {
        let mut registry = self.registry.borrow_mut();
        let sequence = registry.sequence;
        registry.sequence = sequence
            .checked_add(1)
            .expect("hardware key handler IDs exhausted");
        Rc::make_mut(&mut registry.handlers).push((sequence, Rc::new(handler)));
        HardwareKeySubscription {
            sequence,
            registry: Rc::downgrade(&self.registry),
        }
    }

    /// Dispatches over a snapshot without holding a borrow during callbacks.
    #[must_use]
    pub fn dispatch(&self, event: &KeyboardEvent) -> HardwareKeyDispatch {
        let snapshot = self.registry.borrow().handlers.clone();
        let mut report = HardwareKeyDispatch::default();
        for (_, handler) in snapshot.iter() {
            match catch_unwind(AssertUnwindSafe(|| handler(event))) {
                Ok(handled) => report.handled |= handled,
                Err(payload) => report.failures.push(payload),
            }
        }
        report
    }
}

/// Removes one hardware-key handler registration on drop.
#[must_use = "keep the subscription to receive hardware key events"]
pub struct HardwareKeySubscription {
    sequence: u64,
    registry: Weak<RefCell<HandlerRegistry>>,
}

impl Drop for HardwareKeySubscription {
    fn drop(&mut self) {
        if let Some(registry) = self.registry.upgrade() {
            let removed = {
                let mut registry = registry.borrow_mut();
                let handlers = Rc::make_mut(&mut registry.handlers);
                handlers
                    .iter()
                    .position(|(sequence, _)| *sequence == self.sequence)
                    .map(|index| handlers.remove(index))
            };
            drop(removed);
        }
    }
}
