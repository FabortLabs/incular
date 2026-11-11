//! Transport: the local DevTools WebSocket requests, laid out like a network
//! panel (method filter, status filter, duration waterfall, request details).

use super::icons::IconName;
use super::shared::{
    Variant, button, caption, card, column, divider, empty_state, gap, line_text, meter, metric,
    mono, refresh, search_box, search_input_style, section, spacer, state_surface, status_badge,
    surface_raised, tab_list, wide_row,
};
use super::{danger, primary, text_muted, text_primary, ui_text};
use crate::{
    activity::{RequestActivity, RequestStatus},
    inspector::Shared,
};
use incular::controls::TextField;
use incular::material::RawMaterialButton;
use incular::prelude::*;
use incular::widgets::internal::TextEditingController;

/// Which requests the table shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransportFilter {
    #[default]
    All,
    Pending,
    Failed,
}

fn failed(status: RequestStatus) -> bool {
    matches!(
        status,
        RequestStatus::Failed | RequestStatus::TimedOut | RequestStatus::Disconnected
    )
}

fn status_color(status: RequestStatus) -> Color {
    match status {
        RequestStatus::Completed => super::success(),
        RequestStatus::Pending => super::shared::warning(),
        _ => danger(),
    }
}

/// `1.2 KB`, `340 B`.
pub(crate) fn format_bytes(bytes: usize) -> String {
    match bytes {
        0..=1023 => format!("{bytes} B"),
        1024..=1_048_575 => format!("{:.1} KB", bytes as f64 / 1024.),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.),
    }
}

fn format_ms(us: u64) -> String {
    format!("{:.2} ms", us as f64 / 1000.)
}

/// Nearest-rank percentile of `values` (sorted in place).
fn percentile(values: &mut [u64], rank: f64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let index = ((rank * values.len() as f64).ceil() as usize).clamp(1, values.len()) - 1;
    Some(values[index])
}

const METHOD_WIDTH: f32 = 230.;
const STATUS_WIDTH: f32 = 120.;
const DURATION_WIDTH: f32 = 90.;
const SIZE_WIDTH: f32 = 150.;

fn table_header() -> Widget {
    Container::with_child(Padding::new(
        EdgeInsets::only(16., 10., 16., 10.),
        Row::new([
            Widget::from(SizedBox::new().width(METHOD_WIDTH).child(caption("Method"))),
            SizedBox::new()
                .width(STATUS_WIDTH)
                .child(caption("Status"))
                .into(),
            SizedBox::new()
                .width(DURATION_WIDTH)
                .child(caption("Duration"))
                .into(),
            SizedBox::new()
                .width(SIZE_WIDTH)
                .child(caption("Sent / received"))
                .into(),
            Expanded::new(caption("Waterfall")).into(),
        ]),
    ))
    .color(surface_raised())
    .into()
}

