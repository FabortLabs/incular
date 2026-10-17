//! Retained models for the protocol v3 inspection views: counter history with
//! per-second rates, and flattened semantics/layer trees with search.
//!
//! Everything here is plain data so deltas and tree flattening stay cheap to
//! test without a window or a connected target.

use incular_devtools_protocol::{LayerNodeWire, RuntimeDiagnosticsWire, SemanticsNodeWire};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

/// Hard cap on rows produced for one tree view.
pub const MAX_VISIBLE_ROWS: usize = 8192;
/// Protocol version that introduced the read-only inspection requests.
pub const INSPECTION_PROTOCOL_VERSION: u32 = 3;

/// One polled set of cumulative counters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticsSample {
    pub at: Instant,
    pub wire: RuntimeDiagnosticsWire,
}

/// The two most recent samples, which is all a per-second delta needs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticsHistory {
    previous: Option<DiagnosticsSample>,
    latest: Option<DiagnosticsSample>,
}

impl DiagnosticsHistory {
    /// Records a new poll result.
    pub fn push(&mut self, wire: RuntimeDiagnosticsWire, at: Instant) {
        self.previous = self.latest.take();
        self.latest = Some(DiagnosticsSample { at, wire });
    }

    /// Forgets the previous sample so rates restart from the next poll.
    pub fn reset_deltas(&mut self) {
        self.previous = None;
    }

    /// Drops every sample, for example after a reconnect.
    pub fn clear(&mut self) {
        self.previous = None;
        self.latest = None;
    }

    pub fn latest(&self) -> Option<&RuntimeDiagnosticsWire> {
        self.latest.as_ref().map(|sample| &sample.wire)
    }

    /// Per-second change of one cumulative counter between the last two polls.
    pub fn rate(&self, counter: impl Fn(&RuntimeDiagnosticsWire) -> u64) -> Option<f64> {
        let (previous, latest) = (self.previous.as_ref()?, self.latest.as_ref()?);
        per_second(
            counter(&previous.wire),
            counter(&latest.wire),
            latest.at.checked_duration_since(previous.at)?,
        )
    }
}

/// Per-second rate between two cumulative readings. Returns `None` when the
/// counter went backwards (the target restarted or reset it) or no time passed.
pub fn per_second(previous: u64, current: u64, elapsed: Duration) -> Option<f64> {
    let seconds = elapsed.as_secs_f64();
    (current >= previous && seconds > 0.001).then(|| (current - previous) as f64 / seconds)
}

/// Cache hit rate in percent, or `None` before the first lookup.
pub fn hit_rate(hits: u64, misses: u64) -> Option<f64> {
    let total = hits.checked_add(misses)?;
    (total > 0).then(|| hits as f64 * 100. / total as f64)
}

/// Compact human count: `1 234`, `12.3k`, `4.5M`.
pub fn format_count(value: u64) -> String {
    match value {
        0..=9_999 => {
            let digits = value.to_string();
            if digits.len() <= 3 {
                digits
            } else {
                format!(
                    "{} {}",
                    &digits[..digits.len() - 3],
                    &digits[digits.len() - 3..]
                )
            }
        }
        10_000..=999_999 => format!("{:.1}k", value as f64 / 1_000.),
        _ => format!("{:.2}M", value as f64 / 1_000_000.),
    }
}

/// Formats a rate such as `+12.5/s`; absent rates render as an en dash.
pub fn format_rate(rate: Option<f64>) -> String {
    match rate {
        None => "–".into(),
        Some(value) if value < 0.05 => "0/s".into(),
        Some(value) if value < 100. => format!("+{value:.1}/s"),
        Some(value) => format!("+{value:.0}/s"),
    }
}

/// Behaviour a wire node must offer to live in a [`TreeView`].
pub trait TreeNode {
    fn node_id(&self) -> u64;
    fn parent_id(&self) -> Option<u64>;
    fn child_ids(&self) -> &[u64];
    /// Lower-case text that the search box matches against.
    fn search_text(&self) -> String;
}

