//! Console: target logs and DevTools diagnostics, with severity filters that
//! show their counts, text search, repeat grouping, pause and copy.

use super::clipboard::Clipboard;
use super::icons::IconName;
use super::shared::{
    Variant, button, caption, card, column, count_bubble, danger_wash, divider, empty_state, gap,
    hrow, info, line_text, mono, refresh, search_box, search_input_style, spacer, status_badge,
    tab_list, text_style, tint, warning, wide_row,
};
use super::{danger, text_muted, text_primary, ui_text};
use crate::inspector::{ConsoleCounts, ConsoleEntry, ConsoleLevel, Shared, session_start_ms};
use incular::controls::TextField;
use incular::prelude::*;
use incular::widgets::internal::TextEditingController;

/// Session-relative timestamp such as `+01:42.153`.
pub(crate) fn session_time(time_ms: u64) -> String {
    let elapsed = time_ms.saturating_sub(session_start_ms());
    let minutes = elapsed / 60_000;
    let seconds = (elapsed / 1000) % 60;
    let millis = elapsed % 1000;
    if minutes >= 60 {
        format!("+{}:{:02}:{:02}", minutes / 60, minutes % 60, seconds)
    } else {
        format!("+{minutes:02}:{seconds:02}.{millis:03}")
    }
}

fn level_color(level: &str) -> Color {
    match level.to_ascii_lowercase().as_str() {
        "error" | "fatal" => danger(),
        "warn" | "warning" => warning(),
        "debug" | "trace" => text_muted(),
        _ => info(),
    }
}

/// Consecutive identical messages collapse into one row with a count.
fn group_repeats(entries: Vec<ConsoleEntry>) -> Vec<(ConsoleEntry, usize)> {
    let mut grouped: Vec<(ConsoleEntry, usize)> = Vec::new();
    for entry in entries {
        if let Some((last, count)) = grouped.last_mut()
            && last.level == entry.level
            && last.target == entry.target
            && last.message == entry.message
        {
            *count += 1;
            last.time_ms = entry.time_ms;
            continue;
        }
        grouped.push((entry, 1));
    }
    grouped
}

