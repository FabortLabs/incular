//! Runner-side DevTools glue: agent lifecycle, per-frame command draining,
//! debug overlays drawn OUTSIDE the widget tree (compositor-only), and
//! Select Widget input interception.

use crate::devtools_overlay::{self, FrameSample};
use incular_devtools::AgentHandle;
use incular_devtools::commands::UiCommand;
use incular_devtools_protocol::{
    DebugOption, DebugValue, DevWidgetId, DevWindowId, DevtoolsProfilerMode, ErrorCode,
    MemorySnapshot, RequestMethod, ResponsePayload, TargetEvent, TreeDelta, WidgetNode,
};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

type PhaseCounters = HashMap<DevWidgetId, [u64; 5]>;

/// How long a phase flash takes to fade out.
const FLASH_DURATION: Duration = Duration::from_millis(500);
/// Phases in draw order: build, layout, paint, semantics, composite.
const PHASE_OPTIONS: [DebugOption; 5] = [
    DebugOption::HighlightBuild,
    DebugOption::HighlightLayout,
    DebugOption::HighlightPaint,
    DebugOption::HighlightSemantics,
    DebugOption::HighlightComposite,
];
const PHASE_COLORS: [incular_core::Color; 5] = [
    incular_core::Color::rgba(248, 98, 88, 255),
    incular_core::Color::rgba(246, 194, 62, 255),
    incular_core::Color::rgba(114, 166, 255, 255),
    incular_core::Color::rgba(78, 212, 146, 255),
    incular_core::Color::rgba(170, 108, 244, 255),
];
const MAX_FLASHES: usize = 4_000;

/// One element whose phase counters recently advanced. Each phase fades on
/// its own clock so a later layout flash does not restart an older build one.
struct Flash {
    bounds: [f32; 4],
    started: [Option<Instant>; 5],
}

struct RainbowEntry {
    bounds: [f32; 4],
    generation: u8,
}

#[derive(Clone)]
struct PropertyOverride {
    original: DebugValue,
}

/// Overlay + inspect-mode state for the whole application.
#[derive(Default)]
pub struct DevToolsState {
    agent: Option<AgentHandle>,
    /// Stable per-session mapping protocol id -> platform window.
    window_map: std::collections::HashMap<DevWindowId, incular_platform::WindowId>,
    /// Window id currently in Select Widget mode.
    select_window: Option<incular_platform::WindowId>,
    /// Window whose pick gesture was consumed: pointer input stays captured
    /// until the matching release so the app never sees an unpaired Up.
    pick_capture: Option<incular_platform::WindowId>,
    /// Hovered element while selecting.
    hover: Option<(DevWidgetId, [f32; 4])>,
    hover_content: Option<[f32; 4]>,
    hover_label: Option<String>,
    highlight_label: Option<String>,
    /// Node highlighted from the DevTools tree (per window).
    highlight: Option<(DevWidgetId, [f32; 4])>,
    highlight_content: Option<[f32; 4]>,
    highlight_baseline: Option<f32>,
    highlight_clip: Option<[f32; 4]>,
    /// Window-space rectangle selected from the Semantics/Layers inspectors.
    highlight_rect: Option<(incular_platform::WindowId, [f32; 4])>,
    options: std::collections::BTreeSet<DebugOption>,
    /// Per-window bounded geometry cached for the diagnostic display-list
    /// pass. This lives outside the target Widget tree.
    layout_bounds: std::collections::HashMap<incular_platform::WindowId, Vec<[f32; 4]>>,
    hit_regions: std::collections::HashMap<incular_platform::WindowId, Vec<[f32; 4]>>,
    semantics_bounds: std::collections::HashMap<incular_platform::WindowId, Vec<[f32; 4]>>,
    scroll_viewports: std::collections::HashMap<incular_platform::WindowId, Vec<[f32; 4]>>,
    layer_bounds: std::collections::HashMap<incular_platform::WindowId, Vec<[f32; 4]>>,
    last_layout_bounds_sample: Option<Instant>,
    phase_counters: HashMap<incular_platform::WindowId, PhaseCounters>,
    phase_flashes: HashMap<incular_platform::WindowId, HashMap<DevWidgetId, Flash>>,
    repaint_rainbow: HashMap<incular_platform::WindowId, HashMap<DevWidgetId, RainbowEntry>>,
    /// Most recent presented frames per window for the performance overlay.
    frame_history: HashMap<incular_platform::WindowId, VecDeque<FrameSample>>,
    animation_speed: f32,
    frame_counter: u64,
    profiler_mode: DevtoolsProfilerMode,
    recording: bool,
    recorded_frames: u64,
    /// Only windows explicitly opened in the inspector pay for delta state.
    tree_subscriptions: std::collections::HashSet<DevWindowId>,
    tree_cache:
        std::collections::HashMap<DevWindowId, std::collections::HashMap<DevWidgetId, WidgetNode>>,
    last_tree_sample: Option<Instant>,
    property_overrides: std::collections::HashMap<(DevWidgetId, String), PropertyOverride>,
}

