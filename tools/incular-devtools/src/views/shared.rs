use incular::controls::TextFieldStyle;
use incular::material::RawMaterialButton;
use incular::prelude::*;

pub use super::theme::{
    accent_text, background, border, control, control_active, control_focus, control_hover, danger,
    danger_hover, danger_wash, focus_ring, gpu_color, hover_overlay, on_accent, pressed_overlay,
    primary, primary_hover, success, surface, surface_raised, text_disabled, text_muted,
    text_primary, violet, warning,
};

pub fn text_style(size: f32, color: Color) -> TextStyle {
    TextStyle::new()
        .font_family("Segoe UI")
        .font_size(size)
        .color(color)
}

pub fn ui_text(value: impl Into<String>, size: f32, color: Color) -> Widget {
    Text::new(value).style(text_style(size, color)).into()
}

pub(crate) fn heading(value: impl Into<String>, size: f32) -> Widget {
    Text::new(value)
        .style(text_style(size, text_primary()).font_weight(FontWeight::W600))
        .into()
}

pub(crate) fn mono(value: impl Into<String>, size: f32, color: Color) -> Widget {
    Text::new(value)
        .style(text_style(size, color).font_family("Consolas"))
        .into()
}

pub fn gap(width: f32, height: f32) -> Widget {
    Widget::box_(Size::new(width, height), Color::TRANSPARENT)
}

pub(crate) fn column(children: impl IntoIterator<Item = Widget>) -> Widget {
    Column::new(children)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .main_axis_size(MainAxisSize::Min)
        .into()
}

pub fn compact_button(
    label: impl Into<String>,
    selected: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    state_surface(
        RawMaterialButton::new(label)
            .size(Size::new(0., 30.))
            .padding(EdgeInsets::symmetric(10., 6.))
            .label_style(text_style(
                12.,
                if selected { primary() } else { text_primary() },
            ))
            .on_press(callback),
        if selected {
            control_active()
        } else {
            control()
        },
        4.,
    )
}

/// Paints `fill` under a transparent action surface.
///
/// Hover and pressed states are translucent overlays, so they read on any
/// fill in both themes (subtle when hovered, darker when pressed). A
/// transparent action surface also draws the keyboard-focus ring, which a
/// filled one would not. Every interactive element goes through here or
/// [`state_colors`], never a literal colour.
pub(crate) fn state_surface(button: RawMaterialButton, fill: Color, radius: f32) -> Widget {
    let button: Widget = state_colors(button).into();
    if fill.alpha == 0 {
        button
    } else {
        DecoratedBox::new(button)
            .background(fill)
            .radius(radius)
            .into()
    }
}

/// The shared hover, pressed, focus and disabled colours of a button.
pub(crate) fn state_colors(button: RawMaterialButton) -> RawMaterialButton {
    button
        .color(Color::TRANSPARENT)
        .hover_color(hover_overlay())
        .pressed_color(pressed_overlay())
        .focused_color(focus_ring())
        .disabled_color(Color::TRANSPARENT)
}

pub(crate) fn badge(label: impl Into<String>, color: Color) -> Widget {
    DecoratedBox::new(Padding::new(
        EdgeInsets::symmetric(8., 4.),
        ui_text(label, 11., color),
    ))
    .background(control())
    .radius(4.)
    .into()
}

pub(crate) fn page_title(title: &str, description: &str) -> Widget {
    column([
        heading(title, 25.),
        gap(1., 6.),
        ui_text(description, 13., text_muted()),
        gap(1., 22.),
    ])
}

pub(crate) fn input_style() -> TextFieldStyle {
    TextFieldStyle {
        background: Some(background()),
        foreground: Some(text_primary()),
        placeholder_color: Some(text_muted()),
        border: Some(Border::new(1., border())),
        border_focused: Some(Border::new(1., primary())),
        border_radius: Some(3.),
        padding: Some(EdgeInsets::symmetric(9., 5.)),
    }
}

pub(crate) fn quiet_button(
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    let (button, fill) = quiet_raw(label, selected, enabled, callback);
    state_surface(button, fill, 4.)
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
    let (button, fill) = quiet_raw(label, selected, enabled, callback);
    state_surface(button.on_hover(enter).on_exit(exit), fill, 4.)
}

