//! Native accessibility projection for Incular.
//!
//! [`incular_semantics::SemanticsTree`] remains the retained, platform-neutral
//! source of truth. This crate projects it to AccessKit at the desktop-native
//! boundary; it does not introduce a second semantic model. AccessKit node IDs
//! are private to one live native window and never enter widget/facade APIs.

use accesskit::{
    Action, ActionData, ActionRequest, CustomAction as AccessKitCustomAction, Invalid, Live, Node,
    NodeId, Rect as AccessKitRect, Role as AccessKitRole, TextPosition,
    TextSelection as AccessKitSelection, Toggled, Tree, TreeId, TreeUpdate,
};
use incular_core::{Offset, Rect, Size};
use std::{
    borrow::Cow,
    collections::{HashMap, hash_map::Entry},
};

pub use incular_semantics::*;

const HOST_NODE_ID: NodeId = NodeId(0);

/// An owned semantic tree snapshot suitable for a platform bridge.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SemanticsSnapshot {
    /// The designated root, the first of `roots`.
    pub root: Option<SemanticNodeId>,
    /// Every parentless node in logical order.
    pub roots: Vec<SemanticNodeId>,
    pub nodes: Vec<SemanticNode>,
}

impl From<&SemanticsTree> for SemanticsSnapshot {
    fn from(tree: &SemanticsTree) -> Self {
        Self {
            root: tree.root(),
            roots: tree.roots().to_vec(),
            nodes: tree
                .iter()
                .map(|(_, node)| platform_semantic_node(node).into_owned())
                .collect(),
        }
    }
}

/// An action emitted by a native accessibility bridge for a semantic node.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticActionRequest {
    pub node: SemanticNodeId,
    pub action: SemanticAction,
}

/// Small platform-neutral testing seam retained from the previous adapter API.
pub trait SemanticsAdapter {
    fn publish(&mut self, snapshot: SemanticsSnapshot);
    fn poll_action(&mut self) -> Option<SemanticActionRequest>;
}

/// Change categories emitted by the mobile accessibility projection. Native
/// Android and iOS bridges translate these into their platform notifications.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobileAccessibilityEventKind {
    ContentChanged,
    BoundsChanged,
    FocusChanged,
    ValueChanged,
    StateChanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobileAccessibilityEvent {
    pub native_id: u64,
    pub kind: MobileAccessibilityEventKind,
}

/// A node snapshot using stable native-facing IDs while retaining the
/// renderer-independent semantic node for platform conversion.
#[derive(Clone, Debug, PartialEq)]
pub struct MobileAccessibilityNode {
    pub native_id: u64,
    pub semantic: SemanticNode,
    pub children: Vec<u64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MobileAccessibilityUpdate {
    pub revision: u64,
    /// The designated root, the first of `roots`.
    pub root: Option<u64>,
    /// Native IDs of every parentless node in logical order. A bridge must
    /// expose all of them: the graph is a forest under layout-only widgets.
    pub roots: Vec<u64>,
    pub nodes: Vec<MobileAccessibilityNode>,
    pub removed: Vec<u64>,
    pub events: Vec<MobileAccessibilityEvent>,
}

/// Incremental, generation-safe semantic projection for mobile screen-reader
/// bridges. It deliberately has no JNI, Objective-C, or window references;
/// each mobile crate can consume the same update contract on its UI thread.
pub struct MobileAccessibilityProjection {
    semantic_to_native: HashMap<SemanticNodeId, u64>,
    native_to_semantic: HashMap<u64, SemanticNodeId>,
    previous: HashMap<SemanticNodeId, SemanticNode>,
    previous_roots: Vec<SemanticNodeId>,
    synced_revision: Option<u64>,
    /// The tree's removal count at the last sync.
    synced_removals: u64,
    next_native_id: u64,
    active: bool,
    force_full_update: bool,
    /// Nodes whose value an increase or decrease action changed ahead of the
    /// next sync, in action order.
    optimistic_values: Vec<u64>,
    diagnostics: AccessibilityDiagnostics,
}

impl Default for MobileAccessibilityProjection {
    fn default() -> Self {
        Self::new()
    }
}

impl MobileAccessibilityProjection {
    #[must_use]
    pub fn new() -> Self {
        Self {
            semantic_to_native: HashMap::new(),
            native_to_semantic: HashMap::new(),
            previous: HashMap::new(),
            previous_roots: Vec::new(),
            synced_revision: None,
            synced_removals: 0,
            next_native_id: 1,
            active: true,
            force_full_update: true,
            optimistic_values: Vec::new(),
            diagnostics: AccessibilityDiagnostics::default(),
        }
    }

    #[must_use]
    pub fn diagnostics(&self) -> AccessibilityDiagnostics {
        self.diagnostics
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.force_full_update = true;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.force_full_update = true;
    }

    #[must_use]
    pub fn native_node_id(&self, node: SemanticNodeId) -> Option<u64> {
        self.semantic_to_native.get(&node).copied()
    }

    #[must_use]
    pub fn semantic_node_id(&self, native_id: u64) -> Option<SemanticNodeId> {
        self.native_to_semantic.get(&native_id).copied()
    }

    fn native_roots(&self, roots: &[SemanticNodeId]) -> Vec<u64> {
        roots
            .iter()
            .filter_map(|id| self.semantic_to_native.get(id).copied())
            .collect()
    }

