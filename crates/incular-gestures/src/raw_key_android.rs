//! Legacy Android wire fields and physical/logical key identities.

use incular_core::{
    KeyState, KeyboardEvent, KeyboardKey, LogicalKeyboardKey, Modifiers, PhysicalKeyboardKey,
};

use super::{
    RawKeyboardModifier, RawKeyboardSide, lookup_numeric as lookup, portable_hid, portable_named,
};
use crate::{RawKeyEvent, RawKeyPlatformData};

#[path = "raw_key_android_catalog.rs"]
mod catalog;

/// A legacy Android key packet. Device identity and repeat count are retained
/// but are not part of Flutter's six-field value equality.
#[derive(Clone, Copy, Debug, Default)]
pub struct AndroidRawKeyData {
    pub flags: u32,
    pub code_point: u32,
    pub plain_code_point: u32,
    pub key_code: u32,
    pub scan_code: u32,
    pub meta_state: u32,
    pub event_source: u32,
    pub vendor_id: u32,
    pub product_id: u32,
    pub device_id: i32,
    pub repeat_count: u32,
}

impl PartialEq for AndroidRawKeyData {
    fn eq(&self, other: &Self) -> bool {
        self.flags == other.flags
            && self.code_point == other.code_point
            && self.plain_code_point == other.plain_code_point
            && self.key_code == other.key_code
            && self.scan_code == other.scan_code
            && self.meta_state == other.meta_state
    }
}

impl Eq for AndroidRawKeyData {}

enum ModifierMask {
    Sided(u32, u32, u32),
    Single(u32),
}

impl AndroidRawKeyData {
    pub const SHIFT: u32 = 0x01;
    pub const LEFT_SHIFT: u32 = 0x40;
    pub const RIGHT_SHIFT: u32 = 0x80;
    pub const ALT: u32 = 0x02;
    pub const LEFT_ALT: u32 = 0x10;
    pub const RIGHT_ALT: u32 = 0x20;
    pub const CONTROL: u32 = 0x1000;
    pub const LEFT_CONTROL: u32 = 0x2000;
    pub const RIGHT_CONTROL: u32 = 0x4000;
    pub const META: u32 = 0x10000;
    pub const LEFT_META: u32 = 0x20000;
    pub const RIGHT_META: u32 = 0x40000;
    pub const CAPS_LOCK: u32 = 0x100000;
    pub const NUM_LOCK: u32 = 0x200000;
    pub const SCROLL_LOCK: u32 = 0x400000;
    pub const FUNCTION: u32 = 0x08;
    pub const SYMBOL: u32 = 0x04;
    const COMBINING_CHARACTER_MASK: u32 = 0x7fff_ffff;
    pub const SOURCE_JOYSTICK: u32 = 0x0100_0010;
    pub const FLAG_SOFT_KEYBOARD: u32 = 0x02;
    pub const VIRTUAL_KEYBOARD_DEVICE_ID: i32 = -1;

    #[must_use]
    pub const fn is_virtual_keyboard(self) -> bool {
        self.flags & Self::FLAG_SOFT_KEYBOARD != 0
            || self.device_id == Self::VIRTUAL_KEYBOARD_DEVICE_ID
    }

    /// The unmodified label, including a combining mark without the Android
    /// high-bit marker. The event character is supplied separately.
    #[must_use]
    pub fn key_label(self) -> Option<char> {
        (self.plain_code_point != 0)
            .then(|| char::from_u32(self.plain_code_point & Self::COMBINING_CHARACTER_MASK))
            .flatten()
    }

    #[must_use]
    pub fn flutter_physical_key_id(self) -> u64 {
        if let Some(physical) = lookup(catalog::PHYSICAL, self.scan_code) {
            return u64::from(physical);
        }
        if self.event_source & Self::SOURCE_JOYSTICK == Self::SOURCE_JOYSTICK {
            let from_dpad = match self.flutter_logical_key() {
                LogicalKeyboardKey::ARROW_UP => Some(PhysicalKeyboardKey::ARROW_UP),
                LogicalKeyboardKey::ARROW_DOWN => Some(PhysicalKeyboardKey::ARROW_DOWN),
                LogicalKeyboardKey::ARROW_LEFT => Some(PhysicalKeyboardKey::ARROW_LEFT),
                LogicalKeyboardKey::ARROW_RIGHT => Some(PhysicalKeyboardKey::ARROW_RIGHT),
                _ => None,
            };
            if let Some(physical) = from_dpad {
                return u64::from(physical.usb_hid_usage());
            }
        }
        LogicalKeyboardKey::ANDROID_PLANE + u64::from(self.scan_code)
    }

