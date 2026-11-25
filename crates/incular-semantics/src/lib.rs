//! Renderer- and platform-neutral retained accessibility semantics.
//!
//! This crate owns the semantic data model and its generational retained tree.
//! Widget trees map their persistent identities to these nodes, while runtimes
//! and native adapters communicate exclusively through `SemanticNodeId` and
//! `SemanticAction`.

use incular_core::{Arena, ArenaId, Rect};
use std::cmp::Ordering;

mod attributed_string;
mod custom_action;
mod label_builder;
pub use attributed_string::{
    AttributedString, InvalidStringAttribute, SemanticTextAttributes, StringAttribute,
};
pub use custom_action::{CustomSemanticsAction, CustomSemanticsActionId};
pub use incular_core::TextDirection;
pub use label_builder::SemanticsLabelBuilder;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SemanticNodeId(pub ArenaId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Button,
    Text,
    TextField,
    TextArea,
    List,
    ListItem,
    ScrollView,
    GenericContainer,
    Checkbox,
    Radio,
    /// A two-state toggle distinct from a momentary button.
    Switch,
    Slider,
    Menu,
    Dialog,
    Image,
    Heading,
    Link,
    Group,
    Tab,
    TabList,
    TabPanel,
    MenuItem,
    ProgressBar,
    Meter,
    SearchField,
}

pub type SemanticRole = Role;

/// What this annotation blocks from accessibility focus, independently
/// of the widget's real input focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AccessibilityFocusBlockType {
    #[default]
    None,
    BlockNode,
    BlockSubtree,
}

impl AccessibilityFocusBlockType {
    #[must_use]
    pub const fn blocks_focus(self) -> bool {
        !matches!(self, Self::None)
    }

    /// A merged node receives the strongest block of its contributors.
    #[must_use]
    pub const fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::BlockSubtree, _) | (_, Self::BlockSubtree) => Self::BlockSubtree,
            (Self::BlockNode, _) | (_, Self::BlockNode) => Self::BlockNode,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticActionKind {
    Focus,
    Activate,
    SetText,
    SetSelection,
    Increment,
    Decrement,
    ScrollForward,
    ScrollBackward,
    ScrollLeft,
    ScrollRight,
    ScrollUp,
    ScrollDown,
    /// Flutter `SemanticsAction.scrollToOffset`.
    ScrollToOffset,
    /// Flutter's `SemanticsAction.longPress`: a press-and-hold activation.
    LongPress,
    /// Flutter's `SemanticsAction.showOnScreen`: reveal the node inside its
    /// nearest scroll viewport.
    ShowOnScreen,
    /// An application-defined action with a canonical identifier.
    Custom,
}

impl SemanticActionKind {
    /// Whether this action requests a physical rather than logical scroll.
    #[must_use]
    pub const fn is_directional_scroll(self) -> bool {
        matches!(
            self,
            Self::ScrollLeft | Self::ScrollRight | Self::ScrollUp | Self::ScrollDown
        )
    }
}

/// A request received from a native accessibility adapter. Text offsets use
/// the editing model's UTF-8 byte indexing convention.
#[derive(Clone, Debug, PartialEq)]
pub enum SemanticAction {
    Focus,
    Activate,
    SetText(String),
    SetSelection {
        base: usize,
        extent: usize,
    },
    Increment,
    Decrement,
    ScrollForward,
    ScrollBackward,
    ScrollLeft,
    ScrollRight,
    ScrollUp,
    ScrollDown,
    /// Jump to an absolute logical offset; a one-dimensional view takes
    /// the coordinate on its own axis.
    ScrollToOffset {
        x: f64,
        y: f64,
    },
    LongPress,
    ShowOnScreen,
    Custom(CustomSemanticsActionId),
}

