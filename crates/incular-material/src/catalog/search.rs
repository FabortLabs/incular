//! Flutter's `SearchBar`, `SearchAnchor` and `SearchController`
//! (`material/search_anchor.dart`) with the Material 3 defaults.

use std::rc::Rc;

use incular_config::{CrossAxisAlignment, EdgeInsets, MainAxisSize};
use incular_core::Color;
use incular_widgets::{
    BorderRadius, Column, ConstrainedBox, Container, EditableText, Expanded, GestureDetector,
    ListView, OverlayPortal, Row, SizedBox, Text, TextEditingController, TransientPlacement,
    TransientRole, Widget,
};

use super::{Revision, theme_of};
use crate::{Divider, Icons};

/// Flutter's `SearchController`: the query text and whether the search view
/// is open.
#[derive(Clone)]
pub struct SearchController {
    text: TextEditingController,
    open: Rc<std::cell::Cell<bool>>,
    revision: Revision,
}

impl Default for SearchController {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchController {
    #[must_use]
    pub fn new() -> Self {
        Self {
            text: TextEditingController::new(),
            open: Rc::new(std::cell::Cell::new(false)),
            revision: Revision::default(),
        }
    }

    /// The text field controller.
    #[must_use]
    pub fn text_controller(&self) -> &TextEditingController {
        &self.text
    }

    #[must_use]
    pub fn text(&self) -> String {
        self.text.text()
    }

    pub fn set_text(&self, text: impl Into<String>) {
        self.text.set_text(text.into());
        self.revision.bump();
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open.get()
    }

    /// `SearchController.openView`.
    pub fn open_view(&self) {
        if !self.open.replace(true) {
            self.revision.bump();
        }
    }

    /// `SearchController.closeView`: closes the view, setting the query to
    /// `selected_text` when given.
    pub fn close_view(&self, selected_text: Option<String>) {
        if let Some(text) = selected_text {
            self.text.set_text(text);
        }
        if self.open.replace(false) {
            self.revision.bump();
        }
    }
}

type TextCallback = Rc<dyn Fn(&str)>;

/// Flutter's `SearchBar`: a stadium-shaped text field with leading and
/// trailing slots.
#[derive(Clone)]
pub struct SearchBar {
    controller: SearchController,
    hint_text: Option<String>,
    leading: Option<Widget>,
    trailing: Vec<Widget>,
    on_tap: Option<Rc<dyn Fn()>>,
    on_changed: Option<TextCallback>,
    on_submitted: Option<TextCallback>,
    background_color: Option<Color>,
    elevation: Option<f32>,
    padding: EdgeInsets,
    min_height: f32,
}

impl SearchBar {
    /// `_SearchBarDefaultsM3`: 56 high, padding 8 on both sides.
    pub const MIN_HEIGHT: f32 = 56.;

    #[must_use]
    pub fn new() -> Self {
        Self::with_controller(SearchController::new())
    }

    #[must_use]
    pub fn with_controller(controller: SearchController) -> Self {
        Self {
            controller,
            hint_text: None,
            leading: None,
            trailing: Vec::new(),
            on_tap: None,
            on_changed: None,
            on_submitted: None,
            background_color: None,
            elevation: None,
            padding: EdgeInsets::symmetric(8., 0.),
            min_height: Self::MIN_HEIGHT,
        }
    }

    #[must_use]
    pub fn controller(&self) -> &SearchController {
        &self.controller
    }

    #[must_use]
    pub fn hint_text(mut self, hint: impl Into<String>) -> Self {
        self.hint_text = Some(hint.into());
        self
    }