impl TreeNode for SemanticsNodeWire {
    fn node_id(&self) -> u64 {
        self.id
    }
    fn parent_id(&self) -> Option<u64> {
        self.parent
    }
    fn child_ids(&self) -> &[u64] {
        &self.children
    }
    fn search_text(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.role,
            self.label.as_deref().unwrap_or_default(),
            self.value.as_deref().unwrap_or_default(),
            self.description.as_deref().unwrap_or_default(),
            self.flags.join(" "),
        )
        .to_lowercase()
    }
}

impl TreeNode for LayerNodeWire {
    fn node_id(&self) -> u64 {
        self.id
    }
    fn parent_id(&self) -> Option<u64> {
        self.parent
    }
    fn child_ids(&self) -> &[u64] {
        &self.children
    }
    fn search_text(&self) -> String {
        format!("{} {}", self.kind, self.summary).to_lowercase()
    }
}

/// One visible tree row, referring to a node by its index in the snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibleRow {
    pub index: usize,
    pub depth: u16,
    pub has_children: bool,
    /// True when the node itself matches the active search.
    pub matched: bool,
}

/// A pre-order node snapshot with expansion, selection and search state.
#[derive(Clone, Debug)]
pub struct TreeView<N> {
    nodes: Vec<N>,
    index: HashMap<u64, usize>,
    roots: Vec<usize>,
    pub expanded: HashSet<u64>,
    pub selected: Option<u64>,
    pub query: String,
    pub truncated: bool,
    pub loaded: bool,
    /// Levels opened when the first snapshot arrives.
    pub initial_depth: u16,
}

impl<N> Default for TreeView<N> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            index: HashMap::new(),
            roots: Vec::new(),
            expanded: HashSet::new(),
            selected: None,
            query: String::new(),
            truncated: false,
            loaded: false,
            initial_depth: 3,
        }
    }
}