    /// Produces a full update on activation and only changed nodes afterwards.
    pub fn sync(&mut self, tree: &SemanticsTree) -> Option<MobileAccessibilityUpdate> {
        if !self.active {
            return None;
        }
        let revision = tree.revision();
        if !self.force_full_update && self.synced_revision == Some(revision) {
            return None;
        }
        let full = self.force_full_update;
        // Only nodes the retained tree inserted or changed since the last
        // sync can differ from what was published, unless the revision went
        // backwards because this is a different tree.
        let regressed = self.synced_revision.is_some_and(|synced| revision < synced);
        let since = if full || regressed {
            0
        } else {
            self.synced_revision.unwrap_or(0)
        };
        let mut changed = Vec::new();
        for (id, node) in tree.changed_since(since) {
            if !self.ensure_native_id(id) {
                self.active = false;
                self.force_full_update = true;
                return None;
            }
            let semantic = platform_semantic_node(node);
            if full || self.previous.get(&id) != Some(semantic.as_ref()) {
                changed.push((id, semantic));
            }
        }
        changed.sort_by_key(|(id, _)| self.semantic_to_native[id]);
        let removed = if !regressed && self.synced_removals == tree.diagnostics().nodes_removed {
            Vec::new()
        } else {
            self.previous
                .keys()
                .filter(|id| tree.node(**id).is_none())
                .copied()
                .collect::<Vec<_>>()
        };
        let roots_changed = self.previous_roots != tree.roots();
        if !full && changed.is_empty() && removed.is_empty() && !roots_changed {
            self.synced_revision = Some(revision);
            self.synced_removals = tree.diagnostics().nodes_removed;
            return None;
        }

        let mut nodes = Vec::with_capacity(changed.len());
        let mut events = Vec::new();
        for (id, semantic) in &changed {
            let native_id = self.semantic_to_native[id];
            let children = semantic
                .children
                .iter()
                .filter_map(|child| self.semantic_to_native.get(child).copied())
                .collect();
            nodes.push(MobileAccessibilityNode {
                native_id,
                semantic: mobile_semantic_node(semantic),
                children,
            });
            if full || !self.previous.contains_key(id) {
                events.push(MobileAccessibilityEvent {
                    native_id,
                    kind: MobileAccessibilityEventKind::ContentChanged,
                });
            } else if let Some(old) = self.previous.get(id) {
                if old.state.focused != semantic.state.focused {
                    events.push(MobileAccessibilityEvent {
                        native_id,
                        kind: MobileAccessibilityEventKind::FocusChanged,
                    });
                }
                if old.value != semantic.value {
                    events.push(MobileAccessibilityEvent {
                        native_id,
                        kind: MobileAccessibilityEventKind::ValueChanged,
                    });
                }
                if old.state != semantic.state {
                    events.push(MobileAccessibilityEvent {
                        native_id,
                        kind: MobileAccessibilityEventKind::StateChanged,
                    });
                }
                if old.bounds != semantic.bounds {
                    events.push(MobileAccessibilityEvent {
                        native_id,
                        kind: MobileAccessibilityEventKind::BoundsChanged,
                    });
                }
                if old.role != semantic.role
                    || old.label != semantic.label
                    || old.description != semantic.description
                    || old.actions != semantic.actions
                    || old.children != semantic.children
                {
                    events.push(MobileAccessibilityEvent {
                        native_id,
                        kind: MobileAccessibilityEventKind::ContentChanged,
                    });
                }
            }
        }
        let mut removed_native = removed
            .iter()
            .filter_map(|id| self.semantic_to_native.get(id).copied())
            .collect::<Vec<_>>();
        removed_native.sort_unstable();
        for id in removed {
            self.previous.remove(&id);
            if let Some(native_id) = self.semantic_to_native.remove(&id) {
                self.native_to_semantic.remove(&native_id);
            }
        }
        if full {
            self.previous.clear();
        }
        for (id, semantic) in changed {
            self.previous.insert(id, semantic.into_owned());
        }
        if roots_changed {
            self.previous_roots.clear();
            self.previous_roots.extend_from_slice(tree.roots());
        }
        self.synced_revision = Some(revision);
        self.synced_removals = tree.diagnostics().nodes_removed;
        self.force_full_update = false;
        let roots = self.native_roots(tree.roots());
        Some(MobileAccessibilityUpdate {
            revision,
            root: roots.first().copied(),
            roots,
            nodes,
            removed: removed_native,
            events,
        })
    }

    /// Validates that a native action is still supported by the live semantic
    /// node before handing it to the runtime.
    pub fn translate_action(
        &mut self,
        native_id: u64,
        action: SemanticAction,
    ) -> Option<SemanticActionRequest> {
        self.diagnostics.actions_received = self.diagnostics.actions_received.wrapping_add(1);
        if !self.active {
            self.diagnostics.unsupported_actions =
                self.diagnostics.unsupported_actions.wrapping_add(1);
            return None;
        }
        let Some(node) = self.native_to_semantic.get(&native_id).copied() else {
            self.diagnostics.stale_actions_rejected =
                self.diagnostics.stale_actions_rejected.wrapping_add(1);
            return None;
        };
        let Some(semantic) = self.previous.get(&node) else {
            self.diagnostics.stale_actions_rejected =
                self.diagnostics.stale_actions_rejected.wrapping_add(1);
            return None;
        };
        if !supports(semantic, action.kind())
            || matches!(
                &action,
                SemanticAction::Custom(id)
                    if !semantic.state.custom_actions.contains(id)
                        || !CustomSemanticsAction::get_action(*id)
                            .is_some_and(|action| action.label().is_some())
            )
        {
            self.diagnostics.unsupported_actions =
                self.diagnostics.unsupported_actions.wrapping_add(1);
            return None;
        }
        // Flutter's Android and iOS bridges show the adjusted value at once
        // (`semanticsNode.value = semanticsNode.increasedValue`), before the
        // framework's next update confirms or replaces it.
        let adjusted = match action {
            SemanticAction::Increment => semantic.state.increased_value.clone(),
            SemanticAction::Decrement => semantic.state.decreased_value.clone(),
            _ => None,
        };
        if let Some(value) = adjusted
            && let Some(semantic) = self.previous.get_mut(&node)
        {
            semantic.value = Some(value);
            if !self.optimistic_values.contains(&native_id) {
                self.optimistic_values.push(native_id);
            }
        }
        self.diagnostics.actions_dispatched = self.diagnostics.actions_dispatched.wrapping_add(1);
        Some(SemanticActionRequest { node, action })
    }

