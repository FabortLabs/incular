//! Flutter's `ExpansionTile`/`ExpansibleController` and
//! `ExpansionPanel`/`ExpansionPanelList`.

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_config::{CrossAxisAlignment, EdgeInsets, MainAxisSize};
use incular_core::Color;
use incular_widgets::{
    AnimatedRotation, Column, Container, Expanded, GestureDetector, Row, SizedBox, Text, Widget,
};

use super::{Revision, theme_of};
use crate::material_theme::helpers::alpha;
use crate::{Divider, ListTile};

/// `_kExpand`: the expansion animation's duration.
const EXPAND_DURATION: Duration = Duration::from_millis(200);
/// `Icons.expand_more`, a down chevron.
const EXPAND_MORE: &str = "⌄";

type ExpansionCallback = Rc<dyn Fn(bool)>;

/// Flutter's `ExpansionTileController` (`ExpansibleController`).
#[derive(Clone, Default)]
pub struct ExpansionTileController {
    expanded: Rc<Cell<bool>>,
    revision: Revision,
    on_change: Rc<std::cell::RefCell<Option<ExpansionCallback>>>,
}

impl ExpansionTileController {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_expanded(&self) -> bool {
        self.expanded.get()
    }

    pub fn expand(&self) {
        self.set_expanded(true);
    }

    pub fn collapse(&self) {
        self.set_expanded(false);
    }

    pub fn toggle(&self) {
        self.set_expanded(!self.is_expanded());
    }

    fn set_expanded(&self, expanded: bool) {
        if self.expanded.replace(expanded) != expanded {
            self.revision.bump();
            let callback = self.on_change.borrow().clone();
            if let Some(callback) = callback {
                callback(expanded);
            }
        }
    }
}

/// Flutter's `ExpansionTile`: a [`ListTile`] that shows its children when
/// expanded, its trailing chevron turning half a turn.
#[derive(Clone)]
pub struct ExpansionTile {
    title: Widget,
    subtitle: Option<Widget>,
    leading: Option<Widget>,
    trailing: Option<Widget>,
    children: Vec<Widget>,
    controller: ExpansionTileController,
    on_expansion_changed: Option<Rc<dyn Fn(bool)>>,
    tile_padding: Option<EdgeInsets>,
    children_padding: EdgeInsets,
    expanded_cross_axis_alignment: CrossAxisAlignment,
    background_color: Option<Color>,
    collapsed_background_color: Option<Color>,
    icon_color: Option<Color>,
    collapsed_icon_color: Option<Color>,
}

impl ExpansionTile {
    #[must_use]
    pub fn new(title: impl Into<Widget>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            leading: None,
            trailing: None,
            children: Vec::new(),
            controller: ExpansionTileController::new(),
            on_expansion_changed: None,
            tile_padding: None,
            children_padding: EdgeInsets::zero(),
            expanded_cross_axis_alignment: CrossAxisAlignment::Center,
            background_color: None,
            collapsed_background_color: None,
            icon_color: None,
            collapsed_icon_color: None,
        }
    }

    #[must_use]
    pub fn text(title: impl Into<String>) -> Self {
        Self::new(Text::new(title))
    }

    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<Widget>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    #[must_use]
    pub fn leading(mut self, leading: impl Into<Widget>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    /// Replaces the rotating chevron.
    #[must_use]
    pub fn trailing(mut self, trailing: impl Into<Widget>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    #[must_use]
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        self.children = children.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn controller(mut self, controller: ExpansionTileController) -> Self {
        let expanded = self.controller.is_expanded();
        self.controller = controller;
        if expanded {
            self.controller.expanded.set(true);
        }
        self
    }

    #[must_use]
    pub fn initially_expanded(self, expanded: bool) -> Self {
        self.controller.expanded.set(expanded);
        self
    }

    #[must_use]
    pub fn on_expansion_changed(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_expansion_changed = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn tile_padding(mut self, padding: EdgeInsets) -> Self {
        self.tile_padding = Some(padding);
        self
    }

    #[must_use]
    pub fn children_padding(mut self, padding: EdgeInsets) -> Self {
        self.children_padding = padding;
        self
    }

    #[must_use]
    pub fn expanded_cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.expanded_cross_axis_alignment = alignment;
        self
    }

    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    #[must_use]
    pub fn collapsed_background_color(mut self, color: Color) -> Self {
        self.collapsed_background_color = Some(color);
        self
    }

    #[must_use]
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
    }

    #[must_use]
    pub fn collapsed_icon_color(mut self, color: Color) -> Self {
        self.collapsed_icon_color = Some(color);
        self
    }
}

