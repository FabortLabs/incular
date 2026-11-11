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

/// Workspace colours.
///
/// The tone scale follows shadcn/ui's zinc ramp: the page `background` is the
/// darkest (or, in light, a soft grey), cards sit one step up on `surface`
/// without an outline, and `surface_raised` marks wells nested inside a card
/// (table headers, code blocks, tab lists). `border` is only a hairline
/// separator inside cards, never a card outline.
#[derive(Debug, PartialEq, Eq)]
pub struct Palette {
    pub background: Color,
    pub surface: Color,
    pub surface_raised: Color,
    pub border: Color,
    /// Sidebar and top bar fill, between the page and a card.
    pub chrome: Color,
    /// Informational accent (console info level, links in notes).
    pub info: Color,
    /// Soft shadow under cards; fully transparent where tone alone separates.
    pub card_shadow: Color,
    /// Shadow under floating panels (menus, palette, dialogs).
    pub overlay_shadow: Color,
    /// Dimming layer behind modal dialogs.
    pub scrim: Color,
    pub control: Color,
    pub control_hover: Color,
    pub control_active: Color,
    pub control_focus: Color,
    /// Brand violet accent: selection, primary actions, links.
    pub primary: Color,
    /// Positive states (healthy FPS, within budget, applied overrides).
    pub success: Color,
    /// Keyboard-focus ring drawn around focused controls.
    pub focus_ring: Color,
    /// Label colour of a disabled control.
    pub text_disabled: Color,
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
    /// Faint accent wash behind a hovered list row; reads on the surface and
    /// on the selected-row tint without turning grey.
    pub row_hover: Color,
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
    background: rgb(9, 9, 11),
    surface: rgb(20, 20, 24),
    surface_raised: rgb(28, 28, 33),
    border: rgb(39, 39, 45),
    chrome: rgb(14, 14, 17),
    info: rgb(96, 165, 250),
    card_shadow: Color::rgba(0, 0, 0, 0),
    overlay_shadow: Color::rgba(0, 0, 0, 150),
    scrim: Color::rgba(0, 0, 0, 150),
    control: rgb(36, 36, 42),
    control_hover: rgb(46, 46, 53),
    control_active: rgb(42, 33, 74),
    control_focus: rgb(58, 45, 102),
    primary: rgb(167, 139, 250),
    success: rgb(74, 222, 128),
    focus_ring: rgb(196, 181, 253),
    text_disabled: rgb(82, 82, 91),
    danger: rgb(248, 113, 113),
    warning: rgb(251, 191, 36),
    violet: rgb(192, 132, 252),
    text_primary: rgb(244, 244, 245),
    text_muted: rgb(161, 161, 170),
    accent_text: rgb(196, 181, 253),
    on_accent: rgb(20, 14, 40),
    primary_hover: rgb(186, 164, 252),
    danger_hover: rgb(252, 140, 140),
    danger_wash: Color::rgba(248, 113, 113, 18),
    hover_overlay: Color::rgba(255, 255, 255, 14),
    pressed_overlay: Color::rgba(255, 255, 255, 28),
    row_hover: Color::rgba(167, 139, 250, 20),
    gpu_color: rgb(34, 211, 238),
    phase_build: rgb(96, 165, 250),
    phase_layout: rgb(52, 211, 153),
    phase_paint: rgb(192, 132, 252),
    phase_semantics: rgb(244, 114, 182),
    phase_composite: rgb(251, 146, 60),
    phase_other: rgb(113, 113, 122),
};

pub static LIGHT: Palette = Palette {
    background: rgb(244, 244, 246),
    surface: rgb(255, 255, 255),
    surface_raised: rgb(247, 247, 249),
    border: rgb(228, 228, 231),
    chrome: rgb(250, 250, 251),
    info: rgb(37, 99, 235),
    card_shadow: Color::rgba(16, 16, 24, 14),
    overlay_shadow: Color::rgba(16, 16, 24, 46),
    scrim: Color::rgba(9, 9, 11, 72),
    control: rgb(240, 240, 243),
    control_hover: rgb(231, 231, 235),
    control_active: rgb(237, 233, 254),
    control_focus: rgb(221, 214, 254),
    primary: rgb(109, 40, 217),
    success: rgb(21, 128, 61),
    focus_ring: rgb(91, 33, 182),
    text_disabled: rgb(161, 161, 170),
    danger: rgb(220, 38, 38),
    warning: rgb(180, 83, 9),
    violet: rgb(126, 34, 206),
    text_primary: rgb(9, 9, 11),
    text_muted: rgb(82, 82, 91),
    accent_text: rgb(91, 33, 182),
    on_accent: rgb(255, 255, 255),
    primary_hover: rgb(124, 58, 237),
    danger_hover: rgb(239, 68, 68),
    danger_wash: Color::rgba(220, 38, 38, 16),
    hover_overlay: Color::rgba(9, 9, 11, 10),
    pressed_overlay: Color::rgba(9, 9, 11, 22),
    row_hover: Color::rgba(109, 40, 217, 14),
    gpu_color: rgb(8, 145, 178),
    phase_build: rgb(37, 99, 235),
    phase_layout: rgb(5, 150, 105),
    phase_paint: rgb(147, 51, 234),
    phase_semantics: rgb(219, 39, 119),
    phase_composite: rgb(234, 88, 12),
    phase_other: rgb(148, 148, 156),
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

/// `color` with its alpha replaced, for tinted fills behind accent text.
pub const fn tint(color: Color, alpha: u8) -> Color {
    Color::rgba(color.red, color.green, color.blue, alpha)
}

accessors!(
    background,
    surface,
    surface_raised,
    border,
    chrome,
    info,
    card_shadow,
    overlay_shadow,
    scrim,
    control,
    control_hover,
    control_active,
    control_focus,
    primary,
    success,
    focus_ring,
    text_disabled,
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
    row_hover,
    gpu_color,
);
