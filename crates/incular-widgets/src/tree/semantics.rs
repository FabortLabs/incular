//! Semantic-tree construction and semantic action classification.
//!
//! The pass is incremental. Invalidation marks render objects with
//! [`SemanticsDirty`] and marks the path to them, and the pass enters only
//! marked subtrees: a clean subtree keeps its retained semantic nodes and
//! only lends its top-level nodes to the parent's child list. A frame with
//! nothing marked does no semantic work at all.

use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SemanticCollectionContext {
    item_index: Option<usize>,
    set_size: Option<usize>,
}

/// One element waiting to be visited by the semantics pass.
#[derive(Clone, Copy)]
pub(super) struct SemanticVisit {
    element: ElementId,
    /// Index in `builds` of the nearest collected semantic ancestor, or
    /// `TOP_LEVEL`.
    parent: u32,
    collection: Option<SemanticCollectionContext>,
    /// Collect the full content of every element below, whatever its marks
    /// say.
    force_content: bool,
    /// Every bound below may have moved, although content is unchanged.
    force_bounds: bool,
    /// The parent's world transform followed by its child content
    /// transform; this element's own offset is not applied yet.
    parent_world: CoreTransform,
}

/// What the pass collected for one element.
// Boxing the full build would add an allocation per rebuilt node.
#[allow(clippy::large_enum_variant)]
pub(super) enum Collected {
    /// A complete node: every property was rebuilt.
    Full(SemanticBuild),
    /// A retained node whose properties are unchanged; only its bounds and
    /// child list are rewritten.
    Bounds { element: ElementId, bounds: Rect },
}

impl Collected {
    fn element(&self) -> ElementId {
        match self {
            Self::Full(build) => build.element,
            Self::Bounds { element, .. } => *element,
        }
    }
}

/// Parent index of a top-level semantic node.
const TOP_LEVEL: u32 = u32::MAX;

/// Buffers the semantics pass reuses, so a pass that collects a few nodes
/// allocates only for those nodes.
#[derive(Default)]
pub(super) struct SemanticsScratch {
    visits: Vec<SemanticVisit>,
    builds: Vec<Collected>,
    /// Child ids of the node being written.
    child_ids: Vec<SemanticNodeId>,
    /// Child edges in visit order: (parent build index, sequence, child).
    edges: Vec<(u32, u32, ElementId)>,
    /// Elements still to search inside a clean subtree.
    retained: Vec<ElementId>,
    /// Every semantic node with a parent edge (or top-level slot) this pass,
    /// sorted for lookup.
    attached: Vec<SemanticNodeId>,
    /// Nodes dropped from a re-collected child list.
    detached: Vec<SemanticNodeId>,
}

impl WidgetTree {
    /// Enables or disables the semantics pass. Trees start enabled; a native
    /// host disables it while no assistive technology is listening, so
    /// ordinary frames skip collection and the retained semantic graph is
    /// released. A mounted `SemanticsDebugger` keeps collection running.
    pub fn set_semantics_enabled(&mut self, enabled: bool) {
        self.semantics_enabled = enabled;
        if !enabled {
            // Removing nodes one by one keeps the revision monotonic, so a
            // re-enabled projection never mistakes a rebuilt graph for an
            // unchanged one.
            for (_, node) in self.semantic_ids.drain() {
                let _ = self.semantics.remove(node);
            }
            self.semantics.set_root(None);
            self.semantic_roots.clear();
            // Marks recorded while the graph is released are not kept up to
            // date, so the next pass collects everything.
            self.semantics_full_pass = true;
        }
    }

    #[must_use]
    pub const fn semantics_enabled(&self) -> bool {
        self.semantics_enabled
    }

