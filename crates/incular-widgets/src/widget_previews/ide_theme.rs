//! Theme inputs supplied by the preview host, independent of a design system.

use std::collections::BTreeMap;

use incular_config::Brightness;
use incular_core::Color;

/// Host color overrides. With no host mode, the preview defaults to dark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IdeTheme {
    pub background_color: Option<Color>,
    pub foreground_color: Option<Color>,
    pub mode: Option<Brightness>,
}

impl IdeTheme {
    #[must_use]
    pub fn brightness(self) -> Brightness {
        self.mode.unwrap_or(Brightness::Dark)
    }

    #[must_use]
    pub fn ide_specified_theme(self) -> bool {
        self.mode.is_some()
    }

    /// A supplied background is used only when its luminance is suitable
    /// for the selected mode. Alpha is preserved; it is not part of the
    /// perceived luminance comparison. Other styling belongs to the host.
    #[must_use]
    pub fn background_for(self, mode: Brightness, fallback_surface: Color) -> Color {
        const THRESHOLD: f64 = 0.12;
        self.background_color
            .filter(|&color| match mode {
                Brightness::Dark => luminance(color) <= THRESHOLD,
                Brightness::Light => luminance(color) >= 1. - THRESHOLD,
            })
            .unwrap_or(fallback_surface)
    }
}

/// The decoded query values remain available, but only the named color and
/// theme parameters affect the preview.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IdeThemeQueryParams(BTreeMap<String, Option<String>>);

impl IdeThemeQueryParams {
    #[must_use]
    pub const fn new(params: BTreeMap<String, Option<String>>) -> Self {
        Self(params)
    }

    #[must_use]
    pub fn params(&self) -> &BTreeMap<String, Option<String>> {
        &self.0
    }

    #[must_use]
    pub fn background_color(&self) -> Option<Color> {
        self.color("backgroundColor")
    }

    #[must_use]
    pub fn foreground_color(&self) -> Option<Color> {
        self.color("foregroundColor")
    }

    /// Only the exact value `light` opts out of the dark default.
    #[must_use]
    pub fn dark_mode(&self) -> bool {
        self.0.get("theme").and_then(Option::as_deref) != Some("light")
    }

    #[must_use]
    pub fn to_theme(&self) -> IdeTheme {
        IdeTheme {
            background_color: self.background_color(),
            foreground_color: self.foreground_color(),
            mode: Some(if self.dark_mode() {
                Brightness::Dark
            } else {
                Brightness::Light
            }),
        }
    }

    fn color(&self, key: &str) -> Option<Color> {
        self.0
            .get(key)
            .and_then(Option::as_deref)
            .and_then(parse_css_hex_color)
    }
}

fn parse_css_hex_color(value: &str) -> Option<Color> {
    let mut hex = value.replace('#', "").replace("%23", "");
    if !hex.is_ascii() {
        return None;
    }
    if matches!(hex.len(), 3 | 4) {
        hex = hex.chars().flat_map(|c| [c, c]).collect();
    }
    if hex.len() == 6 {
        hex.push_str("ff");
    }
    if hex.len() == 8 {
        // CSS places alpha last; the UI color constructor places it first.
        let packed = u32::from_str_radix(&hex, 16).ok()?;
        return Some(Color::from_argb32(packed.rotate_right(8)));
    }
    // The reference also accepts shorter packed hexadecimal values.
    let raw = hex.trim();
    let packed = if raw.starts_with('-') {
        i64::from_str_radix(raw, 16).ok()? as u32
    } else {
        u64::from_str_radix(raw, 16).ok()? as u32
    };
    Some(Color::from_argb32(packed))
}

fn luminance(color: Color) -> f64 {
    fn linear(component: u8) -> f64 {
        let component = f64::from(component) / 255.;
        if component <= 0.03928 {
            component / 12.92
        } else {
            ((component + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linear(color.red) + 0.7152 * linear(color.green) + 0.0722 * linear(color.blue)
}
