//! Flutter's `AnimatedIcon`/`AnimatedIcons` and the ink splash factories
//! (`InkRipple`, `InkSplash`, `InkSparkle`).
//!
//! Flutter's animated icons morph vector paths. Incular's Material icons are
//! glyphs, so an [`AnimatedIcon`] cross-fades between its start and end
//! glyph while turning half a turn, the visual rhythm of the originals.

use incular_core::Color;
use incular_widgets::{Center, Opacity, SizedBox, Stack, Text, Widget};

use crate::SplashFactory;

/// Flutter's `AnimatedIconData`: the icon at progress 0 and at progress 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimatedIconData {
    pub start: &'static str,
    pub end: &'static str,
    /// Whether the icon turns while it morphs.
    pub rotates: bool,
}

/// Flutter's `AnimatedIcons` catalog.
pub struct AnimatedIcons;

impl AnimatedIcons {
    pub const ADD_EVENT: AnimatedIconData = AnimatedIconData {
        start: "+",
        end: "▦",
        rotates: false,
    };
    pub const ARROW_MENU: AnimatedIconData = AnimatedIconData {
        start: "←",
        end: "☰",
        rotates: true,
    };
    pub const CLOSE_MENU: AnimatedIconData = AnimatedIconData {
        start: "×",
        end: "☰",
        rotates: true,
    };
    pub const ELLIPSIS_SEARCH: AnimatedIconData = AnimatedIconData {
        start: "…",
        end: "⌕",
        rotates: false,
    };
    pub const EVENT_ADD: AnimatedIconData = AnimatedIconData {
        start: "▦",
        end: "+",
        rotates: false,
    };
    pub const HOME_MENU: AnimatedIconData = AnimatedIconData {
        start: "⌂",
        end: "☰",
        rotates: false,
    };
    pub const LIST_VIEW: AnimatedIconData = AnimatedIconData {
        start: "☰",
        end: "▦",
        rotates: false,
    };
    pub const MENU_ARROW: AnimatedIconData = AnimatedIconData {
        start: "☰",
        end: "←",
        rotates: true,
    };
    pub const MENU_CLOSE: AnimatedIconData = AnimatedIconData {
        start: "☰",
        end: "×",
        rotates: true,
    };
    pub const MENU_HOME: AnimatedIconData = AnimatedIconData {
        start: "☰",
        end: "⌂",
        rotates: false,
    };
    pub const PAUSE_PLAY: AnimatedIconData = AnimatedIconData {
        start: "Ⅱ",
        end: "▶",
        rotates: false,
    };
    pub const PLAY_PAUSE: AnimatedIconData = AnimatedIconData {
        start: "▶",
        end: "Ⅱ",
        rotates: false,
    };
    pub const SEARCH_ELLIPSIS: AnimatedIconData = AnimatedIconData {
        start: "⌕",
        end: "…",
        rotates: false,
    };
    pub const VIEW_LIST: AnimatedIconData = AnimatedIconData {
        start: "▦",
        end: "☰",
        rotates: false,
    };
}

/// Flutter's `AnimatedIcon` at a given progress from 0 to 1, typically an
/// animation controller's value.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimatedIcon {
    pub icon: AnimatedIconData,
    pub progress: f32,
    pub size: f32,
    pub color: Option<Color>,
    pub semantic_label: Option<String>,
}

impl AnimatedIcon {
    #[must_use]
    pub fn new(icon: AnimatedIconData, progress: f32) -> Self {
        Self {
            icon,
            progress: progress.clamp(0., 1.),
            size: 24.,
            color: None,
            semantic_label: None,
        }
    }

    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    #[must_use]
    pub fn semantic_label(mut self, label: impl Into<String>) -> Self {
        self.semantic_label = Some(label.into());
        self
    }
}

impl From<AnimatedIcon> for Widget {
    fn from(icon: AnimatedIcon) -> Self {
        let glyph = |text: &'static str| {
            let mut glyph =
                Text::new(text).style(incular_text::TextStyle::new().font_size(icon.size));
            if let Some(color) = icon.color {
                glyph = glyph.color(color);
            }
            Widget::from(Center::new(glyph))
        };
        let progress = icon.progress;
        let turns = if icon.icon.rotates {
            progress * 0.5
        } else {
            0.
        };
        let stack: Widget = Stack::new([
            Widget::from(Opacity::new(1. - progress, glyph(icon.icon.start))),
            Widget::from(Opacity::new(progress, glyph(icon.icon.end))),
        ])
        .into();
        let turned: Widget = if turns == 0. {
            stack
        } else {
            incular_widgets::AnimatedRotation::new(turns, std::time::Duration::ZERO, stack).into()
        };
        let sized: Widget = SizedBox::new()
            .width(icon.size)
            .height(icon.size)
            .child(turned)
            .into();
        match icon.semantic_label {
            Some(label) => incular_widgets::Semantics::new(sized).label(label).into(),
            None => sized,
        }
    }
}

/// Flutter's `InkRipple`: `InkRipple.splashFactory` for a theme or button.
pub struct InkRipple;
/// Flutter's `InkSplash`.
pub struct InkSplash;
/// Flutter's `InkSparkle`, the Material 3 default on Android.
pub struct InkSparkle;
/// Flutter's `NoSplash`.
pub struct NoSplash;

impl InkRipple {
    pub const SPLASH_FACTORY: SplashFactory = SplashFactory::Ripple;
}

impl InkSplash {
    pub const SPLASH_FACTORY: SplashFactory = SplashFactory::Splash;
}

impl InkSparkle {
    pub const SPLASH_FACTORY: SplashFactory = SplashFactory::Sparkle;
}

impl NoSplash {
    pub const SPLASH_FACTORY: SplashFactory = SplashFactory::NoSplash;
}