    pub fn update_semantics(&mut self) {
        if !self.semantics_enabled && self.tracked.semantics_debuggers.is_empty() {
            return;
        }
        let stamp = (
            self.diagnostics.layouts,
            self.diagnostics.compositor_only_updates,
            self.diagnostics.scroll_offset_updates,
        );
        if stamp != self.semantics_geometry_stamp {
            self.semantics_geometry_stamp = stamp;
            if !self.tracked.followers.is_empty() {
                // Follower visibility gates on compositor leader
                // publication, which otherwise happens only inside paint.
                // Publish first so collection reflects this frame's layout
                // instead of the previous paint; the gate itself is
                // unchanged, so genuinely hidden content stays out.
                self.compositor.publish_leader_links();
                let followers = std::mem::take(&mut self.tracked.followers);
                for &follower in &followers {
                    if let Some(render) = self.elements.get(follower.0).map(|entry| entry.render) {
                        self.mark_render_semantics(render, SemanticsDirty::Subtree);
                    }
                }
                self.tracked.followers = followers;
            }
        }
        let root = self
            .root
            .and_then(|root| self.elements.get(root.0).map(|entry| (root, entry.render)));
        let full = std::mem::take(&mut self.semantics_full_pass);
        let Some((root, root_render)) = root else {
            self.semantic_roots.clear();
            self.semantics.set_root(None);
            return;
        };
        if !full
            && !self
                .render_live(root_render, "semantic root render must remain live")
                .subtree_needs_semantics
        {
            return;
        }
        let _phase_guard = self.guard_phase_root(FramePhase::Semantics);
        #[cfg(feature = "devtools")]
        let trace = self.devtools_trace_begin_element(root, TracePhase::Semantics);
        let mut scratch = std::mem::take(&mut self.semantics_scratch);
        self.collect_semantics(root, full, &mut scratch);
        self.apply_semantics(&mut scratch);
        self.semantics_scratch = scratch;
        #[cfg(feature = "devtools")]
        self.devtools_trace_end(trace);
    }

    /// Walks the marked part of the tree from `root`, collecting a node for
    /// every entered element that has semantics and an edge for every
    /// semantic node (collected or retained) under its semantic parent.
    fn collect_semantics(&mut self, root: ElementId, full: bool, scratch: &mut SemanticsScratch) {
        let SemanticsScratch {
            visits,
            builds,
            edges,
            retained,
            ..
        } = scratch;
        visits.clear();
        builds.clear();
        edges.clear();
        visits.push(SemanticVisit {
            element: root,
            parent: TOP_LEVEL,
            collection: None,
            force_content: full,
            force_bounds: full,
            parent_world: CoreTransform::IDENTITY,
        });
        while let Some(visit) = visits.pop() {
            let render = self
                .element_live(visit.element, "semantic traversal element must remain live")
                .render;
            let node = self.render_live_mut(render, "semantic render must remain live");
            if !visit.force_content && !visit.force_bounds && !node.subtree_needs_semantics {
                self.retained_semantic_edges(visit.element, visit.parent, edges, retained);
                continue;
            }
            let dirty = node.semantics_dirty;
            node.semantics_dirty = SemanticsDirty::Clean;
            node.subtree_needs_semantics = false;
            let scope = VisitScope {
                // A node whose own content or whose descendants' content is
                // stale rebuilds fully; a Button merges descendant text into
                // its label, so descendant content counts for it too.
                content: visit.force_content || dirty.any_content(),
                subtree_content: visit.force_content || dirty.subtree_content(),
                bounds_below: visit.force_bounds || dirty.bounds_below(),
            };
            self.collect_semantics_inner(visit, scope, builds, edges, visits);
        }
    }

    /// Lends the top-level semantic nodes of a clean subtree to `parent`.
    /// Its marks are clear, so the nodes it emitted last pass are current
    /// and they are exactly the elements with a semantic id.
    fn retained_semantic_edges(
        &self,
        element: ElementId,
        parent: u32,
        edges: &mut Vec<(u32, u32, ElementId)>,
        stack: &mut Vec<ElementId>,
    ) {
        stack.clear();
        stack.push(element);
        while let Some(element) = stack.pop() {
            if self.semantic_ids.contains_key(&element) {
                edges.push((parent, edges.len() as u32, element));
                continue;
            }
            let Some(entry) = self.elements.get(element.0) else {
                continue;
            };
            if semantics_hidden(entry) {
                continue;
            }
            stack.extend(self.semantic_children(entry).iter().rev());
        }
    }