impl DevToolsState {
    pub fn new(agent: Option<AgentHandle>) -> Self {
        Self {
            agent,
            last_tree_sample: None,
            animation_speed: 1.,
            ..Self::default()
        }
    }

    pub fn is_selecting(&self, window: incular_platform::WindowId) -> bool {
        self.select_window == Some(window)
    }

    /// True while pointer input for `window` belongs to the picker: either
    /// Select Widget mode is active or a consumed pick has not been released.
    pub fn captures_pointer(&self, window: incular_platform::WindowId) -> bool {
        self.is_selecting(window) || self.pick_capture == Some(window)
    }

    /// Enters Select Widget mode for a platform window.
    pub fn begin_inspection(&mut self, window: incular_platform::WindowId) {
        self.select_window = Some(window);
        self.pick_capture = None;
    }

    /// Cancels the picker without dispatching the gesture to application widgets.
    pub fn cancel_inspection(&mut self, window: incular_platform::WindowId) -> bool {
        if self.pick_capture == Some(window) {
            self.pick_capture = None;
        }
        if !self.is_selecting(window) {
            return false;
        }
        self.select_window = None;
        self.hover = None;
        self.hover_content = None;
        self.hover_label = None;
        if let Some(window) = self
            .window_map
            .iter()
            .find_map(|(dev, native)| (*native == window).then_some(*dev))
        {
            let _ = self.push_frame(TargetEvent::InspectModeEnded { window });
        }
        true
    }

    /// Enables or disables one debug overlay.
    pub fn set_option(&mut self, option: DebugOption, enabled: bool) {
        if enabled {
            self.options.insert(option);
        } else {
            self.options.remove(&option);
        }
    }

    pub fn animation_speed(&self) -> f32 {
        self.animation_speed
    }

    /// Records a hovered candidate during Select Widget mode.
    pub fn note_hover(&mut self, id: DevWidgetId, bounds: [f32; 4]) {
        self.hover = Some((id, bounds));
    }

    /// Completes selection: emits the event and exits mode unless locked.
    pub fn complete_selection(
        &mut self,
        window: incular_platform::WindowId,
    ) -> Option<DevWidgetId> {
        if self.is_selecting(window) {
            self.select_window = None;
            self.hover.as_ref().map(|(id, _)| *id)
        } else {
            None
        }
    }

    /// Maps a generational protocol window id to its live platform window.
    fn resolve_window(
        &mut self,
        application: &incular_runtime::Application,
        id: DevWindowId,
    ) -> Option<incular_platform::WindowId> {
        if let Some(existing) = self.window_map.get(&id) {
            return Some(*existing);
        }
        let mapped = application.active_window_ids().into_iter().find(|window| {
            u64::from(window.index()) + 1 == id.index()
                && u64::from(window.generation()) == id.generation()
        })?;
        self.window_map.insert(id, mapped);
        Some(mapped)
    }

    /// Monotonic per-session frame counter for streamed records.
    pub fn next_frame(&mut self) -> u64 {
        self.frame_counter = self
            .frame_counter
            .checked_add(1)
            .expect("DevTools frame/tree revision exhausted");
        self.frame_counter
    }