impl SemanticAction {
    #[must_use]
    pub const fn kind(&self) -> SemanticActionKind {
        match self {
            Self::Focus => SemanticActionKind::Focus,
            Self::Activate => SemanticActionKind::Activate,
            Self::SetText(_) => SemanticActionKind::SetText,
            Self::SetSelection { .. } => SemanticActionKind::SetSelection,
            Self::Increment => SemanticActionKind::Increment,
            Self::Decrement => SemanticActionKind::Decrement,
            Self::ScrollForward => SemanticActionKind::ScrollForward,
            Self::ScrollBackward => SemanticActionKind::ScrollBackward,
            Self::ScrollLeft => SemanticActionKind::ScrollLeft,
            Self::ScrollRight => SemanticActionKind::ScrollRight,
            Self::ScrollUp => SemanticActionKind::ScrollUp,
            Self::ScrollDown => SemanticActionKind::ScrollDown,
            Self::ScrollToOffset { .. } => SemanticActionKind::ScrollToOffset,
            Self::LongPress => SemanticActionKind::LongPress,
            Self::ShowOnScreen => SemanticActionKind::ShowOnScreen,
            Self::Custom(_) => SemanticActionKind::Custom,
        }
    }
}

/// Flutter's `OrdinalSortKey`: orders a node among its siblings. Keys with the
/// same `name` form one group; keys without a name sort before named ones.
#[derive(Clone, Debug, PartialEq)]
pub struct OrdinalSortKey {
    pub order: f64,
    pub name: Option<String>,
}

impl OrdinalSortKey {
    /// A key with no group name. A non-finite `order` becomes `0.0`.
    #[must_use]
    pub fn new(order: f64) -> Self {
        Self {
            order: if order.is_finite() { order } else { 0.0 },
            name: None,
        }
    }

    /// A key in the named group `name`.
    #[must_use]
    pub fn named(order: f64, name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::new(order)
        }
    }

    /// Flutter's ordinal comparison: unnamed keys precede named ones,
    /// different names compare by UTF-16 code units, and identical names
    /// defer to the numeric value. Signed zero remains a numeric tie.
    #[must_use]
    pub fn compare_to(&self, other: &Self) -> Ordering {
        match (&self.name, &other.name) {
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(left), Some(right)) if left != right => {
                left.encode_utf16().cmp(right.encode_utf16())
            }
            _ if self.order == other.order => Ordering::Equal,
            _ => self.order.total_cmp(&other.order),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextSelection {
    pub base: usize,
    pub extent: usize,
}

/// Value of a checkable control. `None` in [`SemanticState::checked`] means the
/// node is not checkable; an indeterminate control is still checkable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckedState {
    #[default]
    Unchecked,
    Checked,
    Indeterminate,
}

impl CheckedState {
    #[must_use]
    pub const fn is_checked(self) -> bool {
        matches!(self, Self::Checked)
    }
}

impl From<bool> for CheckedState {
    fn from(checked: bool) -> Self {
        if checked {
            Self::Checked
        } else {
            Self::Unchecked
        }
    }
}

impl std::fmt::Display for CheckedState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Unchecked => "false",
            Self::Checked => "true",
            Self::Indeterminate => "mixed",
        })
    }
}

/// Main axis used by a one-dimensional semantic scroll view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticScrollAxis {
    Horizontal,
    Vertical,
}

