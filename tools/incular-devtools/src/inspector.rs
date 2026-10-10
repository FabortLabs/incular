use crate::{activity::ActivityLog, performance::TraceRange};
use incular_devtools_protocol::{
    DebugOption, DebugProperty, DebugValue, DeepFrameTrace, DevSignalId, DevWidgetId, DevWindowId,
    DevtoolsProfilerMode, EditableValue, FrameRecordEvent, LayoutDetails, MemorySnapshot,
    NodeDetails, SignalSubscriber, SignalSummary, TargetInfo, TreeDelta, WidgetNode, WindowSummary,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub(crate) type Shared = Arc<Mutex<InspectorModel>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ConnectionState {
    #[default]
    Discovering,
    Connecting,
    Authenticating,
    Connected,
    Disconnected,
    Stopping,
}

impl ConnectionState {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Discovering => "discovering",
            Self::Connecting => "connecting",
            Self::Authenticating => "authenticating",
            Self::Connected => "connected",
            Self::Disconnected => "disconnected",
            Self::Stopping => "stopping",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRow {
    pub(crate) id: DevWidgetId,
    pub(crate) depth: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeNavigation {
    Previous,
    Next,
    CollapseOrParent,
    ExpandOrChild,
    First,
    Last,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ConsoleEntry {
    pub level: String,
    pub target: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConsoleLevel {
    #[default]
    All,
    Errors,
    Warnings,
    Info,
    Debug,
}

impl ConsoleLevel {
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::Errors,
        Self::Warnings,
        Self::Info,
        Self::Debug,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All levels",
            Self::Errors => "Errors",
            Self::Warnings => "Warnings",
            Self::Info => "Info",
            Self::Debug => "Debug / trace",
        }
    }

    fn matches(self, level: &str) -> bool {
        match self {
            Self::All => true,
            Self::Errors => matches!(level.to_ascii_lowercase().as_str(), "error" | "fatal"),
            Self::Warnings => matches!(level.to_ascii_lowercase().as_str(), "warn" | "warning"),
            Self::Info => level.eq_ignore_ascii_case("info"),
            Self::Debug => matches!(level.to_ascii_lowercase().as_str(), "debug" | "trace"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum InspectorSection {
    #[default]
    Properties,
    Layout,
    Signals,
    Why,
    Semantics,
}

/// Protocol-facing inspector state. It remains UI-framework independent so
/// generation handling, deltas, and virtualization can be tested cheaply.
#[derive(Default)]
pub struct InspectorModel {
    pub(crate) connection: ConnectionState,
    pub(crate) connected: bool,
    pub(crate) error: Option<String>,
    pub(crate) target: String,
    pub(crate) target_info: Option<TargetInfo>,
    pub(crate) windows: Vec<WindowSummary>,
    pub(crate) active_window: Option<DevWindowId>,
    pub(crate) nodes: HashMap<DevWidgetId, WidgetNode>,
    pub(crate) tree_payload_bytes: usize,
    pub(crate) roots: HashMap<DevWindowId, DevWidgetId>,
    pub(crate) expanded: HashSet<DevWidgetId>,
    pub(crate) rows: Vec<TreeRow>,
    pub(crate) selected: Option<DevWidgetId>,
    pub(crate) selection_revision: u64,
    pub(crate) picked_revision: u64,
    pub(crate) selection_history: VecDeque<DevWidgetId>,
    pub(crate) history_cursor: Option<usize>,
    pub(crate) focused_root: Option<DevWidgetId>,
    pub(crate) hovered: Option<DevWidgetId>,
    pub(crate) select_mode: bool,
    pub(crate) details: Option<NodeDetails>,
    pub(crate) frames: VecDeque<FrameRecordEvent>,
    pub(crate) deep_traces: VecDeque<DeepFrameTrace>,
    pub(crate) deep_trace_events: usize,
    pub(crate) profiler_mode: DevtoolsProfilerMode,
    pub(crate) recording: bool,
    pub(crate) selected_frame: Option<(DevWindowId, u64)>,
    pub(crate) range_anchor: Option<u64>,
    pub(crate) selected_range: Option<(u64, u64)>,
    pub(crate) trace_range: TraceRange,
    pub(crate) timeline_offset: usize,
    pub(crate) timeline_visible: usize,
    pub(crate) memory: Option<MemorySnapshot>,
    pub(crate) memory_a: Option<MemorySnapshot>,
    pub(crate) memory_b: Option<MemorySnapshot>,
    pub(crate) console: VecDeque<ConsoleEntry>,
    pub(crate) console_payload_bytes: usize,
    pub(crate) console_filter: String,
    pub(crate) console_level: ConsoleLevel,
    pub(crate) console_frozen: Option<Vec<ConsoleEntry>>,
    pub(crate) activity: ActivityLog,
    pub(crate) notice: Option<String>,
    pub(crate) frame_arrivals: HashMap<DevWindowId, VecDeque<Instant>>,
    pub(crate) signals: Vec<SignalSummary>,
    pub(crate) selected_signal: Option<DevSignalId>,
    pub(crate) signal_subscribers: Vec<SignalSubscriber>,
    pub(crate) debug_options: HashSet<DebugOption>,
    pub(crate) animation_scale: Option<f32>,
    pub(crate) tree_retry_sent: bool,
    pub(crate) tree_resync_pending: bool,
    pub(crate) tree_revision: u64,
    pub(crate) tree_truncated: bool,
    pub(crate) search: String,
}

pub fn editable_value(signal: &SignalSummary, input: &str) -> Option<EditableValue> {
    match signal.type_name.rsplit("::").next()? {
        "bool" => input.parse().ok().map(EditableValue::Bool),
        "i64" => input.parse().ok().map(EditableValue::Int),
        "u64" => input.parse().ok().map(EditableValue::Uint),
        "f64" => input.parse().ok().map(EditableValue::Float),
        "String" => Some(EditableValue::Str(input.to_owned())),
        _ => None,
    }
}

impl InspectorModel {
    pub const FRAME_HISTORY: usize = 300;
    pub const MAX_TRACE_EVENTS: usize = 200_000;
    pub const MAX_TREE_NODES: usize = 131_072;
    pub const MAX_FRAME_ARRIVALS: usize = 512;
    pub const MAX_TREE_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;
    pub const MAX_DETAILS_PAYLOAD_BYTES: usize = 4 * 1024 * 1024;
    pub const MAX_SIGNALS_PAYLOAD_BYTES: usize = 4 * 1024 * 1024;
    pub const MAX_SUBSCRIBERS_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
    pub const MAX_TARGET_INFO_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
    pub const MAX_CONSOLE_PAYLOAD_BYTES: usize = 4 * 1024 * 1024;
    pub const MAX_CONSOLE_ENTRY_BYTES: usize = 8 * 1024;
    pub(crate) const CONSOLE_HISTORY: usize = 500;

    /// Returns the number of currently visible virtual tree rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Returns the currently selected widget, if any.
    pub fn selected_node(&self) -> Option<DevWidgetId> {
        self.selected
    }

    /// Reports whether a widget's descendants are currently visible.
    pub fn is_expanded(&self, id: DevWidgetId) -> bool {
        self.expanded.contains(&id)
    }

    /// Returns the number of retained deep-trace frames.
    pub fn deep_trace_count(&self) -> usize {
        self.deep_traces.len()
    }

    /// Returns the number of retained deep-trace events tracked by the model.
    pub fn deep_trace_event_count(&self) -> usize {
        self.deep_trace_events
    }

    /// Computes the event count from retained frames for invariant checks.
    pub fn deep_trace_event_total(&self) -> usize {
        self.deep_traces
            .iter()
            .map(|trace| trace.events.len())
            .sum()
    }

    /// Replaces the two snapshots used by the memory-diff view.
    pub fn set_memory_snapshots(&mut self, first: MemorySnapshot, second: MemorySnapshot) {
        self.memory_a = Some(first);
        self.memory_b = Some(second);
    }

    pub fn push_console(
        &mut self,
        level: impl Into<String>,
        target: impl Into<String>,
        message: impl Into<String>,
    ) {
        let mut entry = ConsoleEntry {
            level: level.into(),
            target: target.into(),
            message: message.into(),
        };
        truncate_utf8(&mut entry.level, 64);
        truncate_utf8(&mut entry.target, 512);
        truncate_utf8(
            &mut entry.message,
            Self::MAX_CONSOLE_ENTRY_BYTES
                .saturating_sub(entry.level.len())
                .saturating_sub(entry.target.len()),
        );
        let bytes = console_entry_bytes(&entry);
        while self.console.len() >= Self::CONSOLE_HISTORY
            || self.console_payload_bytes.saturating_add(bytes) > Self::MAX_CONSOLE_PAYLOAD_BYTES
        {
            let Some(removed) = self.console.pop_front() else {
                break;
            };
            self.console_payload_bytes = self
                .console_payload_bytes
                .saturating_sub(console_entry_bytes(&removed));
        }
        self.console_payload_bytes = self.console_payload_bytes.saturating_add(bytes);
        self.console.push_back(entry);
    }

    pub(crate) fn note_frame_arrival(&mut self, window: DevWindowId) {
        let now = Instant::now();
        let arrivals = self.frame_arrivals.entry(window).or_default();
        arrivals.push_back(now);
        while arrivals
            .front()
            .is_some_and(|arrival| now.duration_since(*arrival) > Duration::from_secs(2))
        {
            arrivals.pop_front();
        }
        while arrivals.len() > Self::MAX_FRAME_ARRIVALS {
            arrivals.pop_front();
        }
    }

    pub(crate) fn fps(&self, window: Option<DevWindowId>) -> Option<f32> {
        let arrivals = self.frame_arrivals.get(&window?)?;
        let first = *arrivals.front()?;
        let last = *arrivals.back()?;
        let elapsed = last.duration_since(first).as_secs_f32();
        (arrivals.len() > 1 && elapsed > f32::EPSILON)
            .then(|| ((arrivals.len() - 1) as f32 / elapsed).min(240.))
    }

    pub(crate) fn latest_frame(&self, window: Option<DevWindowId>) -> Option<&FrameRecordEvent> {
        self.frames
            .iter()
            .rev()
            .find(|frame| Some(frame.window) == window)
    }

    pub fn set_console_level(&mut self, level: ConsoleLevel) {
        self.console_level = level;
    }

    pub fn set_console_query(&mut self, query: impl Into<String>) {
        self.console_filter = query.into();
    }

    pub fn set_console_paused(&mut self, paused: bool) {
        if paused && self.console_frozen.is_none() {
            self.console_frozen = Some(self.console.iter().cloned().collect());
        } else if !paused {
            self.console_frozen = None;
        }
    }

    pub fn clear_console(&mut self) {
        self.console.clear();
        self.console_payload_bytes = 0;
        if let Some(frozen) = &mut self.console_frozen {
            frozen.clear();
        }
    }

    pub fn console_bytes(&self) -> usize {
        self.console_payload_bytes
    }

    pub fn filtered_console(&self) -> Vec<ConsoleEntry> {
        let query = self.console_filter.trim().to_ascii_lowercase();
        let entries: Box<dyn Iterator<Item = &ConsoleEntry> + '_> = match &self.console_frozen {
            Some(frozen) => Box::new(frozen.iter()),
            None => Box::new(self.console.iter()),
        };
        entries
            .filter(|entry| self.console_level.matches(&entry.level))
            .filter(|entry| {
                query.is_empty()
                    || entry.level.to_ascii_lowercase().contains(&query)
                    || entry.target.to_ascii_lowercase().contains(&query)
                    || entry.message.to_ascii_lowercase().contains(&query)
            })
            .cloned()
            .collect()
    }

    /// A portable report of retained diagnostics, without discovery credentials.
    pub fn diagnostic_report(&self) -> serde_json::Value {
        serde_json::json!({
            "format": "incular-devtools-report", "version": 1,
            "target": self.target_info, "active_window": self.active_window,
            "frames": self.frames, "deep_traces": self.deep_traces,
            "memory": self.memory, "baseline": self.memory_a, "comparison": self.memory_b,
            "console": self.console, "transport": self.activity.entries().collect::<Vec<_>>(),
            "selected_widget": self.details,
        })
    }

    pub fn push_deep_trace(&mut self, mut trace: DeepFrameTrace) {
        if trace.events.len() > Self::MAX_TRACE_EVENTS {
            let dropped = trace.events.len() - Self::MAX_TRACE_EVENTS;
            trace.events.truncate(Self::MAX_TRACE_EVENTS);
            trace.truncated = true;
            trace.dropped_events = trace
                .dropped_events
                .saturating_add(u32::try_from(dropped).unwrap_or(u32::MAX));
        }
        while self.deep_traces.len() >= Self::FRAME_HISTORY
            || self.deep_trace_events.saturating_add(trace.events.len()) > Self::MAX_TRACE_EVENTS
        {
            let Some(removed) = self.deep_traces.pop_front() else {
                break;
            };
            self.deep_trace_events = self.deep_trace_events.saturating_sub(removed.events.len());
        }
        self.deep_trace_events = self.deep_trace_events.saturating_add(trace.events.len());
        self.deep_traces.push_back(trace);
    }

    pub fn memory_diff_lines(&self) -> Vec<String> {
        let (Some(a), Some(b)) = (&self.memory_a, &self.memory_b) else {
            return Vec::new();
        };
        let signed = |before: usize, after: usize| -> String {
            let delta = after as i128 - before as i128;
            format!("{delta:+}")
        };
        vec![
            format!("Memory diff: {} → {}", a.label, b.label),
            format!(
                "  RSS {} MB → {} MB ({})",
                a.counts.rss_mb,
                b.counts.rss_mb,
                signed(a.counts.rss_mb, b.counts.rss_mb)
            ),
            format!(
                "  elements {} → {} ({}) · render objects {} → {} ({})",
                a.counts.elements,
                b.counts.elements,
                signed(a.counts.elements, b.counts.elements),
                a.counts.render_objects,
                b.counts.render_objects,
                signed(a.counts.render_objects, b.counts.render_objects),
            ),
            format!(
                "  semantics {} → {} ({}) · signals {} → {} ({}) · tasks {} → {} ({})",
                a.counts.semantics_nodes,
                b.counts.semantics_nodes,
                signed(a.counts.semantics_nodes, b.counts.semantics_nodes),
                a.counts.signals,
                b.counts.signals,
                signed(a.counts.signals, b.counts.signals),
                a.counts.tasks_active,
                b.counts.tasks_active,
                signed(a.counts.tasks_active, b.counts.tasks_active),
            ),
        ]
    }

    pub fn apply_tree(&mut self, revision: u64, deltas: impl IntoIterator<Item = TreeDelta>) {
        if revision < self.tree_revision {
            return;
        }
        self.tree_revision = revision;
        for delta in deltas {
            match delta {
                TreeDelta::Snapshot {
                    window,
                    root,
                    mut nodes,
                    truncated,
                } => {
                    self.active_window.get_or_insert(window);
                    if self.active_window == Some(window) {
                        let oversized = nodes.len().saturating_add(1) > Self::MAX_TREE_NODES;
                        if oversized {
                            nodes.truncate(Self::MAX_TREE_NODES.saturating_sub(1));
                        }
                        self.nodes.clear();
                        self.tree_payload_bytes = 0;
                        self.roots.insert(window, root.id);
                        self.expanded.insert(root.id);
                        self.selected.get_or_insert(root.id);
                        let (root, root_trimmed) = normalize_widget_node(*root);
                        self.tree_payload_bytes = widget_node_bytes(&root);
                        self.nodes.insert(root.id, root);
                        let mut payload_truncated = root_trimmed;
                        for node in nodes {
                            let (node, trimmed) = normalize_widget_node(node);
                            let bytes = widget_node_bytes(&node);
                            if self.nodes.contains_key(&node.id) {
                                self.error =
                                    Some("malformed widget tree: duplicate node identity".into());
                                payload_truncated = true;
                                continue;
                            }
                            if self.tree_payload_bytes.saturating_add(bytes)
                                > Self::MAX_TREE_PAYLOAD_BYTES
                            {
                                payload_truncated = true;
                                continue;
                            }
                            self.tree_payload_bytes = self.tree_payload_bytes.saturating_add(bytes);
                            self.nodes.insert(node.id, node);
                            payload_truncated |= trimmed;
                        }
                        self.tree_truncated = truncated || oversized || payload_truncated;
                    }
                }
                TreeDelta::Insert { node } | TreeDelta::Update { node } => {
                    let (node, trimmed) = normalize_widget_node(*node);
                    let old_bytes = self.nodes.get(&node.id).map(widget_node_bytes).unwrap_or(0);
                    let next_bytes = self
                        .tree_payload_bytes
                        .saturating_sub(old_bytes)
                        .saturating_add(widget_node_bytes(&node));
                    if (self.nodes.contains_key(&node.id)
                        || self.nodes.len() < Self::MAX_TREE_NODES)
                        && next_bytes <= Self::MAX_TREE_PAYLOAD_BYTES
                    {
                        self.tree_payload_bytes = next_bytes;
                        self.nodes.insert(node.id, node);
                        self.tree_truncated |= trimmed;
                    } else {
                        self.tree_truncated = true;
                    }
                }
                TreeDelta::Remove { id } => self.remove_subtree(id),
                TreeDelta::Move {
                    id,
                    parent,
                    position,
                } => self.move_node(id, parent, position),
            }
        }
        self.rebuild_rows();
    }

    pub(crate) fn remove_subtree(&mut self, id: DevWidgetId) {
        let mut work = vec![id];
        let mut visited = HashSet::new();
        while let Some(current) = work.pop() {
            if !visited.insert(current) {
                continue;
            }
            if let Some(node) = self.nodes.remove(&current) {
                self.tree_payload_bytes = self
                    .tree_payload_bytes
                    .saturating_sub(widget_node_bytes(&node));
                work.extend(node.child_ids);
            }
            self.expanded.remove(&current);
            if self.selected == Some(current) {
                self.selected = None;
                self.details = None;
            }
        }
        for node in self.nodes.values_mut() {
            node.child_ids.retain(|child| *child != id);
        }
    }

    pub(crate) fn move_node(&mut self, id: DevWidgetId, parent: DevWidgetId, position: usize) {
        for node in self.nodes.values_mut() {
            node.child_ids.retain(|child| *child != id);
        }
        if let Some(node) = self.nodes.get_mut(&id) {
            node.parent = Some(parent);
        }
        if let Some(parent_node) = self.nodes.get_mut(&parent) {
            parent_node
                .child_ids
                .insert(position.min(parent_node.child_ids.len()), id);
        }
        self.tree_payload_bytes = self.nodes.values().map(widget_node_bytes).sum();
    }

    pub(crate) fn rebuild_rows(&mut self) {
        self.rows.clear();
        let Some(window) = self.active_window else {
            return;
        };
        let Some(root) = self
            .focused_root
            .filter(|id| self.nodes.contains_key(id))
            .or_else(|| self.roots.get(&window).copied())
        else {
            return;
        };
        let query = self.search.trim().to_lowercase();
        let terms = query.split_whitespace().collect::<Vec<_>>();
        let mut work = vec![(root, 0_u16)];
        let mut visited = HashSet::new();
        while let Some((id, depth)) = work.pop() {
            if !visited.insert(id) {
                continue;
            }
            let Some(node) = self.nodes.get(&id) else {
                continue;
            };
            let matches = terms.iter().all(|term| {
                let contains = |value: &str| value.to_lowercase().contains(*term);
                match term.split_once(':') {
                    Some(("type", value)) => node.type_name.to_lowercase().contains(value),
                    Some(("text", value)) => node
                        .label
                        .as_deref()
                        .is_some_and(|text| text.to_lowercase().contains(value)),
                    Some(("key", value)) => node
                        .key
                        .as_deref()
                        .is_some_and(|key| key.to_lowercase().contains(value)),
                    Some(("id", value)) => node.id.to_string().to_lowercase().contains(value),
                    Some(("has", "children")) => !node.child_ids.is_empty(),
                    _ => {
                        contains(&node.type_name)
                            || node.label.as_deref().is_some_and(contains)
                            || node.key.as_deref().is_some_and(contains)
                            || contains(&node.id.to_string())
                    }
                }
            });
            if matches {
                self.rows.push(TreeRow {
                    id,
                    depth: if terms.is_empty() { depth } else { 0 },
                });
            }
            if self.expanded.contains(&id) || !query.is_empty() {
                for child in node.child_ids.iter().rev() {
                    work.push((*child, depth.saturating_add(1)));
                }
            }
        }
    }

    pub fn reveal(&mut self, id: DevWidgetId) {
        let mut current = self.nodes.get(&id).and_then(|node| node.parent);
        let mut visited = HashSet::new();
        while let Some(parent) = current {
            if !visited.insert(parent) {
                self.error = Some("malformed widget tree: parent cycle".into());
                break;
            }
            self.expanded.insert(parent);
            current = self.nodes.get(&parent).and_then(|node| node.parent);
        }
        self.rebuild_rows();
    }

    pub fn set_search(&mut self, query: impl Into<String>) {
        self.search = query.into();
        self.rebuild_rows();
    }

    /// Selects and reveals a live node, retaining a bounded navigation history.
    pub fn select_node(&mut self, id: DevWidgetId) -> bool {
        if !self.nodes.contains_key(&id) {
            return false;
        }
        if self
            .focused_root
            .is_some_and(|root| !self.ancestor_path(id).contains(&root))
        {
            self.focused_root = None;
        }
        self.reveal(id);
        if !self.rows.iter().any(|row| row.id == id) {
            self.search.clear();
            self.rebuild_rows();
        }
        if self.selected != Some(id) {
            self.details = None;
            if self.selection_history.is_empty()
                && let Some(previous) = self
                    .selected
                    .filter(|previous| self.nodes.contains_key(previous))
            {
                self.selection_history.push_back(previous);
                self.history_cursor = Some(0);
            }
            if let Some(cursor) = self.history_cursor {
                self.selection_history.truncate(cursor + 1);
            }
            if self.selection_history.back() != Some(&id) {
                self.selection_history.push_back(id);
            }
            if self.selection_history.len() > 64 {
                self.selection_history.pop_front();
            }
            self.history_cursor = self.selection_history.len().checked_sub(1);
        }
        self.selected = Some(id);
        self.selection_revision = self.selection_revision.saturating_add(1);
        true
    }

    pub fn selection_history_step(&mut self, forward: bool) -> Option<DevWidgetId> {
        let mut cursor = self.history_cursor?;
        loop {
            cursor = if forward {
                cursor
                    .checked_add(1)
                    .filter(|next| *next < self.selection_history.len())?
            } else {
                cursor.checked_sub(1)?
            };
            let id = self.selection_history[cursor];
            if self.nodes.contains_key(&id) {
                self.history_cursor = Some(cursor);
                self.selected = Some(id);
                self.details = None;
                self.focused_root = None;
                self.search.clear();
                self.reveal(id);
                self.selection_revision = self.selection_revision.saturating_add(1);
                return Some(id);
            }
        }
    }

    pub fn ancestor_path(&self, id: DevWidgetId) -> Vec<DevWidgetId> {
        let mut path = Vec::new();
        let mut current = Some(id);
        let mut visited = HashSet::new();
        while let Some(id) = current {
            if !visited.insert(id) {
                break;
            }
            let Some(node) = self.nodes.get(&id) else {
                break;
            };
            path.push(id);
            current = node.parent;
        }
        path.reverse();
        path
    }

    pub fn focus_subtree(&mut self, id: Option<DevWidgetId>) {
        self.focused_root = id.filter(|id| self.nodes.contains_key(id));
        if let Some(root) = self.focused_root {
            self.expanded.insert(root);
        }
        self.rebuild_rows();
    }

    pub fn expand_branch(&mut self, id: DevWidgetId, expand: bool) {
        let mut work = vec![id];
        let mut visited = HashSet::new();
        while let Some(id) = work.pop() {
            if !visited.insert(id) {
                continue;
            }
            if let Some(node) = self.nodes.get(&id) {
                work.extend(node.child_ids.iter().copied());
                if expand && !node.child_ids.is_empty() {
                    self.expanded.insert(id);
                } else {
                    self.expanded.remove(&id);
                }
            }
        }
        self.rebuild_rows();
    }

    pub fn navigate_tree(&mut self, direction: TreeNavigation) -> Option<DevWidgetId> {
        let index = self
            .rows
            .iter()
            .position(|row| Some(row.id) == self.selected);
        let searching = !self.search.trim().is_empty();
        if searching
            && matches!(
                direction,
                TreeNavigation::CollapseOrParent | TreeNavigation::ExpandOrChild
            )
        {
            return None;
        }
        let Some(index) = index else {
            let id = match direction {
                TreeNavigation::Previous | TreeNavigation::Last => self.rows.last()?.id,
                _ => self.rows.first()?.id,
            };
            return self.select_node(id).then_some(id);
        };
        let current = self.rows.get(index)?.id;
        let next = match direction {
            TreeNavigation::Previous => {
                self.rows
                    .get(if searching && index == 0 {
                        self.rows.len() - 1
                    } else {
                        index.saturating_sub(1)
                    })?
                    .id
            }
            TreeNavigation::Next => {
                self.rows
                    .get(if searching {
                        (index + 1) % self.rows.len()
                    } else {
                        (index + 1).min(self.rows.len() - 1)
                    })?
                    .id
            }
            TreeNavigation::First => self.rows.first()?.id,
            TreeNavigation::Last => self.rows.last()?.id,
            TreeNavigation::CollapseOrParent => {
                if self.search.is_empty() && self.expanded.contains(&current) {
                    self.toggle_expanded(current);
                    return None;
                }
                if Some(current) == self.focused_root {
                    return None;
                }
                self.nodes.get(&current)?.parent?
            }
            TreeNavigation::ExpandOrChild => {
                let child = *self.nodes.get(&current)?.child_ids.first()?;
                if self.search.is_empty() && !self.expanded.contains(&current) {
                    self.toggle_expanded(current);
                    return None;
                }
                child
            }
        };
        self.select_node(next).then_some(next)
    }

    pub fn toggle_expanded(&mut self, id: DevWidgetId) -> bool {
        if self
            .nodes
            .get(&id)
            .is_none_or(|node| node.child_ids.is_empty())
        {
            return false;
        }
        if !self.expanded.insert(id) {
            self.expanded.remove(&id);
        }
        self.rebuild_rows();
        true
    }

    pub fn collapse_all(&mut self) {
        self.expanded.clear();
        self.rebuild_rows();
    }

    pub fn expand_all(&mut self) {
        self.expanded = self
            .nodes
            .iter()
            .filter_map(|(id, node)| (!node.child_ids.is_empty()).then_some(*id))
            .collect();
        self.rebuild_rows();
    }

    pub(crate) fn details_lines(&self, section: InspectorSection) -> Vec<String> {
        let Some(details) = &self.details else {
            let Some(selected) = self.selected else {
                return vec!["Select a widget to inspect its properties.".into()];
            };
            let Some(node) = self.nodes.get(&selected) else {
                return vec!["The selected widget is no longer retained.".into()];
            };
            return vec![
                format!("{}  {}", node.type_name, node.key.as_deref().unwrap_or("")),
                format!(
                    "id: {} · children: {} · revision: {}",
                    node.id,
                    node.child_ids.len(),
                    node.revision
                ),
                node.label.as_ref().map_or_else(
                    || "No semantic label".into(),
                    |label| format!("label: {label}"),
                ),
                "Select a rendered child to inspect detailed retained layout diagnostics.".into(),
            ];
        };
        let mut lines = vec![format!(
            "{}  {}",
            details.type_name,
            details.key.as_deref().unwrap_or("")
        )];
        match section {
            InspectorSection::Properties => {
                lines.push(format!("id: {} · render {:?}", details.id, details.render));
                lines.push(format!(
                    "bounds: {:?} · size: {:?} · offset: {:?}",
                    details.state.world_bounds, details.state.size, details.state.offset
                ));
                lines.push(format!(
                    "BUILD {} · LAYOUT {} · PAINT {} · COMPOSITE {}",
                    details.state.builds,
                    details.state.layouts,
                    details.state.paints,
                    details.state.composites
                ));
                if let Some(source) = &details.source {
                    lines.push(format!(
                        "source: {}:{}:{}",
                        source.file, source.line, source.column
                    ));
                }
                lines.extend(details.properties.iter().map(property_line));
            }
            InspectorSection::Layout => {
                if let Some(layout) = &details.layout {
                    lines.push(format!(
                        "resolved {:.1} × {:.1} · local ({:.1}, {:.1})",
                        layout.resolved_size[0],
                        layout.resolved_size[1],
                        layout.local_offset[0],
                        layout.local_offset[1]
                    ));
                    lines.push(layout.incoming_constraints.as_ref().map_or_else(
                        || "incoming constraints unavailable".into(),
                        |constraints| format!("incoming: {}", debug_value(constraints)),
                    ));
                    lines.push(format!(
                        "world ({:.1}, {:.1}) {:.1} × {:.1} · baseline {:?} · clip {:?}",
                        layout.world_bounds[0],
                        layout.world_bounds[1],
                        layout.world_bounds[2],
                        layout.world_bounds[3],
                        layout.baseline,
                        layout.clip
                    ));
                    if let Some(padding) = layout.padding {
                        lines.push(format!(
                            "padding L{:.1} T{:.1} R{:.1} B{:.1}",
                            padding[0], padding[1], padding[2], padding[3]
                        ));
                    }
                    lines.extend(layout_detail_lines(&layout.details));
                } else {
                    lines.push("This retained node has no layout snapshot.".into());
                }
                if !details.layout_history.is_empty() {
                    lines.push("Recent Deep layout changes".into());
                    lines.extend(details.layout_history.iter().rev().take(16).map(|entry| {
                        format!(
                            "#{} {:.1}×{:.1} → {:.1}×{:.1} · {}",
                            entry.sequence,
                            entry.old_size[0],
                            entry.old_size[1],
                            entry.new_size[0],
                            entry.new_size[1],
                            entry.cause.as_deref().unwrap_or("cause unavailable")
                        )
                    }));
                }
            }
            InspectorSection::Signals => {
                if details.consumed_signals.is_empty() {
                    lines.push("No debug-visible Signals were consumed by this node.".into());
                } else {
                    lines.push("Consumed Signals".into());
                    lines.extend(details.consumed_signals.iter().map(|id| {
                        let name = self
                            .signals
                            .iter()
                            .find(|signal| signal.id == *id)
                            .and_then(|signal| signal.name.as_deref())
                            .unwrap_or("<unnamed>");
                        format!("{name} · {id}")
                    }));
                }
            }
            InspectorSection::Why => {
                lines.push(details.invalidation.as_ref().map_or_else(
                    || "No retained rebuild cause is available.".into(),
                    |reason| format!("Why did this rebuild? {reason:?}"),
                ));
                for cause in &details.invalidation_causes {
                    lines.push(format!("{cause:?}  →  {} BUILD", details.id));
                }
                for change in &details.property_changes {
                    lines.push(format!(
                        "{}: {} → {}",
                        change.name,
                        change
                            .old
                            .as_ref()
                            .map(debug_value)
                            .unwrap_or_else(|| "<unset>".into()),
                        change
                            .new
                            .as_ref()
                            .map(debug_value)
                            .unwrap_or_else(|| "<unset>".into()),
                    ));
                }
                for (phase, reason) in [
                    ("LAYOUT", details.work_reasons.layout.as_deref()),
                    ("PAINT", details.work_reasons.paint.as_deref()),
                    ("COMPOSITE", details.work_reasons.composite.as_deref()),
                ] {
                    if let Some(reason) = reason {
                        lines.push(format!("Why {phase}? {reason}"));
                    }
                }
            }
            InspectorSection::Semantics => {
                lines.push(format!("semantic node: {:?}", details.semantics));
                lines.push(format!(
                    "semantic updates: {}",
                    details.state.semantic_updates
                ));
                lines.push(format!(
                    "semantic/world bounds: {:?}",
                    details.state.world_bounds
                ));
                let semantic_properties = details.properties.iter().filter(|property| {
                    let name = property.name.to_ascii_lowercase();
                    name.contains("semantic") || name.contains("label") || name.contains("role")
                });
                lines.extend(semantic_properties.map(property_line));
            }
        }
        lines
    }
}

fn normalize_widget_node(mut node: WidgetNode) -> (WidgetNode, bool) {
    let mut truncated = false;
    truncated |= truncate_utf8(&mut node.type_name, 1024);
    if let Some(key) = &mut node.key {
        truncated |= truncate_utf8(key, 1024);
    }
    if let Some(label) = &mut node.label {
        truncated |= truncate_utf8(label, 4096);
    }
    (node, truncated)
}

fn truncate_utf8(value: &mut String, max_bytes: usize) -> bool {
    if value.len() <= max_bytes {
        return false;
    }
    let mut end = max_bytes.min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    true
}

fn widget_node_bytes(node: &WidgetNode) -> usize {
    std::mem::size_of::<WidgetNode>()
        .saturating_add(node.type_name.len())
        .saturating_add(node.key.as_ref().map_or(0, String::len))
        .saturating_add(node.label.as_ref().map_or(0, String::len))
        .saturating_add(
            node.child_ids
                .len()
                .saturating_mul(std::mem::size_of::<DevWidgetId>()),
        )
}

fn console_entry_bytes(entry: &ConsoleEntry) -> usize {
    entry
        .level
        .len()
        .saturating_add(entry.target.len())
        .saturating_add(entry.message.len())
}

pub(crate) fn layout_detail_lines(details: &LayoutDetails) -> Vec<String> {
    match details {
        LayoutDetails::Box => vec!["  Box layout".into()],
        LayoutDetails::Flex {
            axis,
            available_main,
            non_flex_extent,
            flexible_extent,
            used_extent,
            remaining_extent,
            overflow,
            children,
        } => {
            let mut lines = vec![format!(
                "  Flex {axis}: available {} · non-flex {:.1} · flexible {:.1} · used {:.1} · free {:.1} · overflow {:.1}",
                available_main.map_or_else(|| "unbounded".into(), |value| format!("{value:.1}")),
                non_flex_extent,
                flexible_extent,
                used_extent,
                remaining_extent,
                overflow
            )];
            lines.extend(children.iter().map(|child| {
                format!(
                    "    #{} {}: {:.1} × {:.1} @ ({:.1}, {:.1}) · flex {:?} {:?} · allocation {:?}",
                    child.index,
                    child.type_name,
                    child.size[0],
                    child.size[1],
                    child.offset[0],
                    child.offset[1],
                    child.flex,
                    child.fit,
                    child.allocated_main_extent
                )
            }));
            lines
        }
        LayoutDetails::Stack {
            alignment,
            indexed_active,
            children,
        } => {
            let mut lines = vec![format!(
                "  Stack alignment ({:.1}, {:.1}) · active {:?}",
                alignment[0], alignment[1], indexed_active
            )];
            lines.extend(children.iter().map(|child| format!(
                "    #{} {}: ({:.1}, {:.1}) {:.1} × {:.1} · painted {} · anchors L{:?} R{:?} T{:?} B{:?}",
                child.index,
                child.type_name,
                child.bounds[0], child.bounds[1], child.bounds[2], child.bounds[3], child.painted,
                child.left, child.right, child.top, child.bottom
            )));
            lines
        }
        LayoutDetails::Positioned {
            left,
            right,
            top,
            bottom,
            width,
            height,
        } => vec![format!(
            "  Positioned: L{left:?} R{right:?} T{top:?} B{bottom:?} W{width:?} H{height:?}"
        )],
        LayoutDetails::Transform {
            matrix,
            determinant,
            invertible,
        } => vec![format!(
            "  Transform [{:.3} {:.3} {:.3} {:.3} {:.1} {:.1}] · det {:.3} · inverse {}",
            matrix[0],
            matrix[1],
            matrix[2],
            matrix[3],
            matrix[4],
            matrix[5],
            determinant,
            if *invertible {
                "available"
            } else {
                "unavailable"
            }
        )],
        LayoutDetails::Fitted {
            fit,
            alignment,
            source_size,
            destination_size,
            determinant,
            invertible,
            ..
        } => vec![format!(
            "  Fitted {fit}: source {source_size:?} → {:.1} × {:.1}, align ({:.1}, {:.1}), det {:.3}, inverse {}",
            destination_size[0],
            destination_size[1],
            alignment[0],
            alignment[1],
            determinant,
            if *invertible {
                "available"
            } else {
                "unavailable"
            }
        )],
        LayoutDetails::Scroll {
            viewport_extent,
            content_extent,
            offset,
            min_scroll,
            max_scroll,
            ..
        } => vec![format!(
            "  Scroll: viewport {:.1} · content {:.1} · offset {:.1} · range {:.1}..{:.1}",
            viewport_extent, content_extent, offset, min_scroll, max_scroll
        )],
        LayoutDetails::LazyViewport {
            item_count,
            materialized_start,
            materialized_end,
            materialized_items,
            viewport_extent,
            cache_extent,
            scroll_offset,
        } => vec![format!(
            "  Lazy viewport: {item_count} items · materialized {materialized_start}..{materialized_end} ({materialized_items}) · viewport {:.1} · cache {:.1} · offset {:.1}",
            viewport_extent, cache_extent, scroll_offset
        )],
        LayoutDetails::Text {
            text_length,
            max_lines,
            overflow,
            line_count,
        } => vec![format!(
            "  Text: {text_length} chars · {line_count:?} retained lines · max {max_lines:?} · {overflow}"
        )],
        LayoutDetails::Custom { layout_kind } => vec![format!("  retained layout: {layout_kind}")],
    }
}

pub(crate) fn property_line(property: &DebugProperty) -> String {
    let override_mark = if property.overridden {
        " [DEV OVERRIDE]"
    } else {
        ""
    };
    format!(
        "{}: {}{override_mark}",
        property.name,
        debug_value(&property.value)
    )
}

pub(crate) fn debug_value(value: &DebugValue) -> String {
    match value {
        DebugValue::Bool(value) => value.to_string(),
        DebugValue::Int(value) => value.to_string(),
        DebugValue::Uint(value) => value.to_string(),
        DebugValue::Float(value) => format!("{value:.3}"),
        DebugValue::Str(value) | DebugValue::Enum(value) => value.clone(),
        DebugValue::Color(r, g, b, a) => format!("#{r:02X}{g:02X}{b:02X}{a:02X}"),
        DebugValue::Size([width, height]) => format!("{width:.1} × {height:.1}"),
        DebugValue::Offset([x, y]) => format!("({x:.1}, {y:.1})"),
        DebugValue::Rect([x, y, width, height]) => {
            format!("({x:.1}, {y:.1}) {width:.1} × {height:.1}")
        }
        DebugValue::Insets([left, top, right, bottom]) => {
            format!("L{left:.1} T{top:.1} R{right:.1} B{bottom:.1}")
        }
        DebugValue::Constraints {
            min_width,
            max_width,
            min_height,
            max_height,
        } => {
            let maximum = |value: f32| {
                if value == f32::INFINITY {
                    "unbounded".into()
                } else {
                    format!("{value:.1}")
                }
            };
            format!(
                "w {min_width:.1}..{}; h {min_height:.1}..{}",
                maximum(*max_width),
                maximum(*max_height)
            )
        }
        DebugValue::Optional(Some(value)) => debug_value(value),
        DebugValue::Optional(None) => "none".into(),
        DebugValue::List(values) => format!("{} values", values.len()),
        DebugValue::Redacted => "<redacted>".into(),
    }
}

#[doc(hidden)]
pub fn parse_debug_value(template: &DebugValue, input: &str) -> Option<DebugValue> {
    let input = input.trim();
    let floats = |expected: usize| -> Option<Vec<f32>> {
        let values = input
            .split([',', ' ', '×'])
            .filter(|part| !part.is_empty())
            .map(str::parse::<f32>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        (values.len() == expected && values.iter().all(|value| value.is_finite())).then_some(values)
    };
    match template {
        DebugValue::Bool(_) => input.parse().ok().map(DebugValue::Bool),
        DebugValue::Int(_) => input.parse().ok().map(DebugValue::Int),
        DebugValue::Uint(_) => input.parse().ok().map(DebugValue::Uint),
        DebugValue::Float(_) => input
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(DebugValue::Float),
        DebugValue::Str(_) => Some(DebugValue::Str(input.into())),
        DebugValue::Enum(_) => Some(DebugValue::Enum(input.into())),
        DebugValue::Color(_, _, _, _) => {
            let hex = input.strip_prefix('#').unwrap_or(input);
            if !matches!(hex.len(), 6 | 8) {
                return None;
            }
            let color = u32::from_str_radix(hex, 16).ok()?;
            let (r, g, b, a) = if hex.len() == 6 {
                (
                    ((color >> 16) & 0xff) as u8,
                    ((color >> 8) & 0xff) as u8,
                    (color & 0xff) as u8,
                    255,
                )
            } else {
                (
                    ((color >> 24) & 0xff) as u8,
                    ((color >> 16) & 0xff) as u8,
                    ((color >> 8) & 0xff) as u8,
                    (color & 0xff) as u8,
                )
            };
            Some(DebugValue::Color(r, g, b, a))
        }
        DebugValue::Size(_) => floats(2).map(|v| DebugValue::Size([v[0], v[1]])),
        DebugValue::Offset(_) => floats(2).map(|v| DebugValue::Offset([v[0], v[1]])),
        DebugValue::Rect(_) => floats(4).map(|v| DebugValue::Rect([v[0], v[1], v[2], v[3]])),
        DebugValue::Insets(_) => floats(4).map(|v| DebugValue::Insets([v[0], v[1], v[2], v[3]])),
        DebugValue::Constraints { .. }
        | DebugValue::Optional(_)
        | DebugValue::List(_)
        | DebugValue::Redacted => None,
    }
}
