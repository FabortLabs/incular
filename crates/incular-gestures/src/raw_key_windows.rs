//! Numeric identities and flags in Flutter's legacy Win32 keyboard packet.

use incular_core::{KeyState, KeyboardEvent, KeyboardKey, LogicalKeyboardKey, Modifiers};

use super::{
    RawKeyPlatformData, RawKeyboardModifier, RawKeyboardSide, lookup_numeric as lookup,
    portable_hid, portable_named,
};
use crate::RawKeyEvent;

#[path = "raw_key_windows_catalog.rs"]
mod catalog;

/// Source fields in a legacy Win32 packet. The mask is the Flutter wire
/// mask, not a bitwise combination of Windows virtual-key codes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowsRawKeyData {
    pub key_code: u32,
    pub scan_code: u32,
    pub character_code_point: u32,
    pub modifiers: u32,
}

impl WindowsRawKeyData {
    pub const SHIFT: u32 = 1 << 0;
    pub const LEFT_SHIFT: u32 = 1 << 1;
    pub const RIGHT_SHIFT: u32 = 1 << 2;
    pub const CONTROL: u32 = 1 << 3;
    pub const LEFT_CONTROL: u32 = 1 << 4;
    pub const RIGHT_CONTROL: u32 = 1 << 5;
    pub const ALT: u32 = 1 << 6;
    pub const LEFT_ALT: u32 = 1 << 7;
    pub const RIGHT_ALT: u32 = 1 << 8;
    pub const LEFT_META: u32 = 1 << 9;
    pub const RIGHT_META: u32 = 1 << 10;
    pub const CAPS_LOCK: u32 = 1 << 11;
    pub const NUM_LOCK: u32 = 1 << 12;
    pub const SCROLL_LOCK: u32 = 1 << 13;
    pub const VK_PROCESSKEY: u32 = 0xe5;

    /// Whether an ordinary keydown should dispatch rather than letting IME
    /// composition deliver the resulting text.
    #[must_use]
    pub const fn should_dispatch_key_down(self) -> bool {
        self.key_code != Self::VK_PROCESSKEY
    }

    #[must_use]
    pub fn key_label(self) -> Option<char> {
        char::from_u32(self.character_code_point).filter(|&ch| ch != '\0')
    }

    #[must_use]
    pub fn flutter_physical_key_id(self) -> u64 {
        lookup(catalog::PHYSICAL, self.scan_code).map_or(
            LogicalKeyboardKey::WINDOWS_PLANE + u64::from(self.scan_code),
            u64::from,
        )
    }

    #[must_use]
    pub fn flutter_logical_key(self) -> LogicalKeyboardKey {
        let id = lookup(catalog::NUM_PAD, self.key_code)
            .or_else(|| self.printable_label().map(|ch| u64::from(u32::from(ch))))
            .or_else(|| lookup(catalog::LOGICAL, self.key_code).map(|(id, _)| id))
            .unwrap_or(LogicalKeyboardKey::WINDOWS_PLANE | u64::from(self.key_code));
        LogicalKeyboardKey::new(id)
    }

    /// Reconciliation treats a generic Shift, Control, or Alt with no
    /// sided bit set as both sides. The Windows packet has no generic Meta.
    #[must_use]
    pub fn pressed_modifier_side(self, modifier: RawKeyboardModifier) -> Option<RawKeyboardSide> {
        use RawKeyboardModifier as M;
        use RawKeyboardSide as S;
        let (generic, left, right) = match modifier {
            M::Shift => (Self::SHIFT, Self::LEFT_SHIFT, Self::RIGHT_SHIFT),
            M::Control => (Self::CONTROL, Self::LEFT_CONTROL, Self::RIGHT_CONTROL),
            M::Alt => (Self::ALT, Self::LEFT_ALT, Self::RIGHT_ALT),
            M::Meta => (0, Self::LEFT_META, Self::RIGHT_META),
            M::CapsLock => return (self.modifiers & Self::CAPS_LOCK != 0).then_some(S::All),
            M::NumLock => return (self.modifiers & Self::NUM_LOCK != 0).then_some(S::All),
            M::ScrollLock => return (self.modifiers & Self::SCROLL_LOCK != 0).then_some(S::All),
            M::Function | M::Symbol => return None,
        };
        match (self.modifiers & left != 0, self.modifiers & right != 0) {
            (true, false) => Some(S::Left),
            (false, true) => Some(S::Right),
            (true, true) => Some(S::All),
            (false, false) => (self.modifiers & generic != 0).then_some(S::All),
        }
    }

    #[must_use]
    pub fn is_modifier_pressed(self, modifier: RawKeyboardModifier, side: RawKeyboardSide) -> bool {
        use RawKeyboardSide as S;
        match self.pressed_modifier_side(modifier) {
            Some(S::All | S::Any) => true,
            Some(pressed) => side == S::Any || pressed == side,
            None => false,
        }
    }

    /// Decode an explicitly supplied legacy wire packet. Native Windows input
    /// remains on the ordinary portable keyboard path.
    #[must_use]
    pub fn into_event(self, state: KeyState, repeat: bool) -> RawKeyEvent {
        let (code, location) = portable_hid(self.flutter_physical_key_id());
        let id = self.flutter_logical_key();
        let key = char::from_u32(u32::try_from(id.key_id()).unwrap_or(u32::MAX))
            .or_else(|| self.printable_label())
            .map(|ch| KeyboardKey::Character(ch.to_string()))
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
        ] {
            modifiers.set(
                portable,
                self.is_modifier_pressed(raw, RawKeyboardSide::Any),
            );
        }
        RawKeyEvent::with_platform_data(
            KeyboardEvent {
                key,
                code,
                location,
                state,
                modifiers,
                repeat,
                is_composing: !self.should_dispatch_key_down(),
            },
            RawKeyPlatformData::Windows(self),
        )
    }

    fn printable_label(self) -> Option<char> {
        self.key_label().filter(|ch| !ch.is_control())
    }
}
