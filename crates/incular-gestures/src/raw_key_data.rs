//! Legacy framework key payloads received explicitly from the embedder.

use incular_core::{
    Code, KeyState, KeyboardEvent, KeyboardKey, Location, LogicalKeyboardKey, Modifiers,
};

use crate::RawKeyEvent;

#[path = "raw_key_fuchsia_catalog.rs"]
mod fuchsia;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RawKeyPlatformData {
    Fuchsia(FuchsiaRawKeyData),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawKeyboardModifier {
    Alt,
    Control,
    Shift,
    Meta,
    CapsLock,
    NumLock,
    ScrollLock,
    Function,
    Symbol,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawKeyboardSide {
    Any,
    All,
    Left,
    Right,
}

/// The three source fields of a legacy Fuchsia keyboard event. The raw
/// numeric identities are kept alongside the portable widget event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuchsiaRawKeyData {
    pub hid_usage: u32,
    pub code_point: u32,
    pub modifiers: u32,
}

impl FuchsiaRawKeyData {
    pub const CAPS_LOCK: u32 = 0x001;
    pub const LEFT_SHIFT: u32 = 0x002;
    pub const RIGHT_SHIFT: u32 = 0x004;
    pub const LEFT_CONTROL: u32 = 0x008;
    pub const RIGHT_CONTROL: u32 = 0x010;
    pub const LEFT_ALT: u32 = 0x020;
    pub const RIGHT_ALT: u32 = 0x040;
    pub const LEFT_META: u32 = 0x080;
    pub const RIGHT_META: u32 = 0x100;

    #[must_use]
    pub fn key_label(self) -> Option<char> {
        (self.code_point != 0)
            .then(|| char::from_u32(self.code_point))
            .flatten()
    }

    #[must_use]
    pub fn flutter_physical_key_id(self) -> u64 {
        self.physical().map_or(
            LogicalKeyboardKey::FUCHSIA_PLANE + u64::from(self.hid_usage),
            |(physical, _, _)| u64::from(physical),
        )
    }

    #[must_use]
    pub fn flutter_logical_key(self) -> LogicalKeyboardKey {
        LogicalKeyboardKey::new(self.logical().0)
    }

    #[must_use]
    pub fn is_modifier_pressed(self, modifier: RawKeyboardModifier, side: RawKeyboardSide) -> bool {
        use RawKeyboardModifier as M;
        use RawKeyboardSide as S;
        let (left, right) = match modifier {
            M::Shift => (Self::LEFT_SHIFT, Self::RIGHT_SHIFT),
            M::Control => (Self::LEFT_CONTROL, Self::RIGHT_CONTROL),
            M::Alt => (Self::LEFT_ALT, Self::RIGHT_ALT),
            M::Meta => (Self::LEFT_META, Self::RIGHT_META),
            M::CapsLock => return self.modifiers & Self::CAPS_LOCK != 0,
            M::NumLock | M::ScrollLock | M::Function | M::Symbol => return false,
        };
        let down = self.modifiers;
        match side {
            S::Any => down & (left | right) != 0,
            S::All => down & (left | right) == left | right,
            S::Left => down & left != 0,
            S::Right => down & right != 0,
        }
    }

    /// Decode one explicitly supplied platform message. This does not
    /// register a Fuchsia OS host or manufacture metadata for native keys.
    #[must_use]
    pub fn into_event(self, state: KeyState, repeat: bool) -> RawKeyEvent {
        let (_, code, location) =
            self.physical()
                .unwrap_or((0, Code::Unidentified, Location::Standard));
        let (_, named) = self.logical();
        let key = named
            .map(KeyboardKey::Named)
            .or_else(|| {
                self.key_label()
                    .map(|label| KeyboardKey::Character(label.to_string()))
            })
            .unwrap_or_default();
        let mut modifiers = Modifiers::empty();
        for (raw, modifier) in [
            (RawKeyboardModifier::Shift, Modifiers::SHIFT),
            (RawKeyboardModifier::Control, Modifiers::CONTROL),
            (RawKeyboardModifier::Alt, Modifiers::ALT),
            (RawKeyboardModifier::Meta, Modifiers::META),
            (RawKeyboardModifier::CapsLock, Modifiers::CAPS_LOCK),
        ] {
            modifiers.set(
                modifier,
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
                is_composing: false,
            },
            RawKeyPlatformData::Fuchsia(self),
        )
    }

    fn physical(self) -> Option<(u32, Code, Location)> {
        let index = fuchsia::PHYSICAL
            .binary_search_by_key(&self.hid_usage, |row| row.0)
            .ok()?;
        let (_, platform, portable, location) = fuchsia::PHYSICAL[index];
        Some((platform, portable, location))
    }

    fn logical(self) -> (u64, Option<incular_core::NamedKey>) {
        let combined = if self.code_point == 0 {
            LogicalKeyboardKey::FUCHSIA_PLANE | u64::from(self.hid_usage)
        } else {
            u64::from(self.code_point)
        };
        fuchsia::LOGICAL
            .binary_search_by_key(&combined, |row| row.0)
            .ok()
            .map_or((combined, None), |index| {
                let (_, id, portable) = fuchsia::LOGICAL[index];
                (id, portable)
            })
    }
}