impl From<ExpansionTile> for Widget {
    fn from(tile: ExpansionTile) -> Self {
        *tile.controller.on_change.borrow_mut() = tile.on_expansion_changed.clone();
        let tile = Rc::new(tile);
        Widget::stateful_builder(tile.controller.revision.cell(), move |context| {
            let theme = theme_of(context);
            let scheme = &theme.core().color_scheme;
            let expanded = tile.controller.is_expanded();
            // `_ExpansionTileDefaultsM3`: icon `primary` when expanded,
            // `onSurfaceVariant` when collapsed.
            let icon_color = if expanded {
                tile.icon_color.unwrap_or(scheme.primary)
            } else {
                tile.collapsed_icon_color
                    .unwrap_or(scheme.on_surface_variant)
            };
            let trailing = tile.trailing.clone().unwrap_or_else(|| {
                AnimatedRotation::new(
                    if expanded { 0.5 } else { 0. },
                    EXPAND_DURATION,
                    Text::new(EXPAND_MORE).color(icon_color),
                )
                .into()
            });
            let mut list_tile = ListTile::new(tile.title.clone()).trailing(trailing);
            if let Some(subtitle) = &tile.subtitle {
                list_tile = list_tile.subtitle(subtitle.clone());
            }
            if let Some(leading) = &tile.leading {
                list_tile = list_tile.leading(leading.clone());
            }
            if let Some(padding) = tile.tile_padding {
                list_tile = list_tile.content_padding(padding);
            }
            let controller = tile.controller.clone();
            let header: Widget = GestureDetector::new(list_tile)
                .on_tap(move || controller.toggle())
                .into();
            let mut column = vec![header];
            if expanded {
                column.push(
                    Container::new()
                        .padding(tile.children_padding)
                        .child(
                            Column::new(tile.children.clone())
                                .main_axis_size(MainAxisSize::Min)
                                .cross_axis_alignment(tile.expanded_cross_axis_alignment),
                        )
                        .into(),
                );
            }
            let background = if expanded {
                tile.background_color
            } else {
                tile.collapsed_background_color
            };
            let mut container = Container::new().child(
                Column::new(column)
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch),
            );
            if let Some(color) = background {
                container = container.color(color);
            }
            container.into()
        })
    }
}

type HeaderBuilder = Rc<dyn Fn(bool) -> Widget>;

/// Flutter's `ExpansionPanel`.
#[derive(Clone)]
pub struct ExpansionPanel {
    pub header_builder: HeaderBuilder,
    pub body: Widget,
    pub is_expanded: bool,
    pub can_tap_on_header: bool,
    pub background_color: Option<Color>,
}

impl ExpansionPanel {
    #[must_use]
    pub fn new(header_builder: impl Fn(bool) -> Widget + 'static, body: impl Into<Widget>) -> Self {
        Self {
            header_builder: Rc::new(header_builder),
            body: body.into(),
            is_expanded: false,
            can_tap_on_header: false,
            background_color: None,
        }
    }

    #[must_use]
    pub fn is_expanded(mut self, expanded: bool) -> Self {
        self.is_expanded = expanded;
        self
    }

    #[must_use]
    pub fn can_tap_on_header(mut self, can_tap: bool) -> Self {
        self.can_tap_on_header = can_tap;
        self
    }

    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }
}

/// Flutter's `ExpansionPanelList`: elevated panels separated by dividers,
/// each with an expand icon; expanded panels gain a gap around them.
#[derive(Clone)]
pub struct ExpansionPanelList {
    children: Vec<ExpansionPanel>,
    expansion_callback: Option<Rc<dyn Fn(usize, bool)>>,
    elevation: f32,
    expanded_header_padding: EdgeInsets,
    divider_color: Option<Color>,
}

/// `_kPanelHeaderCollapsedHeight`, `_kPanelHeaderExpandedHeight`.
const PANEL_HEADER_COLLAPSED_HEIGHT: f32 = 48.;
const PANEL_HEADER_EXPANDED_HEIGHT: f32 = 64.;
/// `kExpandedEdgeInsets`... the gap `MergeableMaterial` opens around an
/// expanded panel.
const PANEL_GAP: f32 = 16.;

