//! Material's desktop text selection toolbars: Flutter's
//! `DesktopTextSelectionToolbar`, `DesktopTextSelectionToolbarButton`,
//! `CupertinoDesktopTextSelectionToolbar` (which Material uses on macOS) and
//! the `AdaptiveTextSelectionToolbar` that picks between them.

use crate::{ButtonStyleConfig, Material, MaterialType, TextButton, Theme};
use incular_config::EdgeInsets;
use incular_config::{Brightness, Clip, TargetPlatform};
use incular_controls::{ControlState, StateValue};
use incular_core::{Color, Offset, Size};
use incular_widgets::{
    Alignment, BackdropFilter, BorderRadius, BorderSide, BoxShadow, BuildContext, Builder,
    ClipRRect, Column, ContextMenuButtonItem, CustomSingleChildLayout, DecoratedShapeBox,
    DesktopTextSelectionToolbarLayoutDelegate, MainAxisSize, Padding, RoundedSuperellipseBorder,
    ShapeDecoration, SizedBox, Text, TextContextMenu, TextSelectionToolbarAnchors, TextStyle,
    Widget,
};
use std::rc::Rc;

const TOOLBAR_SCREEN_PADDING: f32 = 8.0;
const TOOLBAR_WIDTH: f32 = 222.0;

/// The screen padding and anchor placement shared by both desktop toolbars.
fn desktop_toolbar_layout(
    context: &BuildContext<'_>,
    anchor: Offset,
    toolbar: impl Into<Widget>,
) -> Widget {
    let padding_above = context.safe_insets().top + TOOLBAR_SCREEN_PADDING;
    let local_adjustment = Offset::new(TOOLBAR_SCREEN_PADDING, padding_above);
    Padding::new(
        EdgeInsets::from_ltrb(
            TOOLBAR_SCREEN_PADDING,
            padding_above,
            TOOLBAR_SCREEN_PADDING,
            TOOLBAR_SCREEN_PADDING,
        ),
        CustomSingleChildLayout::new(
            DesktopTextSelectionToolbarLayoutDelegate {
                anchor: anchor - local_adjustment,
            },
            toolbar,
        ),
    )
    .into()
}

fn toolbar_column(children: Vec<Widget>) -> Widget {
    Column::new(children)
        .main_axis_size(MainAxisSize::Min)
        .into()
}

fn theme_brightness(context: &BuildContext<'_>) -> Brightness {
    Theme::of(context).map_or(Brightness::Light, |theme| {
        theme.core().color_scheme.brightness
    })
}

/// Flutter's Material `DesktopTextSelectionToolbar`: a 222-wide card with
/// 7-pixel corners and elevation 1, holding its buttons in a column.
#[derive(Clone)]
pub struct DesktopTextSelectionToolbar {
    anchor: Offset,
    children: Vec<Widget>,
}

impl DesktopTextSelectionToolbar {
    #[must_use]
    pub fn new(anchor: Offset, children: Vec<Widget>) -> Self {
        debug_assert!(!children.is_empty(), "a toolbar needs at least one child");
        Self { anchor, children }
    }
}

impl From<DesktopTextSelectionToolbar> for Widget {
    fn from(value: DesktopTextSelectionToolbar) -> Self {
        Builder::new(move |context| {
            let card_color = Theme::of(context).map(|theme| theme.colors().card_color);
            let mut material = Material::new(toolbar_column(value.children.clone()))
                .material_type(MaterialType::Card)
                .border_radius(BorderRadius::circular(7.0))
                .clip_behavior(Clip::AntiAlias)
                .elevation(1.0);
            if let Some(color) = card_color {
                material = material.color(color);
            }
            let toolbar = SizedBox::from_dimensions(Some(TOOLBAR_WIDTH), None, Some(material));
            desktop_toolbar_layout(context, value.anchor, toolbar)
        })
        .into()
    }
}

/// Flutter's `DesktopTextSelectionToolbarButton`: a full-width, left-aligned
/// text button with square corners and a basic cursor.
#[derive(Clone)]
pub struct DesktopTextSelectionToolbarButton {
    on_pressed: Option<Rc<dyn Fn()>>,
    child: Option<Widget>,
    text: Option<String>,
}

impl DesktopTextSelectionToolbarButton {
    #[must_use]
    pub fn new(on_pressed: Option<Rc<dyn Fn()>>, child: impl Into<Widget>) -> Self {
        Self {
            on_pressed,
            child: Some(child.into()),
            text: None,
        }
    }

    /// `DesktopTextSelectionToolbarButton.text`: 14-pixel, regular-weight
    /// text, white on a dark scheme and black87 otherwise.
    #[must_use]
    pub fn text(on_pressed: Option<Rc<dyn Fn()>>, text: impl Into<String>) -> Self {
        Self {
            on_pressed,
            child: None,
            text: Some(text.into()),
        }
    }
}

