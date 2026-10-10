mod application;
mod console;
mod geometry;
mod memory;
mod network;
mod overview;
mod performance;
mod shared;
mod shell;
mod tree;
mod widget_inspector;

pub use self::shared::{
    APP_BACKGROUND, BORDER, CONTROL, CONTROL_ACTIVE, DANGER, PRIMARY, SUCCESS, SURFACE, TEXT_MUTED,
    TEXT_PRIMARY, compact_button, gap, section, ui_text,
};
pub(crate) use self::shell::initial_tool_view;
pub use self::shell::{ShellData, ToolView, build_shell};

use crate::{
    inspector::{ConnectionState, InspectorSection, Shared},
    performance::{TraceRange, flamegraph_boxes, rank_traces},
    transport::ClientBridge,
};
use incular::prelude::*;
use incular::widgets::internal::{ScrollController, TextEditingController};
use incular_devtools_protocol::{
    DebugOption, DevWidgetId, DevWindowId, DevtoolsProfilerMode, FrameRecordEvent, NodeDetails,
    RequestMethod, SignalSubscriber, SignalSummary, TargetInfo, WindowSummary,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
    sync::Arc,
};

struct ViewSnapshot {
    header: String,
    connection: ConnectionState,
    target_info: Option<TargetInfo>,
    connected: bool,
    windows: Vec<WindowSummary>,
    active_window: Option<DevWindowId>,
    row_count: usize,
    select_mode: bool,
    has_selection: bool,
    details: Vec<String>,
    selected_details: Option<NodeDetails>,
    frames: Vec<FrameRecordEvent>,
    statistics: crate::performance::FrameStatistics,
    fps: Option<f32>,
    latest_frame: Option<FrameRecordEvent>,
    console_entries: Vec<crate::inspector::ConsoleEntry>,
    signals: Vec<SignalSummary>,
    selected_signal: Option<SignalSummary>,
    signal_subscribers: Vec<SignalSubscriber>,
    debug_options: HashSet<DebugOption>,
    animation_scale: f32,
    profiler_mode: DevtoolsProfilerMode,
    recording: bool,
    flame_boxes: Vec<crate::performance::FlameBox>,
    ranked: Vec<crate::performance::RankedTrace>,
    trace_status: Option<String>,
    selected_frame_detail: Option<String>,
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
    let property_value = TextEditingController::new();
    let property_binding = Rc::new(RefCell::new(None::<(DevWidgetId, String)>));
    let tool_view = Signal::new(initial_tool_view());
    let failures_only = Signal::new(false);
    let inspector_section = Signal::new(InspectorSection::Properties);
    let visual_tools_open = Signal::new(false);
    let property_filter = TextEditingController::new();
    let collapsed_groups = Signal::new(HashSet::new());
    let tree_focus = FocusNode::new();
    let keyboard_focus = FocusNode::new();
    let tree_split = Signal::new(0.59_f32);
    let last_reveal = Rc::new(Cell::new(None::<(DevWindowId, DevWidgetId, u64)>));
    let last_pick = Rc::new(Cell::new(0_u64));
    let tree_scroll = ScrollController::new();
    let inspector_scroll = ToolView::ALL.map(|_| ScrollController::new());
    let app_shared = Arc::clone(&shared);
    let app_bridge = bridge.clone();
    let app_tick = tick.clone();
    let app_update_subscription_started = update_subscription_started.clone();
    let app_search = search.clone();
    let app_console_filter = console_filter.clone();
    let app_signal_value = signal_value.clone();
    let app_property_value = property_value.clone();
    let app_property_binding = property_binding.clone();
    let app_tool_view = tool_view.clone();
    let app_inspector_section = inspector_section.clone();
    let app_tree_scroll = tree_scroll.clone();
    let app_inspector_scroll = inspector_scroll.clone();
    let synchronize_inspector = incular::runtime::Effect::new({
        let shared = app_shared.clone();
        let tick = app_tick.clone();
        let search = app_search.clone();
        let property_value = app_property_value.clone();
        let property_binding = app_property_binding.clone();
        let section = app_inspector_section.clone();
        let view = app_tool_view.clone();
        let scroll = app_tree_scroll.clone();
        let workspace_focus = keyboard_focus.clone();
        let tree_focus = tree_focus.clone();
        move || {
            let _ = tick.get();
            let section = section.get();
            let binding = property_binding.borrow().clone();
            let (query, picked_revision, selection, editable) = {
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
                let editable = if section == InspectorSection::Properties {
                    state.details.as_ref().and_then(|details| {
                        details
                            .properties
                            .iter()
                            .filter(|property| property.editable)
                            .find(|property| {
                                binding.as_ref().is_some_and(|(id, name)| {
                                    *id == details.id && *name == property.name
                                })
                            })
                            .or_else(|| {
                                details.properties.iter().find(|property| property.editable)
                            })
                            .map(|property| {
                                (
                                    (details.id, property.name.clone()),
                                    crate::inspector::debug_value(&property.value),
                                )
                            })
                    })
                } else {
                    None
                };
                (
                    state.search.clone(),
                    state.picked_revision,
                    selection,
                    editable,
                )
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
            if let Some((next_binding, value)) = editable
                && binding.as_ref() != Some(&next_binding)
            {
                property_value.set_text(value);
                *property_binding.borrow_mut() = Some(next_binding);
            }
        }
    });
    Application::new_with_options(options, move |cx| {
        // Only this tiny UI-thread Signal changes. The model remains shared
        // and bounded, avoiding full tree copies every DevTools repaint.
        let _ = app_tick.get();
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
            let state = app_shared.lock().expect("inspector state");
            let header = if state.connected {
                format!("Connected to {}", state.target)
            } else {
                format!("DevTools {}", state.connection.label())
            };
            let target_info = state.target_info.clone();
            let visible_frames = state.timeline_visible.max(6);
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
            let fps = state.fps(state.active_window);
            let latest_frame = state.latest_frame(state.active_window).cloned();
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
            let selected_frame_detail = state
                    .selected_frame
                    .and_then(|(window, selected)| {
                        state
                            .frames
                            .iter()
                            .find(|frame| frame.window == window && frame.frame == selected)
                    })
                    .map(|frame| {
                        format!(
                            "Frame #{} · CPU {}µs · budget {} · input {} runtime {} BUILD {} LAYOUT {} PAINT {} SEMANTICS {} COMPOSITE {} · renderer prepare {} encode {} submit {} · GPU {} · draws {} instances {} uploads {}B",
                            frame.frame,
                            frame.timings.cpu_total,
                            frame
                                .budget_us
                                .map_or_else(|| "unavailable".into(), |value| format!("{value}µs")),
                            frame.timings.event_processing,
                            frame.timings.runtime_messages,
                            frame.timings.build,
                            frame.timings.layout,
                            frame.timings.paint,
                            frame.timings.semantics,
                            frame.timings.composite,
                            frame.timings.prepare,
                            frame.timings.encode,
                            frame.timings.submit,
                            frame
                                .timings
                                .gpu_us
                                .map_or_else(|| "unavailable".into(), |value| format!("{value:.0}µs")),
                            frame.draw_calls,
                            frame.instances,
                            frame.upload_bytes,
                        )
                    });
            ViewSnapshot {
                header,
                connection: state.connection,
                target_info,
                connected: state.connected,
                windows: state.windows.clone(),
                active_window: state.active_window,
                row_count: state.rows.len(),
                select_mode: state.select_mode,
                has_selection: state.selected.is_some(),
                details: state.details_lines(active_inspector_section),
                selected_details: state.details.clone(),
                frames,
                statistics,
                fps,
                latest_frame,
                console_entries,
                signals: state.signals.clone(),
                selected_signal: state
                    .selected_signal
                    .and_then(|id| state.signals.iter().find(|signal| signal.id == id).cloned()),
                signal_subscribers: state.signal_subscribers.clone(),
                debug_options: state.debug_options.clone(),
                animation_scale: state.animation_scale.unwrap_or(1.),
                profiler_mode: state.profiler_mode,
                recording: state.recording,
                flame_boxes,
                ranked,
                trace_status,
                selected_frame_detail,
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
        let page_content = match active_view {
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
                        debug_options: snapshot.debug_options.clone(),
                        animation_scale: snapshot.animation_scale,
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
                        property_value: app_property_value.clone(),
                        property_binding: app_property_binding.clone(),
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
            ToolView::Memory => memory::build_memory(Arc::clone(&app_shared), app_bridge.clone()),
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
                    active_window: snapshot.active_window,
                    frames: snapshot.frames,
                    latest_frame: snapshot.latest_frame,
                    profiler_mode: snapshot.profiler_mode,
                    recording: snapshot.recording,
                    flame_boxes: snapshot.flame_boxes,
                    ranked: snapshot.ranked,
                    trace_status: snapshot.trace_status,
                    selected_frame_detail: snapshot.selected_frame_detail,
                    selected_frame: snapshot.selected_frame,
                    trace_range: snapshot.trace_range,
                    selected_range: snapshot.selected_range,
                },
                Arc::clone(&app_shared),
                app_bridge.clone(),
                app_tick.clone(),
            ),
        };
        let refresh_bridge = app_bridge.clone();
        let active_window = snapshot.active_window;
        let connected = snapshot.connected;
        let refresh_action = compact_button(
            if connected { "Refresh" } else { "Reconnect" },
            false,
            move || {
                if connected {
                    refresh_bridge.send_or_report(RequestMethod::GetTargetInfo);
                    refresh_bridge.send_or_report(RequestMethod::ListSignals);
                    if let Some(window) = active_window {
                        refresh_bridge.send_or_report(RequestMethod::GetWidgetTree { window });
                    }
                } else {
                    refresh_bridge.retry();
                }
            },
        );
        let export_shared = Arc::clone(&app_shared);
        let export_tick = app_tick.clone();
        let export_action = compact_button("Export report", false, move || {
            let report = export_shared.lock().map(|state| state.diagnostic_report());
            if let Ok(report) = report {
                let result = crate::report::save(&report);
                if let Ok(mut state) = export_shared.lock() {
                    state.notice = Some(match result {
                        Ok(path) => format!("Report saved: {}", path.display()),
                        Err(error) => {
                            state.push_console("error", "devtools::export", error.to_string());
                            format!("Export failed: {error}")
                        }
                    });
                }
                shared::refresh(&export_tick);
            }
        });
        let shell = shell::build_shell(shell::ShellData {
            active_view,
            tool_view: app_tool_view.clone(),
            header: snapshot.header,
            connected: snapshot.connected,
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
            export_action,
            notice: snapshot.notice.or(snapshot.error),
        });
        let keyboard_shared = app_shared.clone();
        let keyboard_bridge = app_bridge.clone();
        let keyboard_tick = app_tick.clone();
        let keyboard_view = app_tool_view.clone();
        let keyboard: Widget = KeyboardListener::new(shell)
            .focus_node(keyboard_focus.clone())
            .autofocus(true)
            .on_key(move |event| {
                if !event.state.is_down() || event.repeat {
                    return false;
                }
                if event.code == Code::KeyC
                    && event
                        .modifiers
                        .contains(Modifiers::CONTROL | Modifiers::SHIFT)
                {
                    keyboard_view.set(ToolView::Widgets);
                    return tree::inspect_mode(
                        &keyboard_shared,
                        &keyboard_bridge,
                        &keyboard_tick,
                        None,
                    );
                }
                if event.code == Code::Escape
                    && keyboard_shared.lock().is_ok_and(|state| state.select_mode)
                {
                    return tree::inspect_mode(
                        &keyboard_shared,
                        &keyboard_bridge,
                        &keyboard_tick,
                        Some(false),
                    );
                }
                false
            })
            .into();
        let mut theme = incular::controls::ControlTheme::default();
        theme.typography.body = shared::text_style(12., TEXT_PRIMARY);
        incular::controls::ControlThemeScope::new(theme, keyboard).into()
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
