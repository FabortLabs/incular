//! Overview: live health figures, automatic insights, frame activity, quick
//! actions, the connected target and the latest diagnostics.

use super::command_palette::CommandAction;
use super::console::session_time;
use super::icons::IconName;
use super::shared::{
    CONTROL_RADIUS, Variant, badge, button, column, divider, dot, empty_state, gap, glyph,
    key_value, line_text, section_with, sparkline, stat_card, state_surface, surface_raised,
    text_style, tint, wide_row,
};
use super::{border, danger, primary, success, text_muted, text_primary, ui_text, warning};
use crate::{
    activity::RequestStatus,
    diagnostics::{CacheHealth, cache_health},
    inspector::Shared,
    performance::FrameStatistics,
};
use incular::material::RawMaterialButton;
use incular::prelude::*;
use incular_devtools_protocol::FrameRecordEvent;
use std::rc::Rc;

fn rounded_bar(canvas: &mut Canvas, x: f32, y: f32, width: f32, height: f32, color: Color) {
    canvas.rrect(
        RRect::uniform(
            incular::core::Rect::from_origin_size(Offset::new(x, y), Size::new(width, height)),
            (width / 2.).min(2.),
        ),
        color,
    );
}

pub(crate) fn frame_chart(frames: Vec<FrameRecordEvent>, height: f32) -> Widget {
    if frames.is_empty() {
        return empty_state(
            IconName::Activity,
            "Waiting for frame activity",
            "Interact with your application to collect samples. A static UI presents no frames.",
        );
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
        let plot_height = height - 8.;
        let mut canvas = Canvas::default();
        for y in [0., plot_height / 2., plot_height] {
            canvas.rect(
                incular::core::Rect::from_origin_size(Offset::new(0., y), Size::new(width, 1.)),
                border(),
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
                    warning(),
                );
            }
        }
        let step = width / frames.len() as f32;
        for (index, frame) in frames.iter().enumerate() {
            let bar = (frame.timings.cpu_total as f32 / max * plot_height).max(2.);
            rounded_bar(
                &mut canvas,
                index as f32 * step,
                plot_height - bar,
                (step - 2.).max(1.),
                bar,
                if frame.over_budget {
                    danger()
                } else {
                    primary()
                },
            );
        }
        CustomPaint::new(Size::new(width, height), canvas.finish()).into()
    })
    .into()
}

