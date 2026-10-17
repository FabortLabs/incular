//! Rendering diagnostics: text cache, widget pipeline, semantics, compositor
//! and the per-widget rebuild ranking.

use super::shared::{
    badge, column, heading, mono, page_title, panel, refresh, text_style, warning,
};
use super::theme::accent_text;
use super::{
    ToolView, border, compact_button, danger, gap, primary, section, text_muted, text_primary,
    ui_text,
};
use crate::{
    diagnostics::{
        CacheHealth, DiagnosticsHistory, cache_health, format_count, format_rate, hit_rate,
    },
    inspector::Shared,
    transport::ClientBridge,
};
use incular::{material::RawMaterialButton, prelude::*};
use incular_devtools_protocol::{
    DevWidgetId, RequestMethod, RuntimeDiagnosticsWire, WorkCountEntry,
};

const TOP_REBUILDS: u32 = 50;

pub(crate) struct RenderingData {
    pub(crate) supported: bool,
    pub(crate) protocol: Option<u32>,
    pub(crate) history: DiagnosticsHistory,
    pub(crate) work_counts: Vec<WorkCountEntry>,
    pub(crate) window: Option<incular_devtools_protocol::DevWindowId>,
}

/// Sends the two requests behind this view.
pub(crate) fn request_refresh(
    bridge: &ClientBridge,
    window: Option<incular_devtools_protocol::DevWindowId>,
) {
    if let Some(window) = window {
        bridge.send_or_report(RequestMethod::GetRuntimeDiagnostics { window });
        bridge.send_or_report(RequestMethod::GetWorkCounts {
            window,
            limit: TOP_REBUILDS,
        });
    }
}

/// One counter line: label, cumulative value and optional per-second change.
fn counter_row(label: &str, value: u64, rate: Option<f64>, warn: bool) -> Widget {
    Padding::new(
        EdgeInsets::symmetric(0., 4.),
        Row::new([
            Widget::from(Expanded::new(ui_text(label, 12., text_muted()))),
            mono(
                format_count(value),
                12.,
                if warn && value > 0 {
                    danger()
                } else {
                    text_primary()
                },
            ),
            SizedBox::from_size(Size::new(84., 16.))
                .child(Align::new(
                    Alignment::CENTER_RIGHT,
                    mono(
                        format_rate(rate),
                        11.,
                        if rate.is_some_and(|rate| rate >= 0.05) {
                            primary()
                        } else {
                            text_muted()
                        },
                    ),
                ))
                .into(),
        ]),
    )
    .into()
}

/// A hero figure on top of a card, in the style of the overview metrics.
fn hero(label: &str, value: impl Into<String>, detail: impl Into<String>, color: Color) -> Widget {
    column([
        ui_text(label, 10., text_muted()),
        gap(1., 4.),
        Text::new(value)
            .style(text_style(26., color).font_weight(FontWeight::W600))
            .into(),
        gap(1., 2.),
        ui_text(detail, 11., text_muted()),
        gap(1., 12.),
        Container::with_child(gap(1., 1.))
            .color(border())
            .height(1.)
            .into(),
        gap(1., 8.),
    ])
}

fn card(title: &str, description: &str, hero: Widget, rows: Vec<Widget>) -> Widget {
    section(
        title,
        description,
        Column::new([hero, column(rows)])
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .main_axis_size(MainAxisSize::Min)
            .into(),
    )
}

