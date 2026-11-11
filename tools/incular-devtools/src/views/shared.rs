//! The DevTools component library.
//!
//! The look follows shadcn/ui: soft cards that separate from the page by tone
//! (and, in the light theme, a faint shadow) rather than an outline, 8 px
//! rounded controls, pill badges tinted with their accent, filled inputs and
//! segmented tab lists. Every colour comes from the active palette.

use super::icons::{IconName, icon};
use incular::controls::TextFieldStyle;
use incular::material::RawMaterialButton;
use incular::prelude::*;

pub use super::theme::{
    accent_text, background, border, card_shadow, chrome, control, control_active, control_focus,
    control_hover, danger, danger_hover, danger_wash, focus_ring, gpu_color, hover_overlay, info,
    on_accent, overlay_shadow, pressed_overlay, primary, primary_hover, scrim, success, surface,
    surface_raised, text_disabled, text_muted, text_primary, tint, violet, warning,
};

/// Corner radius of cards and floating panels.
pub const CARD_RADIUS: f32 = 12.;
/// Corner radius of buttons, inputs and tab lists.
pub const CONTROL_RADIUS: f32 = 8.;
/// Height of a standard button.
pub const CONTROL_HEIGHT: f32 = 32.;

pub fn text_style(size: f32, color: Color) -> TextStyle {
    TextStyle::new()
        .font_family("Segoe UI")
        .font_size(size)
        .color(color)
}

pub fn ui_text(value: impl Into<String>, size: f32, color: Color) -> Widget {
    Text::new(value).style(text_style(size, color)).into()
}

/// Single-line text that ellipsises instead of overflowing.
pub(crate) fn line_text(value: impl Into<String>, size: f32, color: Color) -> Widget {
    Text::new(value)
        .style(text_style(size, color))
        .max_lines(Some(1))
        .overflow(TextOverflow::Ellipsis)
        .into()
}

pub(crate) fn strong(value: impl Into<String>, size: f32, color: Color) -> Widget {
    Text::new(value)
        .style(text_style(size, color).font_weight(FontWeight::W600))
        .max_lines(Some(1))
        .overflow(TextOverflow::Ellipsis)
        .into()
}

pub(crate) fn heading(value: impl Into<String>, size: f32) -> Widget {
    Text::new(value)
        .style(text_style(size, text_primary()).font_weight(FontWeight::W600))
        .into()
}

pub(crate) fn mono_style(size: f32, color: Color) -> TextStyle {
    text_style(size, color).font_family("Consolas")
}

pub(crate) fn mono(value: impl Into<String>, size: f32, color: Color) -> Widget {
    Text::new(value).style(mono_style(size, color)).into()
}

/// Uppercase, letter-spaced caption for table headers and group labels.
pub(crate) fn caption(value: impl Into<String>) -> Widget {
    Text::new(value.into().to_uppercase())
        .style(
            text_style(10., text_muted())
                .font_weight(FontWeight::W600)
                .letter_spacing(0.6),
        )
        .max_lines(Some(1))
        .overflow(TextOverflow::Ellipsis)
        .into()
}

pub fn gap(width: f32, height: f32) -> Widget {
    Widget::box_(Size::new(width, height), Color::TRANSPARENT)
}

/// Takes the remaining main-axis space in a row or column.
pub(crate) fn spacer() -> Widget {
    Expanded::new(gap(1., 1.)).into()
}

pub(crate) fn column(children: impl IntoIterator<Item = Widget>) -> Widget {
    Column::new(children)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_size(MainAxisSize::Min)
        .into()
}

/// A row whose children are vertically centred and which hugs its content.
pub(crate) fn hrow(children: impl IntoIterator<Item = Widget>) -> Widget {
    Row::new(children)
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .into()
}

/// A full-width row whose children are vertically centred.
pub(crate) fn wide_row(children: impl IntoIterator<Item = Widget>) -> Widget {
    Row::new(children)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .into()
}

/// A hairline separator spanning the available width.
pub(crate) fn divider() -> Widget {
    LayoutBuilder::new(|_, constraints| {
        Widget::box_(
            Size::new(constraints.max_width().clamp(1., 8000.), 1.),
            border(),
        )
    })
    .into()
}