    /// The children whose semantics `entry` exposes: none for a node that
    /// merges its subtree, the selected child of an indexed stack, and the
    /// children from the last one that blocks its previous siblings.
    fn semantic_children<'a>(&self, entry: &'a Element) -> &'a [ElementId] {
        if matches!(entry.widget.kind(), WidgetKind::Button(_))
            || entry.widget.semantic_properties().merge_descendants
        {
            return &[];
        }
        if let WidgetKind::IndexedStack { index, .. } = entry.widget.kind() {
            return entry
                .children
                .get(*index)
                .map(std::slice::from_ref)
                .unwrap_or_default();
        }
        let first_visible_child = entry
            .children
            .iter()
            .rposition(|child| {
                self.elements
                    .get(child.0)
                    .is_some_and(|child| child.widget.semantic_properties().block_previous_siblings)
            })
            .unwrap_or(0);
        &entry.children[first_visible_child..]
    }

    /// Writes the collected nodes and edges into the retained graph:
    /// allocates IDs for new nodes, rebuilds the child list of every
    /// collected node, and removes nodes no longer attached anywhere.
    fn apply_semantics(&mut self, scratch: &mut SemanticsScratch) {
        let SemanticsScratch {
            builds,
            child_ids,
            edges,
            attached,
            detached,
            ..
        } = scratch;
        // Allocate stable IDs for every new node before wiring any edges, so
        // a parent's child list can name children collected after it. Native
        // accessibility must receive a coherent tree on the same frame a
        // transient opens, not one semantic pass later.
        #[cfg(feature = "devtools")]
        let mut inserted = HashSet::new();
        for build in builds.iter() {
            let Collected::Full(build) = build else {
                continue;
            };
            if self.semantic_ids.contains_key(&build.element) {
                continue;
            }
            let id = self.semantics.insert(SemanticNode {
                id: SemanticNodeId(ArenaId::from_parts(0, 0)),
                role: build.role,
                label: build.label.clone(),
                value: build.value.clone(),
                description: build.description.clone(),
                bounds: build.bounds,
                state: build.state.clone(),
                actions: build.actions.clone(),
                children: Vec::new(),
            });
            self.semantic_ids.insert(build.element, id);
            #[cfg(feature = "devtools")]
            inserted.insert(build.element);
        }
        let semantic_id = |ids: &HashMap<ElementId, SemanticNodeId>, element: &ElementId| {
            *ids.get(element)
                .expect("semantic id allocated before edges are wired")
        };
        // Edges were recorded in visit order, so sorting by parent keeps
        // each child list in logical order. Top-level edges sort last.
        edges.sort_unstable_by_key(|&(parent, sequence, _)| (parent, sequence));
        attached.clear();
        attached.extend(
            edges
                .iter()
                .map(|(_, _, child)| semantic_id(&self.semantic_ids, child)),
        );
        attached.sort_unstable_by_key(|id| id.0);
        detached.clear();
        let mut cursor = 0;
        // Collection builds fresh values each pass, so moving them into the
        // retained graph avoids a second copy of every label and action list.
        for (index, build) in builds.drain(..).enumerate() {
            let element = build.element();
            let id = semantic_id(&self.semantic_ids, &element);
            let start = cursor;
            while cursor < edges.len() && edges[cursor].0 == index as u32 {
                cursor += 1;
            }
            child_ids.clear();
            child_ids.extend(
                edges[start..cursor]
                    .iter()
                    .map(|(_, _, child)| semantic_id(&self.semantic_ids, child)),
            );
            if let Some(old) = self.semantics.node(id) {
                detached.extend(
                    old.children
                        .iter()
                        .filter(|child| !is_attached(attached, **child)),
                );
            }
            #[cfg(feature = "devtools")]
            let semantic_revision_before = self.semantics.revision();
            match build {
                Collected::Full(build) => {
                    let _ = self.semantics.update(
                        id,
                        SemanticNode {
                            id,
                            role: build.role,
                            label: build.label,
                            value: build.value,
                            description: build.description,
                            bounds: build.bounds,
                            state: build.state,
                            actions: build.actions,
                            children: child_ids.clone(),
                        },
                    );
                }
                Collected::Bounds { bounds, .. } => {
                    let _ = self.semantics.update_geometry(id, bounds, child_ids);
                }
            }
            #[cfg(feature = "devtools")]
            if (inserted.contains(&element)
                || self.semantics.revision() != semantic_revision_before)
                && let Some(element) = self.elements.get_mut(element.0)
            {
                element.dev.semantic_updates = element.dev.semantic_updates.saturating_add(1);
            }
        }
        let top_level = &edges[cursor..];
        let roots_unchanged = top_level.len() == self.semantic_roots.len()
            && top_level
                .iter()
                .zip(&self.semantic_roots)
                .all(|((_, _, child), root)| semantic_id(&self.semantic_ids, child) == *root);
        if !roots_unchanged {
            detached.extend(
                self.semantic_roots
                    .iter()
                    .filter(|root| !is_attached(attached, **root)),
            );
            self.semantic_roots.clear();
            self.semantic_roots.extend(
                top_level
                    .iter()
                    .map(|(_, _, child)| semantic_id(&self.semantic_ids, child)),
            );
        }
        // A detached node takes its semantic subtree with it, except for any
        // node this pass attached somewhere else.
        let mut removed_any = false;
        while let Some(id) = detached.pop() {
            if is_attached(attached, id) {
                continue;
            }
            if let Some(node) = self.semantics.remove(id) {
                removed_any = true;
                detached.extend(node.children);
            }
        }
        if removed_any {
            let semantics = &self.semantics;
            self.semantic_ids
                .retain(|_, node| semantics.node(*node).is_some());
        }
        self.semantics
            .set_root(self.semantic_roots.first().copied());
    }

    /// Collects `visit.element`'s semantic node, if any, and queues its
    /// semantic children in reverse so they pop in logical order.
    fn collect_semantics_inner(
        &self,
        visit: SemanticVisit,
        scope: VisitScope,
        out: &mut Vec<Collected>,
        edges: &mut Vec<(u32, u32, ElementId)>,
        work: &mut Vec<SemanticVisit>,
    ) {
        let SemanticVisit {
            element,
            parent: semantic_parent,
            collection,
            parent_world,
            ..
        } = visit;
        let _node_guard = self.guard_element(FramePhase::Semantics, element);
        let entry = self.element_live(element, "semantic traversal element must remain live");
        if semantics_hidden(entry) {
            return;
        }
        if matches!(
            entry.widget.kind(),
            WidgetKind::CompositedTransformFollower { .. }
        ) && !self.follower_content_visible(entry.render)
        {
            return;
        }
        let render_id = entry.render;
        let render = self.render_live(render_id, "semantic render must remain live");
        // The world transform accumulates down the walk, so bounds cost no
        // walk over the ancestors per node.
        let world = parent_world.then(CoreTransform::translation(render.offset));
        let child_world = world.then(self.child_content_transform(render_id));
        if !scope.content && self.semantic_ids.contains_key(&element) {
            // Nothing but geometry changed here or below, so the retained
            // node keeps its role, text, state and actions; only its bounds
            // and child list are written. A node with a retained id always
            // emitted a node, so its descendants inherit no collection.
            edges.push((semantic_parent, edges.len() as u32, element));
            out.push(Collected::Bounds {
                element,
                bounds: self.semantic_bounds(render_id, world, child_world),
            });
            let this_parent = (out.len() - 1) as u32;
            work.extend(
                self.semantic_children(entry)
                    .iter()
                    .rev()
                    .map(|&child| SemanticVisit {
                        element: child,
                        parent: this_parent,
                        collection: None,
                        force_content: false,
                        force_bounds: scope.bounds_below,
                        parent_world: child_world,
                    }),
            );
            return;
        }
        // IndexedSemantics is a transparent render object. Its explicit index
        // wins over an automatically supplied sliver index and is carried
        // through transparent wrappers until the first semantic node emits.
        let mut collection = collection;
        if let WidgetKind::IndexedSemantics { index, .. } = entry.widget.kind() {
            collection = Some(SemanticCollectionContext {
                item_index: Some(*index),
                set_size: collection.and_then(|context| context.set_size),
            });
        } else if collection.is_none()
            && let Some(parent) = entry.parent.and_then(|parent| self.elements.get(parent.0))
            && let WidgetKind::SliverViewport { config } = parent.widget.kind()
            && let Some(slot) = parent.children.iter().position(|child| *child == element)
        {
            collection = Some(SemanticCollectionContext {
                item_index: parent
                    .sliver_child_semantic_indices()
                    .get(slot)
                    .copied()
                    .flatten()
                    .or_else(|| {
                        parent
                            .sliver_child_ids()
                            .get(slot)
                            .and_then(|id| id.item_index())
                    }),
                set_size: config.delegate.child_count(),
            });
        }
        let (mut role, mut default_label, mut value, mut state, mut actions) =
            match entry.widget.kind() {
                WidgetKind::Button(spec) => (
                    Some(SemanticRole::Button),
                    widget_text(&entry.widget),
                    None,
                    SemanticState {
                        enabled: spec.enabled,
                        focused: self.button_state_live(render_id).focused,
                        focusable: spec.enabled || spec.focusable_when_disabled,
                        ..SemanticState::default()
                    },
                    if spec.has_callback && spec.enabled {
                        vec![SemanticActionKind::Focus, SemanticActionKind::Activate]
                    } else {
                        vec![SemanticActionKind::Focus]
                    },
                ),
                WidgetKind::Text { text, .. } => (
                    Some(SemanticRole::Text),
                    Some(text.clone()),
                    None,
                    SemanticState::default(),
                    vec![],
                ),
                WidgetKind::SelectableText { text, .. } => (
                    Some(SemanticRole::Text),
                    Some(text.clone()),
                    None,
                    SemanticState {
                        focusable: true,
                        focused: self.selectable_text_state_live(render_id).focused,
                        ..SemanticState::default()
                    },
                    vec![SemanticActionKind::Focus],
                ),
                WidgetKind::Image { .. } if entry.widget.semantic_properties().label.is_some() => (
                    Some(SemanticRole::Image),
                    None,
                    None,
                    SemanticState::default(),
                    vec![],
                ),
                WidgetKind::TextField(spec) => {
                    let value = spec.controller.value();
                    let mut actions =
                        vec![SemanticActionKind::Focus, SemanticActionKind::SetSelection];
                    if spec.enabled && !spec.read_only {
                        actions.push(SemanticActionKind::SetText);
                    }
                    (
                        Some(if spec.multiline {
                            SemanticRole::TextArea
                        } else {
                            SemanticRole::TextField
                        }),
                        None,
                        Some(value.text),
                        SemanticState {
                            enabled: spec.enabled,
                            focused: self.text_field_state_live(render_id).focused,
                            focusable: spec.enabled,
                            editable: spec.enabled && !spec.read_only,
                            multiline: spec.multiline,
                            obscured: spec.obscure_text,
                            read_only: spec.read_only,
                            selection: Some(SemanticTextSelection {
                                base: value.selection.base,
                                extent: value.selection.extent,
                            }),
                            ..SemanticState::default()
                        },
                        actions,
                    )
                }
                WidgetKind::Scroll { controller, .. } => (
                    Some(SemanticRole::ScrollView),
                    None,
                    Some(format!(
                        "{:.0}/{:.0}",
                        controller.offset(),
                        controller.max_offset()
                    )),
                    SemanticState::default(),
                    vec![
                        SemanticActionKind::ScrollForward,
                        SemanticActionKind::ScrollBackward,
                    ],
                ),
                WidgetKind::SliverViewport { config } => (
                    Some(SemanticRole::ScrollView),
                    None,
                    Some(format!(
                        "{:.0}/{:.0}",
                        config.controller.offset(),
                        config.controller.max_offset()
                    )),
                    SemanticState::default(),
                    vec![
                        SemanticActionKind::ScrollForward,
                        SemanticActionKind::ScrollBackward,
                    ],
                ),
                _ => (None, None, None, SemanticState::default(), vec![]),
            };
        let explicit_description = entry
            .widget
            .semantic_properties()
            .explicit
            .as_ref()
            .and_then(|semantics| semantics.description.clone());
        if let Some(explicit) = &entry.widget.semantic_properties().explicit {
            role = Some(explicit.role);
            default_label = explicit.label.clone().or(default_label);
            value = explicit.value.clone().or(value);
            state = explicit.state.clone();
            actions = explicit.actions.clone();
        }
        // A native adapter must never advertise an action that the retained
        // runtime cannot route to an existing controller/handler. Explicit
        // semantic metadata configures roles/state; the Semantics callbacks
        // below are the opt-in pathway for arbitrary painted controls.
        for action in [
            SemanticActionKind::Activate,
            SemanticActionKind::Increment,
            SemanticActionKind::Decrement,
            SemanticActionKind::ScrollForward,
            SemanticActionKind::ScrollBackward,
        ] {
            if entry
                .widget
                .semantic_properties()
                .callbacks
                .supports(action)
                && !actions.contains(&action)
            {
                actions.push(action);
            }
        }
        let focusable = self.can_request_focus(element);
        state.focusable = focusable;
        if focusable && !actions.contains(&SemanticActionKind::Focus) {
            actions.push(SemanticActionKind::Focus);
        }
        actions.retain(|action| {
            if *action == SemanticActionKind::Focus {
                return focusable;
            }
            if *action == SemanticActionKind::Activate && self.pointer_actions_blocked(element) {
                return false;
            }
            semantic_action_is_executable(
                entry.widget.kind(),
                *action,
                &entry.widget.semantic_properties().callbacks,
            )
        });
        let this_parent = if let Some(role) = role {
            let mut state = state;
            if let Some(collection) = collection {
                if collection.item_index.is_some() {
                    state.item_index = collection.item_index;
                }
                if collection.set_size.is_some() {
                    state.set_size = collection.set_size;
                }
            }
            edges.push((semantic_parent, edges.len() as u32, element));
            out.push(Collected::Full(SemanticBuild {
                element,
                role,
                label: entry
                    .widget
                    .semantic_properties()
                    .explicit
                    .as_ref()
                    .and_then(|semantics| semantics.label.clone())
                    .or_else(|| entry.widget.semantic_properties().label.clone())
                    .or(default_label),
                value,
                description: entry
                    .widget
                    .semantic_properties()
                    .description
                    .clone()
                    .or(explicit_description),
                bounds: self.semantic_bounds(render_id, world, child_world),
                state,
                actions,
            }));
            (out.len() - 1) as u32
        } else {
            semantic_parent
        };
        // A Button deliberately merges its visual label/icon subtree into the
        // one control node. Other containers preserve logical child order.
        let child_collection = role.is_none().then_some(collection).flatten();
        work.extend(
            self.semantic_children(entry)
                .iter()
                .rev()
                .map(|&child| SemanticVisit {
                    element: child,
                    parent: this_parent,
                    collection: child_collection,
                    force_content: scope.subtree_content,
                    force_bounds: scope.bounds_below,
                    parent_world: child_world,
                }),
        );
    }

    /// World-space bounds of the semantic node of `render_id`.
    fn semantic_bounds(
        &self,
        render_id: RenderObjectId,
        world: CoreTransform,
        child_world: CoreTransform,
    ) -> Rect {
        let render = self.render_live(render_id, "semantic render must remain live");
        let bounds_world = if self.content_transform(render_id).is_some()
            || matches!(render.object.kind(), RenderKind::Follower { .. })
        {
            child_world
        } else {
            world
        };
        bounds_world.transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, render.size))
    }
}