fn request_row(
    entry: &RequestActivity,
    slowest: u64,
    selected: bool,
    on_select: impl Fn() + 'static,
) -> Widget {
    let color = status_color(entry.status);
    let fraction = entry
        .elapsed_us
        .map_or(0., |us| us as f32 / slowest.max(1) as f32);
    let content = Row::new([
        Widget::from(
            SizedBox::new().width(METHOD_WIDTH).child(Padding::new(
                EdgeInsets::only(0., 0., 12., 0.),
                Text::new(entry.method.clone())
                    .style(super::shared::mono_style(12., text_primary()))
                    .max_lines(Some(1))
                    .overflow(TextOverflow::Ellipsis),
            )),
        ),
        SizedBox::new()
            .width(STATUS_WIDTH)
            .child(Align::new(
                Alignment::CENTER_LEFT,
                status_badge(entry.status.label(), color),
            ))
            .into(),
        SizedBox::new()
            .width(DURATION_WIDTH)
            .child(mono(
                entry.elapsed_us.map_or_else(|| "…".into(), format_ms),
                11.5,
                text_muted(),
            ))
            .into(),
        SizedBox::new()
            .width(SIZE_WIDTH)
            .child(mono(
                format!(
                    "{} / {}",
                    format_bytes(entry.request_bytes),
                    format_bytes(entry.response_bytes)
                ),
                11.5,
                text_muted(),
            ))
            .into(),
        Expanded::new(Align::new(
            Alignment::CENTER_LEFT,
            meter(
                fraction,
                if entry.status == RequestStatus::Completed {
                    primary()
                } else {
                    color
                },
                6.,
            ),
        ))
        .into(),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Center);
    state_surface(
        RawMaterialButton::new(format!("Request {} #{}", entry.method, entry.request_id))
            .size(Size::new(0., 40.))
            .content(Padding::new(EdgeInsets::symmetric(16., 0.), content))
            .on_press(on_select),
        if selected {
            super::shared::control_active()
        } else {
            Color::TRANSPARENT
        },
        0.,
    )
}

fn details(entry: &RequestActivity) -> Widget {
    section(
        &entry.method,
        "Request details",
        column([
            status_badge(entry.status.label(), status_color(entry.status)),
            gap(1., 12.),
            super::shared::key_value("Request id", entry.request_id.to_string()),
            super::shared::key_value("Sequence", entry.sequence.to_string()),
            super::shared::key_value(
                "Duration",
                entry
                    .elapsed_us
                    .map_or_else(|| "awaiting a response".into(), format_ms),
            ),
            super::shared::key_value("Sent", format_bytes(entry.request_bytes)),
            super::shared::key_value("Received", format_bytes(entry.response_bytes)),
            gap(1., 10.),
            ui_text(
                "Request and response bodies are never retained; only sizes and timing are kept.",
                11.,
                text_muted(),
            ),
        ]),
    )
}

pub(crate) fn build_network(
    shared: Shared,
    tick: Signal<u64>,
    filter: Signal<TransportFilter>,
    query: TextEditingController,
    selected: Signal<Option<u64>>,
) -> Widget {
    let entries = shared
        .lock()
        .map(|state| state.activity.entries().rev().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let total = entries.len();
    let errors = entries.iter().filter(|entry| failed(entry.status)).count();
    let pending = entries
        .iter()
        .filter(|entry| entry.status == RequestStatus::Pending)
        .count();
    let mut completed = entries
        .iter()
        .filter_map(|entry| entry.elapsed_us)
        .collect::<Vec<_>>();
    let latency = if completed.is_empty() {
        "—".into()
    } else {
        format_ms(completed.iter().sum::<u64>() / completed.len() as u64)
    };
    let p95 = percentile(&mut completed, 0.95).map_or_else(|| "—".into(), format_ms);
    let sent: usize = entries.iter().map(|entry| entry.request_bytes).sum();
    let received: usize = entries.iter().map(|entry| entry.response_bytes).sum();
    let active_filter = filter.get();
    let needle = query.text().trim().to_lowercase();
    let visible = entries
        .iter()
        .filter(|entry| match active_filter {
            TransportFilter::All => true,
            TransportFilter::Pending => entry.status == RequestStatus::Pending,
            TransportFilter::Failed => failed(entry.status),
        })
        .filter(|entry| needle.is_empty() || entry.method.to_lowercase().contains(&needle))
        .cloned()
        .collect::<Vec<_>>();
    let slowest = visible
        .iter()
        .filter_map(|entry| entry.elapsed_us)
        .max()
        .unwrap_or(1);

    let stats = Row::new([
        Expanded::new(metric(
            "Requests",
            total.to_string(),
            "Up to 256 retained requests",
            text_primary(),
        ))
        .into(),
        gap(12., 1.),
        Expanded::new(metric(
            "Failed",
            errors.to_string(),
            "Errors, timeouts and disconnects",
            if errors == 0 {
                super::success()
            } else {
                danger()
            },
        ))
        .into(),
        gap(12., 1.),
        Expanded::new(metric(
            "Average round trip",
            latency,
            &format!("p95 {p95}"),
            primary(),
        ))
        .into(),
        gap(12., 1.),
        Expanded::new(metric(
            "Transferred",
            format_bytes(sent + received),
            &format!(
                "{} sent · {} received",
                format_bytes(sent),
                format_bytes(received)
            ),
            text_primary(),
        ))
        .into(),
    ]);

    let search_tick = tick.clone();
    let field: Widget = TextField::new(query)
        .size(Size::new(0., 20.))
        .style(search_input_style())
        .placeholder("Filter by method…")
        .on_changed(move |_| refresh(&search_tick))
        .into();
    let filters = tab_list(
        [
            (TransportFilter::All, format!("All {total}")),
            (TransportFilter::Pending, format!("Pending {pending}")),
            (TransportFilter::Failed, format!("Failed {errors}")),
        ]
        .map(|(option, label)| {
            let filter = filter.clone();
            let callback: Box<dyn Fn()> = Box::new(move || {
                filter.set(option);
            });
            (label, active_filter == option, callback)
        }),
    );
    let clear_shared = shared.clone();
    let clear_tick = tick.clone();
    let toolbar = card(
        Padding::all(
            12.,
            wide_row([
                SizedBox::new()
                    .width(300.)
                    .child(search_box(field, 34.))
                    .into(),
                gap(10., 1.),
                filters,
                spacer(),
                button(
                    "Clear completed",
                    Some(IconName::Trash),
                    Variant::Secondary,
                    true,
                    move || {
                        if let Ok(mut state) = clear_shared.lock() {
                            state.activity.clear_completed();
                        }
                        refresh(&clear_tick);
                    },
                ),
            ]),
        )
        .into(),
    );

    let chosen = selected.get();
    let mut rows = vec![table_header()];
    for (index, entry) in visible.iter().enumerate() {
        if index > 0 {
            rows.push(divider());
        }
        let sequence = entry.sequence;
        let selected = selected.clone();
        rows.push(request_row(
            entry,
            slowest,
            chosen == Some(sequence),
            move || {
                selected.update(|value| {
                    *value = if *value == Some(sequence) {
                        None
                    } else {
                        Some(sequence)
                    }
                })
            },
        ));
    }
    if visible.is_empty() {
        rows.push(if total == 0 {
            empty_state(
                IconName::Transfer,
                "Waiting for requests",
                "Refresh the target or inspect a widget to record transport activity.",
            )
        } else {
            empty_state(
                IconName::Filter,
                "No requests match",
                "Clear the method filter or choose All.",
            )
        });
    }
    let table = card(
        ClipRRect::new(
            CornerRadii::uniform(super::shared::CARD_RADIUS),
            Column::new(rows)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .main_axis_size(MainAxisSize::Min),
        )
        .into(),
    );
    let detail =
        chosen.and_then(|sequence| entries.iter().find(|entry| entry.sequence == sequence));
    let body: Widget = match detail {
        Some(entry) => Row::new([
            Expanded::new(table).into(),
            gap(14., 1.),
            SizedBox::new().width(320.).child(details(entry)).into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .into(),
        None => table,
    };
    column([
        stats.into(),
        gap(1., 14.),
        toolbar,
        gap(1., 14.),
        body,
        gap(1., 12.),
        line_text(
            "This is the DevTools WebSocket transport. Application HTTP requests need their own instrumentation.",
            11.,
            text_muted(),
        ),
    ])
}