/// Plain-text form of console entries for the clipboard.
pub(crate) fn console_text(entries: &[ConsoleEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            format!(
                "[{}] {} {}: {}",
                session_time(entry.time_ms),
                entry.level.to_uppercase(),
                entry.target,
                entry.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn level_count(level: ConsoleLevel, counts: ConsoleCounts) -> usize {
    match level {
        ConsoleLevel::All => counts.errors + counts.warnings + counts.info + counts.debug,
        ConsoleLevel::Errors => counts.errors,
        ConsoleLevel::Warnings => counts.warnings,
        ConsoleLevel::Info => counts.info,
        ConsoleLevel::Debug => counts.debug,
    }
}

fn row(entry: ConsoleEntry, repeats: usize) -> Widget {
    let color = level_color(&entry.level);
    let lower = entry.level.to_ascii_lowercase();
    let (stripe, wash) = match lower.as_str() {
        "error" | "fatal" => (danger(), danger_wash()),
        "warn" | "warning" => (warning(), tint(warning(), 14)),
        _ => (Color::TRANSPARENT, Color::TRANSPARENT),
    };
    let mut message = vec![
        Expanded::new(
            Text::new(entry.message).style(super::shared::mono_style(12., text_primary())),
        )
        .into(),
    ];
    if repeats > 1 {
        message.push(gap(8., 1.));
        message.push(count_bubble(repeats, color));
    }
    Container::with_child(
        Row::new([
            Widget::box_(Size::new(3., 30.), stripe),
            gap(13., 1.),
            SizedBox::new()
                .width(96.)
                .child(Padding::new(
                    EdgeInsets::symmetric(0., 8.),
                    mono(session_time(entry.time_ms), 11., text_muted()),
                ))
                .into(),
            SizedBox::new()
                .width(78.)
                .child(Padding::new(
                    EdgeInsets::symmetric(0., 6.),
                    hrow([
                        super::shared::dot(color, 6.),
                        gap(7., 1.),
                        Text::new(entry.level.to_uppercase())
                            .style(text_style(10.5, color).font_weight(FontWeight::W600))
                            .into(),
                    ]),
                ))
                .into(),
            SizedBox::new()
                .width(160.)
                .child(Padding::new(
                    EdgeInsets::only(0., 7., 12., 7.),
                    line_text(entry.target, 11.5, text_muted()),
                ))
                .into(),
            Expanded::new(Padding::new(
                EdgeInsets::only(0., 7., 16., 7.),
                Row::new(message).cross_axis_alignment(CrossAxisAlignment::Start),
            ))
            .into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    )
    .alignment(Alignment::TOP_LEFT)
    .color(wash)
    .into()
}

pub(crate) fn build_console(
    entries: Vec<ConsoleEntry>,
    shared: Shared,
    tick: Signal<u64>,
    filter: TextEditingController,
    clipboard: Clipboard,
) -> Widget {
    let (paused, level, total, counts) = shared
        .lock()
        .map(|state| {
            (
                state.console_frozen.is_some(),
                state.console_level,
                state
                    .console_frozen
                    .as_ref()
                    .map_or(state.console.len(), Vec::len),
                state.console_counts(),
            )
        })
        .unwrap_or_default();
    let filtered = entries.len();
    let filter_shared = shared.clone();
    let filter_tick = tick.clone();
    let filter_field: Widget = TextField::new(filter)
        .size(Size::new(0., 20.))
        .style(search_input_style())
        .placeholder("Filter messages, sources or levels…")
        .on_changed(move |query| {
            if let Ok(mut state) = filter_shared.lock() {
                state.set_console_query(query);
            }
            refresh(&filter_tick);
        })
        .into();
    let levels = tab_list(ConsoleLevel::ALL.map(|option| {
        let (level_shared, level_tick) = (shared.clone(), tick.clone());
        let count = level_count(option, counts);
        let label = match option {
            ConsoleLevel::All => format!("All {count}"),
            ConsoleLevel::Debug => format!("Debug {count}"),
            other => format!("{} {count}", other.label()),
        };
        let callback: Box<dyn Fn()> = Box::new(move || {
            if let Ok(mut state) = level_shared.lock() {
                state.set_console_level(option);
            }
            refresh(&level_tick);
        });
        (label, level == option, callback)
    }));
    let pause = {
        let (shared, tick) = (shared.clone(), tick.clone());
        button(
            if paused {
                "Resume live"
            } else {
                "Pause display"
            },
            Some(if paused {
                IconName::Play
            } else {
                IconName::Pause
            }),
            if paused {
                Variant::Active
            } else {
                Variant::Secondary
            },
            true,
            move || {
                if let Ok(mut state) = shared.lock() {
                    state.set_console_paused(!paused);
                }
                refresh(&tick);
            },
        )
    };
    let copy = {
        let entries = entries.clone();
        button(
            "Copy",
            Some(IconName::Copy),
            Variant::Secondary,
            !entries.is_empty(),
            move || {
                clipboard.copy(
                    console_text(&entries),
                    &format!("{} console messages", entries.len()),
                )
            },
        )
    };
    let clear = {
        let (shared, tick) = (shared.clone(), tick.clone());
        button(
            "Clear",
            Some(IconName::Trash),
            Variant::Secondary,
            true,
            move || {
                if let Ok(mut state) = shared.lock() {
                    state.clear_console();
                }
                refresh(&tick);
            },
        )
    };
    let toolbar = card(
        Padding::all(
            12.,
            column([
                wide_row([
                    Expanded::new(search_box(filter_field, 34.)).into(),
                    gap(10., 1.),
                    pause,
                    gap(6., 1.),
                    copy,
                    gap(6., 1.),
                    clear,
                ]),
                gap(1., 10.),
                wide_row([
                    levels,
                    spacer(),
                    ui_text(format!("{filtered} of {total} messages"), 12., text_muted()),
                    gap(10., 1.),
                    if paused {
                        status_badge("Paused · capture continues", warning())
                    } else {
                        status_badge("Live", super::success())
                    },
                ]),
            ]),
        )
        .into(),
    );

    let mut rows: Vec<Widget> = vec![
        Container::with_child(Padding::new(
            EdgeInsets::only(16., 10., 16., 10.),
            Row::new([
                Widget::from(SizedBox::new().width(96.).child(caption("Time"))),
                SizedBox::new().width(78.).child(caption("Level")).into(),
                SizedBox::new().width(160.).child(caption("Source")).into(),
                Expanded::new(caption("Message")).into(),
            ]),
        ))
        .color(super::shared::surface_raised())
        .into(),
    ];
    let grouped = group_repeats(entries);
    for (index, (entry, repeats)) in grouped.into_iter().enumerate() {
        if index > 0 {
            rows.push(divider());
        }
        rows.push(row(entry, repeats));
    }
    if filtered == 0 {
        rows.push(if total > 0 {
            empty_state(
                IconName::Filter,
                "No matching messages",
                "Try another search or select All.",
            )
        } else {
            empty_state(
                IconName::Terminal,
                "No messages yet",
                "Structured target logs and connection diagnostics will appear here.",
            )
        });
    }
    column([
        toolbar,
        gap(1., 14.),
        card(
            ClipRRect::new(
                CornerRadii::uniform(super::shared::CARD_RADIUS),
                Column::new(rows)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .main_axis_size(MainAxisSize::Min),
            )
            .into(),
        ),
    ])
}