/// One automatic finding about the connected application.
struct Insight {
    color: Color,
    icon: IconName,
    title: String,
    detail: String,
    action: Option<(&'static str, CommandAction)>,
}

fn insight_row(insight: Insight, run: &Rc<dyn Fn(CommandAction)>) -> Widget {
    let mut row = vec![
        Container::with_child(Align::new(
            Alignment::CENTER,
            glyph(insight.icon, 15., insight.color),
        ))
        .width(32.)
        .height(32.)
        .color(tint(insight.color, 30))
        .radius(9.)
        .into(),
        gap(12., 1.),
        Expanded::new(column([
            line_text(insight.title, 13., text_primary()),
            gap(1., 2.),
            line_text(insight.detail, 11.5, text_muted()),
        ]))
        .into(),
    ];
    if let Some((label, action)) = insight.action {
        let run = run.clone();
        row.push(gap(10., 1.));
        row.push(button(label, None, Variant::Secondary, true, move || {
            run(action)
        }));
    }
    Padding::new(
        EdgeInsets::symmetric(0., 8.),
        Row::new(row).cross_axis_alignment(CrossAxisAlignment::Center),
    )
    .into()
}

fn quick_action(
    icon: IconName,
    title: &str,
    detail: &str,
    action: CommandAction,
    run: &Rc<dyn Fn(CommandAction)>,
) -> Widget {
    let run = run.clone();
    state_surface(
        RawMaterialButton::new(title.to_owned())
            .size(Size::new(0., 64.))
            .content(Padding::new(
                EdgeInsets::symmetric(14., 0.),
                Row::new([
                    Container::with_child(Align::new(
                        Alignment::CENTER,
                        glyph(icon, 16., primary()),
                    ))
                    .width(34.)
                    .height(34.)
                    .color(super::shared::control_active())
                    .radius(9.)
                    .into(),
                    gap(12., 1.),
                    Expanded::new(column([
                        Text::new(title.to_owned())
                            .style(text_style(13., text_primary()).font_weight(FontWeight::W500))
                            .max_lines(Some(1))
                            .into(),
                        gap(1., 2.),
                        line_text(detail.to_owned(), 11., text_muted()),
                    ]))
                    .into(),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Center),
            ))
            .on_press(move || run(action)),
        surface_raised(),
        CONTROL_RADIUS + 2.,
    )
}

pub(crate) fn build_overview(
    shared: Shared,
    tool_view: Signal<super::ToolView>,
    run: Rc<dyn Fn(CommandAction)>,
) -> Widget {
    let state = shared.lock().expect("inspector state");
    let frames = state
        .frames
        .iter()
        .filter(|frame| Some(frame.window) == state.active_window)
        .cloned()
        .collect::<Vec<_>>();
    let statistics = FrameStatistics::from_frames(&frames);
    let live = state.live_fps(state.active_window, std::time::Instant::now());
    let cpu_spark = state
        .recent_cpu_micros(state.active_window, 60)
        .into_iter()
        .map(|micros| micros as f32)
        .collect::<Vec<_>>();
    let memory = state.memory.clone();
    let rss_history = state
        .memory_history()
        .map(|entry| entry.snapshot.counts.rss_mb as f32)
        .collect::<Vec<_>>();
    let rss_growth = {
        let mut history = state.memory_history();
        match (history.next(), history.next_back()) {
            (Some(first), Some(last)) if first.snapshot.counts.rss_mb > 0 => Some(
                (last.snapshot.counts.rss_mb as f64 - first.snapshot.counts.rss_mb as f64) * 100.
                    / first.snapshot.counts.rss_mb as f64,
            ),
            _ => None,
        }
    };
    let target = state.target_info.clone();
    let node_count = state.nodes.len();
    let connected = state.connected;
    let error = state.error.clone();
    let counts = state.console_counts();
    let failed_requests = state
        .activity
        .entries()
        .filter(|entry| {
            matches!(
                entry.status,
                RequestStatus::Failed | RequestStatus::TimedOut | RequestStatus::Disconnected
            )
        })
        .count();
    let overlays = state.debug_options.len();
    let animation_scale = state.animation_scale.unwrap_or(1.);
    let text_cache = state
        .diagnostics
        .latest()
        .map(|wire| cache_health(wire.text.cache_hits, wire.text.cache_misses));
    let truncated = state.tree_truncated;
    let messages = state
        .console
        .iter()
        .rev()
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    let windows = state.windows.clone();
    drop(state);

    let cpu_color = primary();
    let stats = Row::new([
        Expanded::new(stat_card(
            Some(IconName::Zap),
            "Frame rate",
            if !connected {
                "—".into()
            } else if live.idle {
                "Idle".into()
            } else {
                format!("{} FPS", live.fps)
            },
            if live.idle {
                "No frame in the last second"
            } else {
                "Frames presented in the last second"
            },
            if live.idle || !connected {
                text_muted()
            } else {
                super::header::fps_color(live)
            },
            None,
        ))
        .into(),
        gap(12., 1.),
        Expanded::new(stat_card(
            Some(IconName::Activity),
            "Average CPU frame",
            if statistics.samples == 0 {
                "—".into()
            } else {
                format!("{:.2} ms", statistics.average_us / 1000.)
            },
            &format!(
                "p95 {}",
                if statistics.samples == 0 {
                    "—".to_owned()
                } else {
                    format!("{:.2} ms", statistics.p95_us as f64 / 1000.)
                }
            ),
            text_primary(),
            Some(
                LayoutBuilder::new(move |_, constraints| {
                    sparkline(
                        &cpu_spark,
                        constraints.max_width().clamp(40., 2000.),
                        24.,
                        cpu_color,
                    )
                })
                .into(),
            ),
        ))
        .into(),
        gap(12., 1.),
        Expanded::new(stat_card(
            Some(IconName::Chip),
            "Process memory",
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
            if memory.is_some() {
                "Latest snapshot"
            } else {
                "No snapshot captured yet"
            },
            text_primary(),
            Some(
                LayoutBuilder::new(move |_, constraints| {
                    sparkline(
                        &rss_history,
                        constraints.max_width().clamp(40., 2000.),
                        24.,
                        super::shared::violet(),
                    )
                })
                .into(),
            ),
        ))
        .into(),
        gap(12., 1.),
        Expanded::new(stat_card(
            Some(IconName::Tree),
            "Retained widgets",
            node_count.to_string(),
            if truncated {
                "Loaded in the active window · truncated"
            } else {
                "Loaded in the active window"
            },
            text_primary(),
            None,
        ))
        .into(),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Start);

    // Insights: what needs attention right now, each with a next step.
    let mut insights = Vec::new();
    if !connected {
        insights.push(Insight {
            color: text_muted(),
            icon: IconName::Plug,
            title: "Not connected".into(),
            detail: error
                .clone()
                .unwrap_or_else(|| "Start an app with --devtools to inspect it".into()),
            action: Some(("Reconnect", CommandAction::Reconnect)),
        });
    }
    match statistics.jank_percent() {
        Some(jank) if jank > 5. => insights.push(Insight {
            color: danger(),
            icon: IconName::Alert,
            title: format!("{jank:.1}% of frames miss the display budget"),
            detail: format!(
                "{} of {} budgeted frames ran long. Record in Deep mode to find the widgets responsible.",
                statistics.over_budget, statistics.budgeted_samples
            ),
            action: Some(("Open Performance", CommandAction::View(super::ToolView::Performance))),
        }),
        Some(jank) => insights.push(Insight {
            color: success(),
            icon: IconName::Check,
            title: "Frames fit the display budget".into(),
            detail: format!("{jank:.1}% over budget across {} frames", statistics.budgeted_samples),
            action: None,
        }),
        None if connected => insights.push(Insight {
            color: text_muted(),
            icon: IconName::Info,
            title: "No frame timings yet".into(),
            detail: "The app is idle or has not reported a budget. Interact with it to collect frames."
                .into(),
            action: None,
        }),
        None => {}
    }
    if counts.errors > 0 {
        insights.push(Insight {
            color: danger(),
            icon: IconName::Alert,
            title: format!(
                "{} error{} in the console",
                counts.errors,
                if counts.errors == 1 { "" } else { "s" }
            ),
            detail: "Errors from the target or the DevTools connection".into(),
            action: Some((
                "Open Console",
                CommandAction::View(super::ToolView::Console),
            )),
        });
    } else if counts.warnings > 0 {
        insights.push(Insight {
            color: warning(),
            icon: IconName::Alert,
            title: format!(
                "{} warning{} in the console",
                counts.warnings,
                if counts.warnings == 1 { "" } else { "s" }
            ),
            detail: "Review them before they turn into errors".into(),
            action: Some((
                "Open Console",
                CommandAction::View(super::ToolView::Console),
            )),
        });
    }
    if failed_requests > 0 {
        insights.push(Insight {
            color: warning(),
            icon: IconName::Transfer,
            title: format!(
                "{failed_requests} DevTools request{} failed",
                if failed_requests == 1 { "" } else { "s" }
            ),
            detail: "Errors, timeouts or disconnects on the local transport".into(),
            action: Some((
                "Open Transport",
                CommandAction::View(super::ToolView::Network),
            )),
        });
    }
    if let Some(CacheHealth::Poor(rate)) = text_cache {
        insights.push(Insight {
            color: warning(),
            icon: IconName::Monitor,
            title: format!("Text layout cache hit rate is {rate:.0}%"),
            detail: "Paragraphs are reshaped often; check text that changes every frame".into(),
            action: Some((
                "Open Rendering",
                CommandAction::View(super::ToolView::Rendering),
            )),
        });
    }
    if let Some(growth) = rss_growth.filter(|growth| *growth > 10.) {
        insights.push(Insight {
            color: warning(),
            icon: IconName::Chip,
            title: format!("Process memory grew {growth:.0}% this session"),
            detail: "Compare snapshots A and B around the interaction you suspect".into(),
            action: Some(("Open Memory", CommandAction::View(super::ToolView::Memory))),
        });
    }
    if overlays > 0 {
        insights.push(Insight {
            color: primary(),
            icon: IconName::Layers,
            title: format!(
                "{overlays} overlay{} drawing in the app",
                if overlays == 1 { " is" } else { "s are" }
            ),
            detail: "Overlays cost frame time; turn them off before measuring".into(),
            action: Some(("Turn off", CommandAction::ClearOverlays)),
        });
    }
    if (animation_scale - 1.).abs() > 0.001 {
        insights.push(Insight {
            color: warning(),
            icon: IconName::Pause,
            title: if animation_scale == 0. {
                "Animations are paused".into()
            } else {
                format!("Animations run at {animation_scale}×")
            },
            detail: "Time dilation is on in the target".into(),
            action: Some(("Reset to 1×", CommandAction::AnimationSpeed(1.))),
        });
    }
    let insight_count = insights.len();
    let mut insight_rows = Vec::new();
    for (index, insight) in insights.into_iter().enumerate() {
        if index > 0 {
            insight_rows.push(divider());
        }
        insight_rows.push(insight_row(insight, &run));
    }
    let insights_card = section_with(
        "Insights",
        "Findings computed from live data, each with a next step",
        Some(badge(insight_count.to_string(), text_muted())),
        column(insight_rows),
    );

    let actions_card = section_with(
        "Quick actions",
        "Ctrl+K runs any command",
        None,
        column([
            quick_action(
                IconName::Crosshair,
                "Pick a widget",
                "Click it in the app to inspect",
                CommandAction::PickWidget,
                &run,
            ),
            gap(1., 8.),
            quick_action(
                IconName::Record,
                "Record performance",
                "Capture frames and traces",
                CommandAction::View(super::ToolView::Performance),
                &run,
            ),
            gap(1., 8.),
            quick_action(
                IconName::Chip,
                "Capture memory",
                "Take a resource snapshot",
                CommandAction::CaptureMemory,
                &run,
            ),
            gap(1., 8.),
            quick_action(
                IconName::Download,
                "Export report",
                "Save diagnostics as JSON",
                CommandAction::Export,
                &run,
            ),
        ]),
    );

    let view = tool_view.clone();
    let frame_card = section_with(
        "Frame activity",
        "CPU time per frame · violet within budget · red over budget · dashed line is the budget",
        Some(button(
            "Open Performance",
            Some(IconName::ArrowRight),
            Variant::Secondary,
            true,
            move || {
                view.set(super::ToolView::Performance);
            },
        )),
        column([
            wide_row([
                badge(format!("{} samples", statistics.samples), text_muted()),
                gap(8., 1.),
                badge(
                    statistics.jank_percent().map_or_else(
                        || "Budget unavailable".into(),
                        |value| format!("{value:.1}% over budget"),
                    ),
                    if statistics.over_budget > 0 {
                        danger()
                    } else {
                        success()
                    },
                ),
            ]),
            gap(1., 16.),
            frame_chart(frames, 120.),
        ]),
    );

    let mut identity = Vec::new();
    if let Some(target) = target {
        identity.extend([
            key_value("Process", format!("PID {}", target.pid)),
            key_value("Executable", target.executable),
            key_value("Platform", target.platform),
            key_value("Framework", format!("Incular {}", target.framework_version)),
            key_value("Protocol", format!("v{}", target.protocol_version)),
            key_value(
                "Windows",
                windows
                    .iter()
                    .map(|window| {
                        format!(
                            "{} ({:.0}×{:.0})",
                            window.title, window.logical_size[0], window.logical_size[1]
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        ]);
    } else {
        identity.push(ui_text(
            "Run a DevTools-enabled application with --devtools, then connect.",
            12.,
            text_muted(),
        ));
    }
    let target_card = section_with(
        "Connected target",
        "Session identity",
        Some(super::shared::status_badge(
            if connected { "Connected" } else { "Offline" },
            if connected { success() } else { text_muted() },
        )),
        column(identity),
    );

    let mut activity = Vec::new();
    if messages.is_empty() {
        activity.push(empty_state(
            IconName::Terminal,
            "All quiet",
            "Target diagnostics and connection events will appear here.",
        ));
    } else {
        for (index, entry) in messages.into_iter().enumerate() {
            if index > 0 {
                activity.push(divider());
            }
            let color = match entry.level.to_ascii_lowercase().as_str() {
                "error" | "fatal" => danger(),
                "warn" | "warning" => warning(),
                "debug" | "trace" => text_muted(),
                _ => super::shared::info(),
            };
            activity.push(
                Padding::new(
                    EdgeInsets::symmetric(0., 8.),
                    Row::new([
                        Padding::new(EdgeInsets::only(0., 5., 0., 0.), dot(color, 7.)).into(),
                        gap(10., 1.),
                        Expanded::new(column([
                            line_text(entry.message, 12.5, text_primary()),
                            gap(1., 2.),
                            line_text(
                                format!("{} · {}", entry.target, session_time(entry.time_ms)),
                                11.,
                                text_muted(),
                            ),
                        ]))
                        .into(),
                    ])
                    .cross_axis_alignment(CrossAxisAlignment::Start),
                )
                .into(),
            );
        }
    }
    let view = tool_view;
    let activity_card = section_with(
        "Recent diagnostics",
        "Latest messages from your target",
        Some(button(
            "Open Console",
            Some(IconName::ArrowRight),
            Variant::Secondary,
            true,
            move || {
                view.set(super::ToolView::Console);
            },
        )),
        column(activity),
    );

    column([
        stats.into(),
        gap(1., 14.),
        Row::new([
            Expanded::new(insights_card).flex(3).into(),
            gap(14., 1.),
            Expanded::new(actions_card).flex(2).into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .into(),
        gap(1., 14.),
        frame_card,
        gap(1., 14.),
        Row::new([
            Expanded::new(target_card).into(),
            gap(14., 1.),
            Expanded::new(activity_card).into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .into(),
    ])
}