/// Logical scroll values exported without rounding or string parsing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemanticScrollMetrics {
    pub axis: SemanticScrollAxis,
    pub reverse: bool,
    pub offset: f64,
    pub minimum: f64,
    pub maximum: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticState {
    /// Whether the node accepts input. Flutter's default is "unspecified",
    /// which assistive technology reads as enabled, so the default here is
    /// `true` and only an explicit `false` disables an interactive node.
    pub enabled: bool,
    pub focused: bool,
    pub focusable: bool,
    /// Effective Flutter accessibility focus scope for this semantic node.
    pub accessibility_focus_block_type: AccessibilityFocusBlockType,
    pub selected: bool,
    pub checked: Option<CheckedState>,
    pub expanded: Option<bool>,
    pub read_only: bool,
    pub editable: bool,
    /// Whether a text field's value is visually obscured (for example a
    /// password field).  Keeping this in the portable tree lets native
    /// accessibility adapters make the same privacy decision as the widget
    /// layer without exposing the clear-text paint representation.
    pub obscured: bool,
    pub multiline: bool,
    pub selection: Option<TextSelection>,
    pub item_index: Option<usize>,
    pub set_size: Option<usize>,
    /// Indicates that the node's value is still being produced.
    pub busy: bool,
    /// Validation state exposed independently from enabled/read-only state.
    pub invalid: bool,
    /// Whether the user must provide a value.
    pub required: bool,
    /// Numeric range/value metadata used by sliders, meters, and progress
    /// indicators. Values stay floating point because Flutter controls do.
    pub numeric_value: Option<f64>,
    pub numeric_min: Option<f64>,
    pub numeric_max: Option<f64>,
    pub numeric_step: Option<f64>,
    /// Heading level in the portable tree (1 is the most prominent level).
    pub heading_level: Option<u8>,
    /// Flutter's `hasToggledState`/`isToggled`, for on/off controls such as
    /// switches; `None` when the node has no toggled state.
    pub toggled: Option<bool>,
    /// Flutter's `isHidden`: present in the tree but not currently visible,
    /// such as an off-screen item a screen reader may scroll to.
    pub hidden: bool,
    /// Flutter `hasImplicitScrolling`.
    pub has_implicit_scrolling: bool,
    pub scroll_metrics: Option<SemanticScrollMetrics>,
    /// Flutter's `isLiveRegion`: changes are announced politely.
    pub live_region: bool,
    /// Base direction of the node’s accessible label.
    pub text_direction: Option<TextDirection>,
    /// BCP-47 pronunciation locale inherited from the nearest semantic scope.
    pub locale: Option<std::sync::Arc<str>>,
    /// Optional pronunciation spans for the node’s semantic strings.
    pub text_attributes: Option<std::sync::Arc<SemanticTextAttributes>>,
    /// Flutter's `tooltip`.
    pub tooltip: Option<String>,
    /// Flutter's `linkUrl`: where a link node navigates.
    pub link_url: Option<String>,
    /// Flutter's `identifier`: a stable platform identifier (Android
    /// resource-id, iOS accessibilityIdentifier) for UI automation.
    pub identifier: Option<String>,
    /// Flutter's `increasedValue`: the value an increment action would
    /// produce, for steppers and sliders.
    pub increased_value: Option<String>,
    /// Flutter's `decreasedValue`: the value a decrement action would
    /// produce.
    pub decreased_value: Option<String>,
    /// Flutter's `sortKey`: the traversal position of this node among its
    /// siblings. Siblings with compatible keys are reordered (see
    /// `OrdinalSortKey`); unkeyed siblings keep their widget order.
    pub sort_key: Option<OrdinalSortKey>,
    /// Flutter's `container`: the node is always its own accessibility node,
    /// even without other metadata.
    pub container: bool,
    /// Flutter's `explicitChildNodes`: when this node merges its descendants,
    /// its children stay separate nodes instead.
    pub explicit_child_nodes: bool,
    /// Canonical action identifiers provided by the node or absorbed children.
    pub custom_actions: Box<[CustomSemanticsActionId]>,
}

