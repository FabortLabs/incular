mod application;
mod connection;
mod console;
mod geometry;
mod header;
mod hover;
mod inspection;
mod memory;
mod network;
mod overlays;
mod overview;
mod performance;
mod property_editor;
mod rendering;
mod shared;
mod shell;
pub mod theme;
mod tree;
mod widget_inspector;

pub use self::shared::{
    accent_text, background, border, compact_button, control, control_active, control_focus,
    control_hover, danger, danger_hover, danger_wash, gap, gpu_color, hover_overlay, on_accent,
    pressed_overlay, primary, primary_hover, section, success, surface, surface_raised, text_muted,
    text_primary, ui_text, violet, warning,
};
pub(crate) use self::shell::initial_tool_view;
pub use self::shell::{ShellData, Shortcut, ToolView, build_shell, shortcut_for};
pub use self::theme::{Palette, Theme};

use crate::{
    inspector::{ConnectionState, InspectorSection, Shared},
    performance::{TraceRange, flamegraph_boxes, rank_traces},
    transport::ClientBridge,
};
use incular::prelude::*;
use incular::widgets::internal::{ScrollController, TextEditingController};
use incular_devtools_protocol::{
    DevWidgetId, DevWindowId, DevtoolsProfilerMode, FrameRecordEvent, NodeDetails, RequestMethod,
    SignalSubscriber, SignalSummary, TargetInfo, WindowSummary,
};
use std::{cell::Cell, collections::HashSet, rc::Rc, sync::Arc};

struct ViewSnapshot {
    header: String,
    supports_inspection: bool,
    target_protocol: Option<u32>,
    diagnostics: crate::diagnostics::DiagnosticsHistory,
    work_counts: Vec<incular_devtools_protocol::WorkCountEntry>,
    connection: ConnectionState,
    target_info: Option<TargetInfo>,
    connected: bool,
    windows: Vec<WindowSummary>,
    active_window: Option<DevWindowId>,
    row_count: usize,
    select_mode: bool,
    hover_highlight: bool,
    has_selection: bool,
    details: Vec<String>,
    selected_details: Option<NodeDetails>,
    frames: Vec<FrameRecordEvent>,
    statistics: crate::performance::FrameStatistics,
    fps: Option<f32>,
    live_fps: crate::performance::LiveFps,
    spark: Vec<u32>,
    overlay_count: usize,
    picking: bool,
    console_entries: Vec<crate::inspector::ConsoleEntry>,
    signals: Vec<SignalSummary>,
    selected_signal: Option<SignalSummary>,
    signal_subscribers: Vec<SignalSubscriber>,
    profiler_mode: DevtoolsProfilerMode,
    recording: bool,
    flame_boxes: Vec<crate::performance::FlameBox>,
    ranked: Vec<crate::performance::RankedTrace>,
    trace_status: Option<String>,
    selected_frame_event: Option<FrameRecordEvent>,
    widget_names: std::collections::HashMap<DevWidgetId, String>,
    selected_frame: Option<(DevWindowId, u64)>,
    trace_range: TraceRange,
    selected_range: Option<(u64, u64)>,
    error: Option<String>,
    notice: Option<String>,
}

