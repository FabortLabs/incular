//! Material's text selection toolbars: Flutter's touch `TextSelectionToolbar`
//! and `TextSelectionToolbarTextButton`, `DesktopTextSelectionToolbar`,
//! `DesktopTextSelectionToolbarButton`, `CupertinoDesktopTextSelectionToolbar`
//! (which Material uses on macOS) and the `AdaptiveTextSelectionToolbar` that
//! picks between them.

use crate::{ButtonStyleConfig, Material, MaterialType, TextButton, Theme, ThemeData};
use incular_config::EdgeInsets;
use incular_config::{Brightness, Clip, TargetPlatform};
use incular_controls::{ControlState, StateValue};
use incular_core::{Color, Offset, Size};
use incular_widgets::{
    Alignment, BackdropFilter, BorderRadius, BorderSide, BoxShadow, BuildContext, Builder,
    ClipRRect, Column, ContextMenuButtonItem, CustomSingleChildLayout, DecoratedShapeBox,
    DesktopTextSelectionToolbarLayoutDelegate, MainAxisSize, MouseCursor, Padding,
    RoundedSuperellipseBorder, Row, ShapeDecoration, SizedBox, Text, TextContextMenu,
    TextSelectionToolbarAnchors, TextSelectionToolbarLayoutDelegate, TextStyle, Widget,
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

/// Flutter's `Theme.of`: without an ambient [`Theme`] the toolbars use the
/// light default, as `ThemeData.fallback()` does.
pub(super) fn toolbar_theme(context: &BuildContext<'_>) -> ThemeData {
    Theme::of(context).unwrap_or_else(ThemeData::light)
}

fn theme_brightness(context: &BuildContext<'_>) -> Brightness {
    toolbar_theme(context).core().color_scheme.brightness
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
            let card_color = toolbar_theme(context).colors().card_color;
            let material = Material::new(toolbar_column(value.children.clone()))
                .material_type(MaterialType::Card)
                .border_radius(BorderRadius::circular(7.0))
                .clip_behavior(Clip::AntiAlias)
                .elevation(1.0)
                .color(card_color);
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
                Text::new(value.text.as_deref().unwrap_or_default())
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
                    .mouse_cursor(MouseCursor::Basic)
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
                    aligned_gradient: None,
                    shadows: Vec::new(),
                    image: None,
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
                    aligned_gradient: None,
                    image: None,
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
                    .mouse_cursor(MouseCursor::Basic)
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

/// Flutter `_kToolbarHeight`.
const TOUCH_TOOLBAR_HEIGHT: f32 = 44.0;
/// Flutter `_kToolbarContentDistance`.
const TOOLBAR_CONTENT_DISTANCE: f32 = 8.0;
/// Flutter `kToolbarContentDistanceBelow` (`_kHandleSize - 2`).
const TOOLBAR_CONTENT_DISTANCE_BELOW: f32 = 20.0;

/// Flutter `TextSelectionToolbar._getColor` for the default surface: white
/// on light, `Color(0xff424242)` on dark. A scheme with its own surface
/// uses that instead.
fn touch_toolbar_color(theme: &ThemeData) -> Color {
    let scheme = &theme.core().color_scheme;
    let (default_theme, default_color) = match scheme.brightness {
        Brightness::Light => (ThemeData::light(), Color::WHITE),
        Brightness::Dark => (ThemeData::dark(), Color::rgba(0x42, 0x42, 0x42, 0xFF)),
    };
    if scheme.surface == default_theme.core().color_scheme.surface {
        default_color
    } else {
        scheme.surface
    }
}

/// Flutter's Material `TextSelectionToolbar`, the Android toolbar: a row of
/// buttons on a 44-high pill (corner radius 22) with elevation 1, above
/// `anchor_above` when it fits under the top padding, otherwise below
/// `anchor_below`.
///
/// Flutter folds buttons that do not fit the width into an overflow menu;
/// this toolbar keeps them in one row.
#[derive(Clone)]
pub struct TextSelectionToolbar {
    anchor_above: Offset,
    anchor_below: Offset,
    children: Vec<Widget>,
}

impl TextSelectionToolbar {
    #[must_use]
    pub fn new(anchor_above: Offset, anchor_below: Offset, children: Vec<Widget>) -> Self {
        debug_assert!(!children.is_empty(), "a toolbar needs at least one child");
        Self {
            anchor_above,
            anchor_below,
            children,
        }
    }
}

impl From<TextSelectionToolbar> for Widget {
    fn from(value: TextSelectionToolbar) -> Self {
        Builder::new(move |context| {
            let padding_above = context.safe_insets().top + TOOLBAR_SCREEN_PADDING;
            // Flutter `TextSelectionToolbar.build`: the content-distance gap
            // is taken from the anchor twice, once for the anchor itself and
            // once for the room the toolbar needs.
            let available_height =
                value.anchor_above.y - 2.0 * TOOLBAR_CONTENT_DISTANCE - padding_above;
            let fits_above = TOUCH_TOOLBAR_HEIGHT <= available_height;
            let local_adjustment = Offset::new(TOOLBAR_SCREEN_PADDING, padding_above);
            let theme = toolbar_theme(context);
            let row = Row::new(value.children.clone()).main_axis_size(MainAxisSize::Min);
            let material = Material::new(SizedBox::from_dimensions(
                None,
                Some(TOUCH_TOOLBAR_HEIGHT),
                Some(row),
            ))
            .material_type(MaterialType::Card)
            .border_radius(BorderRadius::circular(TOUCH_TOOLBAR_HEIGHT / 2.0))
            .clip_behavior(Clip::AntiAlias)
            .elevation(1.0)
            .color(touch_toolbar_color(&theme));
            Padding::new(
                EdgeInsets::from_ltrb(
                    TOOLBAR_SCREEN_PADDING,
                    padding_above,
                    TOOLBAR_SCREEN_PADDING,
                    TOOLBAR_SCREEN_PADDING,
                ),
                CustomSingleChildLayout::new(
                    TextSelectionToolbarLayoutDelegate {
                        anchor_above: value.anchor_above
                            - local_adjustment
                            - Offset::new(0.0, TOOLBAR_CONTENT_DISTANCE),
                        anchor_below: value.anchor_below - local_adjustment
                            + Offset::new(0.0, TOOLBAR_CONTENT_DISTANCE_BELOW),
                        fits_above: Some(fits_above),
                    },
                    material,
                ),
            )
            .into()
        })
        .into()
    }
}

/// Flutter's `TextSelectionToolbarTextButton`: a square-cornered text
/// button, black or white by brightness, with wider padding at the row's
/// ends.
#[derive(Clone)]
pub struct TextSelectionToolbarTextButton {
    on_pressed: Option<Rc<dyn Fn()>>,
    text: String,
    padding: EdgeInsets,
}

impl TextSelectionToolbarTextButton {
    /// Flutter `_kMiddlePadding` horizontally.
    const MIDDLE_PADDING: f32 = 9.5;
    /// Flutter `_kEndPadding`.
    const END_PADDING: f32 = 14.5;

    /// The button at `index` of `total` in its toolbar
    /// (`TextSelectionToolbarTextButton.getPadding`).
    #[must_use]
    pub fn text(
        on_pressed: Option<Rc<dyn Fn()>>,
        text: impl Into<String>,
        index: usize,
        total: usize,
    ) -> Self {
        let left = if index == 0 {
            Self::END_PADDING
        } else {
            Self::MIDDLE_PADDING
        };
        let right = if index + 1 == total {
            Self::END_PADDING
        } else {
            Self::MIDDLE_PADDING
        };
        Self {
            on_pressed,
            text: text.into(),
            padding: EdgeInsets::from_ltrb(left, 8.0, right, 8.0),
        }
    }
}

impl From<TextSelectionToolbarTextButton> for Widget {
    fn from(value: TextSelectionToolbarTextButton) -> Self {
        Builder::new(move |context| {
            let foreground = match theme_brightness(context) {
                Brightness::Dark => Color::WHITE,
                Brightness::Light => Color::BLACK,
            };
            let style = TextButton::style_from(
                ButtonStyleConfig::builder()
                    .foreground_color(foreground)
                    .shape(BorderRadius::ZERO)
                    .minimum_size(Size::new(48.0, 48.0))
                    .padding(value.padding)
                    .build(),
            );
            let mut button = TextButton::new(value.text.clone()).style(style);
            match value.on_pressed.clone() {
                Some(on_pressed) => button = button.on_pressed(move || on_pressed()),
                None => button = button.enabled(false),
            }
            button.into()
        })
        .into()
    }
}

/// Flutter's `AdaptiveTextSelectionToolbar`: the platform's toolbar for a
/// list of [`ContextMenuButtonItem`]s.
///
/// Android and Fuchsia get the touch [`TextSelectionToolbar`], macOS the
/// Cupertino desktop toolbar and Linux and Windows the desktop one. iOS gets
/// the touch Material toolbar: Incular has no `CupertinoTextSelectionToolbar`
/// yet.
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
        let total = button_items.len();
        button_items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let label = item.resolved_label();
                let on_pressed = item.on_pressed.clone();
                match platform {
                    TargetPlatform::MacOs => {
                        CupertinoDesktopTextSelectionToolbarButton::text(on_pressed, label).into()
                    }
                    TargetPlatform::Linux | TargetPlatform::Windows => {
                        DesktopTextSelectionToolbarButton::text(on_pressed, label).into()
                    }
                    TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::Ios => {
                        TextSelectionToolbarTextButton::text(on_pressed, label, index, total).into()
                    }
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
                TargetPlatform::Linux | TargetPlatform::Windows => {
                    DesktopTextSelectionToolbar::new(anchor, children).into()
                }
                TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::Ios => {
                    let below = value.anchors.secondary_anchor.unwrap_or(anchor);
                    TextSelectionToolbar::new(anchor, below, children).into()
                }
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
