//! Light and dark palettes for the DevTools workspace.
//!
//! The current theme is a thread-local read while the widget tree is built; the
//! root build depends on a `Signal<Theme>`, so switching rebuilds every colour.

use incular::prelude::Color;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub const fn toggled(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    /// Label of the header button that switches to the *other* theme.
    pub const fn switch_label(self) -> &'static str {
        match self {
            Self::Dark => "Light theme",
            Self::Light => "Dark theme",
        }
    }

    /// Parses `light` or `dark` (case-insensitive).
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    pub fn palette(self) -> &'static Palette {
        match self {
            Self::Dark => &DARK,
            Self::Light => &LIGHT,
        }
    }
}

/// Initial theme from `INCULAR_DEVTOOLS_THEME=light|dark`; dark otherwise.
pub fn initial_theme() -> Theme {
    std::env::var("INCULAR_DEVTOOLS_THEME")
        .ok()
        .and_then(|value| Theme::parse(&value))
        .unwrap_or_default()
}

#[derive(Debug, PartialEq, Eq)]
pub struct Palette {
    pub background: Color,
    pub surface: Color,
    pub surface_raised: Color,
    pub border: Color,
    pub control: Color,
    pub control_hover: Color,
    pub control_active: Color,
    pub control_focus: Color,
    pub primary: Color,
    pub danger: Color,
    pub warning: Color,
    pub violet: Color,
    pub text_primary: Color,
    pub text_muted: Color,
    /// Identifier / type-name accent used in trees.
    pub accent_text: Color,
    /// Label colour on a filled primary or danger button.
    pub on_accent: Color,
    pub primary_hover: Color,
    pub danger_hover: Color,
    /// Faint wash behind rows or regions that exceed the frame budget.
    pub danger_wash: Color,
    /// Translucent overlays for hover and pressed states on bare surfaces.
    pub hover_overlay: Color,
    pub pressed_overlay: Color,
    pub gpu_color: Color,
    pub phase_build: Color,
    pub phase_layout: Color,
    pub phase_paint: Color,
    pub phase_semantics: Color,
    pub phase_composite: Color,
    pub phase_other: Color,
}

const fn rgb(red: u8, green: u8, blue: u8) -> Color {
    Color::rgba(red, green, blue, 255)
}

pub static DARK: Palette = Palette {
    background: rgb(15, 17, 22),
    surface: rgb(20, 23, 30),
    surface_raised: rgb(25, 29, 37),
    border: rgb(43, 48, 59),
    control: rgb(32, 37, 47),
    control_hover: rgb(45, 53, 65),
    control_active: rgb(34, 57, 55),
    control_focus: rgb(48, 65, 69),
    primary: rgb(110, 222, 186),
    danger: rgb(244, 126, 139),
    warning: rgb(232, 186, 113),
    violet: rgb(168, 151, 236),
    text_primary: rgb(230, 233, 239),
    text_muted: rgb(143, 152, 169),
    accent_text: rgb(177, 194, 237),
    on_accent: rgb(15, 17, 22),
    primary_hover: rgb(142, 236, 206),
    danger_hover: rgb(250, 156, 166),
    danger_wash: Color::rgba(244, 126, 139, 14),
    hover_overlay: Color::rgba(255, 255, 255, 22),
    pressed_overlay: Color::rgba(255, 255, 255, 40),
    gpu_color: rgb(34, 211, 238),
    phase_build: rgb(96, 165, 250),
    phase_layout: rgb(52, 211, 153),
    phase_paint: rgb(168, 151, 236),
    phase_semantics: rgb(244, 114, 182),
    phase_composite: rgb(251, 146, 60),
    phase_other: rgb(98, 106, 124),
};

pub static LIGHT: Palette = Palette {
    background: rgb(246, 247, 249),
    surface: rgb(255, 255, 255),
    surface_raised: rgb(241, 243, 246),
    border: rgb(217, 221, 227),
    control: rgb(236, 239, 243),
    control_hover: rgb(224, 229, 236),
    control_active: rgb(212, 238, 229),
    control_focus: rgb(200, 230, 220),
    primary: rgb(12, 125, 96),
    danger: rgb(198, 47, 67),
    warning: rgb(154, 91, 0),
    violet: rgb(109, 79, 209),
    text_primary: rgb(27, 31, 36),
    text_muted: rgb(91, 100, 114),
    accent_text: rgb(52, 74, 170),
    on_accent: rgb(255, 255, 255),
    primary_hover: rgb(16, 146, 112),
    danger_hover: rgb(216, 68, 88),
    danger_wash: Color::rgba(198, 47, 67, 16),
    hover_overlay: Color::rgba(15, 23, 42, 14),
    pressed_overlay: Color::rgba(15, 23, 42, 28),
    gpu_color: rgb(8, 145, 178),
    phase_build: rgb(37, 99, 235),
    phase_layout: rgb(5, 150, 105),
    phase_paint: rgb(124, 58, 237),
    phase_semantics: rgb(219, 39, 119),
    phase_composite: rgb(234, 88, 12),
    phase_other: rgb(148, 157, 172),
};

thread_local! {
    static CURRENT: Cell<Theme> = const { Cell::new(Theme::Dark) };
}

/// Selects the theme used by subsequent widget construction on this thread.
pub fn set_current(theme: Theme) {
    CURRENT.with(|current| current.set(theme));
}

pub fn current() -> Theme {
    CURRENT.with(Cell::get)
}

pub fn palette() -> &'static Palette {
    current().palette()
}

/// WCAG 2.x relative luminance of an opaque sRGB colour.
pub fn relative_luminance(color: Color) -> f64 {
    fn channel(value: u8) -> f64 {
        let value = f64::from(value) / 255.;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(color.red) + 0.7152 * channel(color.green) + 0.0722 * channel(color.blue)
}

/// WCAG contrast ratio between two opaque colours (1.0 to 21.0).
pub fn contrast_ratio(a: Color, b: Color) -> f64 {
    let (a, b) = (relative_luminance(a), relative_luminance(b));
    let (light, dark) = if a > b { (a, b) } else { (b, a) };
    (light + 0.05) / (dark + 0.05)
}

macro_rules! accessors {
    ($($name:ident),* $(,)?) => {
        $(
            #[inline]
            pub fn $name() -> Color {
                palette().$name
            }
        )*
    };
}

accessors!(
    background,
    surface,
    surface_raised,
    border,
    control,
    control_hover,
    control_active,
    control_focus,
    primary,
    danger,
    warning,
    violet,
    text_primary,
    text_muted,
    accent_text,
    on_accent,
    primary_hover,
    danger_hover,
    danger_wash,
    hover_overlay,
    pressed_overlay,
    gpu_color,
);

/// Alias kept for readability where "success" is meant.
pub fn success() -> Color {
    primary()
}
