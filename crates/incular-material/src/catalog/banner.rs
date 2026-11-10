//! Flutter's `MaterialBanner` (`material/banner.dart`) with Material 3
//! defaults.

use incular_config::{CrossAxisAlignment, EdgeInsets, MainAxisAlignment, MainAxisSize};
use incular_core::Color;
use incular_widgets::{Column, Container, Expanded, Row, Widget};

use super::theme_of;
use crate::Divider;

/// Flutter's `MaterialBanner`: a message with actions, shown at the top of
/// a screen.
#[derive(Clone)]
pub struct MaterialBanner {
    content: Widget,
    actions: Vec<Widget>,
    leading: Option<Widget>,
    background_color: Option<Color>,
    divider_color: Option<Color>,
    padding: Option<EdgeInsets>,
    leading_padding: Option<EdgeInsets>,
    force_actions_below: bool,
}

impl MaterialBanner {
    #[must_use]
    pub fn new(
        content: impl Into<Widget>,
        actions: impl IntoIterator<Item = impl Into<Widget>>,
    ) -> Self {
        Self {
            content: content.into(),
            actions: actions.into_iter().map(Into::into).collect(),
            leading: None,
            background_color: None,
            divider_color: None,
            padding: None,
            leading_padding: None,
            force_actions_below: false,
        }
    }

    #[must_use]
    pub fn leading(mut self, leading: impl Into<Widget>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    #[must_use]
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    #[must_use]
    pub fn divider_color(mut self, color: Color) -> Self {
        self.divider_color = Some(color);
        self
    }

    #[must_use]
    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
        self
    }

    #[must_use]
    pub fn leading_padding(mut self, padding: EdgeInsets) -> Self {
        self.leading_padding = Some(padding);
        self
    }

    #[must_use]
    pub fn force_actions_below(mut self, force: bool) -> Self {
        self.force_actions_below = force;
        self
    }
}

impl From<MaterialBanner> for Widget {
    fn from(banner: MaterialBanner) -> Self {
        incular_widgets::Builder::new(move |context| {
            let theme = theme_of(context);
            let scheme = &theme.core().color_scheme;
            let text = &theme.core().text_theme;
            // `_MaterialBannerState.build`: actions go below the content
            // when there are several or when forced.
            let single_line = !banner.force_actions_below && banner.actions.len() == 1;
            let padding = banner.padding.unwrap_or(if single_line {
                EdgeInsets::only(16., 2., 0., 0.)
            } else {
                EdgeInsets::only(16., 24., 16., 4.)
            });
            let leading_padding = banner
                .leading_padding
                .unwrap_or(EdgeInsets::only(0., 0., 16., 0.));
            let actions: Widget = Container::new()
                .padding(EdgeInsets::symmetric(8., 0.))
                .child(
                    Row::new(banner.actions.clone())
                        .main_axis_size(MainAxisSize::Min)
                        .main_axis_alignment(MainAxisAlignment::End)
                        .spacing(8.),
                )
                .into();
            let content: Widget = incular_widgets::DefaultTextStyle::new(
                text.body_medium.clone().color(scheme.on_surface),
                banner.content.clone(),
            )
            .into();
            let mut row: Vec<Widget> = Vec::new();
            if let Some(leading) = &banner.leading {
                row.push(
                    Container::new()
                        .padding(leading_padding)
                        .child(leading.clone())
                        .into(),
                );
            }
            row.push(Expanded::new(content).into());
            if single_line {
                row.push(actions.clone());
            }
            let mut column =
                vec![Widget::from(Container::new().padding(padding).child(
                    Row::new(row).cross_axis_alignment(CrossAxisAlignment::Center),
                ))];
            if !single_line {
                column.push(
                    incular_widgets::Align::new(incular_widgets::Alignment::CENTER_RIGHT, actions)
                        .into(),
                );
            }
            column.push(
                Divider::new()
                    .height(0.)
                    .color(banner.divider_color.unwrap_or(scheme.outline_variant))
                    .into(),
            );
            crate::Material::new(
                Column::new(column)
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch),
            )
            .color(
                banner
                    .background_color
                    .unwrap_or(scheme.surface_container_low),
            )
            .elevation(0.)
            .into()
        })
        .into()
    }
}
