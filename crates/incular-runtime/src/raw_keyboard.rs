//! Passive window-level subscriptions for the legacy raw-key route.

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
};

use incular_core::{KeyboardEvent, LogicalKeyboardKey, PhysicalKeyboardKey};
use incular_widgets::{
    RawKeyDelivery, RawKeyEvent, RawKeyPlatformData, RawKeyboardListenerError, RawKeyboardModifier,
    RawKeyboardSide,
};

type Listener = Rc<dyn Fn(&RawKeyEvent)>;
type List = RefCell<Rc<Vec<Listener>>>;

#[derive(Default)]
pub(crate) struct RawKeyboardListeners(Option<Rc<List>>);

impl RawKeyboardListeners {
    pub(crate) fn add(
        &mut self,
        listener: impl Fn(&RawKeyEvent) + 'static,
    ) -> RawKeyboardSubscription {
        let listeners = self.0.get_or_insert_with(Rc::default);
        let listener: Listener = Rc::new(listener);
        Rc::make_mut(&mut listeners.borrow_mut()).push(listener.clone());
        RawKeyboardSubscription {
            listener,
            registry: Rc::downgrade(listeners),
        }
    }

    pub(crate) fn dispatch(
        &self,
        event: &KeyboardEvent,
        delivery: RawKeyDelivery<'_>,
        mut report: impl FnMut(RawKeyboardListenerError),
    ) {
        let Some(registry) = &self.0 else {
            return;
        };
        let listeners = registry.borrow().clone();
        if listeners.is_empty() {
            return;
        }
        let portable;
        let raw = match delivery {
            RawKeyDelivery::Absent => return,
            RawKeyDelivery::Supplied(raw) => raw,
            RawKeyDelivery::Mirror => {
                portable = RawKeyEvent::from(event.clone());
                &portable
            }
        };
        for listener in listeners.iter() {
            if let Some(error) = RawKeyboardListenerError::catch(raw, || listener(raw)) {
                report(error);
            }
        }
    }
}

/// Removes one raw-key registration on drop. Adding the same callback
/// again creates a separate registration that survives the first drop.
#[must_use = "keep the subscription to receive raw keyboard events"]
pub struct RawKeyboardSubscription {
    listener: Listener,
    registry: Weak<List>,
}

impl Drop for RawKeyboardSubscription {
    fn drop(&mut self) {
        if let Some(registry) = self.registry.upgrade() {
            Rc::make_mut(&mut registry.borrow_mut())
                .retain(|listener| !Rc::ptr_eq(listener, &self.listener));
        }
    }
}

/// Pressed state of explicitly supplied legacy platform packets. Kept
/// separate from normalized hardware events: inferred modifier keys are
/// visible here but are not dispatched as extra hardware events.
#[derive(Debug, Default)]
pub struct LegacyRawKeyboardState {
    pressed: BTreeMap<u64, LogicalKeyboardKey>,
    skipped_windows: BTreeSet<u64>,
}

impl LegacyRawKeyboardState {
    #[must_use]
    pub fn logical_keys_pressed(&self) -> BTreeSet<LogicalKeyboardKey> {
        self.pressed.values().copied().collect()
    }