    /// Drains a bounded amount of queued DevTools work on the UI thread.
    ///
    /// Returns true when another coalesced event-loop wake was scheduled.
    #[must_use]
    pub fn drain(&mut self, application: &mut incular_runtime::Application) -> bool {
        let Some(agent) = self.agent.as_ref().map(|agent| agent.commands.clone()) else {
            return false;
        };
        if let Some(handle) = &self.agent {
            handle.telemetry.forward_app_logs(256);
        }
        let mut repaint_overlays = false;
        if agent.take_session_cleanup() {
            self.reset_session(application);
            repaint_overlays = true;
        }
        const COMMAND_BUDGET: usize = 32;
        for _ in 0..COMMAND_BUDGET {
            let Some(command) = agent.try_next() else {
                break;
            };
            let UiCommand {
                body, completion, ..
            } = command;
            if completion.is_abandoned() {
                continue;
            }
            repaint_overlays |= matches!(
                &body,
                RequestMethod::StartInspectMode { .. }
                    | RequestMethod::StopInspectMode { .. }
                    | RequestMethod::HighlightNode { .. }
                    | RequestMethod::HighlightRect { .. }
                    | RequestMethod::SetDebugOption { .. }
            );
            let reply = match body {
                RequestMethod::StartInspectMode { window } => {
                    if let Some(platform_window) = self.resolve_window(application, window) {
                        self.begin_inspection(platform_window);
                        Ok(ResponsePayload::Ok)
                    } else {
                        Err(ErrorCode::UnknownId)
                    }
                }
                RequestMethod::StopInspectMode { window } => {
                    if let Some(platform_window) = self.resolve_window(application, window) {
                        self.cancel_inspection(platform_window);
                    }
                    Ok(ResponsePayload::Ok)
                }
                RequestMethod::GetTargetInfo => Ok(ResponsePayload::TargetInfo(Box::new(
                    incular_devtools::target_info(
                        env!("CARGO_PKG_VERSION"),
                        application.devtools_windows(),
                    ),
                ))),
                RequestMethod::GetWidgetTree { window } => {
                    match application.devtools_widget_tree(window) {
                        Ok(deltas) => {
                            self.tree_subscriptions.insert(window);
                            self.remember_tree(window, &deltas);
                            Ok(ResponsePayload::WidgetTree {
                                deltas,
                                tree_revision: self.frame_counter,
                            })
                        }
                        Err(code) => Err(code),
                    }
                }
                RequestMethod::GetNodeDetails { id } => match application.devtools_node_details(id)
                {
                    Ok(mut details) => {
                        for property in &mut details.properties {
                            property.overridden = self
                                .property_overrides
                                .contains_key(&(id, property.name.clone()));
                        }
                        Ok(ResponsePayload::NodeDetails(Box::new(details)))
                    }
                    Err(code) => Err(code),
                },
                RequestMethod::EditProperty { id, name, value } => {
                    let original = self
                        .property_overrides
                        .get(&(id, name.clone()))
                        .map(|entry| entry.original.clone())
                        .or_else(|| {
                            application
                                .devtools_node_details(id)
                                .ok()?
                                .properties
                                .into_iter()
                                .find(|property| property.name == name && property.editable)
                                .map(|property| property.value)
                        });
                    if let Some(original) = original
                        && application.devtools_edit_property(id, &name, &value)
                    {
                        self.property_overrides
                            .insert((id, name), PropertyOverride { original });
                        Ok(ResponsePayload::Edited)
                    } else {
                        Err(ErrorCode::Unsupported)
                    }
                }
                RequestMethod::HighlightNode { id, .. } => {
                    let geometry = id.and_then(|id| {
                        application
                            .devtools_node_overlay_geometry(id)
                            .map(|geometry| (id, geometry))
                    });
                    self.highlight_label = geometry.and_then(|(id, geometry)| {
                        let details = application.devtools_node_details(id).ok()?;
                        Some(chip_text(&details.type_name, geometry.bounds))
                    });
                    self.highlight = geometry.map(|(id, geometry)| (id, geometry.bounds));
                    self.highlight_content =
                        geometry.and_then(|(_, geometry)| geometry.content_bounds);
                    self.highlight_baseline =
                        geometry.and_then(|(_, geometry)| geometry.baseline_y);
                    self.highlight_clip = geometry.and_then(|(_, geometry)| geometry.clip_bounds);
                    Ok(ResponsePayload::Ok)
                }
                RequestMethod::HighlightRect { window, rect } => match rect {
                    None => {
                        self.highlight_rect = None;
                        Ok(ResponsePayload::Ok)
                    }
                    Some(rect) if rect.iter().all(|value| value.is_finite()) => {
                        match self.resolve_window(application, window) {
                            Some(platform_window) => {
                                self.highlight_rect = Some((platform_window, rect));
                                Ok(ResponsePayload::Ok)
                            }
                            None => Err(ErrorCode::UnknownId),
                        }
                    }
                    Some(_) => Err(ErrorCode::InvalidRequest),
                },
                RequestMethod::GetRuntimeDiagnostics { window } => application
                    .devtools_runtime_diagnostics(window)
                    .map(|wire| ResponsePayload::RuntimeDiagnostics(Box::new(wire))),
                RequestMethod::GetSemanticsTree { window } => application
                    .devtools_semantics_tree(window)
                    .map(|(nodes, truncated)| ResponsePayload::SemanticsTree {
                        window,
                        nodes,
                        truncated,
                    }),
                RequestMethod::GetLayerTree { window } => application
                    .devtools_layer_tree(window)
                    .map(|(layers, truncated)| ResponsePayload::LayerTree {
                        window,
                        layers,
                        truncated,
                    }),
                RequestMethod::GetWorkCounts { window, limit } => application
                    .devtools_work_counts(window, limit as usize)
                    .map(|entries| ResponsePayload::WorkCounts { window, entries }),
                RequestMethod::SetDebugOption { name, enabled } => {
                    self.set_option(name, enabled);
                    Ok(ResponsePayload::Ok)
                }
                RequestMethod::SetAnimationSpeed { scale } => {
                    self.animation_speed = if scale.is_finite() {
                        scale.clamp(0., incular_widgets::internal::MAX_ANIMATION_TIME_SCALE)
                    } else {
                        1.
                    };
                    application.set_animation_time_scale(self.animation_speed);
                    Ok(ResponsePayload::AnimationSpeedSet(self.animation_speed()))
                }
                RequestMethod::SetProfilerMode { mode } => {
                    self.profiler_mode = mode;
                    application.set_profiler_mode(match mode {
                        DevtoolsProfilerMode::Basic => incular_runtime::ProfilerMode::Normal,
                        DevtoolsProfilerMode::Performance => {
                            incular_runtime::ProfilerMode::Diagnostic
                        }
                        DevtoolsProfilerMode::Deep => incular_runtime::ProfilerMode::Profiling,
                    });
                    Ok(ResponsePayload::ProfilerModeSet(mode))
                }
                RequestMethod::StartRecording => {
                    self.recording = true;
                    self.recorded_frames = 0;
                    Ok(ResponsePayload::RecordingStarted)
                }
                RequestMethod::StopRecording => {
                    self.recording = false;
                    Ok(ResponsePayload::RecordingStopped {
                        frames: self.recorded_frames,
                    })
                }
                RequestMethod::TakeMemorySnapshot { label } => {
                    let mut counts = application.devtools_resource_counts();
                    counts.rss_mb = incular_devtools::process_rss_mb();
                    Ok(ResponsePayload::MemorySnapshot(MemorySnapshot {
                        label,
                        counts,
                    }))
                }
                RequestMethod::ListSignals => Ok(ResponsePayload::Signals {
                    signals: application.devtools_signals(),
                }),
                RequestMethod::GetSignalSubscribers { id } => {
                    Ok(ResponsePayload::SignalSubscribers {
                        subscribers: application.devtools_signal_subscribers(id),
                    })
                }
                RequestMethod::EditSignal { id, value } => {
                    if application.devtools_edit_signal(id, &value) {
                        Ok(ResponsePayload::Edited)
                    } else {
                        Err(incular_devtools_protocol::ErrorCode::Unsupported)
                    }
                }
                RequestMethod::ResetOverrides => {
                    let overrides = std::mem::take(&mut self.property_overrides);
                    for ((id, name), property) in overrides {
                        let _ = application.devtools_revert_property(id, &name, &property.original);
                    }
                    Ok(ResponsePayload::OverridesReset)
                }
                _ => Err(incular_devtools_protocol::ErrorCode::Unsupported),
            };
            completion.complete(reply);
        }
        self.refresh_layout_bounds(application);
        self.refresh_auxiliary_overlays(application);
        if repaint_overlays {
            for window in application.active_window_ids() {
                if let Some(handle) = application.window_handle(window) {
                    let _ = handle.request_redraw();
                }
            }
        }
        agent.acknowledge_wake()
    }

