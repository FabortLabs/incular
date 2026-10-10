use incular::controls::TextFieldStyle;
use incular::material::RawMaterialButton;
use incular::prelude::*;

pub const APP_BACKGROUND: Color = Color::rgba(15, 17, 22, 255);
pub const SURFACE: Color = Color::rgba(20, 23, 30, 255);
pub const SURFACE_RAISED: Color = Color::rgba(25, 29, 37, 255);
pub const BORDER: Color = Color::rgba(43, 48, 59, 255);
pub const CONTROL: Color = Color::rgba(32, 37, 47, 255);
pub const CONTROL_ACTIVE: Color = Color::rgba(34, 57, 55, 255);
pub const PRIMARY: Color = Color::rgba(110, 222, 186, 255);
pub const DANGER: Color = Color::rgba(244, 126, 139, 255);
pub const TEXT_PRIMARY: Color = Color::rgba(230, 233, 239, 255);
pub const TEXT_MUTED: Color = Color::rgba(143, 152, 169, 255);
pub const SUCCESS: Color = PRIMARY;
pub const WARNING: Color = Color::rgba(232, 186, 113, 255);
pub const VIOLET: Color = Color::rgba(168, 151, 236, 255);

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
        .style(text_style(size, TEXT_PRIMARY).font_weight(FontWeight::W600))
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
    RawMaterialButton::new(label)
        .size(Size::new(0., 30.))
        .padding(EdgeInsets::symmetric(10., 6.))
        .label_style(text_style(
            12.,
            if selected { PRIMARY } else { TEXT_PRIMARY },
        ))
        .color(if selected { CONTROL_ACTIVE } else { CONTROL })
        .hover_color(Color::rgba(45, 53, 65, 255))
        .focused_color(Color::rgba(48, 65, 69, 255))
        .on_press(callback)
        .into()
}

pub(crate) fn badge(label: impl Into<String>, color: Color) -> Widget {
    DecoratedBox::new(Padding::new(
        EdgeInsets::symmetric(8., 4.),
        ui_text(label, 11., color),
    ))
    .background(CONTROL)
    .radius(4.)
    .into()
}

pub(crate) fn page_title(title: &str, description: &str) -> Widget {
    column([
        heading(title, 25.),
        gap(1., 6.),
        ui_text(description, 13., TEXT_MUTED),
        gap(1., 22.),
    ])
}

pub(crate) fn input_style() -> TextFieldStyle {
    TextFieldStyle {
        background: Some(APP_BACKGROUND),
        foreground: Some(TEXT_PRIMARY),
        placeholder_color: Some(TEXT_MUTED),
        border: Some(Border::new(1., BORDER)),
        border_focused: Some(Border::new(1., PRIMARY)),
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
    RawMaterialButton::new(label)
        .size(Size::new(0., 26.))
        .padding(EdgeInsets::symmetric(8., 4.))
        .label_style(text_style(
            11.,
            if selected {
                PRIMARY
            } else if enabled {
                TEXT_MUTED
            } else {
                BORDER
            },
        ))
        .color(if selected {
            CONTROL_ACTIVE
        } else {
            Color::TRANSPARENT
        })
        .hover_color(CONTROL)
        .focused_color(CONTROL_ACTIVE)
        .disabled_color(Color::TRANSPARENT)
        .enabled(enabled)
        .on_press(callback)
        .into()
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
    let button: Widget = RawMaterialButton::new(label.clone())
        .size(Size::new(width, if small { 34. } else { 38. }))
        .content(Column::new([
            Expanded::new(Align::new(
                Alignment::CENTER,
                Text::new(label).style(text_style(
                    if small { 11. } else { 12. },
                    if selected { TEXT_PRIMARY } else { TEXT_MUTED },
                )),
            ))
            .into(),
            Widget::box_(
                Size::new(width, 2.),
                if selected {
                    PRIMARY
                } else {
                    Color::TRANSPARENT
                },
            ),
        ]))
        .color(Color::TRANSPARENT)
        .hover_color(SURFACE_RAISED)
        .focused_color(CONTROL_ACTIVE)
        .on_press(callback)
        .into();
    SizedBox::from_size(Size::new(width, height))
        .child(button)
        .into()
}

pub(crate) fn panel(child: Widget) -> Widget {
    Container::with_child(child)
        .alignment(Alignment::TOP_LEFT)
        .color(SURFACE)
        .border(Border::new(1., BORDER))
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
                ui_text(description, 12., TEXT_MUTED),
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
                ui_text(label, 12., TEXT_MUTED),
                gap(1., 8.),
                Text::new(value)
                    .style(text_style(27., color).font_weight(FontWeight::W600))
                    .into(),
                gap(1., 6.),
                ui_text(detail, 11., TEXT_MUTED),
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
                        .width((constraints.max_width() * 0.32).clamp(88., 154.))
                        .child(
                            Text::new(label.clone())
                                .style(text_style(12., TEXT_MUTED))
                                .max_lines(Some(1))
                                .overflow(TextOverflow::Ellipsis),
                        ),
                ),
                Expanded::new(mono(value.clone(), 12., TEXT_PRIMARY)).into(),
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
