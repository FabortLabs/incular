//! Flutter's `SegmentedButton`/`ButtonSegment` and `ToggleButtons`.

use std::{collections::BTreeSet, rc::Rc};

use incular_config::{CrossAxisAlignment, EdgeInsets, MainAxisAlignment, MainAxisSize};
use incular_core::Color;
use incular_widgets::{
    Border, BorderRadius, BorderSide, BoxDecoration, Center, ClipRRect, Container, Expanded,
    IntrinsicHeight, Row, Text, Widget, internal::ActionSurface,
};

use super::theme_of;
use crate::Icons;
use crate::material_theme::helpers::alpha;

/// Flutter's `ButtonSegment`.
#[derive(Clone)]
pub struct ButtonSegment<T> {
    pub value: T,
    pub label: Option<Widget>,
    pub icon: Option<Widget>,
    pub tooltip: Option<String>,
    pub enabled: bool,
}

impl<T> ButtonSegment<T> {
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            value,
            label: None,
            icon: None,
            tooltip: None,
            enabled: true,
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<Widget>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn text(self, label: impl Into<String>) -> Self {
        self.label(Text::new(label))
    }

    #[must_use]
    pub fn icon(mut self, icon: impl Into<Widget>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Flutter's `SegmentedButton`: a row of segments where one (or, with
/// [`Self::multi_selection_enabled`], several) is selected.
#[derive(Clone)]
pub struct SegmentedButton<T: Ord + Clone + 'static> {
    segments: Vec<ButtonSegment<T>>,
    selected: BTreeSet<T>,
    on_selection_changed: Option<Rc<dyn Fn(BTreeSet<T>)>>,
    multi_selection_enabled: bool,
    empty_selection_allowed: bool,
    show_selected_icon: bool,
}

impl<T: Ord + Clone + 'static> SegmentedButton<T> {
    /// `_SegmentedButtonDefaultsM3` minimum height.
    pub const HEIGHT: f32 = 40.;

    #[must_use]
    pub fn new(
        segments: impl IntoIterator<Item = ButtonSegment<T>>,
        selected: impl IntoIterator<Item = T>,
    ) -> Self {
        Self {
            segments: segments.into_iter().collect(),
            selected: selected.into_iter().collect(),
            on_selection_changed: None,
            multi_selection_enabled: false,
            empty_selection_allowed: false,
            show_selected_icon: true,
        }
    }

    #[must_use]
    pub fn on_selection_changed(mut self, callback: impl Fn(BTreeSet<T>) + 'static) -> Self {
        self.on_selection_changed = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn multi_selection_enabled(mut self, enabled: bool) -> Self {
        self.multi_selection_enabled = enabled;
        self
    }

    #[must_use]
    pub fn empty_selection_allowed(mut self, allowed: bool) -> Self {
        self.empty_selection_allowed = allowed;
        self
    }

    #[must_use]
    pub fn show_selected_icon(mut self, show: bool) -> Self {
        self.show_selected_icon = show;
        self
    }

    /// `_SegmentedButtonState._handleOnPressed`: the selection after a press
    /// on `value`, or `None` when the press changes nothing.
    #[must_use]
    pub fn selection_after_press(&self, value: &T) -> Option<BTreeSet<T>> {
        let mut next = self.selected.clone();
        if next.contains(value) {
            if !self.empty_selection_allowed && next.len() == 1 {
                return None;
            }
            next.remove(value);
        } else {
            if !self.multi_selection_enabled {
                next.clear();
            }
            next.insert(value.clone());
        }
        Some(next)
    }
}

impl<T: Ord + Clone + 'static> From<SegmentedButton<T>> for Widget {
    fn from(button: SegmentedButton<T>) -> Self {
        let button = Rc::new(button);
        incular_widgets::Builder::new(move |context| {
            let theme = theme_of(context);
            let scheme = &theme.core().color_scheme;
            let text = &theme.core().text_theme;
            let enabled_all = button.on_selection_changed.is_some();
            let outline = scheme.outline;
            let count = button.segments.len();
            let mut children: Vec<Widget> = Vec::new();
            for (index, segment) in button.segments.iter().enumerate() {
                let selected = button.selected.contains(&segment.value);
                let enabled = enabled_all && segment.enabled;
                let foreground = if !enabled {
                    alpha(scheme.on_surface, 0.38)
                } else if selected {
                    scheme.on_secondary_container
                } else {
                    scheme.on_surface
                };
                let mut content: Vec<Widget> = Vec::new();
                if selected && button.show_selected_icon {
                    content.push(Text::new(Icons::CHECK).color(foreground).into());
                } else if let Some(icon) = &segment.icon {
                    content.push(icon.clone());
                }
                if let Some(label) = &segment.label {
                    content.push(
                        incular_widgets::DefaultTextStyle::new(
                            text.label_large.clone().color(foreground),
                            label.clone(),
                        )
                        .into(),
                    );
                }
                let content = Row::new(content)
                    .main_axis_size(MainAxisSize::Min)
                    .spacing(8.)
                    .cross_axis_alignment(CrossAxisAlignment::Center);
                let mut decoration = BoxDecoration::new();
                if selected {
                    decoration = decoration.color(scheme.secondary_container);
                }
                if index + 1 < count {
                    decoration = decoration.border(Border::only(
                        BorderSide::none(),
                        BorderSide::solid(outline, 1.),
                        BorderSide::none(),
                        BorderSide::none(),
                    ));
                }
                let mut surface = ActionSurface::with_child(
                    Container::new()
                        .decoration(decoration)
                        .padding(EdgeInsets::symmetric(12., 0.))
                        .child(Center::new(content)),
                )
                .color(Color::TRANSPARENT)
                .hover_color(alpha(foreground, 0.08))
                .pressed_color(alpha(foreground, 0.1))
                .focused_color(alpha(foreground, 0.1))
                .enabled(enabled);
                if enabled && let Some(callback) = button.on_selection_changed.clone() {
                    let button = button.clone();
                    let value = segment.value.clone();
                    surface = surface.on_click(move || {
                        if let Some(next) = button.selection_after_press(&value) {
                            callback(next);
                        }
                    });
                }
                if let Some(tooltip) = &segment.tooltip {
                    surface = surface.label(tooltip.clone());
                }
                // Segments share the width equally, as Flutter's
                // `_SegmentedButtonRenderWidget` does.
                children.push(Expanded::new(surface).into());
            }
            let radius = SegmentedButton::<T>::HEIGHT / 2.;
            Container::new()
                .height(SegmentedButton::<T>::HEIGHT)
                .decoration(
                    BoxDecoration::new()
                        .border(Border::all(BorderSide::solid(outline, 1.)))
                        .border_radius(BorderRadius::circular(radius)),
                )
                .child(ClipRRect::new(
                    radius,
                    IntrinsicHeight::new(
                        Row::new(children).cross_axis_alignment(CrossAxisAlignment::Stretch),
                    ),
                ))
                .into()
        })
        .into()
    }
}