    fn reset_session(&mut self, application: &mut incular_runtime::Application) {
        self.select_window = None;
        self.pick_capture = None;
        self.hover = None;
        self.hover_content = None;
        self.hover_label = None;
        self.highlight_label = None;
        self.highlight = None;
        self.highlight_content = None;
        self.highlight_baseline = None;
        self.highlight_clip = None;
        self.highlight_rect = None;
        self.options.clear();
        self.animation_speed = 1.0;
        application.set_animation_time_scale(1.0);
        self.profiler_mode = DevtoolsProfilerMode::Basic;
        application.set_profiler_mode(incular_runtime::ProfilerMode::Normal);
        self.recording = false;
        self.recorded_frames = 0;
        self.tree_subscriptions.clear();
        self.tree_cache.clear();
        self.last_tree_sample = None;
        self.window_map.clear();
        self.layout_bounds.clear();
        self.hit_regions.clear();
        self.semantics_bounds.clear();
        self.scroll_viewports.clear();
        self.layer_bounds.clear();
        self.phase_counters.clear();
        self.phase_flashes.clear();
        self.repaint_rainbow.clear();
        self.frame_history.clear();
        let overrides = std::mem::take(&mut self.property_overrides);
        for ((id, name), property) in overrides {
            let _ = application.devtools_revert_property(id, &name, &property.original);
        }
    }

