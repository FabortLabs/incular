//! Legacy AppKit and UIKit keyboard packets. These do not replace native input.

use incular_core::{KeyState, KeyboardEvent, KeyboardKey, LogicalKeyboardKey as L, Modifiers};

use super::{
    RawKeyPlatformData, RawKeyboardModifier as M, RawKeyboardSide as S, lookup_numeric as lookup,
    portable_hid, portable_named,
};
use crate::RawKeyEvent;

#[path = "raw_key_apple_catalog.rs"]
mod catalog;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppleKeyPlatform {
    MacOs { specified_logical_key: Option<u64> },
    Ios,
}

impl AppleKeyPlatform {
    pub const MAC_OS: Self = Self::MacOs {
        specified_logical_key: None,
    };

    #[must_use]
    pub const fn is_macos(self) -> bool {
        matches!(self, Self::MacOs { .. })
    }
}

#[derive(Clone, Debug)]
pub struct AppleRawKeyData {
    pub platform: AppleKeyPlatform,
    pub key_code: u32,
    /// The source's optional character field, before platform-specific
    /// treatment of empty strings.
    pub characters: Option<String>,
    pub characters_ignoring_modifiers: String,
    pub modifiers: u32,
}

impl PartialEq for AppleRawKeyData {
    fn eq(&self, other: &Self) -> bool {
        self.platform.is_macos() == other.platform.is_macos()
            && self.key_code == other.key_code
            && self.characters.as_deref().unwrap_or_default()
                == other.characters.as_deref().unwrap_or_default()
            && self.characters_ignoring_modifiers == other.characters_ignoring_modifiers
            && self.modifiers == other.modifiers
    }
}

impl Eq for AppleRawKeyData {}

impl AppleRawKeyData {
    pub const CAPS_LOCK: u32 = 0x10000;
    pub const SHIFT: u32 = 0x20000;
    pub const CONTROL: u32 = 0x40000;
    pub const OPTION: u32 = 0x80000;
    pub const COMMAND: u32 = 0x100000;
    pub const NUMERIC_PAD: u32 = 0x200000;
    pub const HELP: u32 = 0x400000;
    pub const FUNCTION: u32 = 0x800000;
    pub const LEFT_SHIFT: u32 = 0x02;
    pub const RIGHT_SHIFT: u32 = 0x04;
    pub const LEFT_CONTROL: u32 = 0x01;
    pub const RIGHT_CONTROL: u32 = 0x2000;
    pub const LEFT_OPTION: u32 = 0x20;
    pub const RIGHT_OPTION: u32 = 0x40;
    pub const LEFT_COMMAND: u32 = 0x08;
    pub const RIGHT_COMMAND: u32 = 0x10;

    #[must_use]
    pub fn key_label(&self) -> &str {
        &self.characters_ignoring_modifiers
    }

    #[must_use]
    pub fn character(&self) -> Option<&str> {
        self.characters
            .as_deref()
            .filter(|text| self.platform.is_macos() || !text.is_empty())
    }

    #[must_use]
    pub fn flutter_physical_key_id(&self) -> u64 {
        let (table, plane) = if self.platform.is_macos() {
            (catalog::MACOS_PHYSICAL, L::WINDOWS_PLANE)
        } else {
            (catalog::IOS_PHYSICAL, L::IOS_PLANE)
        };
        lookup(table, self.key_code).map_or(plane + u64::from(self.key_code), u64::from)
    }

    #[must_use]
    pub fn flutter_logical_key(&self) -> L {
        L::new(self.logical_identity().0)
    }

    #[must_use]
    pub fn should_dispatch(&self) -> bool {
        !self.platform.is_macos() || self.flutter_logical_key() != L::FN
    }

    /// The side field is independent of the aggregate active bit.
    #[must_use]
    pub fn modifier_side(&self, modifier: M) -> Option<S> {
        let Some((aggregate, left, right)) = modifier_flags(modifier) else {
            return Some(S::All);
        };
        match (self.modifiers & left != 0, self.modifiers & right != 0) {
            (true, false) => Some(S::Left),
            (false, true) => Some(S::Right),
            (true, true) => Some(S::All),
            (false, false) => (self.modifiers & aggregate != 0).then_some(S::All),
        }
    }