impl ExpansionPanelList {
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = ExpansionPanel>) -> Self {
        Self {
            children: children.into_iter().collect(),
            expansion_callback: None,
            elevation: 2.,
            expanded_header_padding: EdgeInsets::symmetric(
                0.,
                (PANEL_HEADER_EXPANDED_HEIGHT - PANEL_HEADER_COLLAPSED_HEIGHT) / 2.,
            ),
            divider_color: None,
        }
    }

    /// Called with the panel index and whether it is now expanded.
    #[must_use]
    pub fn expansion_callback(mut self, callback: impl Fn(usize, bool) + 'static) -> Self {
        self.expansion_callback = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = elevation;
        self
    }

    #[must_use]
    pub fn expanded_header_padding(mut self, padding: EdgeInsets) -> Self {
        self.expanded_header_padding = padding;
        self
    }

    #[must_use]
    pub fn divider_color(mut self, color: Color) -> Self {
        self.divider_color = Some(color);
        self
    }
}

impl From<ExpansionPanelList> for Widget {
    fn from(list: ExpansionPanelList) -> Self {
        let list = Rc::new(list);
        incular_widgets::Builder::new(move |context| {
            let theme = theme_of(context);
            let scheme = &theme.core().color_scheme;
            let mut items: Vec<Widget> = Vec::new();
            let count = list.children.len();
            for (index, panel) in list.children.iter().enumerate() {
                let expanded = panel.is_expanded;
                if expanded && index > 0 {
                    items.push(SizedBox::new().height(PANEL_GAP).into());
                }
                let toggle = list
                    .expansion_callback
                    .clone()
                    .map(|callback| Rc::new(move || callback(index, !expanded)) as Rc<dyn Fn()>);
                // `ExpandIcon`: 24px in a 48px button, `onSurface` at 54%.
                let icon: Widget = AnimatedRotation::new(
                    if expanded { 0.5 } else { 0. },
                    EXPAND_DURATION,
                    Text::new(EXPAND_MORE).color(alpha(scheme.on_surface, 0.54)),
                )
                .into();
                let icon: Widget = SizedBox::new()
                    .width(48.)
                    .height(48.)
                    .child(incular_widgets::Center::new(icon))
                    .into();
                let icon = match (&toggle, panel.can_tap_on_header) {
                    (Some(toggle), false) => {
                        let toggle = toggle.clone();
                        GestureDetector::new(icon).on_tap(move || toggle()).into()
                    }
                    _ => icon,
                };
                let header_content: Widget = Container::new()
                    .padding(if expanded {
                        list.expanded_header_padding
                    } else {
                        EdgeInsets::zero()
                    })
                    .constraints(incular_config::Constraints::new(
                        0.,
                        f32::INFINITY,
                        PANEL_HEADER_COLLAPSED_HEIGHT,
                        f32::INFINITY,
                    ))
                    .child((panel.header_builder)(expanded))
                    .into();
                let header: Widget = Row::new([
                    Widget::from(Expanded::new(header_content)),
                    Container::new()
                        .margin(EdgeInsets::only(0., 0., 8., 0.))
                        .child(icon)
                        .into(),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .into();
                let header = match (toggle, panel.can_tap_on_header) {
                    (Some(toggle), true) => {
                        GestureDetector::new(header).on_tap(move || toggle()).into()
                    }
                    _ => header,
                };
                let mut body = vec![header];
                if expanded {
                    body.push(panel.body.clone());
                }
                let mut panel_widget = Container::new().child(
                    Column::new(body)
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch),
                );
                if let Some(color) = panel.background_color {
                    panel_widget = panel_widget.color(color);
                }
                items.push(panel_widget.into());
                let next_expanded = list
                    .children
                    .get(index + 1)
                    .is_some_and(|next| next.is_expanded);
                if expanded && index + 1 < count {
                    items.push(SizedBox::new().height(PANEL_GAP).into());
                } else if index + 1 < count && !next_expanded {
                    let mut divider = Divider::new().height(1.);
                    if let Some(color) = list.divider_color {
                        divider = divider.color(color);
                    }
                    items.push(divider.into());
                }
            }
            crate::Material::new(
                Column::new(items)
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch),
            )
            .color(theme.colors().card_color)
            .elevation(list.elevation)
            .into()
        })
        .into()
    }
}
