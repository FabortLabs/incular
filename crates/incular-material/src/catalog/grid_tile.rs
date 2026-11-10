//! Flutter's `GridTile`/`GridTileBar` and `TabPageSelector`/
//! `TabPageSelectorIndicator`.

use std::rc::Rc;

use incular_config::{CrossAxisAlignment, EdgeInsets, MainAxisSize};
use incular_core::Color;
use incular_widgets::{
    Border, BorderSide, BoxDecoration, BoxShape, Column, Container, Expanded, Positioned, Row,
    Stack, Widget,
};

use super::theme_of;
use crate::TabController;

/// Flutter's `GridTile`: a child with an optional header and footer laid
/// over its top and bottom edges.
#[derive(Clone)]
pub struct GridTile {
    child: Widget,
    header: Option<Widget>,
    footer: Option<Widget>,
}

impl GridTile {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            header: None,
            footer: None,
        }
    }

    #[must_use]
    pub fn header(mut self, header: impl Into<Widget>) -> Self {
        self.header = Some(header.into());
        self
    }

    #[must_use]
    pub fn footer(mut self, footer: impl Into<Widget>) -> Self {
        self.footer = Some(footer.into());
        self
    }
}

impl From<GridTile> for Widget {
    fn from(tile: GridTile) -> Self {
        if tile.header.is_none() && tile.footer.is_none() {
            return tile.child;
        }
        let mut children = vec![Widget::from(Positioned::fill(tile.child))];
        if let Some(header) = tile.header {
            children.push(Positioned::new(header).top(0.).left(0.).right(0.).into());
        }
        if let Some(footer) = tile.footer {
            children.push(Positioned::new(footer).bottom(0.).left(0.).right(0.).into());
        }
        Stack::new(children).into()
    }
}

/// Flutter's `GridTileBar`: 48 high, or 68 with both a title and a
/// subtitle, with white text on an optional background.
#[derive(Clone, Default)]
pub struct GridTileBar {
    background_color: Option<Color>,
    leading: Option<Widget>,
    title: Option<Widget>,
    subtitle: Option<Widget>,
    trailing: Option<Widget>,
}

impl GridTileBar {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    #[must_use]
    pub fn leading(mut self, leading: impl Into<Widget>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<Widget>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<Widget>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    #[must_use]
    pub fn trailing(mut self, trailing: impl Into<Widget>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }
}

impl From<GridTileBar> for Widget {
    fn from(bar: GridTileBar) -> Self {
        incular_widgets::Builder::new(move |context| {
            let theme = theme_of(context);
            let text = &theme.core().text_theme;
            // `GridTileBar.build`: a dark theme's title and subtitle styles.
            let title_style = text.title_medium.clone().color(Color::WHITE);
            let subtitle_style = text
                .body_small
                .clone()
                .color(Color::rgba(255, 255, 255, 179));
            let both = bar.title.is_some() && bar.subtitle.is_some();
            let padding = EdgeInsets::only(
                if bar.leading.is_some() { 8. } else { 16. },
                0.,
                if bar.trailing.is_some() { 8. } else { 16. },
                0.,
            );
            let mut row: Vec<Widget> = Vec::new();
            if let Some(leading) = &bar.leading {
                row.push(
                    Container::new()
                        .padding(EdgeInsets::only(0., 0., 8., 0.))
                        .child(leading.clone())
                        .into(),
                );
            }
            let mut lines: Vec<Widget> = Vec::new();
            if let Some(title) = &bar.title {
                lines.push(
                    incular_widgets::DefaultTextStyle::new(title_style, title.clone()).into(),
                );
            }
            if let Some(subtitle) = &bar.subtitle {
                lines.push(
                    incular_widgets::DefaultTextStyle::new(subtitle_style, subtitle.clone()).into(),
                );
            }
            row.push(
                Expanded::new(
                    Column::new(lines)
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Start),
                )
                .into(),
            );
            if let Some(trailing) = &bar.trailing {
                row.push(
                    Container::new()
                        .padding(EdgeInsets::only(8., 0., 0., 0.))
                        .child(trailing.clone())
                        .into(),
                );
            }
            let mut container = Container::new()
                .height(if both { 68. } else { 48. })
                .padding(padding)
                .child(Row::new(row).cross_axis_alignment(CrossAxisAlignment::Center));
            if let Some(color) = bar.background_color {
                container = container.color(color);
            }
            container.into()
        })
        .into()
    }
}

/// Flutter's `TabPageSelectorIndicator`: a bordered circle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabPageSelectorIndicator {
    pub background_color: Color,
    pub border_color: Color,
    pub size: f32,
    pub border_style_none: bool,
}

impl From<TabPageSelectorIndicator> for Widget {
    fn from(indicator: TabPageSelectorIndicator) -> Self {
        let mut decoration = BoxDecoration::new()
            .shape(BoxShape::Circle)
            .color(indicator.background_color);
        if !indicator.border_style_none {
            decoration =
                decoration.border(Border::all(BorderSide::solid(indicator.border_color, 1.)));
        }
        Container::new()
            .width(indicator.size)
            .height(indicator.size)
            .margin(EdgeInsets::all(4.))
            .decoration(decoration)
            .into()
    }
}

/// Flutter's `TabPageSelector`: one indicator per tab, the selected one
/// filled.
#[derive(Clone)]
pub struct TabPageSelector {
    controller: TabController,
    indicator_size: f32,
    color: Option<Color>,
    selected_color: Option<Color>,
    border_style_none: bool,
}

impl TabPageSelector {
    #[must_use]
    pub fn new(controller: TabController) -> Self {
        Self {
            controller,
            indicator_size: 12.,
            color: None,
            selected_color: None,
            border_style_none: false,
        }
    }

    #[must_use]
    pub fn indicator_size(mut self, size: f32) -> Self {
        self.indicator_size = size;
        self
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    #[must_use]
    pub fn selected_color(mut self, color: Color) -> Self {
        self.selected_color = Some(color);
        self
    }

    /// `borderStyle: BorderStyle.none`.
    #[must_use]
    pub fn hide_border(mut self, hide: bool) -> Self {
        self.border_style_none = hide;
        self
    }
}

impl From<TabPageSelector> for Widget {
    fn from(selector: TabPageSelector) -> Self {
        let selector = Rc::new(selector);
        Widget::stateful_builder(selector.controller.revision_cell(), move |context| {
            let theme = theme_of(context);
            // `fixColor` is transparent; `fixSelectedColor` is
            // `colorScheme.secondary`.
            let selected_color = selector
                .selected_color
                .unwrap_or(theme.core().color_scheme.secondary);
            let color = selector.color.unwrap_or(Color::TRANSPARENT);
            let index = selector.controller.index();
            Row::new((0..selector.controller.length()).map(|tab| {
                Widget::from(TabPageSelectorIndicator {
                    background_color: if tab == index { selected_color } else { color },
                    border_color: selected_color,
                    size: selector.indicator_size,
                    border_style_none: selector.border_style_none,
                })
            }))
            .main_axis_size(MainAxisSize::Min)
            .into()
        })
    }
}
