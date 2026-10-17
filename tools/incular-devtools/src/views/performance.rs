use super::shared::{
    column, heading, key_value, legend_swatch, metric, mono, page_title, panel, quiet_button,
    segmented_control, surface_raised, warning,
};
use super::theme::{
    danger_hover, gpu_color, hover_overlay, on_accent, pressed_overlay, primary_hover,
};
use super::{
    background, border, compact_button, control, danger, gap, primary, section, text_muted,
    text_primary, ui_text,
};
use crate::performance::{
    FlameBox, FramePhase, FrameStatistics, RankedTrace, TraceRange, format_micros, is_janky,
    percent_of_frame, phase_segments, stacked_total,
};
use crate::{inspector::Shared, transport::ClientBridge};
use incular::material::RawMaterialButton;
use incular::prelude::*;
use incular_devtools_protocol::{
    DevWidgetId, DevWindowId, DevtoolsProfilerMode, FrameRecordEvent, RequestMethod, TracePhase,
};
use std::collections::HashMap;

/// Frames shown before the user zooms.
pub(crate) const DEFAULT_VISIBLE_FRAMES: usize = 90;

const BAR_WIDTH: f32 = 8.;
const GPU_WIDTH: f32 = 2.;
const BAR_STEP: f32 = 12.;
const PLOT_HEIGHT: f32 = 150.;

pub(crate) struct PerformanceData {
    pub(crate) statistics: FrameStatistics,
    pub(crate) fps: Option<f32>,
    pub(crate) frames: Vec<FrameRecordEvent>,
    pub(crate) profiler_mode: DevtoolsProfilerMode,
    pub(crate) recording: bool,
    pub(crate) flame_boxes: Vec<FlameBox>,
    pub(crate) ranked: Vec<RankedTrace>,
    pub(crate) trace_status: Option<String>,
    pub(crate) selected_frame_event: Option<FrameRecordEvent>,
    pub(crate) widget_names: HashMap<DevWidgetId, String>,
    pub(crate) selected_frame: Option<(DevWindowId, u64)>,
    pub(crate) trace_range: TraceRange,
    pub(crate) selected_range: Option<(u64, u64)>,
}

pub(crate) fn phase_color(phase: FramePhase) -> Color {
    match phase {
        FramePhase::Build => super::theme::palette().phase_build,
        FramePhase::Layout => super::theme::palette().phase_layout,
        FramePhase::Paint => super::theme::palette().phase_paint,
        FramePhase::Semantics => super::theme::palette().phase_semantics,
        FramePhase::Composite => super::theme::palette().phase_composite,
        FramePhase::Other => super::theme::palette().phase_other,
    }
}

fn trace_phase(phase: TracePhase) -> FramePhase {
    match phase {
        TracePhase::Build => FramePhase::Build,
        TracePhase::Layout => FramePhase::Layout,
        TracePhase::Paint => FramePhase::Paint,
        TracePhase::Semantics => FramePhase::Semantics,
        TracePhase::Composite => FramePhase::Composite,
    }
}

fn widget_label(names: &HashMap<DevWidgetId, String>, id: DevWidgetId) -> String {
    names.get(&id).cloned().unwrap_or_else(|| id.to_string())
}

