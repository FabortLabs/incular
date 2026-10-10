use super::shared::{badge, column, heading, key_value, metric, page_title, panel};
use super::{
    BORDER, DANGER, PRIMARY, TEXT_MUTED, TEXT_PRIMARY, compact_button, gap, section, ui_text,
};
use crate::{inspector::Shared, performance::FrameStatistics, transport::ClientBridge};
use incular::prelude::*;
use incular_devtools_protocol::{FrameRecordEvent, RequestMethod};

pub(crate) fn frame_chart(frames: Vec<FrameRecordEvent>, height: f32) -> Widget {
    if frames.is_empty() {
        return Container::with_child(Align::new(
            Alignment::CENTER,
            column([
                heading("Waiting for frame activity", 15.),
                gap(1., 8.),
                ui_text(
                    "Interact with your application to collect samples.",
                    12.,
                    TEXT_MUTED,
                ),
            ]),
        ))
        .alignment(Alignment::TOP_LEFT)
        .height(height)
        .into();
    }
    LayoutBuilder::new(move |_, constraints| {
        let width = constraints.max_width().clamp(100., 4000.);
        let budget = frames
            .last()
            .and_then(|frame| frame.budget_us)
            .filter(|budget| *budget > 0);
        let max = frames
            .iter()
            .map(|frame| frame.timings.cpu_total)
            .max()
            .unwrap_or(1)
            .max(budget.unwrap_or(1000)) as f32
            * 1.15;
        let plot_height = height - 20.;
        let mut canvas = Canvas::default();
        for y in [0., plot_height / 2., plot_height] {
            canvas.rect(
                incular::core::Rect::from_origin_size(Offset::new(0., y), Size::new(width, 1.)),
                BORDER,
            );
        }
        if let Some(budget) = budget {
            let y = plot_height * (1. - budget as f32 / max);
            for x in (0..width as usize).step_by(9) {
                canvas.rect(
                    incular::core::Rect::from_origin_size(
                        Offset::new(x as f32, y),
                        Size::new(4., 1.),
                    ),
                    super::shared::WARNING,
                );
            }
        }
        let step = width / frames.len() as f32;
        for (index, frame) in frames.iter().enumerate() {
            let bar = (frame.timings.cpu_total as f32 / max * plot_height).max(2.);
            canvas.rect(
                incular::core::Rect::from_origin_size(
                    Offset::new(index as f32 * step, plot_height - bar),
                    Size::new((step - 2.).max(1.), bar),
                ),
                if frame.over_budget { DANGER } else { PRIMARY },
            );
        }
        CustomPaint::new(Size::new(width, height), canvas.finish()).into()
    })
    .into()
}

