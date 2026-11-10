//! The Controls text fields' context menu: a compact popup panel in the
//! control theme, placed at the click like Flutter's desktop toolbars.

use crate::button::Button;
use crate::styles::ButtonVariant;
use crate::theme::current_control_theme;
use incular_config::EdgeInsets;
use incular_core::Offset;
use incular_widgets::{
    Border, BorderRadius, BoxDecoration, BuildContext, Column, Container, CrossAxisAlignment,
    CustomSingleChildLayout, DesktopTextSelectionToolbarLayoutDelegate,
    EditableTextContextMenuBuilder, MainAxisSize, Padding, SizedBox, TextContextMenu, Widget,
};
use std::rc::Rc;

const SCREEN_PADDING: f32 = 8.0;
const MENU_WIDTH: f32 = 200.0;

/// Builds the Controls text context menu for `menu`.
#[must_use]
pub fn text_context_menu(context: &BuildContext<'_>, menu: &TextContextMenu) -> Widget {
    if menu.button_items.is_empty() {
        return SizedBox::shrink().into();
    }
    let theme = current_control_theme(context);
    let items: Vec<Widget> = menu
        .button_items
        .iter()
        .map(|item| {
            let mut button = Button::new(item.resolved_label()).variant(ButtonVariant::Ghost);
            match item.on_pressed.clone() {
                Some(on_pressed) => button = button.on_click(move || on_pressed()),
                None => button = button.enabled(false),
            }
            SizedBox::from_dimensions(
                None,
                Some(theme.menu.item_height),
                Some(button.build(&theme)),
            )
            .into()
        })
        .collect();
    let panel = Container::new()
        .width(MENU_WIDTH)
        .padding(EdgeInsets::all(theme.menu.popup_padding))
        .decoration(
            BoxDecoration::new()
                .color(theme.colors.surface_elevated)
                .border(Border::new(1.0, theme.colors.border))
                .border_radius(BorderRadius::circular(theme.menu.radius)),
        )
        .child(
            Column::new(items)
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch),
        );
    let top = context.safe_insets().top + SCREEN_PADDING;
    Padding::new(
        EdgeInsets::from_ltrb(SCREEN_PADDING, top, SCREEN_PADDING, SCREEN_PADDING),
        CustomSingleChildLayout::new(
            DesktopTextSelectionToolbarLayoutDelegate {
                anchor: menu.anchors.primary_anchor - Offset::new(SCREEN_PADDING, top),
            },
            panel,
        ),
    )
    .into()
}

/// The default `context_menu_builder` of Controls text fields.
#[must_use]
pub fn default_text_context_menu_builder() -> EditableTextContextMenuBuilder {
    Rc::new(text_context_menu)
}
