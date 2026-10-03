//! Theme models and color palettes for Incular Studio.

use incular_core::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    HighContrast,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct StudioTheme {
    pub mode: ThemeMode,
    pub background: Color,
    pub surface: Color,
    pub surface_elevated: Color,
    pub surface_hover: Color,
    pub surface_active: Color,
    pub border: Color,
    pub border_focus: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_active: Color,
    pub selection: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,
    pub success: Color,
    pub line_number: Color,
    pub line_number_active: Color,
    pub divider: Color,
}

impl StudioTheme {
    #[must_use]
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
            ThemeMode::HighContrast => Self::high_contrast(),
        }
    }

    #[must_use]
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            background: Color::rgba(22, 30, 29, 255),
            surface: Color::rgba(30, 41, 39, 255),
            surface_elevated: Color::rgba(40, 54, 49, 255),
            surface_hover: Color::rgba(47, 64, 58, 255),
            surface_active: Color::rgba(55, 74, 65, 255),
            border: Color::rgba(64, 87, 77, 255),
            border_focus: Color::rgba(240, 185, 114, 255),
            text_primary: Color::rgba(245, 246, 229, 255),
            text_secondary: Color::rgba(187, 206, 186, 255),
            text_muted: Color::rgba(145, 170, 149, 255),
            accent: Color::rgba(240, 185, 114, 255),
            accent_hover: Color::rgba(255, 206, 146, 255),
            accent_active: Color::rgba(214, 159, 92, 255),
            selection: Color::rgba(109, 89, 50, 180),
            error: Color::rgba(235, 87, 87, 255),
            warning: Color::rgba(242, 153, 74, 255),
            info: Color::rgba(45, 156, 219, 255),
            success: Color::rgba(39, 174, 96, 255),
            line_number: Color::rgba(139, 159, 141, 255),
            line_number_active: Color::rgba(230, 235, 214, 255),
            divider: Color::rgba(50, 67, 60, 255),
        }
    }

    #[must_use]
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            background: Color::rgba(247, 243, 235, 255),
            surface: Color::rgba(255, 253, 248, 255),
            surface_elevated: Color::rgba(237, 229, 214, 255),
            surface_hover: Color::rgba(234, 224, 206, 255),
            surface_active: Color::rgba(223, 209, 185, 255),
            border: Color::rgba(207, 194, 174, 255),
            border_focus: Color::rgba(158, 62, 34, 255),
            text_primary: Color::rgba(46, 38, 32, 255),
            text_secondary: Color::rgba(94, 79, 63, 255),
            text_muted: Color::rgba(112, 96, 80, 255),
            accent: Color::rgba(158, 62, 34, 255),
            accent_hover: Color::rgba(181, 76, 43, 255),
            accent_active: Color::rgba(134, 49, 26, 255),
            selection: Color::rgba(226, 190, 159, 180),
            error: Color::rgba(220, 50, 50, 255),
            warning: Color::rgba(230, 130, 30, 255),
            info: Color::rgba(30, 140, 205, 255),
            success: Color::rgba(30, 155, 80, 255),
            line_number: Color::rgba(124, 111, 94, 255),
            line_number_active: Color::rgba(46, 38, 32, 255),
            divider: Color::rgba(222, 211, 191, 255),
        }
    }

    #[must_use]
    pub fn high_contrast() -> Self {
        Self {
            mode: ThemeMode::HighContrast,
            background: Color::rgba(0, 0, 0, 255),
            surface: Color::rgba(15, 15, 15, 255),
            surface_elevated: Color::rgba(30, 30, 30, 255),
            surface_hover: Color::rgba(45, 45, 45, 255),
            surface_active: Color::rgba(60, 60, 60, 255),
            border: Color::rgba(255, 255, 255, 255),
            border_focus: Color::rgba(255, 255, 0, 255),
            text_primary: Color::rgba(255, 255, 255, 255),
            text_secondary: Color::rgba(230, 230, 230, 255),
            text_muted: Color::rgba(180, 180, 180, 255),
            accent: Color::rgba(255, 255, 0, 255),
            accent_hover: Color::rgba(255, 255, 100, 255),
            accent_active: Color::rgba(220, 220, 0, 255),
            selection: Color::rgba(255, 255, 255, 120),
            error: Color::rgba(255, 50, 50, 255),
            warning: Color::rgba(255, 180, 0, 255),
            info: Color::rgba(100, 200, 255, 255),
            success: Color::rgba(50, 255, 50, 255),
            line_number: Color::rgba(200, 200, 200, 255),
            line_number_active: Color::rgba(255, 255, 255, 255),
            divider: Color::rgba(200, 200, 200, 255),
        }
    }
}