    fn refresh_auxiliary_overlays(&mut self, application: &incular_runtime::Application) {
        self.hit_regions.clear();
        self.semantics_bounds.clear();
        self.scroll_viewports.clear();
        self.layer_bounds.clear();
        const LIMIT: usize = incular_widgets::devtools::MAX_OVERLAY_NODES;
        for window in application.active_window_ids() {
            if self.options.contains(&DebugOption::HitTestRegions) {
                self.hit_regions.insert(
                    window,
                    application.devtools_window_hit_regions(window, LIMIT),
                );
            }
            if self.options.contains(&DebugOption::SemanticsBounds) {
                self.semantics_bounds.insert(
                    window,
                    application.devtools_window_semantics_bounds(window, LIMIT),
                );
            }
            if self.options.contains(&DebugOption::ScrollViewports) {
                self.scroll_viewports.insert(
                    window,
                    application.devtools_window_scroll_viewports(window, LIMIT),
                );
            }
            if self.options.contains(&DebugOption::LayerBoundaries) {
                self.layer_bounds.insert(
                    window,
                    application.devtools_window_layer_bounds(window, LIMIT),
                );
            }
        }
    }

    fn refresh_layout_bounds(&mut self, application: &incular_runtime::Application) {
        let active = self.options.contains(&DebugOption::LayoutBoundsSubtree)
            || self.options.contains(&DebugOption::LayoutBoundsWholeWindow);
        if !active {
            self.layout_bounds.clear();
            return;
        }
        if self
            .last_layout_bounds_sample
            .is_some_and(|last| last.elapsed() < Duration::from_millis(125))
        {
            return;
        }
        self.last_layout_bounds_sample = Some(Instant::now());
        self.layout_bounds.clear();
        const LIMIT: usize = incular_widgets::devtools::MAX_OVERLAY_NODES;
        if self.options.contains(&DebugOption::LayoutBoundsWholeWindow) {
            for window in application.active_window_ids() {
                self.layout_bounds.insert(
                    window,
                    application.devtools_window_layout_bounds(window, LIMIT),
                );
            }
        } else if let Some((id, _)) = self.highlight
            && let Some((window, bounds)) = application.devtools_subtree_layout_bounds(id, LIMIT)
        {
            self.layout_bounds.insert(window, bounds);
        }
    }

    /// Per-frame overlay maintenance for one window, run right after the
    /// retained frame and before the overlay is painted into it.
    pub fn frame_update(
        &mut self,
        application: &incular_runtime::Application,
        window: incular_platform::WindowId,
    ) {
        self.refresh_layout_bounds(application);
        self.refresh_auxiliary_overlays(application);
        self.refresh_phase_flashes(application, window);
    }

    fn refresh_phase_flashes(
        &mut self,
        application: &incular_runtime::Application,
        window: incular_platform::WindowId,
    ) {
        let flash_on = self.flashes_enabled();
        if !flash_on && !self.options.contains(&DebugOption::RepaintRainbow) {
            self.phase_flashes.remove(&window);
            self.phase_counters.remove(&window);
            self.repaint_rainbow.remove(&window);
            return;
        }
        const LIMIT: usize = incular_widgets::devtools::MAX_OVERLAY_NODES;
        let now = Instant::now();
        let rainbow_on = self.options.contains(&DebugOption::RepaintRainbow);
        let previous = self.phase_counters.entry(window).or_default();
        let flashes = self.phase_flashes.entry(window).or_default();
        let rainbow = self.repaint_rainbow.entry(window).or_default();
        let mut next = HashMap::new();
        let mut live = std::collections::HashSet::new();
        for node in application.devtools_window_phase_nodes(window, LIMIT) {
            let counters = [
                node.builds,
                node.layouts,
                node.paints,
                node.semantics,
                node.composites,
            ];
            live.insert(node.id);
            if let Some(before) = previous.get(&node.id) {
                let changed: [bool; 5] =
                    std::array::from_fn(|index| counters[index] > before[index]);
                if flash_on && changed.into_iter().any(|value| value) && flashes.len() < MAX_FLASHES
                {
                    let flash = flashes.entry(node.id).or_insert(Flash {
                        bounds: node.bounds,
                        started: [None; 5],
                    });
                    flash.bounds = node.bounds;
                    for (index, changed) in changed.into_iter().enumerate() {
                        if changed {
                            flash.started[index] = Some(now);
                        }
                    }
                }
                if rainbow_on && changed[2] {
                    let entry = rainbow.entry(node.id).or_insert(RainbowEntry {
                        bounds: node.bounds,
                        generation: u8::MAX,
                    });
                    entry.generation = entry.generation.wrapping_add(1);
                    entry.bounds = node.bounds;
                } else if let Some(entry) = rainbow.get_mut(&node.id) {
                    entry.bounds = node.bounds;
                }
            }
            next.insert(node.id, counters);
        }
        *previous = next;
        rainbow.retain(|id, _| live.contains(id));
        flashes.retain(|id, flash| {
            live.contains(id)
                && flash
                    .started
                    .iter()
                    .flatten()
                    .any(|started| now.duration_since(*started) < FLASH_DURATION)
        });
    }