    /// The nodes whose value an increase or decrease action changed since
    /// the last call, each with a [`MobileAccessibilityEventKind::ValueChanged`]
    /// event, or `None` when there are none. Flutter's mobile bridges set a
    /// node's value to its `increasedValue` or `decreasedValue` as soon as
    /// the screen reader adjusts it (Android also announces the node as
    /// selected), so the reader speaks the new value without waiting for a
    /// frame. A native bridge applies this update right after
    /// [`Self::translate_action`]; the next [`Self::sync`] confirms the value
    /// or replaces it with a further `ValueChanged`. The update keeps the
    /// revision of the last sync.
    pub fn take_optimistic_update(&mut self) -> Option<MobileAccessibilityUpdate> {
        if self.optimistic_values.is_empty() || !self.active {
            self.optimistic_values.clear();
            return None;
        }
        let mut nodes = Vec::new();
        let mut events = Vec::new();
        for native_id in std::mem::take(&mut self.optimistic_values) {
            let Some(semantic) = self
                .native_to_semantic
                .get(&native_id)
                .and_then(|id| self.previous.get(id))
            else {
                continue;
            };
            nodes.push(MobileAccessibilityNode {
                native_id,
                semantic: mobile_semantic_node(semantic),
                children: semantic
                    .children
                    .iter()
                    .filter_map(|child| self.semantic_to_native.get(child).copied())
                    .collect(),
            });
            events.push(MobileAccessibilityEvent {
                native_id,
                kind: MobileAccessibilityEventKind::ValueChanged,
            });
        }
        if nodes.is_empty() {
            return None;
        }
        let roots = self.native_roots(&self.previous_roots);
        Some(MobileAccessibilityUpdate {
            revision: self.synced_revision.unwrap_or(0),
            root: roots.first().copied(),
            roots,
            nodes,
            removed: Vec::new(),
            events,
        })
    }

    fn ensure_native_id(&mut self, node: SemanticNodeId) -> bool {
        if self.semantic_to_native.contains_key(&node) {
            return true;
        }
        let native_id = self.next_native_id;
        let Some(next_native_id) = self.next_native_id.checked_add(1) else {
            return false;
        };
        self.next_native_id = next_native_id;
        self.semantic_to_native.insert(node, native_id);
        self.native_to_semantic.insert(native_id, node);
        true
    }
}

/// Flutter tags inform framework tree assembly but are not transmitted to
/// accessibility adapters. Compare the projected form as well as exporting
/// it, so a tag-only tree revision produces no native update.
fn platform_semantic_node(node: &SemanticNode) -> Cow<'_, SemanticNode> {
    if node.tags().is_empty() {
        return Cow::Borrowed(node);
    }
    let mut node = node.clone();
    node.state.tags = Box::default();
    Cow::Owned(node)
}

/// Keeps clear-text password values inside the retained runtime while making
/// the mobile projection safe to hand to a native accessibility tree.
fn mobile_semantic_node(node: &SemanticNode) -> SemanticNode {
    let mut node = node.clone();
    if node.state.obscured {
        node.value = None;
    }
    node
}

/// Per-native-window health counters. They intentionally contain no private
/// labels, descriptions, or text-field values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AccessibilityDiagnostics {
    pub adapters_created: u64,
    pub adapters_destroyed: u64,
    pub adapter_activations: u64,
    pub adapter_deactivations: u64,
    pub full_tree_updates: u64,
    pub incremental_tree_updates: u64,
    pub nodes_published: u64,
    pub nodes_updated: u64,
    pub nodes_removed: u64,
    pub actions_received: u64,
    pub actions_dispatched: u64,
    pub stale_actions_rejected: u64,
    pub unsupported_actions: u64,
    pub focus_updates: u64,
    pub bounds_updates: u64,
    pub semantic_updates_skipped_unchanged: u64,
    pub accesskit_conversion_failures: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessKitUpdateKind {
    Full,
    Incremental,
}

/// Generated native update consumed immediately by a Winit adapter.
#[derive(Debug)]
pub struct NativeAccessibilityUpdate {
    kind: AccessKitUpdateKind,
    update: TreeUpdate,
}

impl NativeAccessibilityUpdate {
    #[must_use]
    pub const fn kind(&self) -> AccessKitUpdateKind {
        self.kind
    }