    #[must_use]
    pub fn flutter_logical_key(self) -> LogicalKeyboardKey {
        let id = lookup(catalog::NUM_PAD, self.key_code)
            .or_else(|| {
                self.key_label()
                    .filter(|ch| !ch.is_control())
                    .map(|ch| u64::from(u32::from(ch)))
            })
            .or_else(|| lookup(catalog::LOGICAL, self.key_code).map(|(id, _)| id))
            .unwrap_or(LogicalKeyboardKey::ANDROID_PLANE | u64::from(self.key_code));
        LogicalKeyboardKey::new(id)
    }

    fn family(modifier: RawKeyboardModifier) -> ModifierMask {
        use ModifierMask::{Sided, Single};
        use RawKeyboardModifier as M;
        match modifier {
            M::Shift => Sided(Self::SHIFT, Self::LEFT_SHIFT, Self::RIGHT_SHIFT),
            M::Alt => Sided(Self::ALT, Self::LEFT_ALT, Self::RIGHT_ALT),
            M::Control => Sided(Self::CONTROL, Self::LEFT_CONTROL, Self::RIGHT_CONTROL),
            M::Meta => Sided(Self::META, Self::LEFT_META, Self::RIGHT_META),
            M::CapsLock => Single(Self::CAPS_LOCK),
            M::NumLock => Single(Self::NUM_LOCK),
            M::ScrollLock => Single(Self::SCROLL_LOCK),
            M::Function => Single(Self::FUNCTION),
            M::Symbol => Single(Self::SYMBOL),
        }
    }

    #[must_use]
    pub fn is_modifier_pressed(self, modifier: RawKeyboardModifier, side: RawKeyboardSide) -> bool {
        use RawKeyboardSide as S;
        match Self::family(modifier) {
            ModifierMask::Single(mask) => self.meta_state & mask != 0,
            ModifierMask::Sided(generic, left, right) => {
                self.meta_state & generic != 0
                    && match side {
                        S::Any => true,
                        S::All => self.meta_state & (left | right) == left | right,
                        S::Left => self.meta_state & left != 0,
                        S::Right => self.meta_state & right != 0,
                    }
            }
        }
    }

    /// Flutter asks Any first, then inspects the side. A generic-only flag
    /// has a side of All even though its explicit Left/Right bit tests fail.
    #[must_use]
    pub fn pressed_modifier_side(self, modifier: RawKeyboardModifier) -> Option<RawKeyboardSide> {
        use RawKeyboardSide as S;
        if !self.is_modifier_pressed(modifier, S::Any) {
            return None;
        }
        match Self::family(modifier) {
            ModifierMask::Sided(_, left, right) => {
                match (self.meta_state & left != 0, self.meta_state & right != 0) {
                    (true, false) => Some(S::Left),
                    (false, true) => Some(S::Right),
                    _ => Some(S::All),
                }
            }
            ModifierMask::Single(_) => Some(S::All),
        }
    }

    #[must_use]
    pub fn into_event(self, state: KeyState, character: Option<String>) -> RawKeyEvent {
        let (code, location) = portable_hid(self.flutter_physical_key_id());
        let unicode = u32::try_from(self.flutter_logical_key().key_id())
            .ok()
            .and_then(char::from_u32)
            .or_else(|| {
                lookup(catalog::NUM_PAD, self.key_code)?;
                self.key_label().filter(|ch| !ch.is_control())
            });
        let key = unicode
            .map(|ch| {
                let text = character
                    .as_deref()
                    .filter(|text| !text.is_empty())
                    .map_or_else(|| ch.to_string(), str::to_owned);
                KeyboardKey::Character(text)
            })
            .or_else(|| {
                let (_, name) = lookup(catalog::LOGICAL, self.key_code)?;
                portable_named(name)
            })
            .unwrap_or_default();
        let mut modifiers = Modifiers::empty();
        for (raw, portable) in [
            (RawKeyboardModifier::Shift, Modifiers::SHIFT),
            (RawKeyboardModifier::Control, Modifiers::CONTROL),
            (RawKeyboardModifier::Alt, Modifiers::ALT),
            (RawKeyboardModifier::Meta, Modifiers::META),
            (RawKeyboardModifier::CapsLock, Modifiers::CAPS_LOCK),
            (RawKeyboardModifier::NumLock, Modifiers::NUM_LOCK),
            (RawKeyboardModifier::ScrollLock, Modifiers::SCROLL_LOCK),
            (RawKeyboardModifier::Function, Modifiers::FN),
            (RawKeyboardModifier::Symbol, Modifiers::SYMBOL),
        ] {
            modifiers.set(
                portable,
                self.is_modifier_pressed(raw, RawKeyboardSide::Any),
            );
        }
        RawKeyEvent::with_android_platform_data(
            KeyboardEvent {
                key,
                code,
                location,
                state,
                modifiers,
                repeat: false,
                is_composing: false,
            },
            RawKeyPlatformData::Android(self),
            character,
        )
    }
}
