//! Flutter's `SystemUiOverlayStyle` (`services/system_chrome.dart`): how the
//! status bar and the navigation bar draw their icons and backgrounds. Each
//! field is optional, so a style changes only the properties it names and
//! leaves the rest to the platform default.

use std::{
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
};

use incular_config::Brightness;
use incular_core::Color;

/// Flutter's `SystemUiOverlayStyle`. Set it imperatively on a window, or
/// annotate a region with it so the status bar follows the screen under it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SystemUiOverlayStyle {
    /// Background of the navigation bar. Android only.
    pub system_navigation_bar_color: Option<Color>,
    /// Divider drawn above the navigation bar. Android only.
    pub system_navigation_bar_divider_color: Option<Color>,
    /// Brightness of the navigation bar's icons. Android only.
    pub system_navigation_bar_icon_brightness: Option<Brightness>,
    /// Whether the system enforces contrast behind the navigation bar.
    /// Carried for parity; no host applies it yet.
    pub system_navigation_bar_contrast_enforced: Option<bool>,
    /// Background of the status bar.
    pub status_bar_color: Option<Color>,
    /// Brightness of the status bar's background, which picks iOS's
    /// status-bar text colour. Carried for parity; iOS does not apply it yet.
    pub status_bar_brightness: Option<Brightness>,
    /// Brightness of the status bar's icons. `Brightness::Dark` draws dark
    /// icons for light backgrounds.
    pub status_bar_icon_brightness: Option<Brightness>,
    /// Whether the system enforces contrast behind the status bar.
    /// Carried for parity; no host applies it yet.
    pub system_status_bar_contrast_enforced: Option<bool>,
}

impl SystemUiOverlayStyle {
    /// Light icons, for dark backgrounds (Flutter's `SystemUiOverlayStyle.light`).
    pub const LIGHT: Self = Self {
        system_navigation_bar_color: Some(Color::BLACK),
        system_navigation_bar_divider_color: None,
        system_navigation_bar_icon_brightness: Some(Brightness::Light),
        system_navigation_bar_contrast_enforced: None,
        status_bar_color: None,
        status_bar_brightness: Some(Brightness::Dark),
        status_bar_icon_brightness: Some(Brightness::Light),
        system_status_bar_contrast_enforced: None,
    };

    /// Dark icons, for light backgrounds (Flutter's `SystemUiOverlayStyle.dark`).
    pub const DARK: Self = Self {
        system_navigation_bar_color: Some(Color::BLACK),
        system_navigation_bar_divider_color: None,
        system_navigation_bar_icon_brightness: Some(Brightness::Light),
        system_navigation_bar_contrast_enforced: None,
        status_bar_color: None,
        status_bar_brightness: Some(Brightness::Light),
        status_bar_icon_brightness: Some(Brightness::Dark),
        system_status_bar_contrast_enforced: None,
    };

    /// Replaces named fields, keeping the current value for every
    /// `None` override, as Flutter's `copyWith` does.
    #[must_use]
    pub fn copy_with(self, overrides: Self) -> Self {
        Self {
            system_navigation_bar_color: overrides
                .system_navigation_bar_color
                .or(self.system_navigation_bar_color),
            system_navigation_bar_divider_color: overrides
                .system_navigation_bar_divider_color
                .or(self.system_navigation_bar_divider_color),
            system_navigation_bar_icon_brightness: overrides
                .system_navigation_bar_icon_brightness
                .or(self.system_navigation_bar_icon_brightness),
            system_navigation_bar_contrast_enforced: overrides
                .system_navigation_bar_contrast_enforced
                .or(self.system_navigation_bar_contrast_enforced),
            status_bar_color: overrides.status_bar_color.or(self.status_bar_color),
            status_bar_brightness: overrides
                .status_bar_brightness
                .or(self.status_bar_brightness),
            status_bar_icon_brightness: overrides
                .status_bar_icon_brightness
                .or(self.status_bar_icon_brightness),
            system_status_bar_contrast_enforced: overrides
                .system_status_bar_contrast_enforced
                .or(self.system_status_bar_contrast_enforced),
        }
    }