/// A filled circle, for status and legend markers.
pub(crate) fn dot(color: Color, size: f32) -> Widget {
    Container::new()
        .width(size)
        .height(size)
        .color(color)
        .radius(size / 2.)
        .into()
}

/// The decorative icon of a control.
pub(crate) fn glyph(name: IconName, size: f32, color: Color) -> Widget {
    icon(name, size, color)
}

/// Paints `fill` under a transparent action surface, rounded to `radius`.
///
/// Hover and pressed states are translucent overlays, so they read on any
/// fill in both themes. The clip rounds the overlay with the fill. A
/// transparent action surface also draws the keyboard-focus ring. Every
/// interactive element goes through here or [`state_colors`], never a
/// literal colour.
pub(crate) fn state_surface(button: RawMaterialButton, fill: Color, radius: f32) -> Widget {
    let button: Widget = state_colors(button).into();
    let filled: Widget = if fill.alpha == 0 {
        button
    } else {
        DecoratedBox::new(button)
            .background(fill)
            .radius(radius)
            .into()
    };
    if radius > 0. {
        ClipRRect::new(CornerRadii::uniform(radius), filled).into()
    } else {
        filled
    }
}

/// The shared hover, pressed, focus, disabled colours and pointer cursor of a
/// button.
pub(crate) fn state_colors(button: RawMaterialButton) -> RawMaterialButton {
    button
        .color(Color::TRANSPARENT)
        .hover_color(hover_overlay())
        .pressed_color(pressed_overlay())
        .focused_color(focus_ring())
        .disabled_color(Color::TRANSPARENT)
        .mouse_cursor(MouseCursor::Click)
}

/// Button content padded horizontally and centred vertically in `height`,
/// hugging its own width. An action surface with custom content neither pads
/// it nor bounds it, so every button lays its content out through here.
pub(crate) fn button_content(child: Widget, height: f32, horizontal: f32) -> Widget {
    SizedBox::new()
        .height(height)
        .child(
            Align::new(
                Alignment::CENTER,
                Padding::new(EdgeInsets::symmetric(horizontal, 0.), child),
            )
            .width_factor(1.),
        )
        .into()
}

/// Visual weight of a button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Variant {
    /// Filled with the brand accent; one per region.
    Primary,
    /// Neutral filled control (the default).
    Secondary,
    /// No fill until hovered.
    Ghost,
    /// Destructive or "stop" action.
    Danger,
    /// Toggled on: accent text on an accent-tinted fill.
    Active,
}

impl Variant {
    fn fill(self) -> Color {
        match self {
            Self::Primary => primary(),
            Self::Secondary => control(),
            Self::Ghost => Color::TRANSPARENT,
            Self::Danger => danger(),
            Self::Active => control_active(),
        }
    }

    fn foreground(self, enabled: bool) -> Color {
        if !enabled {
            return text_disabled();
        }
        match self {
            Self::Primary | Self::Danger => on_accent(),
            Self::Secondary => text_primary(),
            Self::Ghost => text_muted(),
            Self::Active => primary(),
        }
    }
}

/// The general button: optional leading icon, a label (also the accessible
/// name) and a [`Variant`].
pub(crate) fn button(
    label: impl Into<String>,
    leading: Option<IconName>,
    variant: Variant,
    enabled: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    labelled_button(label, None, leading, variant, enabled, callback)
}

/// A [`button`] whose accessible name differs from its visible text.
pub(crate) fn labelled_button(
    label: impl Into<String>,
    visible: Option<String>,
    leading: Option<IconName>,
    variant: Variant,
    enabled: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    let label = label.into();
    let foreground = variant.foreground(enabled);
    let weight = if matches!(variant, Variant::Primary | Variant::Danger) {
        FontWeight::W600
    } else {
        FontWeight::W500
    };
    let mut children = Vec::new();
    if let Some(name) = leading {
        children.push(glyph(name, 14., foreground));
        children.push(gap(7., 1.));
    }
    children.push(
        Text::new(visible.unwrap_or_else(|| label.clone()))
            .style(text_style(12.5, foreground).font_weight(weight))
            .max_lines(Some(1))
            .into(),
    );
    let fill = if enabled || variant == Variant::Ghost {
        variant.fill()
    } else {
        control()
    };
    state_surface(
        RawMaterialButton::new(label)
            .size(Size::new(0., CONTROL_HEIGHT))
            .content(button_content(hrow(children), CONTROL_HEIGHT, 12.))
            .enabled(enabled)
            .on_press(callback),
        fill,
        CONTROL_RADIUS,
    )
}

