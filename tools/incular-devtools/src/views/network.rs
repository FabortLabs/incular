use super::shared::{badge, column, metric, mono, page_title, panel, refresh};
use super::{DANGER, PRIMARY, TEXT_MUTED, TEXT_PRIMARY, compact_button, gap, ui_text};
use crate::{activity::RequestStatus, inspector::Shared};
use incular::prelude::*;

pub(crate) fn build_network(
    shared: Shared,
    tick: Signal<u64>,
    failures_only: Signal<bool>,
) -> Widget {
    let entries = shared
        .lock()
        .map(|state| state.activity.entries().rev().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let total = entries.len();
    let errors = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.status,
                RequestStatus::Failed | RequestStatus::TimedOut | RequestStatus::Disconnected
            )
        })
        .count();
    let completed = entries
        .iter()
        .filter_map(|entry| entry.elapsed_us)
        .collect::<Vec<_>>();
    let latency = if completed.is_empty() {
        "—".into()
    } else {
        format!(
            "{:.2} ms",
            completed.iter().map(|value| *value as f64).sum::<f64>()
                / completed.len() as f64
                / 1000.
        )
    };
    let failed = failures_only.get();
    let filter = failures_only;
    let clear_tick = tick;
    let mut rows = vec![
        Container::with_child(Padding::all(
            12.,
            Row::new([
                SizedBox::from_size(Size::new(220., 18.))
                    .child(ui_text("METHOD", 10., TEXT_MUTED))
                    .into(),
                SizedBox::from_size(Size::new(120., 18.))
                    .child(ui_text("STATUS", 10., TEXT_MUTED))
                    .into(),
                SizedBox::from_size(Size::new(95., 18.))
                    .child(ui_text("DURATION", 10., TEXT_MUTED))
                    .into(),
                ui_text("SENT / RECEIVED", 10., TEXT_MUTED),
            ]),
        ))
        .alignment(Alignment::TOP_LEFT)
        .color(super::shared::SURFACE_RAISED)
        .into(),
    ];
    let mut visible = 0;
    for entry in entries {
        if failed
            && !matches!(
                entry.status,
                RequestStatus::Failed | RequestStatus::TimedOut | RequestStatus::Disconnected
            )
        {
            continue;
        }
        let color = match entry.status {
            RequestStatus::Completed => PRIMARY,
            RequestStatus::Pending => super::shared::WARNING,
            _ => DANGER,
        };
        rows.push(
            Container::with_child(Padding::all(
                12.,
                Row::new([
                    Widget::from(SizedBox::from_size(Size::new(220., 24.)).child(mono(
                        entry.method,
                        12.,
                        TEXT_PRIMARY,
                    ))),
                    SizedBox::from_size(Size::new(120., 24.))
                        .child(Align::new(
                            Alignment::CENTER_LEFT,
                            badge(entry.status.label(), color),
                        ))
                        .into(),
                    SizedBox::from_size(Size::new(95., 24.))
                        .child(mono(
                            entry.elapsed_us.map_or_else(
                                || "…".into(),
                                |us| format!("{:.2} ms", us as f64 / 1000.),
                            ),
                            11.,
                            TEXT_MUTED,
                        ))
                        .into(),
                    Expanded::new(mono(
                        format!("{} B / {} B", entry.request_bytes, entry.response_bytes),
                        11.,
                        TEXT_MUTED,
                    ))
                    .into(),
                ]),
            ))
            .alignment(Alignment::TOP_LEFT)
            .color(if visible % 2 == 0 {
                super::SURFACE
            } else {
                super::APP_BACKGROUND
            })
            .into(),
        );
        visible += 1;
    }
    if visible == 0 {
        rows.push(
            Padding::all(
                24.,
                column([
                    super::shared::heading(
                        if failed {
                            "No failed requests"
                        } else {
                            "Waiting for requests"
                        },
                        16.,
                    ),
                    gap(1., 8.),
                    ui_text(
                        "Refresh the target or inspect a widget to record transport activity.",
                        12.,
                        TEXT_MUTED,
                    ),
                ]),
            )
            .into(),
        );
    }
    column([
        page_title(
            "Transport",
            "Inspect the local DevTools connection, request latency, and failures.",
        ),
        Row::new([
            Expanded::new(metric(
                "RECENT REQUESTS",
                total.to_string(),
                "Up to 256 retained requests",
                TEXT_PRIMARY,
            ))
            .into(),
            gap(12., 1.),
            Expanded::new(metric(
                "FAILED REQUESTS",
                errors.to_string(),
                "Errors, timeouts, disconnects",
                if errors == 0 { PRIMARY } else { DANGER },
            ))
            .into(),
            gap(12., 1.),
            Expanded::new(metric(
                "AVERAGE ROUND TRIP",
                latency,
                "Requests with a final outcome",
                super::shared::VIOLET,
            ))
            .into(),
        ])
        .into(),
        gap(1., 20.),
        Row::new([
            compact_button(
                if failed {
                    "Show all requests"
                } else {
                    "Show failures only"
                },
                failed,
                move || {
                    filter.set(!failed);
                },
            ),
            Expanded::new(gap(1., 1.)).into(),
            compact_button("Clear completed", false, move || {
                if let Ok(mut state) = shared.lock() {
                    state.activity.clear_completed();
                }
                refresh(&clear_tick);
            }),
        ])
        .into(),
        gap(1., 14.),
        panel(column(rows)),
        gap(1., 14.),
        ui_text(
            "This is the DevTools WebSocket transport. Application HTTP requests require their own instrumentation.",
            11.,
            TEXT_MUTED,
        ),
    ])
}