    /// Native-runner boundary only; this is deliberately not re-exported by
    /// Incular's facade or prelude.
    #[doc(hidden)]
    #[must_use]
    pub fn into_accesskit(self) -> TreeUpdate {
        self.update
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ProjectedNode {
    semantic: SemanticNode,
    scale_factor: f64,
}

/// The native window node: it parents every semantic root.
#[derive(Clone, Debug, PartialEq)]
struct HostProjection {
    roots: Vec<SemanticNodeId>,
    bounds: Option<Rect>,
    scale_factor: f64,
}

/// Stateful, generation-safe projection from one Incular tree to one native
/// AccessKit tree. One instance belongs to one native window.
pub struct AccessKitProjection {
    semantic_to_native: HashMap<SemanticNodeId, NodeId>,
    native_to_semantic: HashMap<NodeId, SemanticNodeId>,
    previous: HashMap<SemanticNodeId, ProjectedNode>,
    host: Option<HostProjection>,
    /// Native focus published by the last update.
    focus: NodeId,
    synced_revision: Option<u64>,
    /// The tree's removal count at the last sync; unchanged means no
    /// published node can have disappeared.
    synced_removals: u64,
    next_native_id: u64,
    force_full_update: bool,
    diagnostics: AccessibilityDiagnostics,
}

impl Default for AccessKitProjection {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessKitProjection {
    #[must_use]
    pub fn new() -> Self {
        Self {
            semantic_to_native: HashMap::new(),
            native_to_semantic: HashMap::new(),
            previous: HashMap::new(),
            host: None,
            focus: HOST_NODE_ID,
            synced_revision: None,
            synced_removals: 0,
            next_native_id: 1,
            force_full_update: true,
            diagnostics: AccessibilityDiagnostics::default(),
        }
    }

    #[must_use]
    pub const fn diagnostics(&self) -> AccessibilityDiagnostics {
        self.diagnostics
    }

    /// Native bridge/test plumbing only, not an application identity.
    #[doc(hidden)]
    #[must_use]
    pub fn native_node_id(&self, node: SemanticNodeId) -> Option<u64> {
        self.semantic_to_native.get(&node).map(|id| id.0)
    }

    pub fn note_adapter_created(&mut self) {
        self.diagnostics.adapters_created = self.diagnostics.adapters_created.wrapping_add(1);
    }

    pub fn note_adapter_destroyed(&mut self) {
        self.diagnostics.adapters_destroyed = self.diagnostics.adapters_destroyed.wrapping_add(1);
    }

    /// Request a full retained-tree snapshot at native activation.
    pub fn activate(&mut self) {
        self.diagnostics.adapter_activations = self.diagnostics.adapter_activations.wrapping_add(1);
        self.force_full_update = true;
    }

    /// Native deactivation doesn't discard Incular semantics. The next native
    /// activation publishes a complete current tree using the same live-window
    /// node map when possible.
    pub fn deactivate(&mut self) {
        self.diagnostics.adapter_deactivations =
            self.diagnostics.adapter_deactivations.wrapping_add(1);
        self.force_full_update = true;
    }

    /// Records a request that was valid when projected but was rejected by the
    /// retained runtime before UI dispatch (for example, a node removed in the
    /// small interval before the Winit user event was handled).
    pub fn note_runtime_stale_action(&mut self) {
        self.diagnostics.stale_actions_rejected =
            self.diagnostics.stale_actions_rejected.wrapping_add(1);
    }

    /// Projects retained semantic changes. Incular bounds are logical; the
    /// supplied native window scale is applied exactly once for AccessKit.
    pub fn sync(
        &mut self,
        tree: &SemanticsTree,
        scale_factor: f64,
    ) -> Option<NativeAccessibilityUpdate> {
        let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        let revision = tree.revision();
        if !self.force_full_update && self.synced_revision == Some(revision) {
            self.diagnostics.semantic_updates_skipped_unchanged = self
                .diagnostics
                .semantic_updates_skipped_unchanged
                .wrapping_add(1);
            return None;
        }

        let full = self.force_full_update || self.synced_revision.is_none();
        // A scale change re-projects every node's bounds, and a revision
        // that went backwards belongs to a different tree; otherwise only
        // nodes the retained tree inserted or changed since the last sync
        // can differ from what was published.
        let rescaled = self
            .host
            .as_ref()
            .is_some_and(|host| host.scale_factor != scale_factor);
        let regressed = self.synced_revision.is_some_and(|synced| revision < synced);
        let since = if full || rescaled || regressed {
            0
        } else {
            self.synced_revision.unwrap_or(0)
        };
        let mut changed = Vec::new();
        for (id, semantic) in tree.changed_since(since) {
            if !self.ensure_native_id(id) {
                self.force_full_update = true;
                return None;
            }
            if full
                || self
                    .previous
                    .get(&id)
                    .is_none_or(|old| {
                        old.scale_factor != scale_factor
                            || &old.semantic != platform_semantic_node(semantic).as_ref()
                    })
            {
                changed.push(id);
            }
        }
        let host_bounds = roots_bounds(tree);
        let removed: Vec<_> =
            if !regressed && self.synced_removals == tree.diagnostics().nodes_removed {
                Vec::new()
            } else {
                self.previous
                    .keys()
                    .copied()
                    .filter(|id| tree.node(*id).is_none())
                    .collect()
            };
        // The semantic graph is a forest under layout-only widgets: every
        // root must hang off the window node, or AccessKit rejects the
        // update for publishing nodes unreachable from the tree root.
        let host_changed = full
            || self.host.as_ref().is_none_or(|host| {
                host.roots != tree.roots()
                    || host.bounds != host_bounds
                    || host.scale_factor != scale_factor
            });
        if !full && changed.is_empty() && removed.is_empty() && !host_changed {
            self.synced_revision = Some(revision);
            self.synced_removals = tree.diagnostics().nodes_removed;
            self.diagnostics.semantic_updates_skipped_unchanged = self
                .diagnostics
                .semantic_updates_skipped_unchanged
                .wrapping_add(1);
            return None;
        }

        let old_focus = self.focus;
        let mut nodes = Vec::with_capacity(changed.len() + usize::from(host_changed));
        if host_changed {
            let host = HostProjection {
                roots: tree.roots().to_vec(),
                bounds: host_bounds,
                scale_factor,
            };
            nodes.push((HOST_NODE_ID, self.host_node(&host)));
            self.host = Some(host);
        }
        // Native ids are unique per node, so an unstable sort is deterministic here.
        changed.sort_unstable_by_key(|id| self.semantic_to_native[id].0);
        for &id in &changed {
            let semantic = tree.node(id).expect("changed node is current");
            nodes.push((
                self.semantic_to_native[&id],
                self.accesskit_node(semantic, scale_factor),
            ));
        }
        let focus = tree
            .focused_node()
            .and_then(|id| self.semantic_to_native.get(&id).copied())
            .unwrap_or(HOST_NODE_ID);

        self.diagnostics.focus_updates = self
            .diagnostics
            .focus_updates
            .wrapping_add(u64::from(focus != old_focus));
        self.diagnostics.bounds_updates = self.diagnostics.bounds_updates.wrapping_add(
            changed
                .iter()
                .filter(|id| {
                    self.previous.get(id).is_some_and(|old| {
                        tree.node(**id).is_some_and(|semantic| {
                            old.semantic.bounds != semantic.bounds
                                || old.scale_factor != scale_factor
                        })
                    })
                })
                .count() as u64,
        );
        if full {
            self.diagnostics.full_tree_updates = self.diagnostics.full_tree_updates.wrapping_add(1);
            self.diagnostics.nodes_published = self
                .diagnostics
                .nodes_published
                .wrapping_add(tree.len() as u64 + 1);
        } else {
            self.diagnostics.incremental_tree_updates =
                self.diagnostics.incremental_tree_updates.wrapping_add(1);
            self.diagnostics.nodes_updated = self
                .diagnostics
                .nodes_updated
                .wrapping_add(nodes.len().saturating_sub(usize::from(host_changed)) as u64);
            self.diagnostics.nodes_removed = self
                .diagnostics
                .nodes_removed
                .wrapping_add(removed.len() as u64);
        }
        for id in removed {
            self.previous.remove(&id);
            if let Some(native) = self.semantic_to_native.remove(&id) {
                self.native_to_semantic.remove(&native);
            }
        }
        if full {
            self.previous.clear();
        }
        for id in changed {
            let semantic =
                platform_semantic_node(tree.node(id).expect("changed node is current")).into_owned();
            self.previous.insert(
                id,
                ProjectedNode {
                    semantic,
                    scale_factor,
                },
            );
        }
        self.focus = focus;
        self.synced_revision = Some(revision);
        self.synced_removals = tree.diagnostics().nodes_removed;
        self.force_full_update = false;

        Some(NativeAccessibilityUpdate {
            kind: if full {
                AccessKitUpdateKind::Full
            } else {
                AccessKitUpdateKind::Incremental
            },
            update: TreeUpdate {
                nodes,
                tree: full.then(|| {
                    let mut tree = Tree::new(HOST_NODE_ID);
                    tree.toolkit_name = Some("Incular".into());
                    tree.toolkit_version = Some(env!("CARGO_PKG_VERSION").into());
                    tree
                }),
                tree_id: TreeId::ROOT,
                focus,
            },
        })
    }

    /// Converts a native request into an owned, validated Incular request. The
    /// caller must dispatch it through the UI-thread runtime.
    pub fn translate_action(&mut self, request: &ActionRequest) -> Option<SemanticActionRequest> {
        self.diagnostics.actions_received = self.diagnostics.actions_received.wrapping_add(1);
        if request.target_tree != TreeId::ROOT {
            self.diagnostics.unsupported_actions =
                self.diagnostics.unsupported_actions.wrapping_add(1);
            return None;
        }
        let Some(node) = self.native_to_semantic.get(&request.target_node).copied() else {
            self.diagnostics.stale_actions_rejected =
                self.diagnostics.stale_actions_rejected.wrapping_add(1);
            return None;
        };
        let Some(projected) = self.previous.get(&node) else {
            self.diagnostics.stale_actions_rejected =
                self.diagnostics.stale_actions_rejected.wrapping_add(1);
            return None;
        };
        let action = match request.action {
            Action::Click if supports(&projected.semantic, SemanticActionKind::Activate) => {
                Some(SemanticAction::Activate)
            }
            Action::CustomAction if supports(&projected.semantic, SemanticActionKind::Custom) => {
                match request.data.as_ref() {
                    Some(ActionData::CustomAction(raw)) => u32::try_from(*raw)
                        .ok()
                        .map(CustomSemanticsActionId)
                        .filter(|id| {
                            projected.semantic.state.custom_actions.contains(id)
                                && CustomSemanticsAction::get_action(*id)
                                    .is_some_and(|action| action.label().is_some())
                        })
                        .map(SemanticAction::Custom),
                    _ => None,
                }
            }
            Action::Focus if supports(&projected.semantic, SemanticActionKind::Focus) => {
                Some(SemanticAction::Focus)
            }
            Action::ScrollLeft | Action::ScrollRight | Action::ScrollUp | Action::ScrollDown
                if projected.semantic.state.scroll_metrics.is_some() =>
            {
                let action = match request.action {
                    Action::ScrollUp => SemanticAction::ScrollDown,
                    Action::ScrollDown => SemanticAction::ScrollUp,
                    Action::ScrollLeft => SemanticAction::ScrollRight,
                    Action::ScrollRight => SemanticAction::ScrollLeft,
                    _ => unreachable!("matched native scroll directions"),
                };
                supports(&projected.semantic, action.kind()).then_some(action)
            }
            Action::ScrollLeft if supports(&projected.semantic, SemanticActionKind::ScrollLeft) => {
                Some(SemanticAction::ScrollLeft)
            }
            Action::ScrollRight
                if supports(&projected.semantic, SemanticActionKind::ScrollRight) =>
            {
                Some(SemanticAction::ScrollRight)
            }
            Action::ScrollUp if supports(&projected.semantic, SemanticActionKind::ScrollUp) => {
                Some(SemanticAction::ScrollUp)
            }
            Action::SetScrollOffset
                if supports(&projected.semantic, SemanticActionKind::ScrollToOffset) =>
            {
                match request.data.as_ref() {
                    Some(ActionData::SetScrollOffset(point))
                        if point.x.is_finite() && point.y.is_finite() =>
                    {
                        Some(SemanticAction::ScrollToOffset {
                            x: point.x,
                            y: point.y,
                        })
                    }
                    _ => None,
                }
            }
            Action::ScrollDown if supports(&projected.semantic, SemanticActionKind::ScrollDown) => {
                Some(SemanticAction::ScrollDown)
            }
            Action::ScrollDown
                if supports(&projected.semantic, SemanticActionKind::ScrollForward) =>
            {
                Some(SemanticAction::ScrollForward)
            }
            Action::ScrollUp
                if supports(&projected.semantic, SemanticActionKind::ScrollBackward) =>
            {
                Some(SemanticAction::ScrollBackward)
            }
            // Retain the logical fallback for real scroll containers, but
            // never turn a withdrawn custom arrow into a different callback.
            Action::ScrollRight
                if matches!(projected.semantic.role, Role::ScrollView | Role::List)
                    && supports(&projected.semantic, SemanticActionKind::ScrollForward) =>
            {
                Some(SemanticAction::ScrollForward)
            }
            Action::ScrollLeft
                if matches!(projected.semantic.role, Role::ScrollView | Role::List)
                    && supports(&projected.semantic, SemanticActionKind::ScrollBackward) =>
            {
                Some(SemanticAction::ScrollBackward)
            }
            Action::SetValue | Action::ReplaceSelectedText
                if supports(&projected.semantic, SemanticActionKind::SetText) =>
            {
                match request.data.as_ref() {
                    Some(ActionData::Value(value)) => {
                        Some(SemanticAction::SetText(value.to_string()))
                    }
                    _ => None,
                }
            }
            Action::SetTextSelection
                if supports(&projected.semantic, SemanticActionKind::SetSelection) =>
            {
                match request.data.as_ref() {
                    Some(ActionData::SetTextSelection(selection))
                        if selection.anchor.node == request.target_node
                            && selection.focus.node == request.target_node =>
                    {
                        let text = projected.semantic.value.as_deref().unwrap_or_default();
                        Some(SemanticAction::SetSelection {
                            base: utf8_offset(text, selection.anchor.character_index),
                            extent: utf8_offset(text, selection.focus.character_index),
                        })
                    }
                    _ => None,
                }
            }
            Action::Increment if supports(&projected.semantic, SemanticActionKind::Increment) => {
                Some(SemanticAction::Increment)
            }
            Action::Decrement if supports(&projected.semantic, SemanticActionKind::Decrement) => {
                Some(SemanticAction::Decrement)
            }
            Action::Expand if supports(&projected.semantic, SemanticActionKind::Expand) => {
                Some(SemanticAction::Expand)
            }
            Action::Collapse if supports(&projected.semantic, SemanticActionKind::Collapse) => {
                Some(SemanticAction::Collapse)
            }
            Action::ScrollIntoView
                if supports(&projected.semantic, SemanticActionKind::ShowOnScreen) =>
            {
                Some(SemanticAction::ShowOnScreen)
            }
            _ => None,
        };
        let Some(action) = action else {
            self.diagnostics.unsupported_actions =
                self.diagnostics.unsupported_actions.wrapping_add(1);
            return None;
        };
        self.diagnostics.actions_dispatched = self.diagnostics.actions_dispatched.wrapping_add(1);
        Some(SemanticActionRequest { node, action })
    }

    fn ensure_native_id(&mut self, node: SemanticNodeId) -> bool {
        let Entry::Vacant(entry) = self.semantic_to_native.entry(node) else {
            return true;
        };
        let native = NodeId(self.next_native_id);
        let Some(next_native_id) = self.next_native_id.checked_add(1) else {
            return false;
        };
        self.next_native_id = next_native_id;
        entry.insert(native);
        self.native_to_semantic.insert(native, node);
        true
    }

    fn host_node(&self, host: &HostProjection) -> Node {
        let mut node = Node::new(AccessKitRole::Window);
        node.set_label("Incular");
        node.set_children(
            host.roots
                .iter()
                .filter_map(|id| self.semantic_to_native.get(id).copied())
                .collect::<Vec<_>>(),
        );
        if let Some(bounds) = host.bounds {
            node.set_bounds(to_accesskit_rect(bounds, host.scale_factor));
        }
        node
    }

    fn accesskit_node(&self, semantic: &SemanticNode, scale_factor: f64) -> Node {
        let native_id = self.semantic_to_native[&semantic.id];
        let mut node = Node::new(map_role(semantic.role, semantic.state.obscured));
        node.set_bounds(to_accesskit_rect(semantic.bounds, scale_factor));
        if semantic.role == Role::Text && !semantic.state.obscured {
            if let Some(text) = semantic.value.as_ref().or(semantic.label.as_ref()) {
                node.set_value(text.as_str());
            }
        } else if !semantic.state.obscured {
            if let Some(label) = &semantic.label {
                node.set_label(label.as_str());
            }
            if let Some(value) = &semantic.value {
                node.set_value(value.as_str());
            }
        } else if let Some(label) = &semantic.label {
            // Password values never cross the native accessibility boundary,
            // but their accessible label still does.
            node.set_label(label.as_str());
        }
        if let Some(description) = &semantic.description {
            node.set_description(description.as_str());
        }
        if is_interactive_role(semantic.role) && !semantic.state.enabled {
            node.set_disabled();
        }
        if semantic.state.selected {
            node.set_selected(true);
        }
        if let Some(checked) = semantic.state.checked {
            node.set_toggled(match checked {
                incular_semantics::CheckedState::Checked => Toggled::True,
                incular_semantics::CheckedState::Unchecked => Toggled::False,
                incular_semantics::CheckedState::Indeterminate => Toggled::Mixed,
            });
        }
        if semantic.state.checked.is_none()
            && let Some(toggled) = semantic.state.toggled
        {
            node.set_toggled(if toggled {
                Toggled::True
            } else {
                Toggled::False
            });
        }
        if let Some(expanded) = semantic.state.expanded {
            node.set_expanded(expanded);
        }
        if semantic.state.hidden {
            node.set_hidden();
        }
        if semantic.state.live_region {
            node.set_live(Live::Polite);
        }
        if let Some(locale) = &semantic.state.locale {
            node.set_language(locale.as_ref());
        }
        if let Some(direction) = semantic.state.text_direction {
            node.set_text_direction(match direction {
                TextDirection::Ltr => accesskit::TextDirection::LeftToRight,
                TextDirection::Rtl => accesskit::TextDirection::RightToLeft,
            });
        }
        if let Some(tooltip) = &semantic.state.tooltip {
            node.set_tooltip(tooltip.as_str());
        }
        if let Some(url) = &semantic.state.link_url {
            node.set_url(url.as_str());
        }
        if let Some(identifier) = &semantic.state.identifier {
            node.set_author_id(identifier.as_str());
        }
        if semantic.state.busy {
            node.set_busy();
        }
        if semantic.state.is_invalid() {
            node.set_invalid(Invalid::True);
        }
        if semantic.state.required {
            node.set_required();
        }
        if let Some(level) = semantic.state.heading_level {
            node.set_level(usize::from(level));
        }
        if let Some(value) = semantic
            .state
            .numeric_value
            .filter(|value| value.is_finite())
        {
            node.set_numeric_value(value);
        }
        if let Some(value) = semantic.state.numeric_min.filter(|value| value.is_finite()) {
            node.set_min_numeric_value(value);
        }
        if let Some(value) = semantic.state.numeric_max.filter(|value| value.is_finite()) {
            node.set_max_numeric_value(value);
        }
        if let Some(value) = semantic
            .state
            .numeric_step
            .filter(|value| value.is_finite())
        {
            node.set_numeric_value_step(value);
        }
        if semantic.state.read_only {
            node.set_read_only();
        }
        if let Some(size) = semantic.state.set_size {
            node.set_size_of_set(size);
        }
        if let Some(index) = semantic.state.item_index {
            node.set_position_in_set(index.saturating_add(1));
        }
        if matches!(semantic.role, Role::ScrollView | Role::List) {
            use incular_semantics::SemanticScrollAxis;
            match semantic.state.scroll_metrics {
                Some(metrics) => match metrics.axis {
                    SemanticScrollAxis::Horizontal => {
                        node.set_scroll_x(metrics.offset);
                        node.set_scroll_x_min(metrics.minimum);
                        node.set_scroll_x_max(metrics.maximum);
                    }
                    SemanticScrollAxis::Vertical => {
                        node.set_scroll_y(metrics.offset);
                        node.set_scroll_y_min(metrics.minimum);
                        node.set_scroll_y_max(metrics.maximum);
                    }
                },
                None => {
                    let (offset, maximum) = scroll_value(semantic.value.as_deref());
                    node.set_scroll_y(offset);
                    node.set_scroll_y_min(0.0);
                    node.set_scroll_y_max(maximum);
                }
            }
        }
        if semantic.state.editable
            && !semantic.state.obscured
            && let Some(value) = semantic.value.as_deref()
        {
            node.set_character_lengths(
                value
                    .chars()
                    .map(|character| character.len_utf8() as u8)
                    .collect::<Vec<_>>(),
            );
            if let Some(selection) = semantic.state.selection {
                node.set_text_selection(AccessKitSelection {
                    anchor: TextPosition {
                        node: native_id,
                        character_index: character_offset(value, selection.base),
                    },
                    focus: TextPosition {
                        node: native_id,
                        character_index: character_offset(value, selection.extent),
                    },
                });
            }
        }
        for action in &semantic.actions {
            let native = if semantic.state.scroll_metrics.is_some() {
                // AccessKit names the viewport's motion, while Flutter
                // names the contents' swipe. Do not also expose axis-agnostic
                // aliases, which would incorrectly add vertical arrows.
                match action {
                    SemanticActionKind::ScrollForward | SemanticActionKind::ScrollBackward => None,
                    SemanticActionKind::ScrollUp => Some(Action::ScrollDown),
                    SemanticActionKind::ScrollDown => Some(Action::ScrollUp),
                    SemanticActionKind::ScrollLeft => Some(Action::ScrollRight),
                    SemanticActionKind::ScrollRight => Some(Action::ScrollLeft),
                    _ => map_action(*action),
                }
            } else {
                map_action(*action)
            };
            if let Some(action) = native {
                node.add_action(action);
            }
        }
        if supports(semantic, SemanticActionKind::Custom) {
            for id in &semantic.state.custom_actions {
                let Some(action) = CustomSemanticsAction::get_action(*id) else {
                    continue;
                };
                // AccessKit cannot relabel a standard action. Only genuine
                // named actions are advertised as additional native choices.
                if let (Ok(id), Some(label)) = (i32::try_from(id.0), action.label()) {
                    node.push_custom_action(AccessKitCustomAction {
                        id,
                        description: label.into(),
                    });
                }
            }
            if !node.custom_actions().is_empty() {
                node.add_action(Action::CustomAction);
            }
        }
        node.set_children(
            semantic
                .children
                .iter()
                .filter_map(|child| self.semantic_to_native.get(child).copied())
                .collect::<Vec<_>>(),
        );
        node
    }
}

fn supports(node: &SemanticNode, action: SemanticActionKind) -> bool {
    node.actions.contains(&action)
}

fn map_role(role: Role, obscured: bool) -> AccessKitRole {
    match role {
        Role::Button => AccessKitRole::Button,
        Role::Text => AccessKitRole::Label,
        Role::TextField => {
            if obscured {
                AccessKitRole::PasswordInput
            } else {
                AccessKitRole::TextInput
            }
        }
        Role::TextArea => AccessKitRole::MultilineTextInput,
        Role::List => AccessKitRole::List,
        Role::ListItem => AccessKitRole::ListItem,
        Role::Table => AccessKitRole::Table,
        Role::Row => AccessKitRole::Row,
        Role::Cell => AccessKitRole::Cell,
        Role::ColumnHeader => AccessKitRole::ColumnHeader,
        Role::ScrollView => AccessKitRole::ScrollView,
        Role::GenericContainer => AccessKitRole::GenericContainer,
        Role::Checkbox => AccessKitRole::CheckBox,
        Role::Radio => AccessKitRole::RadioButton,
        Role::Switch => AccessKitRole::Switch,
        Role::Slider => AccessKitRole::Slider,
        Role::Menu => AccessKitRole::Menu,
        Role::Dialog => AccessKitRole::Dialog,
        Role::Image => AccessKitRole::Image,
        Role::Heading => AccessKitRole::Heading,
        Role::Link => AccessKitRole::Link,
        Role::Group => AccessKitRole::Group,
        Role::Tab => AccessKitRole::Tab,
        Role::TabList => AccessKitRole::TabList,
        Role::TabPanel => AccessKitRole::TabPanel,
        Role::MenuItem => AccessKitRole::MenuItem,
        Role::ProgressBar => AccessKitRole::ProgressIndicator,
        Role::Meter => AccessKitRole::Meter,
        Role::SearchField => AccessKitRole::SearchInput,
    }
}

fn is_interactive_role(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::TextField
            | Role::TextArea
            | Role::Checkbox
            | Role::Radio
            | Role::Switch
            | Role::Slider
            | Role::Link
            | Role::Tab
            | Role::MenuItem
            | Role::SearchField
    )
}

fn map_action(action: SemanticActionKind) -> Option<Action> {
    match action {
        SemanticActionKind::Focus => Some(Action::Focus),
        SemanticActionKind::Activate => Some(Action::Click),
        SemanticActionKind::SetText => Some(Action::SetValue),
        SemanticActionKind::SetSelection => Some(Action::SetTextSelection),
        SemanticActionKind::Increment => Some(Action::Increment),
        SemanticActionKind::Decrement => Some(Action::Decrement),
        SemanticActionKind::ScrollForward => Some(Action::ScrollDown),
        SemanticActionKind::ScrollBackward => Some(Action::ScrollUp),
        SemanticActionKind::ScrollLeft => Some(Action::ScrollLeft),
        SemanticActionKind::ScrollRight => Some(Action::ScrollRight),
        SemanticActionKind::ScrollUp => Some(Action::ScrollUp),
        SemanticActionKind::ScrollDown => Some(Action::ScrollDown),
        SemanticActionKind::ScrollToOffset => Some(Action::SetScrollOffset),
        SemanticActionKind::ShowOnScreen => Some(Action::ScrollIntoView),
        SemanticActionKind::Expand => Some(Action::Expand),
        SemanticActionKind::Collapse => Some(Action::Collapse),
        // AccessKit has no clipboard, long-press, or distinct dismiss action.
        // A modal's separate Activate still projects to Click. Parameterized
        // custom actions are advertised only with native descriptions.
        SemanticActionKind::LongPress
        | SemanticActionKind::Dismiss
        | SemanticActionKind::Copy
        | SemanticActionKind::Cut
        | SemanticActionKind::Paste
        | SemanticActionKind::MoveCursorForwardByCharacter
        | SemanticActionKind::MoveCursorBackwardByCharacter
        | SemanticActionKind::DidGainAccessibilityFocus
        | SemanticActionKind::DidLoseAccessibilityFocus
        | SemanticActionKind::Custom => None,
    }
}

/// Logical bounds enclosing every semantic root, for the window node.
fn roots_bounds(tree: &SemanticsTree) -> Option<Rect> {
    tree.roots()
        .iter()
        .filter_map(|id| tree.node(*id))
        .map(|node| node.bounds)
        .reduce(|a, b| {
            let left = a.origin.x.min(b.origin.x);
            let top = a.origin.y.min(b.origin.y);
            let right = (a.origin.x + a.size.width).max(b.origin.x + b.size.width);
            let bottom = (a.origin.y + a.size.height).max(b.origin.y + b.size.height);
            Rect::from_origin_size(
                Offset::new(left, top),
                Size::new(right - left, bottom - top),
            )
        })
}

fn to_accesskit_rect(rect: Rect, scale_factor: f64) -> AccessKitRect {
    AccessKitRect {
        x0: f64::from(rect.origin.x) * scale_factor,
        y0: f64::from(rect.origin.y) * scale_factor,
        x1: f64::from(rect.origin.x + rect.size.width) * scale_factor,
        y1: f64::from(rect.origin.y + rect.size.height) * scale_factor,
    }
}

fn scroll_value(value: Option<&str>) -> (f64, f64) {
    let Some((offset, maximum)) = value.and_then(|value| value.split_once('/')) else {
        return (0.0, 0.0);
    };
    let parse = |value: &str| {
        value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .max(0.0)
    };
    (parse(offset), parse(maximum))
}

fn character_offset(text: &str, byte_offset: usize) -> usize {
    let mut boundary = byte_offset.min(text.len());
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    text[..boundary].chars().count()
}

fn utf8_offset(text: &str, character_offset: usize) -> usize {
    text.char_indices()
        .nth(character_offset)
        .map_or(text.len(), |(offset, _)| offset)
}
