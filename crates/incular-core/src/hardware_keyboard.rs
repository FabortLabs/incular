//! Pressed key and lock state over the portable native event stream.

use std::collections::{HashMap, HashSet, hash_map::Entry};

use crate::input::{Code, Key, KeyState, KeyboardEvent, Modifiers, NamedKey};

/// The keys and toggled locks observed from a stream of hardware events.
/// Physical positions own presses so a release still clears its press if
/// the platform changed the logical label while it was held.
#[derive(Clone, Debug, Default)]
pub struct HardwareKeyboardState {
    pressed: HashMap<Code, Key>,
    locks: Modifiers,
}

impl HardwareKeyboardState {
    /// Updates the state before forwarding an event to application handlers.
    pub fn observe(&mut self, event: &KeyboardEvent) {
        match event.state {
            KeyState::Up => {
                self.pressed.remove(&event.code);
            }
            KeyState::Down => {
                if let Entry::Vacant(entry) = self.pressed.entry(event.code) {
                    if !event.repeat {
                        match &event.key {
                            Key::Named(NamedKey::CapsLock) => {
                                self.locks.toggle(Modifiers::CAPS_LOCK)
                            }
                            Key::Named(NamedKey::NumLock) => self.locks.toggle(Modifiers::NUM_LOCK),
                            Key::Named(NamedKey::ScrollLock) => {
                                self.locks.toggle(Modifiers::SCROLL_LOCK)
                            }
                            _ => {}
                        }
                    }
                    entry.insert(event.key.clone());
                }
            }
        }
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
