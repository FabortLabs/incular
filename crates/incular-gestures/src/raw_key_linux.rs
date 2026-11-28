//! Legacy GTK and GLFW packet decoding; independent of the native window backend.

use incular_core::{KeyState, KeyboardEvent, KeyboardKey, LogicalKeyboardKey, Modifiers};

use super::{
    RawKeyPlatformData, RawKeyboardModifier, RawKeyboardSide, lookup_numeric as lookup,
    portable_hid, portable_named,
};
use crate::RawKeyEvent;

#[path = "raw_key_linux_catalog.rs"]
mod catalog;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinuxKeyToolkit {
    #[default]
    Gtk,
    Glfw,
}

/// Numeric fields received from an explicitly supplied Flutter Linux key packet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinuxRawKeyData {
    pub toolkit: LinuxKeyToolkit,
    pub unicode_scalar_values: u32,
    pub scan_code: u32,
    pub key_code: u32,
    /// Modifier mask before the packet's own key transition.
    pub modifiers: u32,
    pub is_down: bool,
    pub specified_logical_key: Option<u64>,
}

impl LinuxRawKeyData {
    #[must_use]
    pub fn key_label(self) -> Option<char> {
        char::from_u32(self.unicode_scalar_values).filter(|&ch| ch != '\0')
    }

    #[must_use]
    pub fn flutter_physical_key_id(self) -> u64 {
        lookup(catalog::PHYSICAL, self.scan_code).map_or(
            LogicalKeyboardKey::WEB_PLANE + u64::from(self.scan_code),
            u64::from,
        )
    }

    #[must_use]
    pub fn flutter_logical_key(self) -> LogicalKeyboardKey {
        let (numpad, plane) = match self.toolkit {
            LinuxKeyToolkit::Gtk => (catalog::GTK_NUM_PAD, LogicalKeyboardKey::GTK_PLANE),
            LinuxKeyToolkit::Glfw => (catalog::GLFW_NUM_PAD, LogicalKeyboardKey::GLFW_PLANE),
        };
        let id = self
            .specified_logical_key
            .or_else(|| lookup(numpad, self.key_code))
            .or_else(|| {
                self.key_label()
                    .filter(|ch| !ch.is_control())
                    .map(|ch| u64::from(u32::from(ch)))
            })
            .or_else(|| self.known_key(self.key_code).map(|(id, _)| id))
            .unwrap_or(plane | u64::from(self.key_code));
        LogicalKeyboardKey::new(id)
    }

    /// Neither toolkit's mask can distinguish the two sides, even if its
    /// physical code identifies the side of the current event.
    #[must_use]
    pub const fn modifier_side(self, _modifier: RawKeyboardModifier) -> RawKeyboardSide {
        RawKeyboardSide::All
    }

    #[must_use]
    pub fn is_modifier_pressed(
        self,
        modifier: RawKeyboardModifier,
        _side: RawKeyboardSide,
    ) -> bool {
        use RawKeyboardModifier as M;
        let (flag, own_key) = match (self.toolkit, modifier) {
            (LinuxKeyToolkit::Gtk, M::Shift) => (1 << 0, matches!(self.key_code, 0xffe1 | 0xffe2)),
            (LinuxKeyToolkit::Gtk, M::CapsLock) => {
                (1 << 1, matches!(self.key_code, 0xffe5 | 0xffe6))
            }
            (LinuxKeyToolkit::Gtk, M::Control) => {
                (1 << 2, matches!(self.key_code, 0xffe3 | 0xffe4))
            }
            (LinuxKeyToolkit::Gtk, M::Alt) => (1 << 3, matches!(self.key_code, 0xffe9 | 0xffea)),
            (LinuxKeyToolkit::Gtk, M::NumLock) => (1 << 4, self.key_code == 0xff7f),
            (LinuxKeyToolkit::Gtk, M::Meta) => (1 << 26, matches!(self.key_code, 0xffeb | 0xffec)),
            (LinuxKeyToolkit::Glfw, M::Shift) => (1 << 0, matches!(self.key_code, 340 | 344)),
            (LinuxKeyToolkit::Glfw, M::Control) => (1 << 1, matches!(self.key_code, 341 | 345)),
            (LinuxKeyToolkit::Glfw, M::Alt) => (1 << 2, matches!(self.key_code, 342 | 346)),
            (LinuxKeyToolkit::Glfw, M::Meta) => (1 << 3, matches!(self.key_code, 343 | 347)),
            (LinuxKeyToolkit::Glfw, M::CapsLock) => (1 << 4, self.key_code == 280),
            (LinuxKeyToolkit::Glfw, M::NumLock) => (1 << 5, self.key_code == 282),
            (_, M::ScrollLock | M::Function | M::Symbol) => return false,
        };
        if own_key {
            self.is_down
        } else {
            self.modifiers & flag != 0
        }
    }

    #[must_use]
    pub fn into_event(self, repeat: bool) -> RawKeyEvent {
        let (code, location) = portable_hid(self.flutter_physical_key_id());
        let id = self.flutter_logical_key().key_id();
        let key = u32::try_from(id)
            .ok()
            .and_then(char::from_u32)
            .filter(|ch| !ch.is_control())
            .map(|ch| KeyboardKey::Character(ch.to_string()))
            .or_else(|| {
                let (known, name) = self.known_key(self.key_code)?;
                (known == id).then(|| portable_named(name)).flatten()
            })
            .or_else(|| {
                self.specified_logical_key?;
                // A layout override can replace the toolkit's known key.
                catalog::GTK_LOGICAL
                    .iter()
                    .chain(catalog::GLFW_LOGICAL)
                    .find_map(|(_, (known, name))| {
                        (*known == id).then(|| portable_named(name)).flatten()
                    })
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
                state: if self.is_down {
                    KeyState::Down
                } else {
                    KeyState::Up
                },
                modifiers,
                repeat,
                is_composing: false,
            },
            RawKeyPlatformData::Linux(self),
        )
    }

    fn known_key(self, code: u32) -> Option<(u64, &'static str)> {
        lookup(
            match self.toolkit {
                LinuxKeyToolkit::Gtk => catalog::GTK_LOGICAL,
                LinuxKeyToolkit::Glfw => catalog::GLFW_LOGICAL,
            },
            code,
        )
    }
}