fn desktop_foreground(brightness: Brightness) -> Color {
    match brightness {
        Brightness::Dark => Color::WHITE,
        // Colors.black87.
        Brightness::Light => Color::rgba(0, 0, 0, 0xDD),
    }
}

impl From<DesktopTextSelectionToolbarButton> for Widget {
    fn from(value: DesktopTextSelectionToolbarButton) -> Self {
        Builder::new(move |context| {
            let foreground = desktop_foreground(theme_brightness(context));
            let child = value.child.clone().unwrap_or_else(|| {
                Text::new(value.text.clone().unwrap_or_default())
                    .style(
                        TextStyle::new()
                            .font_size(14.0)
                            .letter_spacing(-0.15)
                            .color(foreground),
                    )
                    .into()
            });
            let style = TextButton::style_from(
                ButtonStyleConfig::builder()
                    .alignment(Alignment::CENTER_LEFT)
                    .mouse_cursor("basic")
                    .foreground_color(foreground)
                    .shape(BorderRadius::ZERO)
                    .minimum_size(Size::new(48.0, 36.0))
                    .padding(EdgeInsets::from_ltrb(20.0, 0.0, 20.0, 3.0))
                    .build(),
            );
            let mut button = TextButton::with_child(child).style(style);
            match value.on_pressed.clone() {
                Some(on_pressed) => button = button.on_pressed(move || on_pressed()),
                None => button = button.enabled(false),
            }
            SizedBox::from_dimensions(Some(f32::INFINITY), None, Some(button)).into()
        })
        .into()
    }
}

/// Flutter's `CupertinoDesktopTextSelectionToolbar`, used by Material on
/// macOS: a translucent, blurred panel with 8-pixel corners, a hairline
/// border and a soft drop shadow.
///
/// Flutter also boosts the backdrop's saturation threefold; Incular's
/// backdrop filter is blur-only, so that boost is not applied.
#[derive(Clone)]
pub struct CupertinoDesktopTextSelectionToolbar {
    anchor: Offset,
    children: Vec<Widget>,
}

impl CupertinoDesktopTextSelectionToolbar {
    #[must_use]
    pub fn new(anchor: Offset, children: Vec<Widget>) -> Self {
        debug_assert!(!children.is_empty(), "a toolbar needs at least one child");
        Self { anchor, children }
    }
}

impl From<CupertinoDesktopTextSelectionToolbar> for Widget {
    fn from(value: CupertinoDesktopTextSelectionToolbar) -> Self {
        Builder::new(move |context| {
            let dark = theme_brightness(context) == Brightness::Dark;
            let radius = BorderRadius::circular(8.0);
            let border_color = if dark {
                Color::rgba(0x5B, 0x5B, 0x5B, 0xFF)
            } else {
                Color::rgba(0xB8, 0xB8, 0xB8, 0xFF)
            };
            let background = if dark {
                Color::rgba(0x30, 0x30, 0x30, 0xB2)
            } else {
                Color::rgba(0xFF, 0xFF, 0xFF, 0xB2)
            };
            let panel = DecoratedShapeBox::new(
                ShapeDecoration {
                    color: Some(background),
                    gradient: None,
                    shadows: Vec::new(),
                    shape: Rc::new(RoundedSuperellipseBorder {
                        side: BorderSide::new(
                            border_color,
                            1.0,
                            incular_widgets::BorderStyle::Solid,
                        ),
                        border_radius: radius,
                    }),
                },
                Padding::new(EdgeInsets::all(6.0), toolbar_column(value.children.clone())),
            );
            let blurred = ClipRRect::new(
                radius,
                BackdropFilter::new(incular_rendering::GaussianBlur::uniform(20.0), panel),
            );
            let toolbar = DecoratedShapeBox::new(
                ShapeDecoration {
                    color: None,
                    gradient: None,
                    shadows: vec![BoxShadow::new(
                        Color::rgba(0, 0, 0, 60),
                        Offset::new(0.0, 4.0),
                        10.0,
                        0.5,
                    )],
                    shape: Rc::new(RoundedSuperellipseBorder {
                        side: BorderSide::NONE,
                        border_radius: radius,
                    }),
                },
                blurred,
            );
            let toolbar = SizedBox::from_dimensions(Some(TOOLBAR_WIDTH), None, Some(toolbar));
            desktop_toolbar_layout(context, value.anchor, toolbar)
        })
        .into()
    }
}

/// Flutter's `CupertinoDesktopTextSelectionToolbarButton.text`: 14-pixel
/// text in a 4-pixel rounded button that fills with the primary blue and
/// turns its label white while hovered.
///
/// Flutter's `CupertinoButton` also fades to 70% opacity while pressed;
/// this button uses the text button's press feedback instead.
#[derive(Clone)]
pub struct CupertinoDesktopTextSelectionToolbarButton {
    on_pressed: Option<Rc<dyn Fn()>>,
    text: String,
}