    fn flashes_enabled(&self) -> bool {
        PHASE_OPTIONS
            .into_iter()
            .any(|option| self.options.contains(&option))
            || self.options.contains(&DebugOption::FlashRebuilds)
    }

    /// True while a flash is still fading in `window`: the host must keep
    /// redrawing it so the fade completes even if the app goes idle.
    pub fn wants_redraw(&self, window: incular_platform::WindowId) -> bool {
        if !self.flashes_enabled() {
            return false;
        }
        let now = Instant::now();
        self.phase_flashes.get(&window).is_some_and(|flashes| {
            flashes.values().any(|flash| {
                flash
                    .started
                    .iter()
                    .flatten()
                    .any(|started| now.duration_since(*started) < FLASH_DURATION)
            })
        })
    }

    /// Records one presented frame for the performance overlay.
    pub fn record_frame_sample(&mut self, window: incular_platform::WindowId, sample: FrameSample) {
        let history = self.frame_history.entry(window).or_default();
        if history.len() == devtools_overlay::PERFORMANCE_HISTORY {
            history.pop_front();
        }
        history.push_back(sample);
    }

    /// Streams one frame record to connected DevTools clients.
    #[must_use]
    pub fn push_frame(&self, event: TargetEvent) -> bool {
        if let Some(agent) = &self.agent {
            agent.telemetry.try_push(event)
        } else {
            false
        }
    }

    /// Activates bounded node timestamps only for an explicit Deep recording.
    pub fn begin_deep_frame(
        &self,
        application: &mut incular_runtime::Application,
        window: incular_platform::WindowId,
    ) {
        if self.recording && self.profiler_mode == DevtoolsProfilerMode::Deep {
            const MAX_EVENTS_PER_FRAME: usize = 4_096;
            application.devtools_begin_deep_trace(window, MAX_EVENTS_PER_FRAME);
        }
    }

    /// Returns whether this rendered frame belongs to the bounded recording.
    /// At 300 frames capture stops explicitly and a visible diagnostic is sent.
    pub fn note_recorded_frame(&mut self) -> bool {
        if !self.recording {
            return false;
        }
        self.recorded_frames = self.recorded_frames.saturating_add(1);
        if self.recorded_frames >= 300 {
            self.recording = false;
            let _ = self.push_frame(TargetEvent::Log {
                level: "info".into(),
                target: "incular::devtools".into(),
                message: "recording stopped: 300-frame limit reached".into(),
            });
        }
        true
    }

    pub fn deep_recording(&self) -> bool {
        self.profiler_mode == DevtoolsProfilerMode::Deep
            && (self.recording || self.recorded_frames == 300)
    }