fn quiet_raw(
    label: impl Into<String>,
    selected: bool,
    enabled: bool,
    callback: impl Fn() + 'static,
) -> (RawMaterialButton, Color) {
    let button = RawMaterialButton::new(label)
        .size(Size::new(0., 26.))
        .padding(EdgeInsets::symmetric(8., 4.))
        .label_style(text_style(
            11.,
            if selected {
                primary()
            } else if enabled {
                text_muted()
            } else {
                text_disabled()
            },
        ))
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

pub(crate) fn workspace_tab(
    label: impl Into<String>,
    selected: bool,
    small: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    let label = label.into();
    let width = label.chars().count() as f32 * if small { 5.8 } else { 6.5 }
        + if small { 16. } else { 24. };
    let height = if small { 34. } else { 38. };
    let button = RawMaterialButton::new(label.clone())
        .size(Size::new(width, if small { 34. } else { 38. }))
        .content(Column::new([
            Expanded::new(Align::new(
                Alignment::CENTER,
                Text::new(label).style(text_style(
                    if small { 11. } else { 12. },
                    if selected {
                        text_primary()
                    } else {
                        text_muted()
                    },
                )),
            ))
            .into(),
            Widget::box_(
                Size::new(width, 2.),
                if selected {
                    primary()
                } else {
                    Color::TRANSPARENT
                },
            ),
        ]))
        .on_press(callback);
    SizedBox::from_size(Size::new(width, height))
        .child(state_surface(button, Color::TRANSPARENT, 0.))
        .into()
}

pub(crate) fn panel(child: Widget) -> Widget {
    Container::with_child(child)
        .alignment(Alignment::TOP_LEFT)
        .color(surface())
        .border(Border::new(1., border()))
        .radius(7.)
        .into()
}

pub fn section(title: impl Into<String>, description: impl Into<String>, child: Widget) -> Widget {
    panel(
        Padding::all(
            16.,
            column([
                heading(title, 14.),
                gap(1., 4.),
                ui_text(description, 12., text_muted()),
                gap(1., 16.),
                child,
            ]),
        )
        .into(),
    )
}

pub(crate) fn metric(label: &str, value: impl Into<String>, detail: &str, color: Color) -> Widget {
    panel(
        Padding::all(
            16.,
            column([
                ui_text(label, 12., text_muted()),
                gap(1., 8.),
                Text::new(value)
                    .style(text_style(27., color).font_weight(FontWeight::W600))
                    .into(),
                gap(1., 6.),
                ui_text(detail, 11., text_muted()),
            ]),
        )
        .into(),
    )
}

pub(crate) fn key_value(label: impl Into<String>, value: impl Into<String>) -> Widget {
    let label = label.into();
    let value = value.into();
    LayoutBuilder::new(move |_, constraints| {
        Padding::new(
            EdgeInsets::symmetric(0., 5.),
            Row::new([
                Widget::from(
                    SizedBox::new()
                        .width((constraints.max_width() * 0.32).clamp(104., 160.))
                        .child(
                            Text::new(label.clone())
                                .style(text_style(12., text_muted()))
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

/// Joined button group where exactly one option is highlighted. Each entry is
/// `(label, selected, on_press)`; labels double as the semantic labels.
pub(crate) fn segmented_control(
    options: impl IntoIterator<Item = (String, bool, Box<dyn Fn()>)>,
) -> Widget {
    let mut children = Vec::new();
    for (index, (label, selected, callback)) in options.into_iter().enumerate() {
        if index > 0 {
            children.push(gap(2., 1.));
        }
        children.push(state_surface(
            RawMaterialButton::new(label)
                .size(Size::new(0., 28.))
                .padding(EdgeInsets::symmetric(14., 5.))
                .label_style(text_style(
                    12.,
                    if selected { primary() } else { text_muted() },
                ))
                .on_press(callback),
            if selected {
                control_active()
            } else {
                Color::TRANSPARENT
            },
            5.,
        ));
    }
    DecoratedBox::new(Padding::all(
        2.,
        Row::new(children).main_axis_size(MainAxisSize::Min),
    ))
    .background(background())
    .border(Border::new(1., border()))
    .radius(7.)
    .into()
}

/// Small filled square followed by a caption, for chart legends.
pub(crate) fn legend_swatch(color: Color, label: &str) -> Widget {
    Row::new([
        Widget::box_(Size::new(10., 10.), color),
        gap(6., 1.),
        ui_text(label, 11., text_muted()),
    ])
    .main_axis_size(MainAxisSize::Min)
    .into()
}

/// The target-picker button. While picking it turns red and reads
/// "Picking… (Esc)", because the target swallows every click until the pick
/// ends and the user must always be able to see - and cancel - that state.
pub(crate) fn picker_button(picking: bool, enabled: bool, callback: impl Fn() + 'static) -> Widget {
    state_surface(
        RawMaterialButton::new(if picking {
            "Picking… (Esc)"
        } else {
            "Pick widget"
        })
        .size(Size::new(0., 28.))
        .padding(EdgeInsets::symmetric(10., 5.))
        .label_style(
            text_style(
                12.,
                if enabled {
                    on_accent()
                } else {
                    text_disabled()
                },
            )
            .font_weight(FontWeight::W600),
        )
        .enabled(enabled)
        .on_press(callback),
        if !enabled {
            control()
        } else if picking {
            danger()
        } else {
            primary()
        },
        4.,
    )
}

/// A square check indicator followed by a caption; the whole chip is the
/// click target.
pub(crate) fn check_chip(
    label: impl Into<String>,
    on: bool,
    callback: impl Fn() + 'static,
) -> Widget {
    let label = label.into();
    let button = RawMaterialButton::new(label.clone())
        .size(Size::new(0., 28.))
        .padding(EdgeInsets::symmetric(8., 4.))
        .content(Padding::new(
            EdgeInsets::symmetric(8., 0.),
            Row::new([
                Widget::from(
                    Container::new()
                        .width(11.)
                        .height(11.)
                        .color(if on { primary() } else { Color::TRANSPARENT })
                        .border(Border::new(1., if on { primary() } else { text_muted() }))
                        .radius(2.),
                ),
                gap(7., 1.),
                ui_text(label, 11., if on { text_primary() } else { text_muted() }),
            ])
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Center),
        ))
        .on_press(callback);
    state_surface(button, Color::TRANSPARENT, 4.)
}