/// Square icon-only button; `label` is its accessible name and tooltip.
pub(crate) fn icon_button(
    name: IconName,
    label: impl Into<String>,
    active: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    let label = label.into();
    state_surface(
        RawMaterialButton::new(label)
            .size(Size::new(CONTROL_HEIGHT, CONTROL_HEIGHT))
            .content(Align::new(
                Alignment::CENTER,
                glyph(name, 16., if active { primary() } else { text_muted() }),
            ))
            .on_press(callback),
        if active {
            control_active()
        } else {
            Color::TRANSPARENT
        },
        CONTROL_RADIUS,
    )
}

/// Secondary button; `selected` renders it as toggled on.
pub fn compact_button(
    label: impl Into<String>,
    selected: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    button(
        label,
        None,
        if selected {
            Variant::Active
        } else {
            Variant::Secondary
        },
        true,
        callback,
    )
}

/// Filled accent button for the main action of a region.
pub(crate) fn primary_button(
    label: impl Into<String>,
    leading: Option<IconName>,
    callback: impl Fn() + 'static,
) -> Widget {
    button(label, leading, Variant::Primary, true, callback)
}

/// Pill badge tinted with its accent colour; neutral when `color` is muted.
pub(crate) fn badge(label: impl Into<String>, color: Color) -> Widget {
    let neutral = color == text_muted();
    DecoratedBox::new(Padding::new(
        EdgeInsets::symmetric(9., 3.),
        Text::new(label)
            .style(text_style(11., color).font_weight(FontWeight::W500))
            .max_lines(Some(1)),
    ))
    .background(if neutral { control() } else { tint(color, 30) })
    .radius(999.)
    .into()
}

/// A badge with a leading status dot.
pub(crate) fn status_badge(label: impl Into<String>, color: Color) -> Widget {
    DecoratedBox::new(Padding::new(
        EdgeInsets::symmetric(9., 3.),
        hrow([
            dot(color, 6.),
            gap(6., 1.),
            Text::new(label)
                .style(text_style(11., color).font_weight(FontWeight::W500))
                .max_lines(Some(1))
                .into(),
        ]),
    ))
    .background(tint(color, 28))
    .radius(999.)
    .into()
}

/// A count bubble for navigation items (errors, failures).
pub(crate) fn count_bubble(count: usize, color: Color) -> Widget {
    DecoratedBox::new(Padding::new(
        EdgeInsets::symmetric(6., 1.),
        Text::new(if count > 99 {
            "99+".to_owned()
        } else {
            count.to_string()
        })
        .style(text_style(10., color).font_weight(FontWeight::W600)),
    ))
    .background(tint(color, 34))
    .radius(999.)
    .into()
}

/// A keyboard key cap, e.g. `Ctrl`, `K`.
pub(crate) fn kbd(key: impl Into<String>) -> Widget {
    DecoratedBox::new(Padding::new(
        EdgeInsets::symmetric(6., 2.),
        Text::new(key).style(mono_style(10.5, text_muted())),
    ))
    .background(surface_raised())
    .radius(5.)
    .into()
}

/// A chord such as `Ctrl+Shift+C`, drawn as key caps.
pub(crate) fn shortcut_keys(chord: &str) -> Widget {
    let mut keys = Vec::new();
    for (index, key) in chord.split('+').enumerate() {
        if index > 0 {
            keys.push(gap(3., 1.));
        }
        keys.push(kbd(key.trim()));
    }
    hrow(keys)
}