/// How much of a visited element's semantics to rebuild.
#[derive(Clone, Copy)]
struct VisitScope {
    /// The element's own node is rebuilt in full.
    content: bool,
    /// Every element below is rebuilt in full.
    subtree_content: bool,
    /// Every bound below may have changed.
    bounds_below: bool,
}

fn is_attached(attached: &[SemanticNodeId], id: SemanticNodeId) -> bool {
    attached.binary_search_by_key(&id.0, |node| node.0).is_ok()
}

/// Whether `entry` and its subtree are excluded from semantics.
fn semantics_hidden(entry: &Element) -> bool {
    entry.widget.semantic_properties().hidden
        || matches!(
            entry.widget.kind(),
            WidgetKind::Visibility { visible: false, hidden, .. } if !hidden.semantics
        )
}

pub(super) fn semantic_action_is_executable(
    kind: &WidgetKind,
    action: SemanticActionKind,
    callbacks: &SemanticCallbacks,
) -> bool {
    if callbacks.supports(action) {
        return true;
    }
    match action {
        SemanticActionKind::Focus => false,
        SemanticActionKind::Activate => {
            matches!(
                kind,
                WidgetKind::Button(spec)
                    if spec.enabled && (spec.has_callback || spec.action != ActionId(0))
            )
        }
        SemanticActionKind::SetText | SemanticActionKind::SetSelection => {
            matches!(kind, WidgetKind::TextField(_))
        }
        SemanticActionKind::ScrollForward | SemanticActionKind::ScrollBackward => {
            matches!(
                kind,
                WidgetKind::Scroll { .. } | WidgetKind::SliverViewport { .. }
            )
        }
        SemanticActionKind::Increment | SemanticActionKind::Decrement => {
            // A keyboard listener services increment/decrement through key
            // dispatch even without a bound semantic callback. The affordance
            // belongs to the listener's own node: opting out via
            // include_semantics withdraws exactly this advertisement while
            // child semantics and keyboard dispatch keep working. Execution
            // shares the same predicate through the dispatch walk.
            matches!(
                kind,
                WidgetKind::Gesture { callbacks, .. } if callbacks.services_semantic_keys()
            )
        }
    }
}

pub(super) fn widget_text(widget: &Widget) -> Option<String> {
    // Fragments join with one space in tree order. The common single
    // fragment is one clone with no scratch buffers.
    fn gather(widget: &Widget, joined: &mut Option<String>) {
        match widget.kind() {
            WidgetKind::Text { text, .. } | WidgetKind::SelectableText { text, .. } => match joined
            {
                Some(joined) => {
                    joined.push(' ');
                    joined.push_str(text);
                }
                None => *joined = Some(text.clone()),
            },
            _ => {
                with_recursive_tree_stack(|| {
                    for child in widget.children_refs() {
                        gather(child, joined);
                    }
                });
            }
        }
    }
    let mut joined = None;
    gather(widget, &mut joined);
    joined
}