    /// Physical IDs can include the platform fallback planes, not just USB.
    pub fn physical_keys_pressed(&self) -> impl ExactSizeIterator<Item = u64> + '_ {
        self.pressed.keys().copied()
    }

    #[must_use]
    pub fn look_up_layout(&self, physical_id: u64) -> Option<LogicalKeyboardKey> {
        self.pressed.get(&physical_id).copied()
    }

    pub fn clear(&mut self) {
        self.pressed.clear();
    }

    /// Windows sends IME-handled down events with VK_PROCESSKEY, but may
    /// send their release with an ordinary virtual key. Pair by scan code.
    pub(crate) fn prepare_windows(&mut self, raw: &mut RawKeyEvent) -> bool {
        let Some(RawKeyPlatformData::Windows(data)) = raw.data() else {
            return true;
        };
        let key = data.flutter_physical_key_id();
        if raw.is_key_down() {
            if !data.should_dispatch_key_down() {
                self.skipped_windows.insert(key);
                return false;
            }
            self.skipped_windows.remove(&key);
            raw.set_repeat(self.pressed.contains_key(&key));
            true
        } else {
            !self.skipped_windows.remove(&key)
        }
    }

    pub(crate) fn observe(&mut self, event: &RawKeyEvent) {
        use LogicalKeyboardKey as L;
        use PhysicalKeyboardKey as P;
        use RawKeyboardModifier as M;
        use RawKeyboardSide as S;
        let Some(data) = event.data() else { return };
        let (physical, logical) = match data {
            RawKeyPlatformData::Fuchsia(data) => {
                (data.flutter_physical_key_id(), data.flutter_logical_key())
            }
            RawKeyPlatformData::Web(data) => {
                (data.flutter_physical_key_id(), data.flutter_logical_key())
            }
            RawKeyPlatformData::Windows(data) => {
                (data.flutter_physical_key_id(), data.flutter_logical_key())
            }
            _ => return,
        };
        if event.is_key_down() {
            self.pressed.insert(physical, logical);
        } else {
            self.pressed.remove(&physical);
        }
        for (modifier, left, right) in [
            (
                M::Shift,
                (P::SHIFT_LEFT, L::SHIFT_LEFT),
                (P::SHIFT_RIGHT, L::SHIFT_RIGHT),
            ),
            (
                M::Alt,
                (P::ALT_LEFT, L::ALT_LEFT),
                (P::ALT_RIGHT, L::ALT_RIGHT),
            ),
            (
                M::Control,
                (P::CONTROL_LEFT, L::CONTROL_LEFT),
                (P::CONTROL_RIGHT, L::CONTROL_RIGHT),
            ),
            (
                M::Meta,
                (P::META_LEFT, L::META_LEFT),
                (P::META_RIGHT, L::META_RIGHT),
            ),
        ] {
            let left = (usb(left.0), left.1);
            let right = (usb(right.0), right.1);
            match data {
                RawKeyPlatformData::Fuchsia(data) => {
                    self.set_modifier(left, data.is_modifier_pressed(modifier, S::Left));
                    self.set_modifier(right, data.is_modifier_pressed(modifier, S::Right));
                }
                RawKeyPlatformData::Windows(data) => {
                    self.set_modifier(left, data.is_modifier_pressed(modifier, S::Left));
                    self.set_modifier(right, data.is_modifier_pressed(modifier, S::Right));
                }
                RawKeyPlatformData::Web(data) => {
                    if data.is_modifier_pressed(modifier, S::Any) {
                        if !self.pressed.contains_key(&left.0)
                            && !self.pressed.contains_key(&right.0)
                        {
                            self.pressed.insert(left.0, left.1);
                        }
                    } else {
                        self.set_modifier(left, false);
                        self.set_modifier(right, false);
                    }
                }
                _ => unreachable!("unsupported raw packets returned before modifier processing"),
            }
        }
        for (modifier, key, default) in [
            (M::CapsLock, P::CAPS_LOCK, L::CAPS_LOCK),
            (M::NumLock, P::NUM_LOCK, L::NUM_LOCK),
            (M::ScrollLock, P::SCROLL_LOCK, L::SCROLL_LOCK),
        ] {
            let key = usb(key);
            match data {
                RawKeyPlatformData::Fuchsia(data) => {
                    self.set_modifier((key, default), data.is_modifier_pressed(modifier, S::Any));
                }
                RawKeyPlatformData::Windows(data) => {
                    self.set_modifier((key, default), data.is_modifier_pressed(modifier, S::Any));
                }
                RawKeyPlatformData::Web(data) => {
                    let remapped_caps = modifier == M::CapsLock
                        && self
                            .pressed
                            .get(&key)
                            .is_some_and(|logical| *logical != default);
                    // A Web lock mask has no supported fallback key on Any.
                    if !remapped_caps && !data.is_modifier_pressed(modifier, S::Any) {
                        self.pressed.remove(&key);
                    }
                }
                _ => unreachable!("unsupported raw packets returned before modifier processing"),
            }
        }
        if !matches!(data, RawKeyPlatformData::Fuchsia(_)) {
            self.pressed.remove(&usb(P::FN));
        }
        if matches!(data, RawKeyPlatformData::Web(_)) {
            // Real AltRight may represent browser AltGraph even if the
            // packet does not flag the key as a modifier.
            if event.is_key_down() && physical == usb(P::ALT_RIGHT) {
                self.pressed.entry(physical).or_insert(logical);
            }
        }
    }

    fn set_modifier(&mut self, (physical, logical): (u64, LogicalKeyboardKey), down: bool) {
        if down {
            self.pressed.insert(physical, logical);
        } else {
            self.pressed.remove(&physical);
        }
    }
}

const fn usb(key: PhysicalKeyboardKey) -> u64 {
    key.usb_hid_usage() as u64
}