pub(crate) fn input_style() -> TextFieldStyle {
    TextFieldStyle {
        background: Some(control()),
        foreground: Some(text_primary()),
        placeholder_color: Some(text_muted()),
        border: Some(Border::new(1., control())),
        border_focused: Some(Border::new(1., primary())),
        border_radius: Some(CONTROL_RADIUS),
        padding: Some(EdgeInsets::symmetric(10., 5.)),
    }
}

/// A search field with a leading magnifier: `field` is the text input.
pub(crate) fn search_box(field: Widget, height: f32) -> Widget {
    SizedBox::new()
        .height(height)
        .child(Stack::new([
            field,
            Positioned::new(glyph(IconName::Search, 14., text_muted()))
                .left(11.)
                .top((height - 14.) / 2.)
                .into(),
        ]))
        .into()
}

/// Padding for a [`search_box`] text field so text clears the icon.
pub(crate) fn search_input_style() -> TextFieldStyle {
    TextFieldStyle {
        padding: Some(EdgeInsets::only(32., 6., 10., 6.)),
        ..input_style()
    }
}

pub(crate) fn quiet_button(
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    let (button, fill) = quiet_raw(label, None, selected, enabled, callback);
    state_surface(button, fill, 6.)
}

/// A [`quiet_button`] with a leading icon.
pub(crate) fn quiet_icon_button(
    label: impl Into<String>,
    leading: IconName,
    selected: bool,
    enabled: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    let (button, fill) = quiet_raw(label, Some(leading), selected, enabled, callback);
    state_surface(button, fill, 6.)
}

/// A [`quiet_button`] that also reports the pointer entering and leaving it,
/// for rows that preview their widget in the target window.
pub(crate) fn quiet_button_hover(
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    callback: impl Fn() + 'static,
    enter: impl Fn() + 'static,
    exit: impl Fn() + 'static,
) -> Widget {
    let (button, fill) = quiet_raw(label, None, selected, enabled, callback);
    state_surface(button.on_hover(enter).on_exit(exit), fill, 6.)
}

fn quiet_raw(
    label: impl Into<String>,
    leading: Option<IconName>,
    selected: bool,
    enabled: bool,
    callback: impl Fn() + 'static,
) -> (RawMaterialButton, Color) {
    let label = label.into();
    let color = if selected {
        primary()
    } else if enabled {
        text_muted()
    } else {
        text_disabled()
    };
    let mut children = Vec::new();
    if let Some(name) = leading {
        children.push(glyph(name, 13., color));
        children.push(gap(6., 1.));
    }
    children.push(
        Text::new(label.clone())
            .style(text_style(12., color).font_weight(FontWeight::W500))
            .max_lines(Some(1))
            .into(),
    );
    let button = RawMaterialButton::new(label)
        .size(Size::new(0., 28.))
        .content(button_content(hrow(children), 28., 9.))
        .enabled(enabled)
        .on_press(callback);
    (
        button,
        if selected {
            control_active()
        } else {
            Color::TRANSPARENT
        },
    )
}

/// One tab: `(label, selected, on_press)`.
pub(crate) type TabOption = (String, bool, Box<dyn Fn()>);

/// A segmented tab list (shadcn `Tabs`): a raised well whose selected tab is
/// a lifted card-coloured pill. Each entry is `(label, selected, on_press)`;
/// labels double as the accessible names.
pub(crate) fn tab_list(options: impl IntoIterator<Item = TabOption>) -> Widget {
    let mut children = Vec::new();
    for (index, (label, selected, callback)) in options.into_iter().enumerate() {
        if index > 0 {
            children.push(gap(2., 1.));
        }
        let button = RawMaterialButton::new(label.clone())
            .size(Size::new(0., 28.))
            .content(button_content(
                Text::new(label)
                    .style(
                        text_style(
                            12.,
                            if selected {
                                text_primary()
                            } else {
                                text_muted()
                            },
                        )
                        .font_weight(FontWeight::W500),
                    )
                    .max_lines(Some(1))
                    .into(),
                28.,
                12.,
            ))
            .on_press(callback);
        children.push(if selected {
            Container::with_child(state_surface(button, surface(), 6.))
                .decoration(
                    BoxDecoration::new()
                        .color(surface())
                        .border_radius(BorderRadius::circular(6.))
                        .box_shadow([BoxShadow::new(
                            overlay_shadow().with_alpha_scaled(0.35),
                            Offset::new(0., 1.),
                            2.,
                            0.,
                        )]),
                )
                .into()
        } else {
            state_surface(button, Color::TRANSPARENT, 6.)
        });
    }
    DecoratedBox::new(Padding::all(3., hrow(children)))
        .background(surface_raised())
        .radius(CONTROL_RADIUS + 1.)
        .into()
}