    /// Draws the selected/hovered exact world bounds after the retained
    /// display list. This is a compositor-only debug adornment: it neither
    /// enters the widget tree nor changes its constraints, hit testing, or
    /// retained layout state.
    pub fn paint_overlay(
        &self,
        window: incular_platform::WindowId,
        window_size: incular_core::Size,
        list: &mut incular_rendering::DisplayList,
    ) {
        use devtools_overlay::{chrome_highlight, fill, inset, outline, with_alpha};
        // Adjacent Rect commands are consumed by the renderer's existing
        // rect-instance batch. This is one diagnostic display-list pass, not
        // a widget or a GPU pass per target node.
        if let Some(bounds) = self.layout_bounds.get(&window) {
            for &bounds in bounds {
                outline(
                    list,
                    bounds,
                    1.,
                    incular_core::Color::rgba(255, 172, 44, 150),
                );
            }
        }
        for (regions, color) in [
            (
                self.hit_regions.get(&window),
                incular_core::Color::rgba(72, 214, 164, 255),
            ),
            (
                self.semantics_bounds.get(&window),
                incular_core::Color::rgba(246, 214, 72, 255),
            ),
            (
                self.scroll_viewports.get(&window),
                incular_core::Color::rgba(65, 188, 246, 255),
            ),
            (
                self.layer_bounds.get(&window),
                incular_core::Color::rgba(190, 102, 246, 255),
            ),
        ] {
            for &bounds in regions.into_iter().flatten() {
                fill(list, bounds, with_alpha(color, 0.12));
                outline(list, bounds, 1., with_alpha(color, 0.7));
            }
        }
        if self.options.contains(&DebugOption::PaddingContent)
            && let Some(bounds) = self.highlight_content
        {
            outline(
                list,
                bounds,
                1.,
                incular_core::Color::rgba(62, 218, 142, 230),
            );
        }
        self.paint_flashes(window, list);
        if self.options.contains(&DebugOption::RepaintRainbow)
            && let Some(entries) = self.repaint_rainbow.get(&window)
        {
            for entry in entries.values() {
                if entry.generation == u8::MAX {
                    continue;
                }
                let color = devtools_overlay::RAINBOW
                    [usize::from(entry.generation) % devtools_overlay::RAINBOW.len()];
                outline(list, entry.bounds, 2., color);
            }
        }
        if self.options.contains(&DebugOption::Clips)
            && let Some(bounds) = self.highlight_clip
        {
            outline(
                list,
                bounds,
                2.,
                incular_core::Color::rgba(240, 72, 92, 220),
            );
        }
        if self.options.contains(&DebugOption::Baselines)
            && let Some(y) = self.highlight_baseline
            && let Some((_, [x, _, width, _])) = self.highlight
            && width > 0.
        {
            fill(
                list,
                [x, y, width, 1.],
                incular_core::Color::rgba(104, 232, 130, 180),
            );
        }
        if let Some((rect_window, bounds)) = self.highlight_rect
            && rect_window == window
        {
            chrome_highlight(list, bounds, None);
        }
        let picked = self
            .highlight
            .map(|(_, bounds)| (bounds, self.highlight_content, &self.highlight_label))
            .or_else(|| {
                self.hover
                    .filter(|_| self.is_selecting(window))
                    .map(|(_, bounds)| (bounds, self.hover_content, &self.hover_label))
            });
        if let Some((bounds, content, label)) = picked
            && bounds[2] > 0.
            && bounds[3] > 0.
        {
            chrome_highlight(list, bounds, content);
            if let Some(label) = label {
                devtools_overlay::label_chip(list, window_size, bounds, label);
            }
        }
        if self.options.contains(&DebugOption::PerformanceOverlay)
            && let Some(history) = self.frame_history.get(&window)
        {
            devtools_overlay::performance_overlay(list, window_size, history);
        }
        if self.is_selecting(window) {
            // The whole window is captured while picking; make that visible.
            let frame = [0., 0., window_size.width, window_size.height];
            outline(
                list,
                frame,
                3.,
                incular_core::Color::rgba(61, 133, 198, 235),
            );
            outline(
                list,
                inset(frame, 3.),
                1.,
                incular_core::Color::rgba(255, 255, 255, 120),
            );
        }
    }

    fn paint_flashes(
        &self,
        window: incular_platform::WindowId,
        list: &mut incular_rendering::DisplayList,
    ) {
        use devtools_overlay::{contains, fill, inset, outline, with_alpha};
        let Some(flashes) = self.phase_flashes.get(&window) else {
            return;
        };
        let now = Instant::now();
        let fade = |started: Option<Instant>| {
            started.and_then(|started| {
                let age = now.duration_since(started).as_secs_f32();
                let remaining = 1. - age / FLASH_DURATION.as_secs_f32();
                (remaining > 0.).then_some(remaining)
            })
        };
        for (index, option) in PHASE_OPTIONS.into_iter().enumerate() {
            let enabled = self.options.contains(&option)
                || (index == 0 && self.options.contains(&DebugOption::FlashRebuilds));
            if !enabled {
                continue;
            }
            let color = PHASE_COLORS[index];
            let alive: Vec<([f32; 4], f32)> = flashes
                .values()
                .filter_map(|flash| Some((flash.bounds, fade(flash.started[index])?)))
                .filter(|(bounds, _)| bounds[2] > 0. && bounds[3] > 0.)
                .collect();
            // Fill only the innermost changed elements: an ancestor that
            // merely re-laid out because a child changed would otherwise
            // wash the whole window. Every element keeps its own outline.
            const MAX_LEAF_TESTS: usize = 512;
            let leaf_tests = alive.len() <= MAX_LEAF_TESTS;
            for (position, &(bounds, fade)) in alive.iter().enumerate() {
                let is_leaf = !leaf_tests
                    || !alive.iter().enumerate().any(|(other, (inner, _))| {
                        other != position && contains(bounds, *inner) && *inner != bounds
                    });
                if is_leaf {
                    fill(list, bounds, with_alpha(color, 0.22 * fade));
                }
                // Later phases sit further inside so same-rect outlines stay visible.
                outline(
                    list,
                    inset(bounds, index as f32 * 2.),
                    2.,
                    with_alpha(color, 0.9 * fade),
                );
            }
        }
    }

