//! Legacy framework key payloads received explicitly from the embedder.

use incular_core::{
    Code, KeyState, KeyboardEvent, KeyboardKey, Location, LogicalKeyboardKey, Modifiers,
};

use crate::RawKeyEvent;

#[path = "raw_key_fuchsia_catalog.rs"]
mod fuchsia;
#[path = "raw_key_web_catalog.rs"]
mod web;
#[path = "raw_key_windows.rs"]
mod windows;
pub use windows::WindowsRawKeyData;
#[path = "raw_key_android.rs"]
mod android;
pub use android::AndroidRawKeyData;
#[path = "raw_key_linux.rs"]
mod linux;
pub use linux::{LinuxKeyToolkit, LinuxRawKeyData};

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RawKeyPlatformData {
    Android(AndroidRawKeyData),
    Fuchsia(FuchsiaRawKeyData),
    Linux(LinuxRawKeyData),
    Web(WebRawKeyData),
    Windows(WindowsRawKeyData),
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

/// The legacy browser payload. Left and right modifier physical keys have
/// their DOM `code`/location, but browser modifier flags are not sided.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WebRawKeyData {
    pub code: String,
    pub key: String,
    pub location: u32,
    pub meta_state: u32,
    pub key_code: u32,
}

impl WebRawKeyData {
    pub const SHIFT: u32 = 0x01;
    pub const ALT: u32 = 0x02;
    pub const CONTROL: u32 = 0x04;
    pub const META: u32 = 0x08;
    pub const NUM_LOCK: u32 = 0x10;
    pub const CAPS_LOCK: u32 = 0x20;
    pub const SCROLL_LOCK: u32 = 0x40;

    #[must_use]
    pub fn key_label(&self) -> Option<&str> {
        (self.key.encode_utf16().count() == 1).then_some(self.key.as_str())
    }

    #[must_use]
    pub fn is_modifier_pressed(
        &self,
        modifier: RawKeyboardModifier,
        _side: RawKeyboardSide,
    ) -> bool {
        use RawKeyboardModifier as M;
        let mask = match modifier {
            M::Shift => Self::SHIFT,
            M::Alt => Self::ALT,
            M::Control => Self::CONTROL,
            M::Meta => Self::META,
            M::NumLock => Self::NUM_LOCK,
            M::CapsLock => Self::CAPS_LOCK,
            M::ScrollLock => Self::SCROLL_LOCK,
            M::Function | M::Symbol => return false,
        };
        self.meta_state & mask != 0
    }

    /// Flutter's known USB identity, or a stable value in the Web plane.
    #[must_use]
    pub fn flutter_physical_key_id(&self) -> u64 {
        web::PHYSICAL
            .binary_search_by(|(code, _)| code.cmp(&self.code.as_str()))
            .ok()
            .map_or_else(
                || web_key_for_code(&self.code),
                |i| u64::from(web::PHYSICAL[i].1),
            )
    }

    /// Logical identity keeps sided modifiers, numpad keys and printable
    /// case normalization independent of the original DOM key label.
    #[must_use]
    pub fn flutter_logical_key(&self) -> LogicalKeyboardKey {
        let located = web::LOCATION
            .binary_search_by(|(key, _)| key.cmp(&self.key.as_str()))
            .ok()
            .and_then(|i| {
                web::LOCATION[i]
                    .1
                    .get(self.location as usize)
                    .copied()
                    .flatten()
            });
        let named = || {
            web::LOGICAL
                .binary_search_by(|(key, _)| key.cmp(&self.key.as_str()))
                .ok()
                .map(|i| web::LOGICAL[i].1)
        };
        let printable = || {
            self.key_label()
                .and_then(|label| label.chars().next())
                .and_then(|letter| letter.to_lowercase().next())
                .map(|letter| u64::from(u32::from(letter)))
        };
        LogicalKeyboardKey::new(
            located
                .or_else(named)
                .or_else(printable)
                .unwrap_or_else(|| web_key_for_code(&self.code)),
        )
    }

    /// Decode an explicitly supplied legacy Web message for the mounted
    /// keyboard router, preserving the original DOM strings and flags.
    #[must_use]
    pub fn into_event(self, state: KeyState, repeat: bool, is_composing: bool) -> RawKeyEvent {
        let code = if self.code == "Esc" {
            "Escape"
        } else {
            self.code.as_str()
        };
        let code = code.parse::<Code>().unwrap_or(Code::Unidentified);
        let key = match self.key.as_str() {
            "Esc" => "Escape",
            "Del" => "Delete",
            "Left" => "ArrowLeft",
            "Right" => "ArrowRight",
            "Up" => "ArrowUp",
            "Down" => "ArrowDown",
            "Spacebar" => " ",
            key => key,
        }
        .parse::<KeyboardKey>()
        .unwrap_or_default();
        let location = match self.location {
            1 => Location::Left,
            2 => Location::Right,
            3 => Location::Numpad,
            _ => Location::Standard,
        };
        let mut modifiers = Modifiers::empty();
        for (source, target) in [
            (Self::SHIFT, Modifiers::SHIFT),
            (Self::ALT, Modifiers::ALT),
            (Self::CONTROL, Modifiers::CONTROL),
            (Self::META, Modifiers::META),
            (Self::NUM_LOCK, Modifiers::NUM_LOCK),
            (Self::CAPS_LOCK, Modifiers::CAPS_LOCK),
            (Self::SCROLL_LOCK, Modifiers::SCROLL_LOCK),
        ] {
            modifiers.set(target, self.meta_state & source != 0);
        }
        RawKeyEvent::with_platform_data(
            KeyboardEvent {
                key,
                code,
                location,
                state,
                modifiers,
                repeat,
                is_composing,
            },
            RawKeyPlatformData::Web(self),
        )
    }
}

// Dart's String hash is runtime-specific. Mint a stable native identity
// from the physical DOM code, within the same platform plane. Using the
// textual key would alias all unknown keys reported as "Unidentified".
fn web_key_for_code(code: &str) -> u64 {
    let hash = code.bytes().fold(0x811c_9dc5u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    LogicalKeyboardKey::WEB_PLANE + 1 + u64::from(hash & 0x1fff_ffff)
}

fn lookup_numeric<T: Copy>(table: &[(u32, T)], value: u32) -> Option<T> {
    let index = table.binary_search_by_key(&value, |row| row.0).ok()?;
    Some(table[index].1)
}

fn portable_hid(hid: u64) -> (Code, Location) {
    let index = u32::try_from(hid).ok().and_then(|hid| {
        fuchsia::PHYSICAL
            .binary_search_by_key(&hid, |row| row.0)
            .ok()
    });
    index.map_or((Code::Unidentified, Location::Standard), |index| {
        let (_, _, code, location) = fuchsia::PHYSICAL[index];
        (code, location)
    })
}

fn portable_named(name: &str) -> Option<KeyboardKey> {
    let name = match name {
        "ShiftLeft" | "ShiftRight" => "Shift",
        "ControlLeft" | "ControlRight" => "Control",
        "AltLeft" | "AltRight" => "Alt",
        "MetaLeft" | "MetaRight" => "Meta",
        "NumpadEnter" => "Enter",
        name => name,
    };
    name.parse().ok().map(KeyboardKey::Named)
}