impl<N: TreeNode> TreeView<N> {
    /// Replaces the snapshot, keeping selection, expansion and search where
    /// the nodes still exist. The first snapshot opens the top levels.
    pub fn replace(&mut self, nodes: Vec<N>, truncated: bool) {
        let first_load = !self.loaded;
        self.index = nodes
            .iter()
            .enumerate()
            .map(|(position, node)| (node.node_id(), position))
            .collect();
        self.roots = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                node.parent_id()
                    .is_none_or(|parent| !self.index.contains_key(&parent))
            })
            .map(|(position, _)| position)
            .collect();
        self.nodes = nodes;
        self.truncated = truncated;
        self.loaded = true;
        let index = &self.index;
        self.expanded.retain(|id| index.contains_key(id));
        if self.selected.is_some_and(|id| !index.contains_key(&id)) {
            self.selected = None;
        }
        if first_load {
            let seeds = self
                .visible_with(|_| true, Some(self.initial_depth))
                .into_iter()
                .filter(|row| row.has_children)
                .map(|row| self.nodes[row.index].node_id())
                .collect::<Vec<_>>();
            self.expanded.extend(seeds);
        }
    }

    pub fn nodes(&self) -> &[N] {
        &self.nodes
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, id: u64) -> Option<&N> {
        self.index.get(&id).map(|position| &self.nodes[*position])
    }

    pub fn selected_node(&self) -> Option<&N> {
        self.selected.and_then(|id| self.node(id))
    }

    /// Selects an existing node; returns whether the id was found.
    pub fn select(&mut self, id: u64) -> bool {
        let found = self.index.contains_key(&id);
        if found {
            self.selected = Some(id);
        }
        found
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    pub fn toggle(&mut self, id: u64) {
        if !self.expanded.remove(&id) {
            self.expanded.insert(id);
        }
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
    }

    pub fn expand_all(&mut self) {
        self.expanded = self
            .nodes
            .iter()
            .filter(|node| !node.child_ids().is_empty())
            .map(TreeNode::node_id)
            .collect();
    }

    pub fn collapse_all(&mut self) {
        self.expanded.clear();
    }

    /// Whether the search box currently filters rows.
    pub fn searching(&self) -> bool {
        !self.query.trim().is_empty()
    }

    /// Rows to display: the expanded tree, or, while searching, every match
    /// together with its ancestors so results keep their context.
    pub fn visible_rows(&self) -> Vec<VisibleRow> {
        if !self.searching() {
            return self.visible_with(|_| true, None);
        }
        let terms = self
            .query
            .to_lowercase()
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let matches = self
            .nodes
            .iter()
            .map(|node| {
                let text = node.search_text();
                terms.iter().all(|term| text.contains(term.as_str()))
            })
            .collect::<Vec<_>>();
        let mut keep = matches.clone();
        for (position, matched) in matches.iter().enumerate() {
            if !matched {
                continue;
            }
            let mut cursor = self.nodes[position].parent_id();
            let mut hops = 0;
            while let Some(parent) = cursor {
                let Some(parent_position) = self.index.get(&parent).copied() else {
                    break;
                };
                keep[parent_position] = true;
                cursor = self.nodes[parent_position].parent_id();
                hops += 1;
                if hops > 4096 {
                    break;
                }
            }
        }
        let mut rows = self.visible_with(|position| keep[position], None);
        for row in &mut rows {
            row.matched = matches[row.index];
            row.has_children = false;
        }
        rows
    }

    fn visible_with(
        &self,
        include: impl Fn(usize) -> bool,
        depth_limit: Option<u16>,
    ) -> Vec<VisibleRow> {
        let searching = self.searching() && depth_limit.is_none();
        let mut rows = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = self
            .roots
            .iter()
            .rev()
            .copied()
            .map(|position| (position, 0_u16))
            .collect::<Vec<_>>();
        while let Some((position, depth)) = stack.pop() {
            if rows.len() >= MAX_VISIBLE_ROWS {
                break;
            }
            if !include(position) || !seen.insert(position) {
                continue;
            }
            let node = &self.nodes[position];
            let children = node
                .child_ids()
                .iter()
                .filter_map(|child| self.index.get(child).copied())
                .collect::<Vec<_>>();
            rows.push(VisibleRow {
                index: position,
                depth,
                has_children: !children.is_empty(),
                matched: false,
            });
            let descend = match depth_limit {
                Some(limit) => depth + 1 < limit,
                None => searching || self.expanded.contains(&node.node_id()),
            };
            if descend {
                for child in children.into_iter().rev() {
                    stack.push((child, depth.saturating_add(1)));
                }
            }
        }
        rows
    }

    /// Ancestors of a node from the root down, excluding the node itself.
    pub fn ancestors(&self, id: u64) -> Vec<u64> {
        let mut path = Vec::new();
        let mut seen = HashSet::from([id]);
        let mut cursor = self.node(id).and_then(TreeNode::parent_id);
        while let Some(parent) = cursor {
            if !self.index.contains_key(&parent) || !seen.insert(parent) {
                break;
            }
            path.push(parent);
            cursor = self.node(parent).and_then(TreeNode::parent_id);
        }
        path.reverse();
        path
    }

    /// Expands every ancestor so the node becomes a visible row.
    pub fn reveal(&mut self, id: u64) {
        let ancestors = self.ancestors(id);
        self.expanded.extend(ancestors);
    }
}

/// Window-space `[x, y, width, height]` a node highlights, when it has any
/// non-degenerate geometry.
pub fn highlight_bounds(bounds: [f32; 4]) -> Option<[f32; 4]> {
    (bounds.iter().all(|value| value.is_finite()) && bounds[2] > 0. && bounds[3] > 0.)
        .then_some(bounds)
}

/// Coarse relative age such as `just now`, `42s ago`, `7m ago` or `3h ago`.
pub fn format_age(elapsed_ms: u64) -> String {
    let seconds = elapsed_ms / 1000;
    match seconds {
        0..=4 => "just now".into(),
        5..=59 => format!("{seconds}s ago"),
        60..=3599 => format!("{}m ago", seconds / 60),
        3600..=86_399 => format!("{}h ago", seconds / 3600),
        _ => format!("{}d ago", seconds / 86_400),
    }
}