    #[must_use]
    pub fn is_modifier_pressed(&self, modifier: M, side: S) -> bool {
        let Some((aggregate, _, _)) = modifier_flags(modifier) else {
            return modifier == M::CapsLock && self.modifiers & Self::CAPS_LOCK != 0;
        };
        if self.modifiers & aggregate == 0 {
            return false;
        }
        match self.modifier_side(modifier) {
            Some(S::All) => true,
            Some(pressed) => side == S::Any || side == pressed,
            None => false,
        }
    }

    #[must_use]
    pub fn into_event(self, state: KeyState, repeat: bool) -> RawKeyEvent {
        let (code, location) = portable_hid(self.flutter_physical_key_id());
        let (id, name) = self.logical_identity();
        let key = u32::try_from(id)
            .ok()
            .and_then(char::from_u32)
            .map(|ch| KeyboardKey::Character(ch.to_string()))
            .or_else(|| name.and_then(portable_named))
            .or_else(|| {
                if id <= L::VALUE_MASK {
                    Some(KeyboardKey::Character(self.key_label().to_owned()))
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let mut modifiers = Modifiers::empty();
        for (raw, portable) in [
            (M::Shift, Modifiers::SHIFT),
            (M::Control, Modifiers::CONTROL),
            (M::Alt, Modifiers::ALT),
            (M::Meta, Modifiers::META),
            (M::CapsLock, Modifiers::CAPS_LOCK),
        ] {
            modifiers.set(portable, self.is_modifier_pressed(raw, S::Any));
        }
        RawKeyEvent::with_platform_data(
            KeyboardEvent {
                key,
                code,
                location,
                state,
                modifiers,
                repeat,
                is_composing: false,
            },
            RawKeyPlatformData::Apple(self),
        )
    }

    fn logical_identity(&self) -> (u64, Option<&'static str>) {
        let (numpad, known, plane) = if self.platform.is_macos() {
            (
                catalog::MACOS_NUM_PAD,
                catalog::MACOS_LOGICAL,
                L::MACOS_PLANE,
            )
        } else {
            (catalog::IOS_NUM_PAD, catalog::IOS_LOGICAL, L::IOS_PLANE)
        };
        let by_id = |id| {
            known
                .iter()
                .find_map(|(_, (value, name))| (*value == id).then_some(*name))
        };
        if let AppleKeyPlatform::MacOs {
            specified_logical_key: Some(id),
        } = self.platform
        {
            return (id, by_id(id));
        }
        if let Some(id) = lookup(numpad, self.key_code) {
            return (id, by_id(id));
        }
        if !self.platform.is_macos()
            && let Ok(index) =
                catalog::IOS_SPECIAL.binary_search_by_key(&self.key_label(), |entry| entry.0)
        {
            let (id, name) = catalog::IOS_SPECIAL[index].1;
            return (id, Some(name));
        }
        if let Some((id, name)) = lookup(known, self.key_code) {
            return (id, Some(name));
        }
        let label = self.key_label();
        let mut utf16 = label.encode_utf16();
        if let Some(first) = utf16.next() {
            let second = utf16.next();
            if second.is_some()
                || !(L::is_control_character(label) || (0xf700..=0xf8ff).contains(&first))
            {
                if self.platform.is_macos() {
                    let mut chars = label.chars();
                    if let (Some(ch), None) = (chars.next(), chars.next()) {
                        let original = u32::from(ch);
                        let ordinal = if original <= 0xd7ff {
                            ch.to_lowercase().next().map_or(original, u32::from)
                        } else {
                            original
                        };
                        return (u64::from(ordinal), None);
                    }
                } else if utf16.next().is_none() {
                    let value = second.map_or(u32::from(first), |unit| {
                        (u32::from(first) << 16) | u32::from(unit)
                    });
                    return (u64::from(value), None);
                }
            }
        }
        (plane | u64::from(self.key_code), None)
    }
}

fn modifier_flags(modifier: M) -> Option<(u32, u32, u32)> {
    use AppleRawKeyData as A;
    match modifier {
        M::Shift => Some((A::SHIFT, A::LEFT_SHIFT, A::RIGHT_SHIFT)),
        M::Control => Some((A::CONTROL, A::LEFT_CONTROL, A::RIGHT_CONTROL)),
        M::Alt => Some((A::OPTION, A::LEFT_OPTION, A::RIGHT_OPTION)),
        M::Meta => Some((A::COMMAND, A::LEFT_COMMAND, A::RIGHT_COMMAND)),
        _ => None,
    }
}