fn bump(tick: &Signal<u64>) {
    tick.update(|value| {
        *value = value
            .checked_add(1)
            .expect("DevTools UI revision exhausted")
    });
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> incular::core::Rect {
    incular::core::Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn fill_width(build: impl Fn(f32) -> Widget + 'static) -> Widget {
    LayoutBuilder::new(move |_, constraints| {
        let width = constraints.max_width().clamp(100., 4000.);
        SizedBox::new().width(width).child(build(width)).into()
    })
    .into()
}

fn right_cell(width: f32, child: Widget) -> Widget {
    SizedBox::new()
        .width(width)
        .child(Row::new([Expanded::new(gap(1., 1.)).into(), child]))
        .into()
}

fn left_cell(width: f32, child: Widget) -> Widget {
    SizedBox::new().width(width).child(child).into()
}

fn toolbar(
    data: &PerformanceData,
    shared: &Shared,
    bridge: &ClientBridge,
    tick: &Signal<u64>,
) -> Widget {
    let mode_options = [
        (DevtoolsProfilerMode::Basic, "Basic"),
        (DevtoolsProfilerMode::Performance, "Timings"),
        (DevtoolsProfilerMode::Deep, "Deep"),
    ]
    .into_iter()
    .map(|(mode, label)| {
        let mode_bridge = bridge.clone();
        let callback: Box<dyn Fn()> =
            Box::new(move || mode_bridge.send_or_report(RequestMethod::SetProfilerMode { mode }));
        (label.to_string(), data.profiler_mode == mode, callback)
    })
    .collect::<Vec<_>>();

    let recording = data.recording;
    let record_bridge = bridge.clone();
    let record_shared = shared.clone();
    let record_tick = tick.clone();
    let record: Widget = RawMaterialButton::new(if recording {
        "Stop recording"
    } else {
        "Start recording"
    })
    .size(Size::new(0., 32.))
    .padding(EdgeInsets::symmetric(16., 6.))
    .label_style(super::shared::text_style(13., on_accent()).font_weight(FontWeight::W600))
    .color(if recording { danger() } else { primary() })
    .hover_color(if recording {
        danger_hover()
    } else {
        primary_hover()
    })
    .on_press(move || {
        // Reflect the toggle immediately so the label never lags the target's reply.
        if let Ok(mut state) = record_shared.lock() {
            state.recording = !recording;
        }
        bump(&record_tick);
        record_bridge.send_or_report(if recording {
            RequestMethod::StopRecording
        } else {
            RequestMethod::StartRecording
        })
    })
    .into();

    let clear_shared = shared.clone();
    let clear_tick = tick.clone();
    let clear = compact_button("Clear recording", false, move || {
        if let Ok(mut state) = clear_shared.lock() {
            state.frames.clear();
            state.deep_traces.clear();
            state.deep_trace_events = 0;
            state.timeline_offset = 0;
            state.selected_frame = None;
            state.selected_range = None;
            state.range_anchor = None;
        }
        bump(&clear_tick);
    });

    let mut quiet = Vec::new();
    for (label, visible_delta, offset_delta) in [
        ("Zoom in", -12_isize, 0_isize),
        ("Zoom out", 12, 0),
        ("Older frames", 0, 12),
        ("Newer frames", 0, -12),
    ] {
        let timeline_shared = shared.clone();
        let timeline_tick = tick.clone();
        quiet.push(quiet_button(label, false, true, move || {
            if let Ok(mut state) = timeline_shared.lock() {
                let current = if state.timeline_visible == 0 {
                    DEFAULT_VISIBLE_FRAMES
                } else {
                    state.timeline_visible
                };
                state.timeline_visible = current.saturating_add_signed(visible_delta).clamp(6, 300);
                state.timeline_offset = state
                    .timeline_offset
                    .saturating_add_signed(offset_delta)
                    .min(state.frames.len().saturating_sub(1));
            }
            bump(&timeline_tick);
        }));
    }

    let primary = Row::new([
        segmented_control(mode_options),
        gap(12., 1.),
        record,
        gap(8., 1.),
        clear,
    ])
    .main_axis_size(MainAxisSize::Min);
    let secondary = Row::new(
        quiet
            .into_iter()
            .flat_map(|button| [button, gap(2., 1.)])
            .collect::<Vec<_>>(),
    )
    .main_axis_size(MainAxisSize::Min);
    let primary: Widget = primary.into();
    let secondary: Widget = secondary.into();
    panel(
        Padding::new(
            EdgeInsets::symmetric(14., 12.),
            fill_width(move |width| {
                if width > 920. {
                    Row::new([
                        primary.clone(),
                        Expanded::new(gap(1., 1.)).into(),
                        secondary.clone(),
                    ])
                    .into()
                } else {
                    Wrap::new([primary.clone(), secondary.clone()])
                        .spacing(28.)
                        .run_spacing(10.)
                        .into()
                }
            }),
        )
        .into(),
    )
}

fn stats_row(data: &PerformanceData) -> Widget {
    let statistics = data.statistics;
    let empty = statistics.samples == 0;
    Row::new([
        Expanded::new(metric(
            "FPS",
            data.fps.map_or_else(|| "—".into(), |v| format!("{v:.1}")),
            &format!("{} frames retained", statistics.samples),
            if data.fps.is_some() {
                primary()
            } else {
                text_muted()
            },
        ))
        .into(),
        gap(12., 1.),
        Expanded::new(metric(
            "AVERAGE CPU",
            if empty {
                "—".into()
            } else {
                format!("{:.2} ms", statistics.average_us / 1000.)
            },
            "Mean over retained frames",
            text_primary(),
        ))
        .into(),
        gap(12., 1.),
        Expanded::new(metric(
            "P95 CPU",
            if empty {
                "—".into()
            } else {
                format!("{:.2} ms", statistics.p95_us as f64 / 1000.)
            },
            "Nearest-rank 95th percentile",
            text_primary(),
        ))
        .into(),
        gap(12., 1.),
        Expanded::new(metric(
            "JANK",
            statistics
                .jank_percent()
                .map_or_else(|| "—".into(), |value| format!("{value:.1}%")),
            &format!(
                "{} of {} frames over budget",
                statistics.over_budget, statistics.budgeted_samples
            ),
            if statistics.over_budget > 0 {
                danger()
            } else {
                primary()
            },
        ))
        .into(),
    ])
    .into()
}

fn chart_legend(has_budget: bool) -> Widget {
    let mut items = Vec::new();
    for phase in FramePhase::ALL {
        items.push(legend_swatch(phase_color(phase), phase.label()));
    }
    items.push(legend_swatch(gpu_color(), "GPU raster"));
    items.push(legend_swatch(danger(), "Over budget"));
    if has_budget {
        items.push(legend_swatch(warning(), "Frame budget"));
    }
    Wrap::new(items).spacing(16.).run_spacing(6.).into()
}

fn frame_chart(
    frames: Vec<FrameRecordEvent>,
    selected: Option<(DevWindowId, u64)>,
    shared: Shared,
    tick: Signal<u64>,
) -> Widget {
    LayoutBuilder::new(move |_, constraints| {
        let width = constraints.max_width().clamp(100., 4000.);
        let fit = ((width / BAR_STEP) as usize).max(1);
        let skip = frames.len().saturating_sub(fit);
        let visible = &frames[skip..];
        let budget = visible
            .last()
            .and_then(|frame| frame.budget_us)
            .filter(|budget| *budget > 0);
        let max = visible
            .iter()
            .map(|frame| {
                stacked_total(&frame.timings).max(frame.timings.gpu_us.unwrap_or(0.) as u32)
            })
            .max()
            .unwrap_or(1)
            .max(budget.unwrap_or(1000)) as f32
            * 1.12;
        let scale = PLOT_HEIGHT / max;
        let origin = width - visible.len() as f32 * BAR_STEP;

        let mut canvas = Canvas::default();
        for fraction in [0., 0.5, 1.] {
            canvas.rect(rect(0., (PLOT_HEIGHT - 1.) * fraction, width, 1.), border());
        }
        if let Some(budget) = budget {
            let y = PLOT_HEIGHT - budget as f32 * scale;
            for x in (0..width as usize).step_by(8) {
                canvas.rect(rect(x as f32, y, 4., 1.5), warning());
            }
        }
        for (index, frame) in visible.iter().enumerate() {
            let x = origin + index as f32 * BAR_STEP;
            let janky = is_janky(frame);
            let mut top = PLOT_HEIGHT;
            for segment in phase_segments(&frame.timings) {
                if segment.micros == 0 {
                    continue;
                }
                let height = (segment.micros as f32 * scale).max(1.);
                top -= height;
                canvas.rect(
                    rect(x, top.max(0.), BAR_WIDTH, height),
                    phase_color(segment.phase),
                );
            }
            if janky {
                // Cap and outline the individual bar only.
                let cap_top = (top.max(0.) - 3.).max(0.);
                canvas.rect(rect(x, cap_top, BAR_WIDTH, 3.), danger());
                canvas.border(
                    incular::rendering::RRect::uniform(
                        rect(
                            x - 0.5,
                            cap_top - 0.5,
                            BAR_WIDTH + 1.,
                            PLOT_HEIGHT - cap_top + 0.5,
                        ),
                        1.,
                    ),
                    incular::rendering::Border::new(1., danger()),
                );
            }
            if let Some(gpu) = frame.timings.gpu_us.filter(|gpu| *gpu > 0.) {
                let height = (gpu as f32 * scale).max(1.);
                canvas.rect(
                    rect(x + BAR_WIDTH, PLOT_HEIGHT - height, GPU_WIDTH, height),
                    gpu_color(),
                );
            }
            if selected == Some((frame.window, frame.frame)) {
                canvas.border(
                    incular::rendering::RRect::uniform(rect(x - 1., 0., BAR_STEP, PLOT_HEIGHT), 2.),
                    incular::rendering::Border::new(1.5, text_primary()),
                );
            }
        }

        let mut layers: Vec<Widget> =
            vec![CustomPaint::new(Size::new(width, PLOT_HEIGHT), canvas.finish()).into()];
        for (index, frame) in visible.iter().enumerate() {
            let x = origin + index as f32 * BAR_STEP;
            let key = (frame.window, frame.frame);
            let shared = shared.clone();
            let tick = tick.clone();
            let button: Widget = RawMaterialButton::new(format!("Frame #{}", frame.frame))
                .size(Size::new(BAR_STEP, PLOT_HEIGHT))
                .padding(EdgeInsets::symmetric(0., 0.))
                .content(Widget::box_(Size::new(1., 1.), Color::TRANSPARENT))
                .color(Color::TRANSPARENT)
                .hover_color(hover_overlay())
                .pressed_color(pressed_overlay())
                .on_press(move || {
                    if let Ok(mut state) = shared.lock() {
                        state.selected_frame = Some(key);
                        if let Some(anchor) = state.range_anchor.take() {
                            state.selected_range = Some((anchor, key.1));
                        }
                    }
                    bump(&tick);
                })
                .into();
            layers.push(
                Positioned::new(button)
                    .left(x - 1.)
                    .top(0.)
                    .width(BAR_STEP)
                    .height(PLOT_HEIGHT)
                    .into(),
            );
        }
        SizedBox::from_size(Size::new(width, PLOT_HEIGHT))
            .child(Stack::new(layers))
            .into()
    })
    .into()
}

fn empty_chart(recording: bool, mode: DevtoolsProfilerMode) -> Widget {
    panel(
        Padding::all(
            24.,
            column([
                heading("Waiting for frames", 15.),
                gap(1., 8.),
                ui_text(
                    "Frames arrive as the inspected application redraws. Interact with it \
                     (move the pointer, scroll, type) or run an animation to start collecting samples.",
                    12.,
                    text_muted(),
                ),
                gap(1., 6.),
                ui_text(
                    if recording {
                        "Recording is on. Frames appear as the application redraws, so interact with it or run an animation."
                    } else if mode == DevtoolsProfilerMode::Basic {
                        "Press Start recording. Basic collects frame counts only; choose Timings for phase timings or Deep for per-widget traces."
                    } else {
                        "Press Start recording, then interact with the application to capture frames."
                    },
                    12.,
                    text_muted(),
                ),
            ]),
        )
        .into(),
    )
}

fn table_header(columns: Vec<Widget>) -> Widget {
    Padding::new(EdgeInsets::symmetric(0., 4.), Row::new(columns)).into()
}

fn rule() -> Widget {
    fill_width(|width| Widget::box_(Size::new(width, 1.), border()))
}

fn phase_row(label: &str, color: Color, micros: u32, total: u32, bar_max: u32) -> Widget {
    let percent = if total > 0 {
        format!("{:.0}%", micros as f32 * 100. / total as f32)
    } else {
        "—".into()
    };
    Padding::new(
        EdgeInsets::symmetric(0., 4.),
        Row::new([
            left_cell(
                122.,
                Row::new([
                    Widget::box_(Size::new(10., 10.), color),
                    gap(8., 1.),
                    ui_text(label, 12., text_primary()),
                ])
                .into(),
            ),
            right_cell(
                78.,
                mono(format_micros(u64::from(micros)), 12., text_primary()),
            ),
            right_cell(64., mono(percent, 12., text_muted())),
            gap(14., 1.),
            Expanded::new(LayoutBuilder::new(move |_, constraints| {
                let width = constraints.max_width().clamp(10., 4000.);
                let filled = if bar_max > 0 {
                    width * micros as f32 / bar_max as f32
                } else {
                    0.
                };
                let mut canvas = Canvas::default();
                canvas.rect(rect(0., 3., width, 6.), surface_raised());
                canvas.rect(rect(0., 3., filled.min(width), 6.), color);
                CustomPaint::new(Size::new(width, 12.), canvas.finish()).into()
            }))
            .into(),
        ]),
    )
    .into()
}

fn frame_details(frame: &FrameRecordEvent, selected: bool) -> Widget {
    let timings = &frame.timings;
    let segments = phase_segments(&frame.timings);
    let bar_max = segments
        .iter()
        .map(|segment| segment.micros)
        .max()
        .unwrap_or(1);
    let janky = is_janky(frame);

    let mut phase_rows = vec![table_header(vec![
        left_cell(122., ui_text("PHASE", 11., text_muted())),
        right_cell(78., ui_text("TIME", 11., text_muted())),
        right_cell(64., ui_text("% FRAME", 11., text_muted())),
    ])];
    phase_rows.push(rule());
    for segment in segments {
        phase_rows.push(phase_row(
            segment.phase.label(),
            phase_color(segment.phase),
            segment.micros,
            timings.cpu_total,
            bar_max,
        ));
    }
    phase_rows.push(rule());
    phase_rows.push(
        Padding::new(
            EdgeInsets::symmetric(0., 5.),
            Row::new([
                left_cell(122., ui_text("CPU total", 12., text_primary())),
                right_cell(
                    78.,
                    mono(
                        format_micros(u64::from(timings.cpu_total)),
                        12.,
                        text_primary(),
                    ),
                ),
                right_cell(
                    64.,
                    mono(
                        percent_of_frame(timings.cpu_total, timings)
                            .map_or_else(|| String::from("—"), |_| String::from("100%")),
                        12.,
                        text_muted(),
                    ),
                ),
            ]),
        )
        .into(),
    );

    let status = if frame.budget_us.is_none_or(|budget| budget == 0) {
        ("No budget reported", text_muted())
    } else if janky {
        ("Janky, over budget", danger())
    } else {
        ("Within budget", primary())
    };
    let counters = column([
        key_value(
            "Budget",
            frame
                .budget_us
                .filter(|budget| *budget > 0)
                .map_or_else(|| "unavailable".into(), |b| format_micros(u64::from(b))),
        ),
        key_value("Draw calls", frame.draw_calls.to_string()),
        key_value("Instances", frame.instances.to_string()),
        key_value("Upload bytes", frame.upload_bytes.to_string()),
        key_value("Pipelines", frame.pipelines_created.to_string()),
        gap(1., 8.),
        ui_text("RENDERER", 11., text_muted()),
        key_value("Prepare", format_micros(u64::from(timings.prepare))),
        key_value("Encode", format_micros(u64::from(timings.encode))),
        key_value("Submit", format_micros(u64::from(timings.submit))),
        key_value(
            "GPU",
            timings
                .gpu_us
                .map_or_else(|| "unavailable".into(), |gpu| format_micros(gpu as u64)),
        ),
    ]);

    let title = format!(
        "Frame #{} {}",
        frame.frame,
        if selected { "· selected" } else { "· latest" }
    );
    let heading_row = Row::new([
        heading(title, 14.),
        gap(10., 1.),
        super::shared::badge(status.0, status.1),
    ]);
    panel(
        Padding::all(
            16.,
            column([
                heading_row.into(),
                gap(1., 12.),
                fill_width(move |width| {
                    let wide = width > 760.;
                    if wide {
                        Row::new([
                            Expanded::new(column(phase_rows.clone())).into(),
                            gap(28., 1.),
                            SizedBox::new().width(280.).child(counters.clone()).into(),
                        ])
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .into()
                    } else {
                        column([column(phase_rows.clone()), gap(1., 14.), counters.clone()])
                    }
                }),
            ]),
        )
        .into(),
    )
}

fn flamegraph(boxes: Vec<FlameBox>, names: HashMap<DevWidgetId, String>) -> Widget {
    let depth = boxes.iter().map(|item| item.depth).max().unwrap_or(0);
    let height = (f32::from(depth) + 1.) * 22.;
    fill_width(move |width| {
        let scale = width / 900.;
        let mut canvas = Canvas::default();
        let mut layers: Vec<Widget> = Vec::new();
        let mut labels = 0;
        for item in &boxes {
            let x = item.x * scale;
            let w = (item.width * scale - 1.).max(1.);
            let y = f32::from(item.depth) * 22.;
            canvas.rect(rect(x, y, w, 20.), phase_color(trace_phase(item.phase)));
            if w >= 44. && labels < 160 {
                labels += 1;
                let name = widget_label(&names, item.node);
                layers.push(
                    Positioned::new(
                        SizedBox::new().width(w - 8.).child(
                            Text::new(format!(
                                "{name}  {}",
                                format_micros(u64::from(item.duration_us))
                            ))
                            .style(super::shared::text_style(11., background()))
                            .max_lines(Some(1))
                            .overflow(TextOverflow::Ellipsis),
                        ),
                    )
                    .left(x + 4.)
                    .top(y + 3.)
                    .into(),
                );
            }
        }
        let mut stack = vec![CustomPaint::new(Size::new(width, height), canvas.finish()).into()];
        stack.extend(layers);
        SizedBox::from_size(Size::new(width, height))
            .child(Stack::new(stack))
            .into()
    })
}

fn ranked_table(
    ranked: Vec<RankedTrace>,
    names: &HashMap<DevWidgetId, String>,
    bridge: &ClientBridge,
    shared: &Shared,
    tick: &Signal<u64>,
) -> Widget {
    let mut rows = vec![
        table_header(vec![
            Expanded::new(ui_text("WIDGET", 11., text_muted())).into(),
            left_cell(96., ui_text("PHASE", 11., text_muted())),
            right_cell(86., ui_text("TOTAL", 11., text_muted())),
            right_cell(60., ui_text("COUNT", 11., text_muted())),
        ]),
        rule(),
    ];
    for entry in ranked.into_iter().take(24) {
        let name = widget_label(names, entry.node);
        let known = names.contains_key(&entry.node);
        let phase = trace_phase(entry.phase);
        let entry_bridge = bridge.clone();
        let entry_shared = shared.clone();
        let entry_tick = tick.clone();
        let node = entry.node;
        let label = format!("{name} {} {}", phase.label(), format_micros(entry.total_us));
        let content = Row::new([
            Expanded::new(Row::new([
                ui_text(name, 12., text_primary()),
                gap(8., 1.),
                ui_text(
                    if known {
                        format!("#{}", node.index())
                    } else {
                        String::new()
                    },
                    11.,
                    text_muted(),
                ),
            ]))
            .into(),
            left_cell(
                96.,
                Row::new([
                    Widget::box_(Size::new(8., 8.), phase_color(phase)),
                    gap(6., 1.),
                    ui_text(phase.label(), 12., text_muted()),
                ])
                .into(),
            ),
            right_cell(
                86.,
                mono(format_micros(entry.total_us), 12., text_primary()),
            ),
            right_cell(60., mono(entry.count.to_string(), 12., text_muted())),
        ]);
        rows.push(fill_width(move |width| {
            let entry_bridge = entry_bridge.clone();
            let entry_shared = entry_shared.clone();
            let entry_tick = entry_tick.clone();
            RawMaterialButton::new(label.clone())
                .size(Size::new(width, 28.))
                .padding(EdgeInsets::symmetric(6., 4.))
                .content(content.clone())
                .color(Color::TRANSPARENT)
                .hover_color(control())
                .on_press(move || {
                    let window = if let Ok(mut state) = entry_shared.lock() {
                        state.selected = Some(node);
                        state.active_window
                    } else {
                        None
                    };
                    entry_bridge.send_or_report(RequestMethod::GetNodeDetails { id: node });
                    if let Some(window) = window {
                        entry_bridge.send_or_report(RequestMethod::HighlightNode {
                            window,
                            id: Some(node),
                        });
                    }
                    bump(&entry_tick);
                })
                .into()
        }));
    }
    column(rows)
}

pub(crate) fn build_performance(
    data: PerformanceData,
    shared: Shared,
    bridge: ClientBridge,
    tick: Signal<u64>,
) -> Widget {
    let toolbar_widget = toolbar(&data, &shared, &bridge, &tick);
    let stats = stats_row(&data);

    let PerformanceData {
        frames,
        profiler_mode,
        recording,
        flame_boxes,
        ranked,
        trace_status,
        selected_frame_event,
        widget_names,
        selected_frame,
        trace_range,
        selected_range,
        ..
    } = data;

    // The snapshot carries newest-first frames; the chart reads oldest to newest.
    let chronological = frames.into_iter().rev().collect::<Vec<_>>();
    let has_budget = chronological
        .iter()
        .any(|frame| frame.budget_us.is_some_and(|budget| budget > 0));

    let chart: Widget = if chronological.is_empty() {
        empty_chart(recording, profiler_mode)
    } else {
        let worst = chronological
            .iter()
            .map(|frame| stacked_total(&frame.timings))
            .max()
            .unwrap_or(0);
        let budget = chronological
            .last()
            .and_then(|frame| frame.budget_us)
            .filter(|budget| *budget > 0);
        let caption = format!(
            "{} frames shown · worst {}{} · click a bar to inspect a frame",
            chronological.len(),
            format_micros(u64::from(worst)),
            budget.map_or_else(String::new, |budget| format!(
                " · budget {}",
                format_micros(u64::from(budget))
            )),
        );
        column([
            ui_text(caption, 12., text_muted()),
            gap(1., 10.),
            chart_legend(has_budget),
            gap(1., 14.),
            frame_chart(chronological, selected_frame, shared.clone(), tick.clone()),
        ])
    };
    let chart_section = section(
        "Frame timeline",
        "CPU time per frame, stacked by phase. Newest frame on the right.",
        chart,
    );

    let details = selected_frame_event
        .as_ref()
        .map(|frame| frame_details(frame, selected_frame.is_some()));

    // Deep trace section.
    let range_options = [
        (TraceRange::CurrentFrame, "Current frame"),
        (TraceRange::SelectedRange, "Selected range"),
        (TraceRange::EntireRecording, "Entire recording"),
    ]
    .into_iter()
    .map(|(range, label)| {
        let rank_shared = shared.clone();
        let rank_tick = tick.clone();
        let callback: Box<dyn Fn()> = Box::new(move || {
            if let Ok(mut state) = rank_shared.lock() {
                state.trace_range = range;
            }
            bump(&rank_tick);
        });
        (label.to_string(), trace_range == range, callback)
    })
    .collect::<Vec<_>>();
    let range_shared = shared.clone();
    let range_tick = tick.clone();
    let range_button = quiet_button(
        if selected_range.is_some() {
            "Reset range start"
        } else {
            "Set range start"
        },
        selected_range.is_some(),
        true,
        move || {
            if let Ok(mut state) = range_shared.lock() {
                state.range_anchor = state.selected_frame.map(|(_, frame)| frame);
                state.selected_range = None;
            }
            bump(&range_tick);
        },
    );

    let mut deep = Vec::new();
    if flame_boxes.is_empty() && ranked.is_empty() {
        deep.push(ui_text(
            if profiler_mode == DevtoolsProfilerMode::Deep {
                if recording {
                    "Recording: per-widget traces will appear as frames are drawn."
                } else {
                    "Press Start recording to capture per-widget traces."
                }
            } else {
                "Choose the Deep profiler mode and press Start recording to capture per-widget build, layout and paint timings."
            },
            12.,
            text_muted(),
        ));
    } else {
        deep.push(ui_text(
            trace_status.unwrap_or_else(|| "Deep trace".into()),
            12.,
            text_muted(),
        ));
        deep.push(gap(1., 10.));
        deep.push(flamegraph(flame_boxes, widget_names.clone()));
        deep.push(gap(1., 20.));
        deep.push(
            Wrap::new([
                heading("Expensive work", 13.),
                segmented_control(range_options),
                range_button,
            ])
            .spacing(14.)
            .run_spacing(8.)
            .into(),
        );
        deep.push(gap(1., 10.));
        deep.push(ranked_table(ranked, &widget_names, &bridge, &shared, &tick));
    }
    let deep_section = section(
        "Deep trace",
        "Per-widget timings for the selected frame, widest work first.",
        column(deep),
    );

    let mut page = vec![
        page_title(
            "Performance",
            "Record frames, find expensive work, and inspect render phases.",
        ),
        toolbar_widget,
        gap(1., 14.),
        stats,
        gap(1., 14.),
        chart_section,
        gap(1., 14.),
    ];
    if let Some(details) = details {
        page.push(details);
        page.push(gap(1., 14.));
    }
    page.push(deep_section);
    column(page)
}