    /// Samples subscribed trees at most four times per second and sends only
    /// structural/configuration deltas. This bounded cadence prevents a
    /// static 100k-node target from being repeatedly serialized at frame rate.
    pub fn stream_tree_updates(&mut self, application: &incular_runtime::Application) {
        if self.tree_subscriptions.is_empty()
            || self
                .last_tree_sample
                .is_some_and(|last| last.elapsed() < Duration::from_millis(250))
        {
            return;
        }
        self.last_tree_sample = Some(Instant::now());
        let windows: Vec<_> = self.tree_subscriptions.iter().copied().collect();
        for window in windows {
            let Ok(snapshot) = application.devtools_widget_tree(window) else {
                self.tree_subscriptions.remove(&window);
                self.tree_cache.remove(&window);
                continue;
            };
            let next = tree_nodes(&snapshot);
            let deltas = {
                let previous = self.tree_cache.entry(window).or_default();
                let mut deltas = Vec::new();
                for id in previous.keys().filter(|id| !next.contains_key(id)) {
                    deltas.push(TreeDelta::Remove { id: *id });
                }
                for (id, node) in &next {
                    match previous.get(id) {
                        None => deltas.push(TreeDelta::Insert {
                            node: Box::new(node.clone()),
                        }),
                        Some(old) if old != node => deltas.push(TreeDelta::Update {
                            node: Box::new(node.clone()),
                        }),
                        Some(_) => {}
                    }
                }
                deltas
            };
            if !deltas.is_empty() {
                if self.push_frame(TargetEvent::WidgetTreeDeltas {
                    window,
                    deltas,
                    tree_revision: self.frame_counter,
                }) {
                    self.tree_cache.insert(window, next);
                }
            } else {
                self.tree_cache.insert(window, next);
            }
        }
    }

    fn remember_tree(&mut self, window: DevWindowId, deltas: &[TreeDelta]) {
        self.tree_cache.insert(window, tree_nodes(deltas));
    }

    /// Emits user-driven widget selection during Select Widget mode.
    pub fn emit_selection(&mut self, window: incular_platform::WindowId, id: DevWidgetId) {
        let dev_window = self
            .window_map
            .iter()
            .find(|(_, platform)| **platform == window)
            .map(|(id, _)| *id);
        if let Some(dev_window) = dev_window {
            let _ = self.push_frame(TargetEvent::WidgetSelectedByUser {
                window: dev_window,
                id,
            });
        }
    }

    /// Intercepts Select Widget pointer input before normal dispatch. Hover
    /// never alters app state. A primary press over a retained widget reports
    /// it and ends the mode, but the whole gesture stays captured until the
    /// matching release so the app never sees an unpaired Up. While the mode
    /// is active every pointer event is swallowed, including presses over
    /// empty space, which keep the mode running.
    ///
    /// Returns true when the event must not reach application widgets.
    pub fn inspect_pointer(
        &mut self,
        application: &incular_runtime::Application,
        window: incular_platform::WindowId,
        point: incular_core::Offset,
        input: InspectInput,
    ) -> bool {
        if !self.captures_pointer(window) {
            return false;
        }
        if input.released {
            self.pick_capture = None;
            return true;
        }
        if !self.is_selecting(window) {
            // A consumed pick: swallow the remaining drag/move events.
            return true;
        }
        if (input.pressed || input.moved)
            && let Some((id, bounds)) = application.devtools_hit_test(window, point)
        {
            if self.hover.map(|(hovered, _)| hovered) != Some(id) {
                self.hover_label = application
                    .devtools_node_details(id)
                    .ok()
                    .map(|details| chip_text(&details.type_name, bounds));
            }
            self.note_hover(id, bounds);
            self.hover_content = application
                .devtools_node_overlay_geometry(id)
                .and_then(|geometry| geometry.content_bounds);
            if input.pressed {
                self.emit_selection(window, id);
                self.select_window = None;
                self.pick_capture = Some(window);
            }
        }
        true
    }
}

/// Pointer facts about one translated native event, as needed by the picker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InspectInput {
    /// The cursor moved.
    pub moved: bool,
    /// The primary button went down.
    pub pressed: bool,
    /// Any button was released or the contact was cancelled.
    pub released: bool,
}

/// `TypeName  W x H`, as shown on the highlight chip.
fn chip_text(type_name: &str, [_, _, width, height]: [f32; 4]) -> String {
    format!("{type_name}  {} \u{d7} {}", width.round(), height.round())
}

fn tree_nodes(deltas: &[TreeDelta]) -> std::collections::HashMap<DevWidgetId, WidgetNode> {
    let mut nodes = std::collections::HashMap::new();
    for delta in deltas {
        if let TreeDelta::Snapshot {
            root, nodes: rest, ..
        } = delta
        {
            nodes.insert(root.id, (**root).clone());
            nodes.extend(rest.iter().cloned().map(|node| (node.id, node)));
        }
    }
    nodes
}