    /// Reports every property in Flutter diagnostics order, including
    /// unset fields. Colors are rendered as normalized sRGB components.
    #[must_use]
    pub fn diagnostic_properties(self) -> [(&'static str, String); 8] {
        [
            (
                "systemNavigationBarColor",
                diagnostic_color(self.system_navigation_bar_color),
            ),
            (
                "systemNavigationBarDividerColor",
                diagnostic_color(self.system_navigation_bar_divider_color),
            ),
            (
                "systemNavigationBarIconBrightness",
                diagnostic_brightness(self.system_navigation_bar_icon_brightness),
            ),
            (
                "systemNavigationBarContrastEnforced",
                diagnostic_bool(self.system_navigation_bar_contrast_enforced),
            ),
            ("statusBarColor", diagnostic_color(self.status_bar_color)),
            (
                "statusBarBrightness",
                diagnostic_brightness(self.status_bar_brightness),
            ),
            (
                "statusBarIconBrightness",
                diagnostic_brightness(self.status_bar_icon_brightness),
            ),
            (
                "systemStatusBarContrastEnforced",
                diagnostic_bool(self.system_status_bar_contrast_enforced),
            ),
        ]
    }

    /// Flutter's `RenderView._updateSystemChrome` merge: when `upper` (the
    /// annotation at the status bar) and `lower` (the one at the navigation
    /// bar) both exist, `upper` supplies the status-bar fields and `lower`
    /// the navigation-bar fields. Otherwise the one that exists supplies
    /// every field.
    #[must_use]
    pub fn merge_annotations(upper: Option<Self>, lower: Option<Self>) -> Option<Self> {
        match (upper, lower) {
            (None, None) => None,
            (Some(upper), Some(lower)) => Some(Self {
                system_navigation_bar_color: lower.system_navigation_bar_color,
                system_navigation_bar_divider_color: lower.system_navigation_bar_divider_color,
                system_navigation_bar_icon_brightness: lower.system_navigation_bar_icon_brightness,
                system_navigation_bar_contrast_enforced: lower
                    .system_navigation_bar_contrast_enforced,
                status_bar_color: upper.status_bar_color,
                status_bar_brightness: upper.status_bar_brightness,
                status_bar_icon_brightness: upper.status_bar_icon_brightness,
                system_status_bar_contrast_enforced: upper.system_status_bar_contrast_enforced,
            }),
            (Some(style), None) | (None, Some(style)) => Some(style),
        }
    }
}

fn diagnostic_color(color: Option<Color>) -> String {
    color.map_or_else(|| String::from("null"), |color| {
        let scale = |byte| f64::from(byte) / 255.0;
        format!(
            "Color(alpha: {:.4}, red: {:.4}, green: {:.4}, blue: {:.4}, colorSpace: ColorSpace.sRGB)",
            scale(color.alpha), scale(color.red), scale(color.green), scale(color.blue)
        )
    })
}

fn diagnostic_brightness(value: Option<Brightness>) -> String {
    String::from(match value {
        Some(Brightness::Dark) => "Brightness.dark",
        Some(Brightness::Light) => "Brightness.light",
        None => "null",
    })
}

fn diagnostic_bool(value: Option<bool>) -> String {
    String::from(match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    })
}

impl fmt::Display for SystemUiOverlayStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut hash = DefaultHasher::new();
        self.hash(&mut hash);
        write!(
            f,
            "SystemUiOverlayStyle#{:05x}(",
            hash.finish() & 0x000f_ffff
        )?;
        for (index, (name, value)) in self.diagnostic_properties().into_iter().enumerate() {
            if index != 0 {
                f.write_str(", ")?;
            }
            write!(f, "{name}: {value}")?;
        }
        f.write_str(")")
    }
}