fn cards(wire: &RuntimeDiagnosticsWire, history: &DiagnosticsHistory) -> Widget {
    let rate = |f: fn(&RuntimeDiagnosticsWire) -> u64| history.rate(f);
    let text = &wire.text;
    let rate_value = hit_rate(text.cache_hits, text.cache_misses);
    let health = cache_health(text.cache_hits, text.cache_misses);
    let text_card = card(
        "Text layout cache",
        "Shaped paragraph reuse in the text engine",
        hero(
            "HIT RATE",
            rate_value.map_or_else(|| "–".into(), |value| format!("{value:.1}%")),
            {
                let lookups = format_count(text.cache_hits.saturating_add(text.cache_misses));
                if health == CacheHealth::Cold {
                    format!("{lookups} lookups · cold, too few to judge")
                } else {
                    format!("{lookups} lookups")
                }
            },
            match health {
                CacheHealth::Cold => text_primary(),
                CacheHealth::Poor(_) => danger(),
                CacheHealth::Fair(_) => warning(),
                CacheHealth::Good(_) => primary(),
            },
        ),
        vec![
            counter_row(
                "Cache hits",
                text.cache_hits,
                rate(|w| w.text.cache_hits),
                false,
            ),
            counter_row(
                "Cache misses",
                text.cache_misses,
                rate(|w| w.text.cache_misses),
                false,
            ),
            counter_row(
                "Reshaped paragraphs",
                text.paragraphs_reshaped,
                rate(|w| w.text.paragraphs_reshaped),
                false,
            ),
            counter_row(
                "Width reuses",
                text.width_reuses,
                rate(|w| w.text.width_reuses),
                false,
            ),
            counter_row(
                "Fallback runs",
                text.fallback_runs,
                rate(|w| w.text.fallback_runs),
                false,
            ),
            counter_row(
                "Missing clusters",
                text.missing_clusters,
                rate(|w| w.text.missing_clusters),
                true,
            ),
            counter_row(
                "Cache growths",
                text.cache_growths,
                rate(|w| w.text.cache_growths),
                false,
            ),
            counter_row(
                "Cache shrinks",
                text.cache_shrinks,
                rate(|w| w.text.cache_shrinks),
                false,
            ),
        ],
    );
    let widgets = &wire.widgets;
    let pipeline_card = card(
        "Widget pipeline",
        "Retained build, layout and paint work",
        hero(
            "ELEMENTS",
            format_count(widgets.elements_total),
            format!(
                "{} render objects · {} layers",
                format_count(widgets.render_objects_total),
                format_count(widgets.layers_total)
            ),
            text_primary(),
        ),
        vec![
            counter_row(
                "Rebuilds",
                widgets.rebuilds,
                rate(|w| w.widgets.rebuilds),
                false,
            ),
            counter_row(
                "Layouts",
                widgets.layouts,
                rate(|w| w.widgets.layouts),
                false,
            ),
            counter_row("Paints", widgets.paints, rate(|w| w.widgets.paints), false),
            counter_row(
                "Layout cache hits",
                widgets.layout_cache_hits,
                rate(|w| w.widgets.layout_cache_hits),
                false,
            ),
            counter_row(
                "Display lists reused",
                widgets.display_lists_reused,
                rate(|w| w.widgets.display_lists_reused),
                false,
            ),
            counter_row(
                "Reconciliation fast paths",
                widgets.reconciliation_fast_paths,
                rate(|w| w.widgets.reconciliation_fast_paths),
                false,
            ),
        ],
    );
    let semantics = &wire.semantics;
    let semantics_card = card(
        "Semantics",
        "Accessibility tree maintenance",
        hero(
            "NODES",
            format_count(semantics.nodes),
            "Live semantics nodes",
            text_primary(),
        ),
        vec![
            counter_row(
                "Node updates",
                semantics.nodes_updated,
                rate(|w| w.semantics.nodes_updated),
                false,
            ),
            counter_row(
                "Nodes created",
                semantics.nodes_created,
                rate(|w| w.semantics.nodes_created),
                false,
            ),
            counter_row(
                "Property updates",
                semantics.property_updates,
                rate(|w| w.semantics.property_updates),
                false,
            ),
            counter_row(
                "Geometry updates",
                semantics.geometry_updates,
                rate(|w| w.semantics.geometry_updates),
                false,
            ),
        ],
    );
    let compositor = &wire.compositor;
    let compositor_card = card(
        "Compositor",
        "Retained layer updates",
        hero(
            "LAYERS",
            format_count(compositor.layers),
            "Live layers",
            text_primary(),
        ),
        vec![
            counter_row(
                "Repainted pictures",
                compositor.picture_layers_repainted,
                rate(|w| w.compositor.picture_layers_repainted),
                false,
            ),
            counter_row(
                "Layers culled",
                compositor.layers_culled,
                rate(|w| w.compositor.layers_culled),
                false,
            ),
            counter_row(
                "Transform updates",
                compositor.transform_updates,
                rate(|w| w.compositor.transform_updates),
                false,
            ),
            counter_row(
                "Clip updates",
                compositor.clip_updates,
                rate(|w| w.compositor.clip_updates),
                false,
            ),
        ],
    );
    column([
        Row::new([
            Expanded::new(text_card).into(),
            gap(16., 1.),
            Expanded::new(pipeline_card).into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .into(),
        gap(1., 16.),
        Row::new([
            Expanded::new(semantics_card).into(),
            gap(16., 1.),
            Expanded::new(compositor_card).into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .into(),
    ])
}

fn rebuild_table(
    entries: &[WorkCountEntry],
    on_select: impl Fn(DevWidgetId) + Clone + 'static,
    on_hover: impl Fn(DevWidgetId, bool) + Clone + 'static,
) -> Widget {
    let widest = entries
        .iter()
        .map(|entry| entry.builds)
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let header_cell = |label: &str, width: f32| -> Widget {
        SizedBox::from_size(Size::new(width, 18.))
            .child(Align::new(
                Alignment::CENTER_RIGHT,
                ui_text(label, 10., text_muted()),
            ))
            .into()
    };
    let mut rows: Vec<Widget> = vec![
        Padding::new(
            EdgeInsets::symmetric(12., 0.),
            Row::new([
                Widget::from(Expanded::new(ui_text("WIDGET", 10., text_muted()))),
                header_cell("BUILDS", 170.),
                header_cell("LAYOUTS", 80.),
                header_cell("PAINTS", 80.),
            ]),
        )
        .into(),
        gap(1., 6.),
        Widget::box_(Size::new(1., 1.), border()),
    ];
    for entry in entries {
        let id = entry.id;
        let select = on_select.clone();
        let (hover_in, hover_out) = (on_hover.clone(), on_hover.clone());
        let bar = if entry.builds == 0 {
            0.
        } else {
            (entry.builds as f32 / widest * 90.).max(2.)
        };
        let cell = |value: u64, width: f32, color: Color| -> Widget {
            SizedBox::from_size(Size::new(width, 28.))
                .child(Align::new(
                    Alignment::CENTER_RIGHT,
                    mono(format_count(value), 12., color),
                ))
                .into()
        };
        let content: Widget = Padding::new(
            EdgeInsets::symmetric(12., 0.),
            Row::new([
                Widget::from(Expanded::new(Align::new(
                    Alignment::CENTER_LEFT,
                    Text::new(entry.type_name.clone())
                        .style(text_style(12., accent_text()).font_family("Consolas"))
                        .max_lines(Some(1))
                        .overflow(TextOverflow::Ellipsis),
                ))),
                SizedBox::from_size(Size::new(170., 28.))
                    .child(Row::new([
                        Widget::from(Expanded::new(Align::new(
                            Alignment::CENTER_RIGHT,
                            Widget::box_(Size::new(bar, 6.), {
                                let c = primary();
                                Color::rgba(c.red, c.green, c.blue, 140)
                            }),
                        ))),
                        gap(10., 1.),
                        SizedBox::from_size(Size::new(64., 28.))
                            .child(Align::new(
                                Alignment::CENTER_RIGHT,
                                mono(format_count(entry.builds), 12., primary()),
                            ))
                            .into(),
                    ]))
                    .into(),
                cell(entry.layouts, 80., text_primary()),
                cell(entry.paints, 80., text_primary()),
            ]),
        )
        .into();
        rows.push(
            RawMaterialButton::new(format!("Select {} in inspector", entry.type_name))
                .size(Size::new(0., 28.))
                .content(content)
                .color(Color::TRANSPARENT)
                .hover_color(super::theme::hover_overlay())
                .pressed_color(super::theme::pressed_overlay())
                .focused_color(super::theme::focus_ring())
                .on_press(move || select(id))
                .on_hover(move || hover_in(id, true))
                .on_exit(move || hover_out(id, false))
                .into(),
        );
    }
    Column::new(rows)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .main_axis_size(MainAxisSize::Min)
        .into()
}

/// Shown instead of a feature the connected target is too old to serve.
pub(crate) fn newer_target_note(feature: &str, protocol: Option<u32>) -> Widget {
    panel(
        Padding::all(
            18.,
            column([
                heading("Requires a newer target", 14.),
                gap(1., 6.),
                ui_text(
                    format!(
                        "{feature} need DevTools protocol 3; this target speaks protocol {}. Rebuild the application against a current Incular to use this view.",
                        protocol.map_or_else(|| "unknown".into(), |version| version.to_string())
                    ),
                    12.,
                    text_muted(),
                ),
            ]),
        )
        .into(),
    )
}

pub(crate) fn build_rendering(
    data: RenderingData,
    shared: Shared,
    bridge: ClientBridge,
    tick: Signal<u64>,
    tool_view: Signal<ToolView>,
) -> Widget {
    let mut body = vec![page_title(
        "Rendering",
        "Text cache, widget pipeline, semantics and compositor counters. Polled every second while this view is open.",
    )];
    if !data.supported {
        body.push(newer_target_note("Rendering diagnostics", data.protocol));
        return column(body);
    }
    let refresh_bridge = bridge.clone();
    let window = data.window;
    let reset_shared = shared.clone();
    let reset_tick = tick.clone();
    body.push(
        Row::new([
            compact_button("Refresh", false, move || {
                request_refresh(&refresh_bridge, window);
            }),
            gap(8., 1.),
            compact_button("Reset deltas", false, move || {
                if let Ok(mut state) = reset_shared.lock() {
                    state.diagnostics.reset_deltas();
                }
                refresh(&reset_tick);
            }),
            gap(12., 1.),
            badge("polling every 1 s", text_muted()),
        ])
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .into(),
    );
    body.push(gap(1., 16.));
    match data.history.latest() {
        Some(wire) => body.push(cards(wire, &data.history)),
        None => body.push(panel(
            Padding::all(
                18.,
                column([
                    heading("Waiting for the first sample", 14.),
                    gap(1., 6.),
                    ui_text(
                        "Counters appear as soon as the target answers.",
                        12.,
                        text_muted(),
                    ),
                ]),
            )
            .into(),
        )),
    }
    body.push(gap(1., 16.));
    let hover_shared = shared.clone();
    let hover_bridge = bridge.clone();
    let select_shared = shared;
    let select_bridge = bridge;
    let select_tick = tick;
    let table = if data.work_counts.is_empty() {
        ui_text("No rebuilds recorded yet.", 12., text_muted())
    } else {
        rebuild_table(
            &data.work_counts,
            move |id| {
                let known = select_shared
                    .lock()
                    .is_ok_and(|state| state.nodes.contains_key(&id));
                if known {
                    tool_view.set(ToolView::Widgets);
                    super::tree::select(&select_shared, &select_bridge, &select_tick, id);
                } else {
                    if let Ok(mut state) = select_shared.lock() {
                        state.notice =
                            Some("That widget is not in the loaded tree; refresh and retry".into());
                    }
                    refresh(&select_tick);
                }
            },
            move |id, entered| {
                if entered {
                    super::hover::enter_node(&hover_shared, &hover_bridge, id);
                } else {
                    super::hover::leave_node(&hover_shared, &hover_bridge, id);
                }
            },
        )
    };
    body.push(section(
        "Top rebuilds",
        "Elements ranked by build count. Select a row to inspect that widget.",
        table,
    ));
    column(body)
}
