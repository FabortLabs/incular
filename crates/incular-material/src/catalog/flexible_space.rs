//! Flutter's `FlexibleSpaceBar` (`material/flexible_space_bar.dart`): an
//! app bar background whose title shrinks and background parallaxes as the
//! bar collapses.

use std::rc::Rc;

use incular_core::Offset;
use incular_widgets::{
    Align, Alignment, LayoutBuilder, Opacity, Positioned, Stack, Transform, Widget,
};

use super::theme_of;
use crate::{CollapseMode, K_TOOLBAR_HEIGHT};

/// Flutter's `FlexibleSpaceBar`. It reads how far it is collapsed from the
/// height its app bar gives it, between [`Self::expanded_height`] and the
/// toolbar height.
#[derive(Clone)]
pub struct FlexibleSpaceBar {
    title: Option<Widget>,
    background: Option<Widget>,
    center_title: Option<bool>,
    collapse_mode: CollapseMode,
    expanded_title_scale: f32,
    title_padding: Option<incular_config::EdgeInsets>,
    expanded_height: Option<f32>,
    collapsed_height: f32,
}

impl Default for FlexibleSpaceBar {
    fn default() -> Self {
        Self::new()
    }
}

impl FlexibleSpaceBar {
    #[must_use]
    pub fn new() -> Self {
        Self {
            title: None,
            background: None,
            center_title: None,
            collapse_mode: CollapseMode::Parallax,
            expanded_title_scale: 1.5,
            title_padding: None,
            expanded_height: None,
            collapsed_height: K_TOOLBAR_HEIGHT,
        }
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<Widget>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn background(mut self, background: impl Into<Widget>) -> Self {
        self.background = Some(background.into());
        self
    }

    #[must_use]
    pub fn center_title(mut self, center: bool) -> Self {
        self.center_title = Some(center);
        self
    }

    #[must_use]
    pub fn collapse_mode(mut self, mode: CollapseMode) -> Self {
        self.collapse_mode = mode;
        self
    }

    #[must_use]
    pub fn expanded_title_scale(mut self, scale: f32) -> Self {
        self.expanded_title_scale = scale.max(1.);
        self
    }

    #[must_use]
    pub fn title_padding(mut self, padding: incular_config::EdgeInsets) -> Self {
        self.title_padding = Some(padding);
        self
    }

    /// The fully expanded height, the app bar's `expandedHeight`. Without
    /// it, the first height the bar is given counts as fully expanded.
    #[must_use]
    pub fn expanded_height(mut self, height: f32) -> Self {
        self.expanded_height = Some(height);
        self
    }

    #[must_use]
    pub fn collapsed_height(mut self, height: f32) -> Self {
        self.collapsed_height = height;
        self
    }
}

impl From<FlexibleSpaceBar> for Widget {
    fn from(bar: FlexibleSpaceBar) -> Self {
        let bar = Rc::new(bar);
        let first_height = Rc::new(std::cell::Cell::new(None::<f32>));
        LayoutBuilder::new(move |context, constraints| {
            let theme = theme_of(context);
            let current: f32 = if constraints.max_height().is_finite() {
                constraints.max_height()
            } else {
                bar.expanded_height.unwrap_or(bar.collapsed_height)
            };
            let expanded = bar
                .expanded_height
                .unwrap_or_else(|| *first_height.get().get_or_insert(current))
                .max(bar.collapsed_height);
            if first_height.get().is_none() {
                first_height.set(Some(current));
            }
            let range = (expanded - bar.collapsed_height).max(f32::EPSILON);
            // `t`: 0 fully expanded, 1 fully collapsed.
            let t = (1. - (current - bar.collapsed_height) / range).clamp(0., 1.);
            let mut children = Vec::new();
            if let Some(background) = &bar.background {
                // `_getCollapsePadding`.
                let offset = match bar.collapse_mode {
                    CollapseMode::Parallax => -(expanded - current) * 0.5,
                    CollapseMode::Pin => -(expanded - current),
                    CollapseMode::None => 0.,
                };
                // The background fades out over the last part of the collapse
                // (`fadeStart` at the toolbar height plus its own height).
                let fade_start = (expanded - K_TOOLBAR_HEIGHT * 2.).max(0.) / range;
                let opacity = if t <= fade_start {
                    1.
                } else {
                    1. - ((t - fade_start) / (1. - fade_start).max(f32::EPSILON)).clamp(0., 1.)
                };
                children.push(Widget::from(
                    Positioned::new(Opacity::new(
                        opacity,
                        Transform::translation(Offset::new(0., offset), background.clone()),
                    ))
                    .top(0.)
                    .left(0.)
                    .right(0.)
                    .height(expanded),
                ));
            }
            if let Some(title) = &bar.title {
                let center = bar.center_title.unwrap_or(matches!(
                    theme.core().platform,
                    crate::TargetPlatform::Ios | crate::TargetPlatform::MacOs
                ));
                let padding = bar
                    .title_padding
                    .unwrap_or(incular_config::EdgeInsets::only(
                        if center { 0. } else { 72. },
                        0.,
                        0.,
                        16.,
                    ));
                let scale = 1. + (bar.expanded_title_scale - 1.) * (1. - t);
                let style = theme
                    .core()
                    .text_theme
                    .title_large
                    .clone()
                    .color(theme.core().color_scheme.on_surface);
                let title: Widget =
                    incular_widgets::DefaultTextStyle::new(style, title.clone()).into();
                let alignment = if center {
                    Alignment::BOTTOM_CENTER
                } else {
                    Alignment::BOTTOM_LEFT
                };
                children.push(Widget::from(Positioned::fill(
                    incular_widgets::Padding::new(
                        padding,
                        Align::new(alignment, Transform::scale(scale, title)),
                    ),
                )));
            }
            Stack::new(children).into()
        })
        .into()
    }
}