pub(crate) fn create_application(
    shared: Shared,
    bridge: ClientBridge,
    options: WindowOptions,
) -> Result<Application, incular::runtime::WindowError> {
    let tick = Signal::new(0_u64);
    let update_subscription_started = Rc::new(Cell::new(false));
    let search = TextEditingController::new();
    let console_filter = TextEditingController::new();
    let signal_value = TextEditingController::new();
    let property_editors = property_editor::PropertyEditors::default();
    let tool_view = Signal::new(initial_tool_view());
    let theme = Signal::new(theme::initial_theme());
    let failures_only = Signal::new(false);
    let inspector_section = Signal::new(InspectorSection::Properties);
    let visual_tools_open = Signal::new(false);
    let overlays_open = Signal::new(false);
    let overlays_scroll = ScrollController::new();
    let property_filter = TextEditingController::new();
    let collapsed_groups = Signal::new(HashSet::new());
    let tree_focus = FocusNode::new();
    let keyboard_focus = FocusNode::new();
    let tree_split = Signal::new(0.59_f32);
    let last_reveal = Rc::new(Cell::new(None::<(DevWindowId, DevWidgetId, u64)>));
    let last_pick = Rc::new(Cell::new(0_u64));
    let tree_scroll = ScrollController::new();
    let semantics_search = TextEditingController::new();
    let layers_search = TextEditingController::new();
    let semantics_scroll = ScrollController::new();
    let semantics_details_scroll = ScrollController::new();
    let layers_scroll = ScrollController::new();
    let layers_details_scroll = ScrollController::new();
    let last_view_key = Rc::new(Cell::new(None::<(ToolView, bool)>));
    let inspector_scroll = ToolView::ALL.map(|_| ScrollController::new());
    let app_shared = Arc::clone(&shared);
    let app_bridge = bridge.clone();
    let app_tick = tick.clone();
    let app_update_subscription_started = update_subscription_started.clone();
    let app_search = search.clone();
    let app_console_filter = console_filter.clone();
    let app_signal_value = signal_value.clone();
    let app_property_editors = property_editors.clone();
    let app_tool_view = tool_view.clone();
    let app_theme = theme.clone();
    let app_inspector_section = inspector_section.clone();
    let app_tree_scroll = tree_scroll.clone();
    // A row scrolled out from under a still pointer never reports an exit, so
    // any tree scroll drops the hover overlay.
    let scroll_subscription = tree_scroll.add_listener({
        let shared = app_shared.clone();
        let bridge = app_bridge.clone();
        move |_| {
            hover::clear(&shared, &bridge);
            false
        }
    });
    let app_inspector_scroll = inspector_scroll.clone();
    let synchronize_inspector = incular::runtime::Effect::new({
        let shared = app_shared.clone();
        let tick = app_tick.clone();
        let search = app_search.clone();
        let section = app_inspector_section.clone();
        let property_editors = app_property_editors.clone();
        let view = app_tool_view.clone();
        let scroll = app_tree_scroll.clone();
        let workspace_focus = keyboard_focus.clone();
        let tree_focus = tree_focus.clone();
        move || {
            let _ = tick.get();
            let _ = section.get();
            property_editors.flush_pending();
            let (query, picked_revision, selection) = {
                let state = shared.lock().expect("inspector state");
                let selection = state
                    .active_window
                    .zip(state.selected)
                    .and_then(|(window, id)| {
                        state
                            .rows
                            .iter()
                            .position(|row| row.id == id)
                            .map(|index| ((window, id, state.selection_revision), index))
                    });
                (state.search.clone(), state.picked_revision, selection)
            };
            if search.text() != query {
                search.set_text(query);
            }
            if last_pick.replace(picked_revision) != picked_revision {
                view.set(ToolView::Widgets);
                workspace_focus.unfocus();
                tree_focus.request_focus();
            }
            if let Some((selection, index)) = selection
                && last_reveal.get() != Some(selection)
            {
                let y = index as f32 * tree::ROW_HEIGHT;
                let viewport = scroll.viewport_extent();
                let offset = scroll.offset();
                if y < offset || y + tree::ROW_HEIGHT > offset + viewport {
                    scroll.deferred_jump_to((y - viewport * 0.3).max(0.));
                }
                last_reveal.set(Some(selection));
            }
        }
    });
    Application::new_with_options(options, move |cx| -> Widget {
        let _keep_scroll_subscription = &scroll_subscription;
        let root: Widget = {
            // Only this tiny UI-thread Signal changes. The model remains shared
            // and bounded, avoiding full tree copies every DevTools repaint.
            let _ = app_tick.get();
            // Colours are read while widgets are built, so select the palette first.
            let active_theme = app_theme.get();
            theme::set_current(active_theme);
            synchronize_inspector.mount();
            let active_view = app_tool_view.get();
            let active_inspector_section = app_inspector_section.get();
            if !app_update_subscription_started.replace(true) {
                let wait_updates = Arc::clone(&app_bridge.updates);
                let next_updates = Arc::clone(&app_bridge.updates);
                let tick = app_tick.clone();
                let next_tick = app_tick.clone();
                let scope = cx.task_scope();
                let next_scope = scope.clone();
                cx.spawn_into(
                    async move {
                        wait_updates.notified().await;
                    },
                    move |updated, runtime| {
                        if updated.is_ok() {
                            tick.update(|value| {
                                *value = value
                                    .checked_add(1)
                                    .expect("DevTools UI revision exhausted");
                            });
                            arm_model_update_wait(runtime, next_scope, next_updates, next_tick);
                        }
                    },
                );
            }
            let snapshot = {
                let mut state = app_shared.lock().expect("inspector state");
                state.poll_rendering = active_view == ToolView::Rendering;
                let header = state.connection_headline(std::time::Instant::now()).1;
                let target_info = state.target_info.clone();
                let visible_frames = if state.timeline_visible == 0 {
                    crate::views::performance::DEFAULT_VISIBLE_FRAMES
                } else {
                    state.timeline_visible.max(6)
                };
                let frames = state
                    .frames
                    .iter()
                    .filter(|frame| {
                        active_view == ToolView::Application
                            || Some(frame.window) == state.active_window
                    })
                    .rev()
                    .skip(if active_view == ToolView::Application {
                        0
                    } else {
                        state.timeline_offset
                    })
                    .take(if active_view == ToolView::Application {
                        crate::inspector::InspectorModel::FRAME_HISTORY
                    } else {
                        visible_frames
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let statistics = crate::performance::FrameStatistics::from_frames(
                    state
                        .frames
                        .iter()
                        .filter(|frame| Some(frame.window) == state.active_window),
                );
                let now = std::time::Instant::now();
                let live_fps = state.live_fps(state.active_window, now);
                state.live_fps_changed(now);
                let spark = state.recent_cpu_micros(state.active_window, 60);
                let fps = (!live_fps.idle).then_some(live_fps.fps as f32);
                let console_entries = state.filtered_console();
                let selected_trace = state
                    .selected_frame
                    .and_then(|(window, frame)| {
                        state
                            .deep_traces
                            .iter()
                            .find(|trace| trace.window == window && trace.frame == frame)
                    })
                    .or_else(|| {
                        state
                            .deep_traces
                            .iter()
                            .rev()
                            .find(|trace| Some(trace.window) == state.active_window)
                    });
                let flame_boxes = if active_view == ToolView::Performance {
                    selected_trace
                        .map(|trace| flamegraph_boxes(trace, None, 900.))
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                let ranked = if active_view == ToolView::Performance {
                    rank_traces(
                        state.deep_traces.iter().filter(|trace| {
                            Some(trace.window) == state.active_window
                                && match state.trace_range {
                                    TraceRange::CurrentFrame => {
                                        selected_trace.is_some_and(|selected| {
                                            selected.frame == trace.frame
                                                && selected.window == trace.window
                                        })
                                    }
                                    TraceRange::SelectedRange => {
                                        state.selected_range.is_some_and(|(start, end)| {
                                            (start.min(end)..=start.max(end)).contains(&trace.frame)
                                        })
                                    }
                                    TraceRange::EntireRecording => true,
                                }
                        }),
                        None,
                    )
                } else {
                    Vec::new()
                };
                let trace_status = selected_trace.map(|trace| {
                    format!(
                        "Deep frame #{} · {} events{}",
                        trace.frame,
                        trace.events.len(),
                        if trace.truncated {
                            format!(" · truncated, {} dropped", trace.dropped_events)
                        } else {
                            String::new()
                        }
                    )
                });
                let selected_frame_event = state
                    .selected_frame
                    .and_then(|(window, selected)| {
                        state
                            .frames
                            .iter()
                            .find(|frame| frame.window == window && frame.frame == selected)
                    })
                    .or_else(|| state.latest_frame(state.active_window))
                    .cloned();
                let widget_names = if active_view == ToolView::Performance {
                    let mut names = std::collections::HashMap::new();
                    let wanted = flame_boxes
                        .iter()
                        .map(|item| item.node)
                        .chain(ranked.iter().take(24).map(|entry| entry.node));
                    for id in wanted {
                        if let Some(node) = state.nodes.get(&id) {
                            names.insert(id, node.type_name.clone());
                        }
                    }
                    names
                } else {
                    std::collections::HashMap::new()
                };
                let (diagnostics, work_counts) = if active_view == ToolView::Rendering {
                    (state.diagnostics.clone(), state.work_counts.clone())
                } else {
                    Default::default()
                };
                ViewSnapshot {
                    header,
                    supports_inspection: state.supports_inspection(),
                    target_protocol: state
                        .target_protocol
                        .or(state.target_info.as_ref().map(|info| info.protocol_version)),
                    diagnostics,
                    work_counts,
                    connection: state.connection,
                    target_info,
                    connected: state.connected,
                    windows: state.windows.clone(),
                    active_window: state.active_window,
                    row_count: state.rows.len(),
                    select_mode: state.select_mode,
                    hover_highlight: state.hover_highlight_enabled(),
                    has_selection: state.selected.is_some(),
                    details: state.details_lines(active_inspector_section),
                    selected_details: state.details.clone(),
                    frames,
                    statistics,
                    fps,
                    live_fps,
                    spark,
                    overlay_count: state.debug_options.len(),
                    picking: state.select_mode,
                    console_entries,
                    signals: state.signals.clone(),
                    selected_signal: state.selected_signal.and_then(|id| {
                        state.signals.iter().find(|signal| signal.id == id).cloned()
                    }),
                    signal_subscribers: state.signal_subscribers.clone(),
                    profiler_mode: state.profiler_mode,
                    recording: state.recording,
                    flame_boxes,
                    ranked,
                    trace_status,
                    selected_frame_event,
                    widget_names,
                    selected_frame: state.selected_frame,
                    trace_range: state.trace_range,
                    selected_range: state.selected_range,
                    error: state.error.clone(),
                    notice: state.notice.clone(),
                }
            };

            let mut search_field = gap(1., 1.);
            let mut tree_list = gap(1., 1.);
            let mut inspector_toolbar = gap(1., 1.);
            let mut tree_controls = gap(1., 1.);
            let mut inspector_header = gap(1., 1.);
            let mut inspector_tabs = gap(1., 1.);
            let mut tree_breadcrumbs = gap(1., 1.);
            let mut tree_status = String::new();
            let ready = snapshot.connected && snapshot.supports_inspection;
            let key = (active_view, ready);
            if let Some(previous) = last_view_key.replace(Some(key))
                && previous != key
            {
                if previous.0 != active_view {
                    // Leaving a view must not strand a target overlay or a
                    // picker that swallows the target's clicks.
                    hover::clear(&app_shared, &app_bridge);
                    hover::cancel_pick(&app_shared, &app_bridge);
                }
                if ready {
                    match active_view {
                        ToolView::Rendering => {
                            rendering::request_refresh(&app_bridge, snapshot.active_window)
                        }
                        ToolView::Semantics => inspection::request_refresh(
                            &app_bridge,
                            inspection::Kind::Semantics,
                            snapshot.active_window,
                        ),
                        ToolView::Layers => inspection::request_refresh(
                            &app_bridge,
                            inspection::Kind::Layers,
                            snapshot.active_window,
                        ),
                        _ => {}
                    }
                }
            }
            let offline_workspace = !snapshot.connected
                && !matches!(
                    active_view,
                    ToolView::Console | ToolView::Network | ToolView::Application
                );
            let inspection_kind = match active_view {
                ToolView::Semantics => Some(inspection::Kind::Semantics),
                ToolView::Layers => Some(inspection::Kind::Layers),
                _ => None,
            };
            let full_bleed = ready && inspection_kind.is_some() && !offline_workspace;
            let page_content = if offline_workspace {
                connection::build_connection(
                    Arc::clone(&app_shared),
                    app_bridge.clone(),
                    app_tick.clone(),
                )
            } else if let Some(kind) = inspection_kind {
                if ready {
                    let (search, scroll, details_scroll) = match kind {
                        inspection::Kind::Semantics => (
                            semantics_search.clone(),
                            semantics_scroll.clone(),
                            semantics_details_scroll.clone(),
                        ),
                        inspection::Kind::Layers => (
                            layers_search.clone(),
                            layers_scroll.clone(),
                            layers_details_scroll.clone(),
                        ),
                    };
                    inspection::build_inspection(
                        kind,
                        inspection::InspectionContext {
                            shared: Arc::clone(&app_shared),
                            bridge: app_bridge.clone(),
                            tick: app_tick.clone(),
                            search,
                            scroll,
                            details_scroll,
                            split: tree_split.clone(),
                            window: snapshot.active_window,
                        },
                    )
                } else {
                    rendering::newer_target_note(
                        match kind {
                            inspection::Kind::Semantics => "Semantics",
                            inspection::Kind::Layers => "Layers",
                        },
                        snapshot.target_protocol,
                    )
                }
            } else {
                match active_view {
                    ToolView::Semantics | ToolView::Layers => unreachable!("handled above"),
                    ToolView::Rendering => rendering::build_rendering(
                        rendering::RenderingData {
                            supported: snapshot.supports_inspection,
                            protocol: snapshot.target_protocol,
                            history: snapshot.diagnostics.clone(),
                            work_counts: snapshot.work_counts.clone(),
                            window: snapshot.active_window,
                        },
                        Arc::clone(&app_shared),
                        app_bridge.clone(),
                        app_tick.clone(),
                        app_tool_view.clone(),
                    ),
                    ToolView::Overview => overview::build_overview(
                        Arc::clone(&app_shared),
                        app_bridge.clone(),
                        app_tool_view.clone(),
                    ),
                    ToolView::Widgets => {
                        let inspector = widget_inspector::build_inspector(
                            widget_inspector::InspectorData {
                                windows: snapshot.windows.clone(),
                                active_window: snapshot.active_window,
                                select_mode: snapshot.select_mode,
                                hover_highlight: snapshot.hover_highlight,
                                details: snapshot.details.clone(),
                                selected_details: snapshot.selected_details.clone(),
                                has_selection: snapshot.has_selection,
                                signals: snapshot.signals.clone(),
                                selected_signal: snapshot.selected_signal.clone(),
                                signal_subscribers: snapshot.signal_subscribers.clone(),
                            },
                            widget_inspector::InspectorBuildContext {
                                shared: Arc::clone(&app_shared),
                                bridge: app_bridge.clone(),
                                tick: app_tick.clone(),
                                search: app_search.clone(),
                                signal_value: app_signal_value.clone(),
                                property_editors: app_property_editors.clone(),
                                inspector_section: app_inspector_section.clone(),
                                visual_tools_open: visual_tools_open.clone(),
                                tree_scroll: app_tree_scroll.clone(),
                                tree_focus: tree_focus.clone(),
                                workspace_focus: keyboard_focus.clone(),
                                property_filter: property_filter.clone(),
                                collapsed_groups: collapsed_groups.clone(),
                            },
                        );
                        search_field = inspector.search_field;
                        tree_list = inspector.tree_list;
                        inspector_toolbar = inspector.toolbar;
                        inspector_header = inspector.header;
                        inspector_tabs = inspector.tabs;
                        tree_breadcrumbs = inspector.breadcrumbs;
                        tree_controls = inspector.tree_controls;
                        tree_status = inspector.tree_status;
                        inspector.content
                    }
                    ToolView::Console => console::build_console(
                        snapshot.console_entries,
                        Arc::clone(&app_shared),
                        app_tick.clone(),
                        app_console_filter.clone(),
                    ),
                    ToolView::Network => network::build_network(
                        Arc::clone(&app_shared),
                        app_tick.clone(),
                        failures_only.clone(),
                    ),
                    ToolView::Memory => {
                        memory::build_memory(Arc::clone(&app_shared), app_bridge.clone())
                    }
                    ToolView::Application => application::build_application(
                        snapshot.target_info,
                        snapshot.connection,
                        snapshot.windows,
                        snapshot.frames,
                        app_bridge.clone(),
                    ),
                    ToolView::Performance => performance::build_performance(
                        performance::PerformanceData {
                            statistics: snapshot.statistics,
                            fps: snapshot.fps,
                            frames: snapshot.frames,
                            profiler_mode: snapshot.profiler_mode,
                            recording: snapshot.recording,
                            flame_boxes: snapshot.flame_boxes,
                            ranked: snapshot.ranked,
                            trace_status: snapshot.trace_status,
                            selected_frame_event: snapshot.selected_frame_event,
                            widget_names: snapshot.widget_names,
                            selected_frame: snapshot.selected_frame,
                            trace_range: snapshot.trace_range,
                            selected_range: snapshot.selected_range,
                        },
                        Arc::clone(&app_shared),
                        app_bridge.clone(),
                        app_tick.clone(),
                    ),
                }
            };
            let active_window = snapshot.active_window;
            let connected = snapshot.connected;
            let do_refresh: Rc<dyn Fn()> = {
                let refresh_bridge = app_bridge.clone();
                Rc::new(move || {
                    if connected {
                        refresh_bridge.send_or_report(RequestMethod::GetTargetInfo);
                        refresh_bridge.send_or_report(RequestMethod::ListSignals);
                        if let Some(window) = active_window {
                            refresh_bridge.send_or_report(RequestMethod::GetWidgetTree { window });
                        }
                    } else {
                        refresh_bridge.retry();
                    }
                })
            };
            let do_export: Rc<dyn Fn()> = {
                let export_shared = Arc::clone(&app_shared);
                let export_tick = app_tick.clone();
                Rc::new(move || {
                    let report = export_shared.lock().map(|state| state.diagnostic_report());
                    if let Ok(report) = report {
                        let result = crate::report::save(&report);
                        if let Ok(mut state) = export_shared.lock() {
                            state.notice = Some(match result {
                                Ok(path) => format!("Report saved: {}", path.display()),
                                Err(error) => {
                                    state.push_console(
                                        "error",
                                        "devtools::export",
                                        error.to_string(),
                                    );
                                    format!("Export failed: {error}")
                                }
                            });
                        }
                        shared::refresh(&export_tick);
                    }
                })
            };
            let refresh_action =
                compact_button(if connected { "Refresh" } else { "Reconnect" }, false, {
                    let do_refresh = do_refresh.clone();
                    move || do_refresh()
                });
            let export_action = compact_button("Export report", false, {
                let do_export = do_export.clone();
                move || do_export()
            });
            let theme_action = compact_button(active_theme.switch_label(), false, {
                let theme = app_theme.clone();
                move || theme.update(|value| *value = value.toggled())
            });
            let fps_pill =
                header::fps_pill(snapshot.live_fps, &snapshot.spark, snapshot.connected, {
                    let view = app_tool_view.clone();
                    move || {
                        let _ = view.set(ToolView::Performance);
                    }
                });
            let picking_pill = snapshot.picking.then(|| {
                let (shared, bridge, tick) =
                    (app_shared.clone(), app_bridge.clone(), app_tick.clone());
                header::picking_pill(move || {
                    hover::pick(&shared, &bridge, &tick, Some(false));
                })
            });
            let overlays_action =
                header::overlays_button(snapshot.overlay_count, overlays_open.get(), {
                    let open = overlays_open.clone();
                    let focus = keyboard_focus.clone();
                    move || {
                        // Escape closes the menu, so the shortcut handler must own focus.
                        focus.request_focus();
                        open.update(|value| *value = !*value)
                    }
                });
            let overlays_menu = overlays_open.get().then(|| shell::OverlaysMenu {
                panel: overlays::overlays_panel(&app_shared, &app_bridge, &app_tick, true),
                scroll: overlays_scroll.clone(),
                close: {
                    let open = overlays_open.clone();
                    Rc::new(move || {
                        let _ = open.set(false);
                    })
                },
            });
            let shell = shell::build_shell(shell::ShellData {
                active_view,
                tool_view: app_tool_view.clone(),
                header: snapshot.header,
                connected: snapshot.connected,
                connection: snapshot.connection,
                full_bleed,
                row_count: snapshot.row_count,
                search_field,
                tree_list,
                inspector_toolbar,
                inspector_header,
                inspector_tabs,
                tree_breadcrumbs,
                tree_controls,
                tree_status,
                tree_split: tree_split.clone(),
                inspector_scroll: app_inspector_scroll[active_view as usize].clone(),
                page_content,
                refresh_action,
                theme_action,
                fps_pill,
                picking_pill,
                overlays_action,
                overlays_menu,
                export_action,
                notice: snapshot.notice.or(snapshot.error),
            });
            let keyboard_shared = app_shared.clone();
            let keyboard_bridge = app_bridge.clone();
            let keyboard_tick = app_tick.clone();
            let keyboard_view = app_tool_view.clone();
            let keyboard_refresh = do_refresh.clone();
            let keyboard_overlays = overlays_open.clone();
            let keyboard_export = do_export.clone();
            let keyboard: Widget = KeyboardListener::new(shell)
                .focus_node(keyboard_focus.clone())
                .autofocus(true)
                .on_key(move |event| {
                    if !event.state.is_down() || event.repeat {
                        return false;
                    }
                    match shortcut_for(event.code, event.modifiers) {
                        Some(Shortcut::PickWidget) => {
                            keyboard_view.set(ToolView::Widgets);
                            return tree::inspect_mode(
                                &keyboard_shared,
                                &keyboard_bridge,
                                &keyboard_tick,
                                None,
                            );
                        }
                        Some(Shortcut::View(view)) => {
                            keyboard_view.set(view);
                            return true;
                        }
                        Some(Shortcut::NextView) => {
                            keyboard_view.update(|view| *view = view.cycled(1));
                            return true;
                        }
                        Some(Shortcut::PreviousView) => {
                            keyboard_view.update(|view| *view = view.cycled(-1));
                            return true;
                        }
                        Some(Shortcut::Refresh) => {
                            keyboard_refresh();
                            return true;
                        }
                        Some(Shortcut::Export) => {
                            keyboard_export();
                            return true;
                        }
                        None => {}
                    }
                    if event.code == Code::Escape && keyboard_overlays.get() {
                        keyboard_overlays.set(false);
                        return true;
                    }
                    if event.code == Code::Escape
                        && keyboard_shared.lock().is_ok_and(|state| state.select_mode)
                    {
                        hover::pick(
                            &keyboard_shared,
                            &keyboard_bridge,
                            &keyboard_tick,
                            Some(false),
                        );
                        return true;
                    }
                    false
                })
                .into();
            let mut control_theme = incular::controls::ControlTheme::default();
            control_theme.typography.body = shared::text_style(12., text_primary());
            incular::controls::ControlThemeScope::new(control_theme, keyboard).into()
        };
        Directionality::new(TextDirection::Ltr, root).into()
    })
}

fn arm_model_update_wait(
    runtime: &mut Runtime,
    scope: TaskScope,
    updates: Arc<tokio::sync::Notify>,
    tick: Signal<u64>,
) {
    if scope.is_cancelled() {
        return;
    }
    let wait_updates = Arc::clone(&updates);
    let next_updates = updates;
    let next_scope = scope.clone();
    let next_tick = tick.clone();
    runtime.spawner().spawn_into_in(
        &scope,
        async move {
            wait_updates.notified().await;
        },
        move |updated, runtime| {
            if updated.is_ok() {
                tick.update(|value| {
                    *value = value
                        .checked_add(1)
                        .expect("DevTools UI revision exhausted");
                });
                arm_model_update_wait(runtime, next_scope, next_updates, next_tick);
            }
        },
    );
}