/// Flutter's `ToggleButtons`: a bordered row of toggles, each selected or
/// not independently.
#[derive(Clone)]
pub struct ToggleButtons {
    children: Vec<Widget>,
    is_selected: Vec<bool>,
    on_pressed: Option<Rc<dyn Fn(usize)>>,
    border_radius: f32,
    min_size: (f32, f32),
    render_border: bool,
    selected_color: Option<Color>,
    fill_color: Option<Color>,
    color: Option<Color>,
}

impl ToggleButtons {
    #[must_use]
    pub fn new(
        children: impl IntoIterator<Item = impl Into<Widget>>,
        is_selected: impl IntoIterator<Item = bool>,
    ) -> Self {
        Self {
            children: children.into_iter().map(Into::into).collect(),
            is_selected: is_selected.into_iter().collect(),
            on_pressed: None,
            border_radius: 0.,
            // `kMinInteractiveDimension` square.
            min_size: (48., 48.),
            render_border: true,
            selected_color: None,
            fill_color: None,
            color: None,
        }
    }

    #[must_use]
    pub fn on_pressed(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_pressed = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    #[must_use]
    pub fn constraints(mut self, min_width: f32, min_height: f32) -> Self {
        self.min_size = (min_width, min_height);
        self
    }

    #[must_use]
    pub fn render_border(mut self, render: bool) -> Self {
        self.render_border = render;
        self
    }

    #[must_use]
    pub fn selected_color(mut self, color: Color) -> Self {
        self.selected_color = Some(color);
        self
    }

    #[must_use]
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl From<ToggleButtons> for Widget {
    fn from(buttons: ToggleButtons) -> Self {
        let buttons = Rc::new(buttons);
        incular_widgets::Builder::new(move |context| {
            let theme = theme_of(context);
            let scheme = &theme.core().color_scheme;
            // `ToggleButtons` defaults: borders `onSurface` at 12%, selected
            // content `primary`, selected fill `primary` at 12%.
            let border = alpha(scheme.on_surface, 0.12);
            let count = buttons.children.len();
            let children = buttons.children.iter().enumerate().map(|(index, child)| {
                let selected = buttons.is_selected.get(index).copied().unwrap_or(false);
                let enabled = buttons.on_pressed.is_some();
                let foreground = if !enabled {
                    alpha(scheme.on_surface, 0.38)
                } else if selected {
                    buttons.selected_color.unwrap_or(scheme.primary)
                } else {
                    buttons.color.unwrap_or(alpha(scheme.on_surface, 0.87))
                };
                let mut decoration = BoxDecoration::new();
                if selected {
                    decoration =
                        decoration.color(buttons.fill_color.unwrap_or(alpha(scheme.primary, 0.12)));
                }
                if buttons.render_border && index + 1 < count {
                    decoration = decoration.border(Border::only(
                        BorderSide::none(),
                        BorderSide::solid(border, 1.),
                        BorderSide::none(),
                        BorderSide::none(),
                    ));
                }
                let content = incular_widgets::DefaultTextStyle::new(
                    theme
                        .core()
                        .text_theme
                        .body_medium
                        .clone()
                        .color(foreground),
                    child.clone(),
                );
                let mut surface = ActionSurface::with_child(
                    Container::new()
                        .decoration(decoration)
                        .constraints(incular_config::Constraints::new(
                            buttons.min_size.0,
                            f32::INFINITY,
                            buttons.min_size.1,
                            f32::INFINITY,
                        ))
                        .child(Center::new(content)),
                )
                .color(Color::TRANSPARENT)
                .hover_color(alpha(foreground, 0.04))
                .pressed_color(alpha(foreground, 0.16))
                .focused_color(alpha(foreground, 0.12))
                .enabled(enabled);
                if let Some(callback) = buttons.on_pressed.clone() {
                    surface = surface.on_click(move || callback(index));
                }
                Widget::from(surface)
            });
            let row: Widget = IntrinsicHeight::new(
                Row::new(children)
                    .main_axis_size(MainAxisSize::Min)
                    .main_axis_alignment(MainAxisAlignment::Start)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch),
            )
            .into();
            let mut decoration =
                BoxDecoration::new().border_radius(BorderRadius::circular(buttons.border_radius));
            if buttons.render_border {
                decoration = decoration.border(Border::all(BorderSide::solid(border, 1.)));
            }
            Container::new()
                .decoration(decoration)
                .child(ClipRRect::new(buttons.border_radius, row))
                .into()
        })
        .into()
    }
}