/// Joined button group where exactly one option is highlighted; the same look
/// as [`tab_list`].
pub(crate) fn segmented_control(
    options: impl IntoIterator<Item = (String, bool, Box<dyn Fn()>)>,
) -> Widget {
    tab_list(options)
}

/// A card: the base surface for grouped content. It separates from the page
/// by tone and a faint shadow, never an outline.
pub(crate) fn card(child: Widget) -> Widget {
    card_filled(child, surface())
}

pub(crate) fn card_filled(child: Widget, fill: Color) -> Widget {
    let shadow = card_shadow();
    let mut decoration = BoxDecoration::new()
        .color(fill)
        .border_radius(BorderRadius::circular(CARD_RADIUS));
    if shadow.alpha > 0 {
        decoration = decoration.box_shadow([
            BoxShadow::new(shadow, Offset::new(0., 1.), 3., 0.),
            BoxShadow::new(tint(shadow, shadow.alpha / 2), Offset::new(0., 1.), 2., -1.),
        ]);
    }
    Container::with_child(child)
        .alignment(Alignment::TOP_LEFT)
        .decoration(decoration)
        .into()
}

/// A floating panel (menu, palette, dialog) with a deeper shadow.
pub(crate) fn floating(child: Widget) -> Widget {
    Container::with_child(ClipRRect::new(CornerRadii::uniform(CARD_RADIUS), child))
        .alignment(Alignment::TOP_LEFT)
        .decoration(
            BoxDecoration::new()
                .color(surface())
                .border_radius(BorderRadius::circular(CARD_RADIUS))
                .box_shadow([
                    BoxShadow::new(overlay_shadow(), Offset::new(0., 12.), 32., -4.),
                    BoxShadow::new(
                        overlay_shadow().with_alpha_scaled(0.5),
                        Offset::new(0., 2.),
                        6.,
                        0.,
                    ),
                ]),
        )
        .into()
}

pub(crate) fn panel(child: Widget) -> Widget {
    card(child)
}

/// A recessed well inside a card (code, nested lists, empty states).
pub(crate) fn well(child: Widget) -> Widget {
    Container::with_child(child)
        .alignment(Alignment::TOP_LEFT)
        .color(surface_raised())
        .radius(CONTROL_RADIUS + 2.)
        .into()
}

/// A card's title row: title, optional description, optional trailing action.
pub(crate) fn card_header(title: &str, description: &str, action: Option<Widget>) -> Widget {
    let mut text = vec![heading(title, 14.)];
    if !description.is_empty() {
        text.push(gap(1., 4.));
        text.push(ui_text(description, 12., text_muted()));
    }
    let mut row = vec![Expanded::new(column(text)).into()];
    if let Some(action) = action {
        row.push(gap(12., 1.));
        row.push(action);
    }
    Row::new(row)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .into()
}

pub fn section(title: impl Into<String>, description: impl Into<String>, child: Widget) -> Widget {
    section_with(&title.into(), &description.into(), None, child)
}

/// A [`section`] with a trailing header action.
pub(crate) fn section_with(
    title: &str,
    description: &str,
    action: Option<Widget>,
    child: Widget,
) -> Widget {
    card(
        Padding::all(
            20.,
            Column::new([card_header(title, description, action), gap(1., 16.), child])
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .main_axis_size(MainAxisSize::Min),
        )
        .into(),
    )
}

/// A stat card: muted label with a status dot, a large value and a detail
/// line.
pub(crate) fn metric(label: &str, value: impl Into<String>, detail: &str, color: Color) -> Widget {
    stat_card(None, label, value.into(), detail, color, None)
}