pub(crate) fn build_overview(
    shared: Shared,
    bridge: ClientBridge,
    tool_view: Signal<super::ToolView>,
) -> Widget {
    let state = shared.lock().expect("inspector state");
    let frames = state
        .frames
        .iter()
        .filter(|frame| Some(frame.window) == state.active_window)
        .cloned()
        .collect::<Vec<_>>();
    let statistics = FrameStatistics::from_frames(&frames);
    let memory = state.memory.clone();
    let target = state.target_info.clone();
    let node_count = state.nodes.len();
    let connected = state.connected;
    let error = state.error.clone();
    let messages = state
        .console
        .iter()
        .rev()
        .take(3)
        .cloned()
        .collect::<Vec<_>>();
    let window_count = state.windows.len();
    drop(state);
    let mut body = vec![
        page_title(
            "Your application, at a glance",
            "A live view of rendering, retained widgets, and runtime health.",
        ),
        Row::new([
            Expanded::new(metric(
                "AVERAGE CPU FRAME",
                if statistics.samples == 0 {
                    "—".into()
                } else {
                    format!("{:.2} ms", statistics.average_us / 1000.)
                },
                "Observed frame work",
                PRIMARY,
            ))
            .into(),
            gap(12., 1.),
            Expanded::new(metric(
                "P95 FRAME TIME",
                if statistics.samples == 0 {
                    "—".into()
                } else {
                    format!("{:.2} ms", statistics.p95_us as f64 / 1000.)
                },
                "95% of samples finish within",
                TEXT_PRIMARY,
            ))
            .into(),
            gap(12., 1.),
            Expanded::new(metric(
                "PROCESS MEMORY",
                memory.as_ref().map_or_else(
                    || "—".into(),
                    |snapshot| {
                        if snapshot.counts.rss_mb == 0 {
                            "—".into()
                        } else {
                            format!("{} MB", snapshot.counts.rss_mb)
                        }
                    },
                ),
                "Last captured snapshot",
                super::shared::VIOLET,
            ))
            .into(),
            gap(12., 1.),
            Expanded::new(metric(
                "RETAINED WIDGETS",
                node_count.to_string(),
                "Loaded in the active window",
                TEXT_PRIMARY,
            ))
            .into(),
        ])
        .into(),
        gap(1., 20.),
    ];
    if let Some(error) = error {
        body.push(panel(Padding::all(12., ui_text(error, 12., DANGER)).into()));
        body.push(gap(1., 16.));
    }
    let view = tool_view.clone();
    body.push(section(
        "Frame activity",
        "CPU time per frame · mint = within budget · coral = over budget · dashed = display budget",
        column([
            Row::new([
                badge(format!("{} samples", statistics.samples), TEXT_MUTED),
                gap(8., 1.),
                badge(
                    statistics.jank_percent().map_or_else(
                        || "Budget unavailable".into(),
                        |value| format!("{value:.1}% over budget"),
                    ),
                    if statistics.over_budget > 0 {
                        DANGER
                    } else {
                        PRIMARY
                    },
                ),
                Expanded::new(gap(1., 1.)).into(),
                compact_button("Open performance →", false, move || {
                    view.set(super::ToolView::Performance);
                }),
            ])
            .into(),
            gap(1., 20.),
            frame_chart(frames, 110.),
        ]),
    ));
    body.push(gap(1., 20.));
    let mut identity = vec![
        badge(
            if connected {
                "Session connected"
            } else {
                "Waiting for a target"
            },
            if connected { PRIMARY } else { TEXT_MUTED },
        ),
        gap(1., 12.),
    ];
    if let Some(target) = target {
        identity.extend([
            key_value("Process", format!("PID {}", target.pid)),
            key_value("Platform", target.platform),
            key_value("Framework", target.framework_version),
            key_value("Windows", window_count.to_string()),
        ]);
    } else {
        identity.push(ui_text(
            "Run a DevTools-enabled application with --devtools, then connect.",
            12.,
            TEXT_MUTED,
        ));
    }
    let capture = bridge.clone();
    let info = bridge;
    identity.extend([
        gap(1., 12.),
        Wrap::new([
            compact_button("Capture memory", false, move || {
                capture.send_or_report(RequestMethod::TakeMemorySnapshot {
                    label: "manual".into(),
                })
            }),
            compact_button(
                if connected {
                    "Refresh session"
                } else {
                    "Find target"
                },
                false,
                move || {
                    if connected {
                        info.send_or_report(RequestMethod::GetTargetInfo);
                    } else {
                        info.retry();
                    }
                },
            ),
        ])
        .spacing(8.)
        .into(),
    ]);
    let mut activity = Vec::new();
    if messages.is_empty() {
        activity.extend([
            heading("All quiet", 16.),
            gap(1., 8.),
            ui_text(
                "Target diagnostics and connection events will appear here.",
                12.,
                TEXT_MUTED,
            ),
        ]);
    } else {
        for entry in messages {
            activity.extend([
                Row::new([
                    badge(entry.level.to_uppercase(), TEXT_MUTED),
                    gap(8., 1.),
                    ui_text(entry.target, 11., TEXT_MUTED),
                ])
                .into(),
                gap(1., 8.),
                ui_text(entry.message, 12., TEXT_PRIMARY),
                gap(1., 14.),
            ]);
        }
    }
    let view = tool_view;
    activity.push(compact_button("Open console →", false, move || {
        view.set(super::ToolView::Console);
    }));
    body.push(
        Row::new([
            Expanded::new(section(
                "Connected target",
                "Session identity and quick actions",
                column(identity),
            ))
            .into(),
            gap(16., 1.),
            Expanded::new(section(
                "Recent diagnostics",
                "Latest messages from your target",
                column(activity),
            ))
            .into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .into(),
    );
    column(body)
}
