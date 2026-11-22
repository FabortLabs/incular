//! The Controls text fields' context menu in the control theme: on touch
//! platforms a row of buttons above or below the selection, like Flutter's
//! `TextSelectionToolbar`; elsewhere a compact panel at the click, like its
//! desktop toolbars.

use crate::button::Button;
use crate::styles::ButtonVariant;
use crate::theme::{ControlTheme, current_control_theme};
use incular_config::{EdgeInsets, TargetPlatform};
use incular_core::Offset;
use incular_widgets::{
    Border, BorderRadius, BoxDecoration, BuildContext, Column, Container, CrossAxisAlignment,
    CustomSingleChildLayout, DesktopTextSelectionToolbarLayoutDelegate,
    EditableTextContextMenuBuilder, MainAxisSize, Padding, Row, SizedBox, TextContextMenu,
    TextSelectionToolbarLayoutDelegate, Widget,
};
use std::rc::Rc;

/// Flutter `_kToolbarScreenPadding`.
const SCREEN_PADDING: f32 = 8.0;
const MENU_WIDTH: f32 = 200.0;
/// Flutter `_kToolbarContentDistance`: the gap above the selection.
const CONTENT_DISTANCE: f32 = 8.0;
/// Flutter `_kToolbarContentDistanceBelow` (`_kHandleSize - 2`): the gap
/// below the selection, which leaves room for the selection handles.
const CONTENT_DISTANCE_BELOW: f32 = 20.0;

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
    let touch = matches!(
        TargetPlatform::host(),
        TargetPlatform::Android | TargetPlatform::Ios | TargetPlatform::Fuchsia
    );
    if touch {
        touch_toolbar(context, menu, &theme, items)
    } else {
        desktop_panel(context, menu, &theme, items)
    }
}

fn panel_decoration(theme: &ControlTheme) -> BoxDecoration {
    BoxDecoration::new()
        .color(theme.colors.surface_elevated)
        .border(Border::new(1.0, theme.colors.border))
        .border_radius(BorderRadius::circular(theme.menu.radius))
}

/// Flutter's `TextSelectionToolbar`: above the selection when there is
/// room under the top padding, otherwise below it.
fn touch_toolbar(
    context: &BuildContext<'_>,
    menu: &TextContextMenu,
    theme: &ControlTheme,
    items: Vec<Widget>,
) -> Widget {
    let padding_above = context.safe_insets().top + SCREEN_PADDING;
    let toolbar_height = theme.menu.item_height + 2.0 * (theme.menu.popup_padding + 1.0);
    let anchor_above = menu.anchors.primary_anchor;
    let anchor_below = menu.anchors.secondary_anchor.unwrap_or(anchor_above);
    let fits_above = anchor_above.y - CONTENT_DISTANCE - padding_above >= toolbar_height;
    let local = Offset::new(SCREEN_PADDING, padding_above);
    let toolbar = Container::new()
        .padding(EdgeInsets::all(theme.menu.popup_padding))
        .decoration(panel_decoration(theme))
        .child(Row::new(items).main_axis_size(MainAxisSize::Min));
    Padding::new(
        EdgeInsets::from_ltrb(
            SCREEN_PADDING,
            padding_above,
            SCREEN_PADDING,
            SCREEN_PADDING,
        ),
        CustomSingleChildLayout::new(
            TextSelectionToolbarLayoutDelegate {
                anchor_above: anchor_above - local - Offset::new(0.0, CONTENT_DISTANCE),
                anchor_below: anchor_below - local + Offset::new(0.0, CONTENT_DISTANCE_BELOW),
                fits_above: Some(fits_above),
            },
            toolbar,
        ),
    )
    .into()
}

/// A vertical panel whose corner sits at the click.
fn desktop_panel(
    context: &BuildContext<'_>,
    menu: &TextContextMenu,
    theme: &ControlTheme,
    items: Vec<Widget>,
) -> Widget {
    let panel = Container::new()
        .width(MENU_WIDTH)
        .padding(EdgeInsets::all(theme.menu.popup_padding))
        .decoration(panel_decoration(theme))
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