/// A [`metric`] with an optional icon and an optional trailing chart.
pub(crate) fn stat_card(
    leading: Option<IconName>,
    label: &str,
    value: String,
    detail: &str,
    color: Color,
    chart: Option<Widget>,
) -> Widget {
    let label = sentence_case(label);
    let mut header = vec![Expanded::new(line_text(label, 12.5, text_muted())).into()];
    header.push(match leading {
        Some(name) => glyph(name, 15., text_muted()),
        None => dot(color, 7.),
    });
    let mut body = vec![
        Row::new(header)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .into(),
        gap(1., 10.),
        Text::new(value)
            .style(
                text_style(26., color)
                    .font_weight(FontWeight::W600)
                    .letter_spacing(-0.4),
            )
            .max_lines(Some(1))
            .into(),
        gap(1., 4.),
        line_text(detail, 11.5, text_muted()),
    ];
    if let Some(chart) = chart {
        body.push(gap(1., 12.));
        body.push(chart);
    }
    card(
        Padding::all(
            18.,
            Column::new(body)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .main_axis_size(MainAxisSize::Min),
        )
        .into(),
    )
}

/// `"AVERAGE CPU FRAME"` → `"Average CPU frame"`; words that are already
/// acronyms of two to four letters (CPU, RSS, FPS, P95) stay upper case.
pub(crate) fn sentence_case(label: &str) -> String {
    if label.chars().any(|c| c.is_lowercase()) {
        return label.to_owned();
    }
    label
        .split(' ')
        .enumerate()
        .map(|(index, word)| {
            let acronym = (2..=4).contains(&word.len())
                && matches!(word, "CPU" | "RSS" | "FPS" | "P95" | "GPU" | "A" | "B");
            if acronym || word.chars().all(|c| !c.is_alphabetic()) {
                word.to_owned()
            } else if index == 0 {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_string() + &chars.as_str().to_lowercase())
                    .unwrap_or_default()
            } else {
                word.to_lowercase()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn key_value(label: impl Into<String>, value: impl Into<String>) -> Widget {
    let label = label.into();
    let value = value.into();
    LayoutBuilder::new(move |_, constraints| {
        Padding::new(
            EdgeInsets::symmetric(0., 6.),
            Row::new([
                Widget::from(
                    SizedBox::new()
                        .width((constraints.max_width() * 0.34).clamp(104., 170.))
                        .child(
                            Text::new(label.clone())
                                .style(text_style(12.5, text_muted()))
                                .max_lines(Some(1))
                                .overflow(TextOverflow::Ellipsis),
                        ),
                ),
                Expanded::new(mono(value.clone(), 12., text_primary())).into(),
            ]),
        )
        .into()
    })
    .into()
}

pub(crate) fn refresh(tick: &Signal<u64>) {
    tick.update(|value| {
        *value = value
            .checked_add(1)
            .expect("DevTools UI revision exhausted")
    });
}

/// Small rounded square followed by a caption, for chart legends.
pub(crate) fn legend_swatch(color: Color, label: &str) -> Widget {
    hrow([
        Container::new()
            .width(10.)
            .height(10.)
            .color(color)
            .radius(3.)
            .into(),
        gap(6., 1.),
        ui_text(label, 11.5, text_muted()),
    ])
}

/// The target-picker button. While picking it turns red and reads
/// "Picking… (Esc)", because the target swallows every click until the pick
/// ends and the user must always be able to see - and cancel - that state.
pub(crate) fn picker_button(picking: bool, enabled: bool, callback: impl Fn() + 'static) -> Widget {
    button(
        if picking {
            "Picking… (Esc)"
        } else {
            "Pick widget"
        },
        Some(IconName::Crosshair),
        if picking {
            Variant::Danger
        } else {
            Variant::Primary
        },
        enabled,
        callback,
    )
}

/// A checkbox followed by a caption; the whole chip is the click target.
pub(crate) fn check_chip(
    label: impl Into<String>,
    on: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    let label = label.into();
    let indicator: Widget = Container::with_child(if on {
        Align::new(Alignment::CENTER, glyph(IconName::Check, 11., on_accent())).into()
    } else {
        gap(1., 1.)
    })
    .width(16.)
    .height(16.)
    .color(if on { primary() } else { Color::TRANSPARENT })
    .border(Border::new(
        1.5,
        if on { primary() } else { text_disabled() },
    ))
    .radius(4.)
    .into();
    let button = RawMaterialButton::new(label.clone())
        .size(Size::new(0., 30.))
        .content(button_content(
            hrow([
                indicator,
                gap(8., 1.),
                ui_text(label, 12., if on { text_primary() } else { text_muted() }),
            ]),
            30.,
            8.,
        ))
        .on_press(callback);
    state_surface(button, Color::TRANSPARENT, 6.)
}

/// A centred empty state: icon in a soft circle, title and guidance.
pub(crate) fn empty_state(name: IconName, title: &str, description: &str) -> Widget {
    Padding::new(
        EdgeInsets::symmetric(16., 28.),
        Align::new(
            Alignment::TOP_CENTER,
            Column::new([
                Container::with_child(Align::new(
                    Alignment::CENTER,
                    glyph(name, 20., text_muted()),
                ))
                .width(44.)
                .height(44.)
                .color(surface_raised())
                .radius(22.)
                .into(),
                gap(1., 14.),
                heading(title, 14.),
                gap(1., 6.),
                Text::new(description)
                    .style(text_style(12., text_muted()))
                    .align(TextAlign::Center)
                    .into(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .main_axis_size(MainAxisSize::Min),
        ),
    )
    .into()
}

/// A horizontal bar (0..=1) for inline proportions in tables.
pub(crate) fn meter(fraction: f32, color: Color, height: f32) -> Widget {
    let fraction = if fraction.is_finite() {
        fraction.clamp(0., 1.)
    } else {
        0.
    };
    LayoutBuilder::new(move |_, constraints| {
        let width = constraints.max_width().clamp(4., 4000.);
        let mut canvas = Canvas::default();
        canvas.rrect(
            RRect::uniform(
                incular::core::Rect::from_origin_size(Offset::ZERO, Size::new(width, height)),
                height / 2.,
            ),
            surface_raised(),
        );
        if fraction > 0. {
            canvas.rrect(
                RRect::uniform(
                    incular::core::Rect::from_origin_size(
                        Offset::ZERO,
                        Size::new((width * fraction).max(height), height),
                    ),
                    height / 2.,
                ),
                color,
            );
        }
        CustomPaint::new(Size::new(width, height), canvas.finish()).into()
    })
    .into()
}

/// A minimal sparkline of `values` (oldest first), `width` x `height`.
pub(crate) fn sparkline(values: &[f32], width: f32, height: f32, color: Color) -> Widget {
    let mut canvas = Canvas::default();
    let max = values.iter().copied().fold(0_f32, f32::max).max(1e-6);
    let count = values.len();
    if count >= 2 {
        let step = width / (count - 1) as f32;
        let mut builder = Path::builder();
        for (index, value) in values.iter().enumerate() {
            let point = Offset::new(
                index as f32 * step,
                height - 1. - (value / max).clamp(0., 1.) * (height - 2.),
            );
            if index == 0 {
                builder.move_to(point);
            } else {
                builder.line_to(point);
            }
        }
        let line = std::sync::Arc::new(builder.build());
        canvas.stroke_path(
            line,
            color,
            Stroke {
                width: 1.5,
                cap: LineCap::Round,
                join: LineJoin::Round,
                ..Stroke::default()
            },
        );
    } else {
        canvas.rect(
            incular::core::Rect::from_origin_size(
                Offset::new(0., height - 1.),
                Size::new(width, 1.),
            ),
            border(),
        );
    }
    CustomPaint::new(Size::new(width, height), canvas.finish()).into()
}

trait AlphaScale {
    fn with_alpha_scaled(self, factor: f32) -> Self;
}

impl AlphaScale for Color {
    fn with_alpha_scaled(self, factor: f32) -> Self {
        tint(self, (f32::from(self.alpha) * factor).round() as u8)
    }
}