impl CupertinoDesktopTextSelectionToolbarButton {
    #[must_use]
    pub fn text(on_pressed: Option<Rc<dyn Fn()>>, text: impl Into<String>) -> Self {
        Self {
            on_pressed,
            text: text.into(),
        }
    }
}

impl From<CupertinoDesktopTextSelectionToolbarButton> for Widget {
    fn from(value: CupertinoDesktopTextSelectionToolbarButton) -> Self {
        Builder::new(move |context| {
            let dark = theme_brightness(context) == Brightness::Dark;
            // CupertinoColors.black/white and CupertinoColors.activeBlue.
            let label = if dark { Color::WHITE } else { Color::BLACK };
            let primary = if dark {
                Color::rgba(0x0A, 0x84, 0xFF, 0xFF)
            } else {
                Color::rgba(0x00, 0x7A, 0xFF, 0xFF)
            };
            let style = TextButton::style_from(
                ButtonStyleConfig::builder()
                    .alignment(Alignment::CENTER_LEFT)
                    .mouse_cursor("basic")
                    .foreground_color(StateValue::resolver(move |state| {
                        if state.contains(ControlState::HOVERED) {
                            Color::WHITE
                        } else {
                            label
                        }
                    }))
                    .background_color(StateValue::resolver(move |state| {
                        if state.contains(ControlState::HOVERED) {
                            primary
                        } else {
                            Color::TRANSPARENT
                        }
                    }))
                    .overlay_color(Color::TRANSPARENT)
                    .shape(BorderRadius::circular(4.0))
                    .minimum_size(Size::new(0.0, 0.0))
                    .padding(EdgeInsets::from_ltrb(8.0, 2.0, 8.0, 5.0))
                    .build(),
            );
            let text = Text::new(value.text.clone())
                .style(TextStyle::new().font_size(14.0).letter_spacing(-0.15));
            let mut button = TextButton::with_child(text).style(style);
            match value.on_pressed.clone() {
                Some(on_pressed) => button = button.on_pressed(move || on_pressed()),
                None => button = button.enabled(false),
            }
            SizedBox::from_dimensions(Some(f32::INFINITY), None, Some(button)).into()
        })
        .into()
    }
}

/// Flutter's `AdaptiveTextSelectionToolbar`: the platform's toolbar for a
/// list of [`ContextMenuButtonItem`]s.
///
/// Desktop platforms get Flutter's desktop toolbars. Android and iOS get the
/// desktop toolbar too: Incular has no touch selection toolbar yet.
#[derive(Clone)]
pub struct AdaptiveTextSelectionToolbar {
    anchors: TextSelectionToolbarAnchors,
    button_items: Vec<ContextMenuButtonItem>,
}

impl AdaptiveTextSelectionToolbar {
    #[must_use]
    pub fn button_items(
        anchors: TextSelectionToolbarAnchors,
        button_items: Vec<ContextMenuButtonItem>,
    ) -> Self {
        Self {
            anchors,
            button_items,
        }
    }

    /// `AdaptiveTextSelectionToolbar.editableText`.
    #[must_use]
    pub fn editable_text(menu: &TextContextMenu) -> Self {
        Self::button_items(menu.anchors, menu.button_items.clone())
    }

    /// `AdaptiveTextSelectionToolbar.getAdaptiveButtons`.
    #[must_use]
    pub fn adaptive_buttons(
        platform: TargetPlatform,
        button_items: &[ContextMenuButtonItem],
    ) -> Vec<Widget> {
        button_items
            .iter()
            .map(|item| {
                let label = item.resolved_label();
                match platform {
                    TargetPlatform::MacOs => CupertinoDesktopTextSelectionToolbarButton::text(
                        item.on_pressed.clone(),
                        label,
                    )
                    .into(),
                    _ => DesktopTextSelectionToolbarButton::text(item.on_pressed.clone(), label)
                        .into(),
                }
            })
            .collect()
    }
}

impl From<AdaptiveTextSelectionToolbar> for Widget {
    fn from(value: AdaptiveTextSelectionToolbar) -> Self {
        if value.button_items.is_empty() {
            return SizedBox::shrink().into();
        }
        Builder::new(move |context| {
            let platform =
                Theme::of(context).map_or_else(TargetPlatform::host, |theme| theme.core().platform);
            let children =
                AdaptiveTextSelectionToolbar::adaptive_buttons(platform, &value.button_items);
            let anchor = value.anchors.primary_anchor;
            match platform {
                TargetPlatform::MacOs => {
                    CupertinoDesktopTextSelectionToolbar::new(anchor, children).into()
                }
                _ => DesktopTextSelectionToolbar::new(anchor, children).into(),
            }
        })
        .into()
    }
}

/// Material `TextField`'s default `contextMenuBuilder`.
#[must_use]
pub fn default_text_context_menu_builder() -> incular_widgets::EditableTextContextMenuBuilder {
    Rc::new(|_, menu| AdaptiveTextSelectionToolbar::editable_text(menu).into())
}