    #[must_use]
    pub fn leading(mut self, leading: impl Into<Widget>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    #[must_use]
    pub fn trailing(mut self, trailing: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        self.trailing = trailing.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn on_tap(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_tap = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_changed(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.on_changed = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_submitted(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.on_submitted = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    #[must_use]
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = Some(elevation);
        self
    }

    #[must_use]
    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = padding;
        self
    }
}

impl Default for SearchBar {
    fn default() -> Self {
        Self::new()
    }
}

impl From<SearchBar> for Widget {
    fn from(bar: SearchBar) -> Self {
        let bar = Rc::new(bar);
        incular_widgets::Builder::new(move |context| {
            let theme = theme_of(context);
            let scheme = &theme.core().color_scheme;
            let text = &theme.core().text_theme;
            let mut field = EditableText::new(bar.controller.text.clone())
                .style(text.body_large.clone().color(scheme.on_surface))
                .placeholder_color(scheme.on_surface_variant)
                .cursor_color(scheme.primary);
            if let Some(hint) = &bar.hint_text {
                field = field.placeholder(hint.clone());
            }
            if let Some(on_changed) = bar.on_changed.clone() {
                field = field.on_changed(move |value| on_changed(value));
            }
            if let Some(on_submitted) = bar.on_submitted.clone() {
                field = field.on_submit(move |value| on_submitted(&value));
            }
            let mut children: Vec<Widget> = Vec::new();
            if let Some(leading) = &bar.leading {
                children.push(leading.clone());
            }
            children.push(
                Expanded::new(
                    Container::new()
                        .padding(EdgeInsets::symmetric(8., 0.))
                        .child(field),
                )
                .into(),
            );
            children.extend(bar.trailing.iter().cloned());
            let content: Widget = ConstrainedBox::new(
                incular_config::Constraints::new(0., f32::INFINITY, bar.min_height, f32::INFINITY),
                crate::Material::new(
                    Container::new()
                        .padding(bar.padding)
                        .child(Row::new(children).cross_axis_alignment(CrossAxisAlignment::Center)),
                )
                .color(
                    bar.background_color
                        .unwrap_or(scheme.surface_container_high),
                )
                .elevation(bar.elevation.unwrap_or(6.))
                .border_radius(BorderRadius::circular(bar.min_height / 2.)),
            )
            .into();
            match bar.on_tap.clone() {
                Some(on_tap) => GestureDetector::new(content)
                    .on_tap(move || on_tap())
                    .into(),
                None => content,
            }
        })
        .into()
    }
}

type SuggestionsBuilder = Rc<dyn Fn(&SearchController) -> Vec<Widget>>;
type AnchorBuilder = Rc<dyn Fn(&SearchController) -> Widget>;

/// Flutter's `SearchAnchor`: an anchor that opens a search view with a
/// search bar over a list of suggestions.
///
/// On desktop Flutter's view covers the anchor; Incular places it just
/// below the anchor, as other anchored menus are.
#[derive(Clone)]
pub struct SearchAnchor {
    controller: SearchController,
    builder: AnchorBuilder,
    suggestions_builder: SuggestionsBuilder,
    view_hint_text: Option<String>,
    view_leading: Option<Widget>,
    view_trailing: Vec<Widget>,
    view_background_color: Option<Color>,
    view_elevation: Option<f32>,
    view_constraints: incular_config::Constraints,
}

impl SearchAnchor {
    #[must_use]
    pub fn new(
        builder: impl Fn(&SearchController) -> Widget + 'static,
        suggestions_builder: impl Fn(&SearchController) -> Vec<Widget> + 'static,
    ) -> Self {
        Self {
            controller: SearchController::new(),
            builder: Rc::new(builder),
            suggestions_builder: Rc::new(suggestions_builder),
            view_hint_text: None,
            view_leading: None,
            view_trailing: Vec::new(),
            view_background_color: None,
            view_elevation: None,
            // `_SearchViewDefaultsM3`: min 360 wide, 240 high.
            view_constraints: incular_config::Constraints::new(360., 800., 240., 480.),
        }
    }

    /// Flutter's `SearchAnchor.bar`: a [`SearchBar`] that opens the view
    /// when tapped.
    #[must_use]
    pub fn bar(
        suggestions_builder: impl Fn(&SearchController) -> Vec<Widget> + 'static,
        hint: impl Into<String>,
    ) -> Self {
        let hint = hint.into();
        let bar_hint = hint.clone();
        let mut anchor = Self::new(
            move |controller| {
                let open = controller.clone();
                SearchBar::with_controller(controller.clone())
                    .hint_text(bar_hint.clone())
                    .leading(Text::new(Icons::SEARCH))
                    .on_tap(move || open.open_view())
                    .on_changed({
                        let open = controller.clone();
                        move |_| open.open_view()
                    })
                    .into()
            },
            suggestions_builder,
        );
        anchor.view_hint_text = Some(hint);
        anchor
    }

    #[must_use]
    pub fn search_controller(mut self, controller: SearchController) -> Self {
        self.controller = controller;
        self
    }

    #[must_use]
    pub fn controller(&self) -> &SearchController {
        &self.controller
    }

    #[must_use]
    pub fn view_hint_text(mut self, hint: impl Into<String>) -> Self {
        self.view_hint_text = Some(hint.into());
        self
    }

    #[must_use]
    pub fn view_leading(mut self, leading: impl Into<Widget>) -> Self {
        self.view_leading = Some(leading.into());
        self
    }

    #[must_use]
    pub fn view_trailing(mut self, trailing: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        self.view_trailing = trailing.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn view_background_color(mut self, color: Color) -> Self {
        self.view_background_color = Some(color);
        self
    }

    #[must_use]
    pub fn view_elevation(mut self, elevation: f32) -> Self {
        self.view_elevation = Some(elevation);
        self
    }

    #[must_use]
    pub fn view_constraints(mut self, constraints: incular_config::Constraints) -> Self {
        self.view_constraints = constraints;
        self
    }
}

impl From<SearchAnchor> for Widget {
    fn from(anchor: SearchAnchor) -> Self {
        let anchor = Rc::new(anchor);
        Widget::stateful_builder(anchor.controller.revision.cell(), move |context| {
            let controller = anchor.controller.clone();
            let theme = theme_of(context);
            let scheme = &theme.core().color_scheme;
            let child = (anchor.builder)(&controller);
            if !controller.is_open() {
                return OverlayPortal::new(child).show(false).into();
            }
            // `_ViewContent`: a search bar header, a divider, then the
            // suggestions.
            let mut header = SearchBar::with_controller(controller.clone())
                .background_color(Color::TRANSPARENT)
                .elevation(0.)
                .leading(Text::new(Icons::ARROW_BACK).color(scheme.on_surface))
                .on_changed({
                    let controller = controller.clone();
                    move |_| controller.revision.bump()
                });
            if let Some(hint) = &anchor.view_hint_text {
                header = header.hint_text(hint.clone());
            }
            if let Some(leading) = &anchor.view_leading {
                header = header.leading(leading.clone());
            }
            if !anchor.view_trailing.is_empty() {
                header = header.trailing(anchor.view_trailing.clone());
            }
            let suggestions = (anchor.suggestions_builder)(&controller);
            let view: Widget = ConstrainedBox::new(
                anchor.view_constraints,
                crate::Material::new(
                    Column::new([
                        Widget::from(header),
                        Divider::new().height(1.).into(),
                        Widget::from(incular_widgets::Flexible::new(
                            ListView::new(suggestions).shrink_wrap(true),
                        )),
                    ])
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch),
                )
                .color(
                    anchor
                        .view_background_color
                        .unwrap_or(scheme.surface_container_high),
                )
                .elevation(anchor.view_elevation.unwrap_or(6.))
                .border_radius(BorderRadius::circular(28.)),
            )
            .into();
            let dismiss = controller.clone();
            OverlayPortal::new(child)
                .role(TransientRole::ComboBox)
                .placement(TransientPlacement::new())
                .overlay_child(view)
                .show(true)
                .on_dismiss(move |_| dismiss.close_view(None))
                .into()
        })
    }
}

/// A suggestion row for a [`SearchAnchor`]'s view: a `ListTile`-shaped
/// tappable row that closes the view with its text.
#[must_use]
pub fn search_suggestion(controller: &SearchController, text: impl Into<String>) -> Widget {
    let text = text.into();
    let controller = controller.clone();
    let label = text.clone();
    GestureDetector::new(
        Container::new()
            .padding(EdgeInsets::symmetric(16., 0.))
            .child(
                SizedBox::new()
                    .height(56.)
                    .child(incular_widgets::Align::new(
                        incular_widgets::Alignment::CENTER_LEFT,
                        Text::new(label),
                    )),
            ),
    )
    .on_tap(move || controller.close_view(Some(text.clone())))
    .into()
}