impl Default for SemanticState {
    fn default() -> Self {
        Self {
            enabled: true,
            focused: false,
            focusable: false,
            accessibility_focus_block_type: AccessibilityFocusBlockType::None,
            selected: false,
            checked: None,
            expanded: None,
            read_only: false,
            editable: false,
            obscured: false,
            multiline: false,
            selection: None,
            item_index: None,
            set_size: None,
            busy: false,
            invalid: false,
            required: false,
            numeric_value: None,
            numeric_min: None,
            numeric_max: None,
            numeric_step: None,
            heading_level: None,
            toggled: None,
            hidden: false,
            has_implicit_scrolling: false,
            scroll_metrics: None,
            live_region: false,
            text_direction: None,
            locale: None,
            text_attributes: None,
            tooltip: None,
            link_url: None,
            identifier: None,
            increased_value: None,
            decreased_value: None,
            sort_key: None,
            container: false,
            explicit_child_nodes: false,
            custom_actions: Box::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticNode {
    pub id: SemanticNodeId,
    pub role: Role,
    pub label: Option<String>,
    pub value: Option<String>,
    pub description: Option<String>,
    pub bounds: Rect,
    pub state: SemanticState,
    pub actions: Vec<SemanticActionKind>,
    pub children: Vec<SemanticNodeId>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SemanticsDiagnostics {
    pub nodes_created: u64,
    pub nodes_removed: u64,
    pub nodes_updated: u64,
    pub property_updates: u64,
    pub geometry_updates: u64,
    pub native_updates: u64,
    pub actions_received: u64,
    /// Monotonically increases only when the retained semantic graph changes.
    /// Native adapters use this to avoid rebuilding or even walking an
    /// unchanged tree on paint/compositor-only frames.
    pub revisions: u64,
}

/// A retained node and the graph revision that last inserted or changed it.
struct RetainedNode {
    node: SemanticNode,
    changed_at: u64,
}

/// Retained semantic arena. IDs are generational, so stale platform actions
/// cannot address a node that later reuses the same storage slot.
#[derive(Default)]
pub struct SemanticsTree {
    nodes: Arena<RetainedNode>,
    // Latest content revision per contiguous block of 64 arena slots.
    block_revisions: Vec<u64>,
    focused: std::collections::BTreeSet<ArenaId>,
    /// Parentless nodes in logical order. Layout-only ancestors create no
    /// semantic node, so a retained graph can be a forest; the first entry
    /// is the designated root.
    roots: Vec<SemanticNodeId>,
    diagnostics: SemanticsDiagnostics,
}

impl SemanticsTree {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// The designated root: the first of [`Self::roots`].
    #[must_use]
    pub const fn root(&self) -> Option<SemanticNodeId> {
        self.roots.as_slice().first().copied()
    }
    /// Every parentless node in logical order. Platform projections must
    /// attach all of them; the graph is a forest whenever a layout-only
    /// widget has several semantic children.
    #[must_use]
    pub fn roots(&self) -> &[SemanticNodeId] {
        &self.roots
    }
    /// Replaces the roots with a single designated root, or none.
    pub fn set_root(&mut self, root: Option<SemanticNodeId>) {
        self.set_roots(root);
    }
    /// Replaces the roots, in logical order. Changing them advances the
    /// revision so projections republish their host node.
    pub fn set_roots(&mut self, roots: impl IntoIterator<Item = SemanticNodeId>) {
        let mut changed = false;
        let mut len = 0;
        for root in roots {
            match self.roots.get_mut(len) {
                Some(slot) if *slot == root => {}
                Some(slot) => {
                    *slot = root;
                    changed = true;
                }
                None => {
                    self.roots.push(root);
                    changed = true;
                }
            }
            len += 1;
        }
        if len != self.roots.len() {
            self.roots.truncate(len);
            changed = true;
        }
        if changed {
            self.diagnostics.revisions = self.diagnostics.revisions.wrapping_add(1);
        }
    }
    #[must_use]
    pub fn node(&self, id: SemanticNodeId) -> Option<&SemanticNode> {
        self.nodes.get(id.0).map(|retained| &retained.node)
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    #[must_use]
    pub const fn diagnostics(&self) -> SemanticsDiagnostics {
        self.diagnostics
    }

    /// Revision of the retained semantic graph.
    ///
    /// This is deliberately independent from paint and compositor state.  A
    /// platform bridge can use it as a cheap dirty signal before performing
    /// any projection work.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.diagnostics.revisions
    }

    pub fn insert(&mut self, node: SemanticNode) -> SemanticNodeId {
        self.diagnostics.revisions = self.diagnostics.revisions.wrapping_add(1);
        let id = SemanticNodeId(self.nodes.insert(RetainedNode {
            node,
            changed_at: self.diagnostics.revisions,
        }));
        self.nodes.get_mut(id.0).expect("new semantic node").node.id = id;
        if self.node(id).expect("new semantic node").state.focused {
            self.focused.insert(id.0);
        }
        let block = id.0.index() as usize / 64;
        if block == self.block_revisions.len() {
            self.block_revisions.push(0);
        }
        self.block_revisions[block] = self.diagnostics.revisions;
        self.diagnostics.nodes_created += 1;
        id
    }

    pub fn update(&mut self, id: SemanticNodeId, node: SemanticNode) -> bool {
        let Some(retained) = self.nodes.get_mut(id.0) else {
            return false;
        };
        let old = &mut retained.node;
        let properties_changed = old.role != node.role
            || old.label != node.label
            || old.value != node.value
            || old.description != node.description
            || old.state != node.state
            || old.actions != node.actions
            || old.children != node.children;
        let geometry_changed = old.bounds != node.bounds;
        if properties_changed || geometry_changed {
            if old.state.focused != node.state.focused {
                if node.state.focused {
                    self.focused.insert(id.0);
                } else {
                    self.focused.remove(&id.0);
                }
            }
            *old = node;
            self.diagnostics.nodes_updated += 1;
            self.diagnostics.revisions = self.diagnostics.revisions.wrapping_add(1);
            retained.changed_at = self.diagnostics.revisions;
            self.block_revisions[id.0.index() as usize / 64] = retained.changed_at;
        }
        if properties_changed {
            self.diagnostics.property_updates += 1;
        }
        if geometry_changed {
            self.diagnostics.geometry_updates += 1;
        }
        true
    }

    /// Updates only `bounds` and `children` of a retained node, leaving its
    /// role, text, state and actions untouched and uncopied. Reports the same
    /// revision and diagnostics changes as [`Self::update`] would for a node
    /// whose other properties are unchanged. Returns `false` for a stale id.
    pub fn update_geometry(
        &mut self,
        id: SemanticNodeId,
        bounds: Rect,
        children: &[SemanticNodeId],
    ) -> bool {
        self.update_geometry_and_visibility(id, bounds, children, None)
    }

    /// Updates geometry and the visibility flag when it derives from a
    /// retained clipping boundary. Other semantic properties stay untouched.
    pub fn update_geometry_and_visibility(
        &mut self,
        id: SemanticNodeId,
        bounds: Rect,
        children: &[SemanticNodeId],
        hidden: Option<bool>,
    ) -> bool {
        let Some(retained) = self.nodes.get_mut(id.0) else {
            return false;
        };
        let old = &mut retained.node;
        let children_changed = old.children != children;
        let geometry_changed = old.bounds != bounds;
        let visibility_changed = hidden.is_some_and(|hidden| old.state.hidden != hidden);
        if !children_changed && !geometry_changed && !visibility_changed {
            return true;
        }
        old.bounds = bounds;
        if children_changed {
            old.children.clear();
            old.children.extend_from_slice(children);
        }
        if let Some(hidden) = hidden {
            old.state.hidden = hidden;
        }
        if children_changed || visibility_changed {
            self.diagnostics.property_updates += 1;
        }
        if geometry_changed {
            self.diagnostics.geometry_updates += 1;
        }
        self.diagnostics.nodes_updated += 1;
        self.diagnostics.revisions = self.diagnostics.revisions.wrapping_add(1);
        retained.changed_at = self.diagnostics.revisions;
        self.block_revisions[id.0.index() as usize / 64] = retained.changed_at;
        true
    }

    pub fn remove(&mut self, id: SemanticNodeId) -> Option<SemanticNode> {
        let node = self.nodes.remove(id.0)?.node;
        if node.state.focused {
            self.focused.remove(&id.0);
        }
        self.diagnostics.nodes_removed += 1;
        self.diagnostics.revisions = self.diagnostics.revisions.wrapping_add(1);
        self.roots.retain(|root| *root != id);
        Some(node)
    }

    pub fn note_action(&mut self) {
        self.diagnostics.actions_received += 1;
    }
    pub fn note_native_update(&mut self) {
        self.diagnostics.native_updates += 1;
    }
    pub fn iter(&self) -> impl Iterator<Item = (SemanticNodeId, &SemanticNode)> {
        self.nodes
            .iter()
            .map(|(id, retained)| (SemanticNodeId(id), &retained.node))
    }

    /// The first focused live node in arena order.
    pub fn focused_node(&self) -> Option<SemanticNodeId> {
        self.focused.first().copied().map(SemanticNodeId)
    }

    /// Nodes inserted or changed after graph revision `revision`, in arena
    /// order. A projection last synchronized at `revision` reads these
    /// instead of copying and comparing the whole tree. Removals are not
    /// listed; [`SemanticsDiagnostics::nodes_removed`] tells whether any
    /// happened.
    pub fn changed_since(
        &self,
        revision: u64,
    ) -> impl Iterator<Item = (SemanticNodeId, &SemanticNode)> {
        self.block_revisions
            .iter()
            .enumerate()
            .filter(move |(_, latest)| **latest > revision)
            .flat_map(|(block, _)| {
                self.nodes
                    .iter_slots(block * 64..(block * 64).saturating_add(64))
            })
            .filter(move |(_, retained)| retained.changed_at > revision)
            .map(|(id, retained)| (SemanticNodeId(id), &retained.node))
    }

    #[must_use]
    pub fn debug_dump(&self) -> String {
        self.debug_dump_limited(usize::MAX, usize::MAX, usize::MAX)
    }

    /// Produces a deterministic diagnostic dump with hard node, depth, and
    /// text limits. Accessibility trees can be application-sized, so debug
    /// tooling must not recurse or allocate in proportion to an unbounded
    /// subtree. A truncation marker is included in the returned text when a
    /// limit is reached.
    #[must_use]
    pub fn debug_dump_bounded(&self, max_nodes: usize) -> String {
        self.debug_dump_limited(max_nodes, 256, 256)
    }

    /// Lower-level bounded dump used by retained diagnostics and visual
    /// debuggers that need a tighter label/depth budget.
    #[must_use]
    pub fn debug_dump_limited(
        &self,
        max_nodes: usize,
        max_depth: usize,
        max_text_chars: usize,
    ) -> String {
        fn truncate(value: Option<&str>, max_chars: usize) -> Option<String> {
            let value = value?;
            if value.chars().count() <= max_chars {
                return Some(value.to_owned());
            }
            let mut result = value.chars().take(max_chars).collect::<String>();
            result.push('…');
            Some(result)
        }

        // Visit the roots in logical order, then any other parentless node
        // (a hand-built graph may not declare them) in id order. Each node
        // still prints once under its first-visited parent.
        let mut output = String::new();
        let mut parented = std::collections::HashSet::new();
        for (_, node) in self.iter() {
            parented.extend(node.children.iter().copied());
        }
        let mut extra: Vec<SemanticNodeId> = self
            .iter()
            .map(|(id, _)| id)
            .filter(|id| !self.roots.contains(id) && !parented.contains(id))
            .collect();
        extra.sort_by_key(|id| id.0);
        let mut stack = Vec::new();
        for id in extra.into_iter().rev() {
            stack.push((id, 0usize));
        }
        for &root in self.roots.iter().rev() {
            stack.push((root, 0usize));
        }
        let mut visited = 0usize;
        while let Some((id, depth)) = stack.pop() {
            if visited >= max_nodes {
                output.push_str("… semantic dump truncated (node limit)\n");
                break;
            }
            if depth > max_depth {
                output.push_str(&format!(
                    "{}… semantic dump truncated (depth limit)\n",
                    "  ".repeat(max_depth.min(256))
                ));
                continue;
            }
            let Some(node) = self.node(id) else { continue };
            visited = visited.saturating_add(1);
            output.push_str(&format!(
                "{}{:?}#{:?} label={:?} value={:?} bounds=({:.1},{:.1},{:.1},{:.1}) focused={} actions={:?}\n",
                "  ".repeat(depth), node.role, node.id,
                truncate(node.label.as_deref(), max_text_chars),
                truncate(node.value.as_deref(), max_text_chars),
                node.bounds.origin.x, node.bounds.origin.y, node.bounds.size.width,
                node.bounds.size.height, node.state.focused, node.actions,
            ));
            for child in node.children.iter().rev() {
                stack.push((*child, depth.saturating_add(1)));
            }
        }
        output
    }
}
