use super::shared::{badge, column, input_style, page_title, panel, refresh};
use super::{compact_button, danger, gap, primary, text_muted, text_primary, ui_text};
use crate::inspector::{ConsoleEntry, ConsoleLevel, Shared};
use incular::controls::TextField;
use incular::prelude::*;
use incular::widgets::internal::TextEditingController;

pub(crate) fn build_console(
    entries: Vec<ConsoleEntry>,
    shared: Shared,
    tick: Signal<u64>,
    filter: TextEditingController,
) -> Widget {
    let (paused, level, total) = shared
        .lock()
        .map(|state| {
            (
                state.console_frozen.is_some(),
                state.console_level,
                state
                    .console_frozen
                    .as_ref()
                    .map_or(state.console.len(), Vec::len),
            )
        })
        .unwrap_or_default();
    let filtered = entries.len();
    let filter_shared = shared.clone();
    let filter_tick = tick.clone();
    let filter_field: Widget = TextField::new(filter)
        .size(Size::new(300., 34.))
        .style(input_style())
        .placeholder("Search messages or targets…")
        .on_changed(move |query| {
            if let Ok(mut state) = filter_shared.lock() {
                state.set_console_query(query);
            }
            refresh(&filter_tick);
        })
        .into();
    let mut levels = Vec::new();
    for option in ConsoleLevel::ALL {
        let level_shared = shared.clone();
        let level_tick = tick.clone();
        levels.push(compact_button(option.label(), level == option, move || {
            if let Ok(mut state) = level_shared.lock() {
                state.set_console_level(option);
            }
            refresh(&level_tick);
        }));
    }
    let pause_shared = shared.clone();
    let pause_tick = tick.clone();
    let pause = compact_button(
        if paused {
            "Resume live"
        } else {
            "Pause display"
        },
        paused,
        move || {
            if let Ok(mut state) = pause_shared.lock() {
                state.set_console_paused(!paused);
            }
            refresh(&pause_tick);
        },
    );
    let clear_tick = tick;
    let clear = compact_button("Clear", false, move || {
        if let Ok(mut state) = shared.lock() {
            state.clear_console();
        }
        refresh(&clear_tick);
    });
    let mut rows = vec![
        Container::with_child(Padding::all(
            12.,
            Row::new([
                SizedBox::from_size(Size::new(82., 18.))
                    .child(ui_text("LEVEL", 10., text_muted()))
                    .into(),
                SizedBox::from_size(Size::new(170., 18.))
                    .child(ui_text("TARGET", 10., text_muted()))
                    .into(),
                ui_text("MESSAGE", 10., text_muted()),
            ]),
        ))
        .alignment(Alignment::TOP_LEFT)
        .color(super::shared::surface_raised())
        .into(),
    ];
    for (index, entry) in entries.into_iter().enumerate() {
        let color = match entry.level.to_ascii_lowercase().as_str() {
            "error" | "fatal" => danger(),
            "warn" | "warning" => super::shared::warning(),
            "debug" | "trace" => text_muted(),
            _ => primary(),
        };
        rows.push(
            Container::with_child(Padding::all(
                12.,
                Row::new([
                    Widget::from(SizedBox::from_size(Size::new(82., 24.)).child(Align::new(
                        Alignment::CENTER_LEFT,
                        badge(entry.level.to_uppercase(), color),
                    ))),
                    SizedBox::from_size(Size::new(170., 24.))
                        .child(ui_text(entry.target, 11., text_muted()))
                        .into(),
                    Expanded::new(super::shared::mono(entry.message, 12., text_primary())).into(),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Start),
            ))
            .alignment(Alignment::TOP_LEFT)
            .color(if index % 2 == 0 {
                super::surface()
            } else {
                super::background()
            })
            .into(),
        );
    }
    if filtered == 0 {
        rows.push(
            Padding::all(
                24.,
                column([
                    super::shared::heading(
                        if total > 0 {
                            "No matching messages"
                        } else {
                            "No messages yet"
                        },
                        16.,
                    ),
                    gap(1., 8.),
                    ui_text(
                        if total > 0 {
                            "Try another search or select All levels."
                        } else {
                            "Structured target logs and connection diagnostics will appear here."
                        },
                        12.,
                        text_muted(),
                    ),
                ]),
            )
            .into(),
        );
    }
    column([
        page_title(
            "Console",
            "Find the signal in your application's diagnostic output.",
        ),
        Row::new([
            filter_field,
            Expanded::new(gap(1., 1.)).into(),
            pause,
            gap(8., 1.),
            clear,
        ])
        .into(),
        gap(1., 12.),
        Wrap::new(levels).spacing(6.).run_spacing(6.).into(),
        gap(1., 16.),
        Row::new([
            ui_text(format!("{filtered} of {total} messages"), 12., text_muted()),
            gap(12., 1.),
            badge(
                if paused {
                    "DISPLAY PAUSED · capture continues"
                } else {
                    "LIVE"
                },
                if paused {
                    super::shared::warning()
                } else {
                    primary()
                },
            ),
        ])
        .into(),
        gap(1., 12.),
        panel(column(rows)),
    ])
}
