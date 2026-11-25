//! Semantic-tree construction and semantic action classification.
//!
//! The pass is incremental. Invalidation marks render objects with
//! [`SemanticsDirty`] and marks the path to them, and the pass enters only
//! marked subtrees: a clean subtree keeps its retained semantic nodes and
//! only lends its top-level nodes to the parent's child list. A retained
//! node that merely lies on the path to a marked one is not written at all,
//! and its child list stays as it is, so a node that changes among many
//! siblings costs the path to it, not the siblings. A frame with nothing
//! marked does no semantic work at all.

use super::*;
use incular_semantics::{
    AccessibilityFocusBlockType, CustomSemanticsAction, CustomSemanticsActionId, SemanticAction,
    StringAttribute,
};
use std::borrow::Cow;

mod traversal;
use traversal::TraversalItem;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SemanticCollectionContext {
    item_index: Option<usize>,
    set_size: Option<usize>,
}

#[derive(Default)]
struct MergedSemanticSummary {
    label: String,
    label_attributes: Vec<StringAttribute>,
    direction: Option<TextDirection>,
    heading: Option<u8>,
    focus_block: AccessibilityFocusBlockType,
    custom_actions: Vec<CustomSemanticsActionId>,
    has_custom_callback: bool,
}

impl MergedSemanticSummary {
    fn add_label(
        &mut self,
        label: &str,
        direction: Option<TextDirection>,
        attributes: &[StringAttribute],
    ) {
        if self.direction.is_none() {
            self.direction = direction;
        }
        if label.is_empty() {
            return;
        }
        if !self.label.is_empty() {
            self.label.push('\n');
        }
        let embedded = direction.filter(|direction| Some(*direction) != self.direction);
        if let Some(direction) = embedded {
            self.label.push(match direction {
                TextDirection::Ltr => '\u{202a}',
                TextDirection::Rtl => '\u{202b}',
            });
        }
        // The source range follows any separator and opening direction mark.
        // Plain labels don't need to scan previously concatenated text.
        if !attributes.is_empty() {
            let offset = self.label.encode_utf16().count();
            self.label_attributes
                .extend(attributes.iter().map(|attribute| {
                    let range = attribute.range();
                    attribute.with_range((offset + range.start)..(offset + range.end))
                }));
        }
        self.label.push_str(label);
        if embedded.is_some() {
            self.label.push('\u{202c}');
        }
    }

    fn add_custom_actions(&mut self, callbacks: &SemanticCallbacks) {
        for binding in &callbacks.custom {
            self.custom_actions.push(binding.action.get_identifier());
            self.has_custom_callback |= binding.callback.is_some();
        }
    }
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
    /// The visual ancestor set an accessibility subtree block.
    subtree_focus_blocked: bool,
    /// The parent's world transform followed by its child content
    /// transform; this element's own offset is not applied yet.
    parent_world: CoreTransform,
    /// Clip used only for visibility of cached accessibility pages. It
    /// never clips away the semantic node's measured bounds or actions.
    page_clip: Option<Rect>,
}

/// What the pass collected for one element.
#[expect(
    clippy::large_enum_variant,
    reason = "Boxing the full build would add an allocation per rebuilt node."
)]
pub(super) enum Collected {
    /// A complete node: every property was rebuilt.
    Full(SemanticBuild),
    /// A retained node whose properties are unchanged; only its bounds and
    /// child list are rewritten.
    Bounds {
        element: ElementId,
        bounds: Rect,
        hidden: Option<bool>,
    },
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

/// Parent index below a retained node whose child list is not collected
/// this pass: nothing under it records an edge, and its clean children are
/// not visited.
const RETAINED_LIST: u32 = u32::MAX - 1;

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
    traversal_items: Vec<TraversalItem>,
    /// Elements still to search inside a clean subtree.
    retained: Vec<ElementId>,
    /// Every semantic node with a parent edge (or top-level slot) this pass,
    /// sorted for lookup.
    attached: Vec<SemanticNodeId>,
    /// Nodes dropped from a re-collected child list.
    detached: Vec<SemanticNodeId>,
    /// Scroll nodes re-collected after a scroll offset change.
    scrolls: Vec<ElementId>,
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
        if stamp.2 != self.semantics_geometry_stamp.2 {
            // Scrolling moves the compositor only, so a scroll node's offset
            // value and edge actions would go stale. Re-collect the scroll
            // nodes that have one (their own content only). The viewport
            // index keeps this independent of unrelated semantic subtrees.
            let mut scrolls = std::mem::take(&mut self.semantics_scratch.scrolls);
            scrolls.clear();
            scrolls.extend(
                self.tracked
                    .scroll_views
                    .iter()
                    .copied()
                    .filter(|id| self.semantic_ids.contains_key(id)),
            );
            for &id in &scrolls {
                if let Some(render) = self.elements.get(id.0).map(|entry| entry.render) {
                    self.mark_render_semantics(render, SemanticsDirty::Node);
                }
            }
            scrolls.clear();
            self.semantics_scratch.scrolls = scrolls;
        }
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
            subtree_focus_blocked: false,
            parent_world: CoreTransform::IDENTITY,
            page_clip: None,
        });
        // Set when a node below a retained child list turns out to have
        // appeared or disappeared. Marks never announce that (see
        // `SemanticsDirty::is_content_scope`), so it is only a guard: the
        // graph is then collected whole instead of being left inconsistent.
        let mut inconsistent = false;
        while let Some(visit) = visits.pop() {
            let render = self
                .element_live(visit.element, "semantic traversal element must remain live")
                .render;
            let node = self.render_live_mut(render, "semantic render must remain live");
            if !visit.force_content && !visit.force_bounds && !node.subtree_needs_semantics {
                if visit.parent != RETAINED_LIST {
                    self.retained_semantic_edges(visit.element, visit.parent, edges, retained);
                }
                continue;
            }
            let dirty = node.semantics_dirty;
            node.semantics_dirty = SemanticsDirty::Clean;
            node.subtree_needs_semantics = false;
            let scope = VisitScope {
                // A node whose own content is stale, or below which a node
                // may have appeared or disappeared, rebuilds fully.
                content: visit.force_content || dirty.any_content(),
                subtree_content: visit.force_content || dirty.subtree_content(),
                own_bounds: visit.force_bounds || dirty.own_bounds(),
                bounds_below: visit.force_bounds || dirty.bounds_below(),
                descendant_bounds: dirty.has_descendant_bounds(),
            };
            self.collect_semantics_inner(visit, scope, builds, edges, visits, &mut inconsistent);
        }
        if inconsistent && !full {
            debug_assert!(
                false,
                "a semantic node changed presence without a subtree mark"
            );
            self.collect_semantics(root, true, scratch);
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
            stack.extend(self.semantic_children(element, entry).iter().rev());
        }
    }

    /// The children whose semantics `entry` exposes: none for a node that
    /// merges its subtree, the selected child of an indexed stack, and the
    /// children from the last one that blocks its previous siblings.
    fn semantic_children<'a>(
        &self,
        element: ElementId,
        entry: &'a Element,
    ) -> Cow<'a, [ElementId]> {
        if absorbs_descendant_semantics(entry) {
            return Cow::Borrowed(&[]);
        }
        self.visible_semantic_children(element, entry)
    }

    fn visible_semantic_children<'a>(
        &self,
        element: ElementId,
        entry: &'a Element,
    ) -> Cow<'a, [ElementId]> {
        if matches!(entry.widget.kind(), WidgetKind::Button(_))
            || self.fitted_children_have_no_semantic_geometry(entry)
        {
            return Cow::Borrowed(&[]);
        }
        if let WidgetKind::IndexedStack { index, .. } = entry.widget.kind() {
            return Cow::Borrowed(
                index
                    .and_then(|slot| entry.children.get(slot))
                    .map(std::slice::from_ref)
                    .unwrap_or_default(),
            );
        }
        if let Some(grouped) = self.sliver_semantics_children(element, entry) {
            return Cow::Owned(grouped);
        }
        if self.tracked.sibling_blockers.is_empty() {
            return Cow::Borrowed(&entry.children);
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
        Cow::Borrowed(&entry.children[first_visible_child..])
    }

    /// Whether `entry` is a viewport whose `SliverSemantics` anchors adopt
    /// some of its children. A grouped child is marked through the viewport,
    /// not its anchor, so every child is collected again when it is entered.
    fn regroups_sliver_semantics(&self, entry: &Element) -> bool {
        matches!(entry.widget.kind(), WidgetKind::SliverViewport { .. })
            && entry
                .sliver_child_ids()
                .iter()
                .any(|id| crate::scrolling::is_semantics_anchor(*id))
    }

    /// The children a `SliverSemantics` grouping changes: an anchor's are
    /// the other children of its sliver, and its viewport's are the children
    /// left in no group. `None` where no anchor applies.
    fn sliver_semantics_children(
        &self,
        element: ElementId,
        entry: &Element,
    ) -> Option<Vec<ElementId>> {
        let anchors = |viewport: &Element| -> Vec<SliverChildId> {
            viewport
                .sliver_child_ids()
                .iter()
                .copied()
                .filter(|id| crate::scrolling::is_semantics_anchor(*id))
                .collect()
        };
        if matches!(entry.widget.kind(), WidgetKind::SliverViewport { .. }) {
            let anchors = anchors(entry);
            if anchors.is_empty() {
                return None;
            }
            let ids = entry.sliver_child_ids();
            return Some(
                entry
                    .children
                    .iter()
                    .enumerate()
                    .filter(|(slot, _)| {
                        ids.get(*slot).is_none_or(|id| {
                            !anchors
                                .iter()
                                .any(|anchor| crate::scrolling::in_semantics_group(*anchor, *id))
                        })
                    })
                    .map(|(_, child)| *child)
                    .collect(),
            );
        }
        let parent = self.elements.get(entry.parent?.0)?;
        if !matches!(parent.widget.kind(), WidgetKind::SliverViewport { .. }) {
            return None;
        }
        let ids = parent.sliver_child_ids();
        let slot = parent.children.iter().position(|child| *child == element)?;
        let anchor = *ids.get(slot)?;
        if !crate::scrolling::is_semantics_anchor(anchor) {
            return None;
        }
        // A nested anchor's children belong to it, not to this one.
        let nested = anchors(parent);
        Some(
            entry
                .children
                .iter()
                .copied()
                .chain(
                    parent
                        .children
                        .iter()
                        .zip(ids)
                        .filter(|(_, id)| {
                            crate::scrolling::in_semantics_group(anchor, **id)
                                && !nested.iter().any(|inner| {
                                    *inner != anchor
                                        && crate::scrolling::in_semantics_group(anchor, *inner)
                                        && crate::scrolling::in_semantics_group(*inner, **id)
                                })
                        })
                        .map(|(child, _)| *child),
                )
                .collect(),
        )
    }

    /// Writes the collected nodes and edges into the retained graph:
    /// allocates IDs for new nodes, rebuilds the child list of every
    /// collected node, and removes nodes no longer attached anywhere.
    fn apply_semantics(&mut self, scratch: &mut SemanticsScratch) {
        let SemanticsScratch {
            builds,
            child_ids,
            edges,
            traversal_items,
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
        let semantic_id = |ids: &FastHashMap<ElementId, SemanticNodeId>, element: &ElementId| {
            *ids.get(element)
                .expect("semantic id allocated before edges are wired")
        };
        // Edges were recorded in visit order, so sorting by parent keeps
        // each child list in logical order. Top-level edges sort last.
        edges.sort_unstable_by_key(|&(parent, sequence, _)| (parent, sequence));
        self.sort_sibling_groups(edges, builds, traversal_items);
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
                Collected::Bounds { bounds, hidden, .. } => {
                    let _ = self
                        .semantics
                        .update_geometry_and_visibility(id, bounds, child_ids, hidden);
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
        // Every top-level node is a root: layout-only ancestors create no
        // semantic node, so the graph is a forest whenever one has several
        // semantic children. Publishing only the first would leave the rest
        // unreachable from the native window.
        let roots_unchanged = top_level.len() == self.semantics.roots().len()
            && top_level
                .iter()
                .zip(self.semantics.roots())
                .all(|((_, _, child), root)| semantic_id(&self.semantic_ids, child) == *root);
        if !roots_unchanged {
            detached.extend(
                self.semantics
                    .roots()
                    .iter()
                    .filter(|root| !is_attached(attached, **root)),
            );
            let semantic_ids = &self.semantic_ids;
            self.semantics.set_roots(
                top_level
                    .iter()
                    .map(|(_, _, child)| semantic_id(semantic_ids, child)),
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
        inconsistent: &mut bool,
    ) {
        let SemanticVisit {
            element,
            parent: semantic_parent,
            collection,
            parent_world,
            subtree_focus_blocked,
            page_clip,
            ..
        } = visit;
        let _node_guard = self.guard_element(FramePhase::Semantics, element);
        let entry = self.element_live(element, "semantic traversal element must remain live");
        if semantics_hidden(entry) {
            return;
        }
        let own_focus_block = if subtree_focus_blocked {
            AccessibilityFocusBlockType::BlockSubtree
        } else {
            explicit_focus_block(entry)
        };
        let child_focus_blocked = own_focus_block == AccessibilityFocusBlockType::BlockSubtree;
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
        let child_page_clip = match render.object.kind() {
            RenderKind::SliverViewport { config }
                if config.physics.page_geometry_fraction().is_some()
                    && config.physics.allows_implicit_scrolling() =>
            {
                let viewport =
                    world.transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, render.size));
                Some(page_clip.map_or(viewport, |ancestor| {
                    ancestor.intersection(viewport).unwrap_or_default()
                }))
            }
            _ => page_clip,
        };
        let retained = self.semantic_ids.contains_key(&element);
        if !scope.content && retained {
            // Nothing but geometry changed here or below, so the retained
            // node keeps its role, text, state and actions. A node with a
            // retained id always emitted a node, so its descendants inherit
            // no collection.
            if semantic_parent != RETAINED_LIST {
                edges.push((semantic_parent, edges.len() as u32, element));
            }
            let regroups = self.regroups_sliver_semantics(entry);
            let reorder = scope.descendant_bounds
                && self.semantic_reading_direction(element).is_some()
                && self
                    .semantic_ids
                    .get(&element)
                    .and_then(|id| self.semantics.node(*id))
                    .is_some_and(|node| node.children.len() > 1);
            if !scope.own_bounds && !regroups && !reorder {
                // The node only lies on the path to a marked one: its bounds
                // and its child list stand, and only its marked children
                // are entered.
                work.extend(
                    self.semantic_children(element, entry)
                        .iter()
                        .enumerate()
                        .rev()
                        .filter(|&(_, &child)| self.semantics_marked(child))
                        .map(|(index, &child)| SemanticVisit {
                            element: child,
                            parent: RETAINED_LIST,
                            collection: None,
                            force_content: false,
                            force_bounds: false,
                            subtree_focus_blocked: child_focus_blocked,
                            page_clip: child_page_clip,
                            parent_world: if index < entry.children.len() {
                                child_world
                            } else {
                                parent_world
                            },
                        }),
                );
                return;
            }
            // Its bounds and child list are written.
            let bounds = self.semantic_bounds(render_id, world, child_world);
            out.push(Collected::Bounds {
                element,
                bounds,
                hidden: page_clip.map(|clip| {
                    entry
                        .widget
                        .semantic_properties()
                        .explicit
                        .as_ref()
                        .is_some_and(|explicit| explicit.state.hidden)
                        || !semantic_clip_overlaps(clip, bounds)
                }),
            });
            let this_parent = (out.len() - 1) as u32;
            work.extend(
                self.semantic_children(element, entry)
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(index, &child)| SemanticVisit {
                        element: child,
                        parent: this_parent,
                        collection: None,
                        force_content: regroups,
                        force_bounds: scope.bounds_below || regroups,
                        subtree_focus_blocked: child_focus_blocked,
                        page_clip: child_page_clip,
                        parent_world: if index < entry.children.len() {
                            child_world
                        } else {
                            parent_world
                        },
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
                            .filter(|id| !crate::scrolling::is_semantics_anchor(**id))
                            .and_then(|id| id.item_index())
                    }),
                set_size: config.delegate.child_count(),
            });
        }
        let (mut role, mut default_label, mut value, mut state, mut actions) = match entry
            .widget
            .kind()
        {
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
                    actions_with_room(&[SemanticActionKind::Focus, SemanticActionKind::Activate])
                } else {
                    actions_with_room(&[SemanticActionKind::Focus])
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
                actions_with_room(&[SemanticActionKind::Focus]),
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
                let focused = self.text_field_state_live(render_id).focused;
                let mut actions = actions_with_room(&[
                    SemanticActionKind::Focus,
                    SemanticActionKind::SetSelection,
                ]);
                // Flutter advertises setText only on the focused, editable field.
                if spec.enabled && !spec.read_only && focused {
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
                        focused,
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
                scroll_edge_actions(controller),
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
                scroll_edge_actions(&config.controller),
            ),
            WidgetKind::Gesture { callbacks, .. } => {
                let policy = entry.widget.semantic_properties().callbacks.gesture_policy;
                let actions: Vec<_> = GestureSemanticPolicy::ACTIONS
                    .into_iter()
                    .filter(|action| gesture_supports(callbacks, policy, *action))
                    .collect();
                (
                    (!actions.is_empty()).then_some(SemanticRole::GenericContainer),
                    None,
                    None,
                    SemanticState::default(),
                    actions,
                )
            }
            _ => (None, None, None, SemanticState::default(), vec![]),
        };
        let properties = entry.widget.semantic_properties();
        // An action-only annotation also creates a node.
        if role.is_none()
            && (properties.merge_descendants || !properties.callbacks.custom.is_empty())
        {
            role = Some(SemanticRole::GenericContainer);
        }
        let mut merged = MergedSemanticSummary::default();
        merged.add_custom_actions(&properties.callbacks);
        if absorbs_descendant_semantics(entry) {
            if let Some(own) = element_own_label(entry) {
                merged.add_label(
                    &own,
                    self.semantic_label_direction(element, entry),
                    element_label_attributes(entry),
                );
            } else {
                merged.direction = entry
                    .widget
                    .semantic_properties()
                    .explicit
                    .as_ref()
                    .and_then(|node| node.state.text_direction);
            }
            for &child in self.visible_semantic_children(element, entry).iter() {
                self.push_merged_semantics(child, &mut merged);
            }
        }
        let merged_label = (!merged.label.is_empty()).then_some(merged.label);
        if let Some(explicit) = &entry.widget.semantic_properties().explicit {
            if explicit.role == SemanticRole::GenericContainer && role.is_some() {
                // A role-less Semantics wrapper over a widget that has its
                // own role (a Button, a Text): keep that role, state and
                // actions, and overlay only the metadata the wrapper set.
                default_label = explicit.label.clone().or(default_label);
                value = explicit.value.clone().or(value);
                overlay_semantic_state(&mut state, &explicit.state);
                for action in &explicit.actions {
                    if !actions.contains(action) {
                        actions.push(*action);
                    }
                }
            } else {
                role = Some(explicit.role);
                default_label = explicit.label.clone().or(default_label);
                value = explicit.value.clone().or(value);
                state = explicit.state.clone();
                actions = actions_with_room(&explicit.actions);
            }
        }
        if merged_label.is_some()
            && (!merged.label_attributes.is_empty() || state.text_attributes.is_some())
        {
            let attributes = Arc::make_mut(state.text_attributes.get_or_insert_with(Arc::default));
            attributes.label = merged.label_attributes;
            if attributes.is_empty() {
                state.text_attributes = None;
            }
        }
        state.text_direction = state.text_direction.or(merged.direction).or_else(|| {
            (default_label.is_some() || entry.widget.semantic_properties().label.is_some())
                .then(|| self.semantic_label_direction(element, entry))
                .flatten()
        });
        // Flutter absorption preserves the parent's level if present.
        // Otherwise the first descendant with a heading supplies it.
        state.heading_level = state.heading_level.or(merged.heading);
        state.accessibility_focus_block_type = own_focus_block.merge(merged.focus_block);
        if state.heading_level.is_some() && role == Some(SemanticRole::GenericContainer) {
            role = Some(SemanticRole::Heading);
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
            SemanticActionKind::ScrollLeft,
            SemanticActionKind::ScrollRight,
            SemanticActionKind::ScrollUp,
            SemanticActionKind::ScrollDown,
            SemanticActionKind::LongPress,
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
        // Flutter advertises showOnScreen on every node inside a viewport.
        // Only nodes with a role are emitted, so role-less elements skip it
        // (and the per-element allocation it would cost).
        if role.is_some()
            && !actions.contains(&SemanticActionKind::ShowOnScreen)
            && self.has_scroll_ancestor(element)
        {
            actions.push(SemanticActionKind::ShowOnScreen);
        }
        let focus_blocked = state.accessibility_focus_block_type.blocks_focus();
        let focusable = !focus_blocked && self.can_request_focus(element);
        if focus_blocked {
            // The portable projection uses false for the omitted focused
            // flag. The independent retained keyboard focus is untouched.
            state.focused = false;
        }
        state.focusable = focusable;
        if focusable && !actions.contains(&SemanticActionKind::Focus) {
            actions.push(SemanticActionKind::Focus);
        }
        actions.retain(|action| {
            if *action == SemanticActionKind::Focus {
                return focusable;
            }
            if (*action == SemanticActionKind::Activate || action.is_directional_scroll())
                && self.pointer_actions_blocked(element)
            {
                return false;
            }
            semantic_action_is_executable(
                entry.widget.kind(),
                *action,
                &entry.widget.semantic_properties().callbacks,
            )
        });
        merged.custom_actions.sort_unstable_by_key(|id| id.0);
        merged.custom_actions.dedup();
        state.custom_actions = merged.custom_actions.into_boxed_slice();
        if merged.has_custom_callback {
            actions.push(SemanticActionKind::Custom);
        }
        if semantic_parent == RETAINED_LIST && role.is_some() != retained {
            // The parent's retained child list no longer describes this
            // node; see `collect_semantics`.
            *inconsistent = true;
            return;
        }
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
            if semantic_parent != RETAINED_LIST {
                edges.push((semantic_parent, edges.len() as u32, element));
            }
            let bounds = self.semantic_bounds(render_id, world, child_world);
            state.hidden |= page_clip.is_some_and(|clip| !semantic_clip_overlaps(clip, bounds));
            out.push(Collected::Full(SemanticBuild {
                element,
                role,
                label: merged_label.or_else(|| {
                    entry
                        .widget
                        .semantic_properties()
                        .explicit
                        .as_ref()
                        .and_then(|semantics| semantics.label.clone())
                        .or_else(|| entry.widget.semantic_properties().label.clone())
                        .or(default_label)
                }),
                value,
                description: entry
                    .widget
                    .semantic_properties()
                    .description
                    .clone()
                    .or_else(|| {
                        entry
                            .widget
                            .semantic_properties()
                            .explicit
                            .as_ref()
                            .and_then(|semantics| semantics.description.clone())
                    }),
                bounds,
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
        let regroups = self.regroups_sliver_semantics(entry);
        // Below a retained child list a clean child has nothing to lend.
        let only_marked = this_parent == RETAINED_LIST
            && !scope.subtree_content
            && !scope.bounds_below
            && !regroups;
        work.extend(
            self.semantic_children(element, entry)
                .iter()
                .enumerate()
                .rev()
                .filter(|&(_, &child)| !only_marked || self.semantics_marked(child))
                .map(|(index, &child)| SemanticVisit {
                    element: child,
                    parent: this_parent,
                    collection: child_collection,
                    force_content: scope.subtree_content || regroups,
                    force_bounds: scope.bounds_below || regroups,
                    subtree_focus_blocked: child_focus_blocked,
                    page_clip: child_page_clip,
                    // A sliver anchor's own children come first; the
                    // grouped ones after them sit in the viewport's space,
                    // beside the anchor.
                    parent_world: if index < entry.children.len() {
                        child_world
                    } else {
                        parent_world
                    },
                }),
        );
    }

    /// Reorders each sibling group the way Flutter's
    /// `SemanticsNode._childrenInTraversalOrder` does: siblings form runs of
    /// compatible sort keys, a keyed run is ordered by key (ties keep widget
    /// order), and an unkeyed run keeps its place.
    fn sort_sibling_groups(
        &self,
        edges: &mut [(u32, u32, ElementId)],
        builds: &[Collected],
        traversal_items: &mut Vec<TraversalItem>,
    ) {
        let mut start = 0;
        while start < edges.len() {
            let parent = edges[start].0;
            let end = start
                + edges[start..]
                    .iter()
                    .take_while(|(group, _, _)| *group == parent)
                    .count();
            if end - start > 1 {
                if let Some(parent) = builds.get(parent as usize).map(Collected::element)
                    && let Some(direction) = self.semantic_reading_direction(parent)
                {
                    self.sort_by_reading_order(
                        parent,
                        direction,
                        &mut edges[start..end],
                        traversal_items,
                    );
                }
                self.sort_sibling_group(&mut edges[start..end]);
            }
            start = end;
        }
    }

    fn semantic_reading_direction(&self, element: ElementId) -> Option<TextDirection> {
        self.elements
            .get(element.0)?
            .render_context
            .read::<TextDirection>()
    }

    fn semantic_label_direction(
        &self,
        element: ElementId,
        entry: &Element,
    ) -> Option<TextDirection> {
        entry
            .widget
            .semantic_properties()
            .explicit
            .as_ref()
            .and_then(|node| node.state.text_direction)
            .or_else(|| match entry.widget.kind() {
                WidgetKind::Text { text_direction, .. } => *text_direction,
                _ => None,
            })
            .or_else(|| self.semantic_reading_direction(element))
    }

    fn sort_by_reading_order(
        &self,
        parent: ElementId,
        direction: TextDirection,
        edges: &mut [(u32, u32, ElementId)],
        items: &mut Vec<TraversalItem>,
    ) {
        let Some(parent_render) = self.render_id(parent) else {
            return;
        };
        items.clear();
        for &edge in edges.iter() {
            let Some(render) = self.render_id(edge.2) else {
                return;
            };
            let Some(node) = self.renders.get(render.0) else {
                return;
            };
            let Some(transform) = self.semantic_traversal_transform(render, parent_render) else {
                return;
            };
            let Some(item) = TraversalItem::new(edge, transform, node.size) else {
                return;
            };
            items.push(item);
        }
        traversal::order(items, direction);
        for (edge, item) in edges.iter_mut().zip(items.iter()) {
            *edge = item.edge;
        }
    }

    /// Ordinary semantic ancestry follows render ancestry, so accumulate the
    /// relative transform without allocating a world path per sibling.
    fn semantic_traversal_transform(
        &self,
        child: RenderObjectId,
        parent: RenderObjectId,
    ) -> Option<CoreTransform> {
        let node = self.renders.get(child.0)?;
        let content = if self.content_transform(child).is_some()
            || matches!(node.object.kind(), RenderKind::Follower { .. })
        {
            self.child_content_transform(child)
        } else {
            CoreTransform::IDENTITY
        };
        let mut relative = CoreTransform::translation(node.offset).then(content);
        let mut cursor = node.parent;
        while let Some(ancestor) = cursor {
            let node = self.renders.get(ancestor.0)?;
            relative = self.child_content_transform(ancestor).then(relative);
            if ancestor == parent {
                return Some(relative);
            }
            relative = CoreTransform::translation(node.offset).then(relative);
            cursor = node.parent;
        }
        // Traversal-grafted nodes, e.g. an overlay portal, may not descend
        // from their semantic parent in the render tree.
        Some(
            self.render_world_transform(parent)
                .inverse()?
                .then(self.render_world_transform(child))
                .then(content),
        )
    }

    /// The sort key the element's explicit semantics declares, if any.
    fn element_sort_key(&self, element: ElementId) -> Option<&incular_semantics::OrdinalSortKey> {
        self.elements
            .get(element.0)
            .and_then(|entry| widget_sort_key(&entry.widget))
    }

    fn sort_sibling_group(&self, group: &mut [(u32, u32, ElementId)]) {
        if group
            .iter()
            .all(|(_, _, element)| self.element_sort_key(*element).is_none())
        {
            return;
        }
        // Runs are contiguous and each sorts in place; a run's members are
        // all compatible, so sorting it leaves the run boundaries intact.
        let mut run_start = 0;
        while run_start < group.len() {
            let mut run_end = run_start + 1;
            while run_end < group.len()
                && sort_keys_compatible(
                    self.element_sort_key(group[run_end - 1].2),
                    self.element_sort_key(group[run_end].2),
                )
            {
                run_end += 1;
            }
            if self.element_sort_key(group[run_start].2).is_some() {
                // A stable sort keeps widget order among equal keys.
                group[run_start..run_end].sort_by(|a, b| {
                    let a = self.element_sort_key(a.2).expect("keyed sort run");
                    let b = self.element_sort_key(b.2).expect("keyed sort run");
                    a.compare_to(b)
                });
            }
            run_start = run_end;
        }
    }

    /// Whether a visible `element` sits in a scroll viewport, so an
    /// assistive technology may ask for it to be shown on screen.
    fn has_scroll_ancestor(&self, element: ElementId) -> bool {
        let mut current = self.elements.get(element.0).and_then(|entry| entry.parent);
        while let Some(parent) = current {
            let Some(entry) = self.elements.get(parent.0) else {
                return false;
            };
            if matches!(
                entry.widget.kind(),
                WidgetKind::Scroll { .. } | WidgetKind::SliverViewport { .. }
            ) {
                return true;
            }
            current = entry.parent;
        }
        false
    }

    /// Flutter's `showOnScreen`: the nearest viewport that allows an
    /// implicit reveal scrolls to the element's bounds. A viewport that
    /// declines passes the request to its ancestors without moving itself.
    pub fn semantic_show_on_screen(&self, element: ElementId) -> bool {
        let Some(node) = self
            .semantic_ids
            .get(&element)
            .and_then(|id| self.semantics.node(*id))
        else {
            return false;
        };
        let Some((viewport, controller, axis, reverse, _)) = self
            .scrollable_viewport_chain(element)
            .into_iter()
            .find(|(_, _, _, _, physics)| physics.allows_implicit_scrolling())
        else {
            return false;
        };
        let Some(view) = self
            .semantic_ids
            .get(&viewport)
            .and_then(|id| self.semantics.node(*id))
        else {
            return false;
        };
        let (start, end, view_start, view_end) = match axis {
            Axis::Vertical => (
                node.bounds.origin.y,
                node.bounds.origin.y + node.bounds.size.height,
                view.bounds.origin.y,
                view.bounds.origin.y + view.bounds.size.height,
            ),
            Axis::Horizontal => (
                node.bounds.origin.x,
                node.bounds.origin.x + node.bounds.size.width,
                view.bounds.origin.x,
                view.bounds.origin.x + view.bounds.size.width,
            ),
        };
        let delta = if start < view_start {
            start - view_start
        } else if end > view_end {
            end - view_end
        } else {
            return true;
        };
        // A reversed viewport's offset grows toward the leading edge.
        let signed = if reverse { -delta } else { delta };
        let target = (controller.offset() + signed)
            .clamp(controller.min_offset(), controller.max_offset().max(0.0));
        controller.reveal_at(target);
        true
    }

    /// Whether `element` or anything below it is marked for this pass.
    fn semantics_marked(&self, element: ElementId) -> bool {
        self.elements
            .get(element.0)
            .and_then(|entry| self.renders.get(entry.render.0))
            .is_some_and(|render| render.subtree_needs_semantics)
    }

    fn push_merged_semantics(&self, element: ElementId, merged: &mut MergedSemanticSummary) {
        let Some(entry) = self.elements.get(element.0) else {
            return;
        };
        if !self.can_contribute_merged_semantics(entry) {
            return;
        }
        if let Some(label) = element_own_label(entry) {
            merged.add_label(
                &label,
                self.semantic_label_direction(element, entry),
                element_label_attributes(entry),
            );
        } else if merged.direction.is_none() {
            merged.direction = entry
                .widget
                .semantic_properties()
                .explicit
                .as_ref()
                .and_then(|node| node.state.text_direction);
        }
        merged.add_custom_actions(&entry.widget.semantic_properties().callbacks);
        merged.focus_block = merged.focus_block.merge(explicit_focus_block(entry));
        if merged.heading.is_none() {
            merged.heading = entry
                .widget
                .semantic_properties()
                .explicit
                .as_ref()
                .and_then(|properties| properties.state.heading_level);
        }
        for &child in self.visible_semantic_children(element, entry).iter() {
            self.push_merged_semantics(child, merged);
        }
    }

    fn can_contribute_merged_semantics(&self, entry: &Element) -> bool {
        !semantics_hidden(entry)
            && (!matches!(
                entry.widget.kind(),
                WidgetKind::CompositedTransformFollower { .. }
            ) || self.follower_content_visible(entry.render))
    }

    /// Resolves an advertised custom action to the current owning widget.
    /// Only a merged node can borrow handlers from its visible descendants.
    #[doc(hidden)]
    #[must_use]
    pub fn custom_semantic_action_callback(
        &self,
        element: ElementId,
        action: CustomSemanticsActionId,
    ) -> Option<Rc<dyn Fn() + 'static>> {
        let node = *self.semantic_ids.get(&element)?;
        if self
            .semantics
            .node(node)?
            .state
            .custom_actions
            .binary_search_by_key(&action.0, |id| id.0)
            .is_err()
        {
            return None;
        }
        let entry = self.elements.get(element.0)?;
        let action = CustomSemanticsAction::get_action(action)?;
        self.find_custom_semantic_action(element, &action, absorbs_descendant_semantics(entry))
    }

    fn find_custom_semantic_action(
        &self,
        element: ElementId,
        action: &CustomSemanticsAction,
        search_descendants: bool,
    ) -> Option<Rc<dyn Fn() + 'static>> {
        let entry = self.elements.get(element.0)?;
        if !self.can_contribute_merged_semantics(entry) {
            return None;
        }
        if let Some(callback) = entry
            .widget
            .semantic_properties()
            .callbacks
            .custom
            .iter()
            .filter(|binding| &binding.action == action)
            .find_map(|binding| binding.callback.as_ref())
        {
            return Some(callback.clone());
        }
        if search_descendants {
            for &child in self.visible_semantic_children(element, entry).iter() {
                if let Some(callback) = self.find_custom_semantic_action(child, action, true) {
                    return Some(callback);
                }
            }
        }
        None
    }

    /// Selects a retained semantic action target at a logical window point.
    #[doc(hidden)]
    #[must_use]
    pub fn semantic_action_target_at(
        &self,
        position: Offset,
        action: &SemanticAction,
    ) -> Option<SemanticNodeId> {
        self.semantics
            .roots()
            .iter()
            .rev()
            .find_map(|&node| self.semantic_action_in_subtree(node, position, action))
    }

    fn semantic_action_in_subtree(
        &self,
        node: SemanticNodeId,
        position: Offset,
        action: &SemanticAction,
    ) -> Option<SemanticNodeId> {
        let semantic = self.semantics.node(node)?;
        // Test the world AABB before reconstructing the local coordinates.
        if !semantic.bounds.contains(position) {
            return None;
        }
        let element = self.element_for_semantic_node(node)?;
        let render_id = self.render_id(element)?;
        let render = self.renders.get(render_id.0)?;
        let world = self.render_world_transform(render_id);
        let child_world = world.then(self.child_content_transform(render_id));
        let local = self
            .semantic_bounds_transform(render_id, world, child_world)
            .inverse_transform_point(position)?;
        // Like Flutter's Rect.contains, a semantic box excludes its right
        // and bottom edges. The pointer path has separate edge semantics.
        if !(local.x >= 0.0
            && local.x < render.size.width
            && local.y >= 0.0
            && local.y < render.size.height)
        {
            return None;
        }

        if let Some(child) = semantic
            .children
            .iter()
            .rev()
            .find_map(|&child| self.semantic_action_in_subtree(child, position, action))
        {
            return Some(child);
        }
        if !semantic.actions.contains(&action.kind()) {
            return None;
        }
        match action {
            SemanticAction::Custom(id) => self
                .custom_semantic_action_callback(element, *id)
                .is_some()
                .then_some(node),
            _ => Some(node),
        }
    }

    fn fitted_children_have_no_semantic_geometry(&self, entry: &Element) -> bool {
        matches!(entry.widget.kind(), WidgetKind::FittedBox { .. })
            && !self.fitted_content_is_visible(entry.render)
    }

    /// World-space bounds of the semantic node of `render_id`.
    fn semantic_bounds(
        &self,
        render_id: RenderObjectId,
        world: CoreTransform,
        child_world: CoreTransform,
    ) -> Rect {
        let render = self.render_live(render_id, "semantic render must remain live");
        self.semantic_bounds_transform(render_id, world, child_world)
            .transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, render.size))
    }

    fn semantic_bounds_transform(
        &self,
        render_id: RenderObjectId,
        world: CoreTransform,
        child_world: CoreTransform,
    ) -> CoreTransform {
        let render = self.render_live(render_id, "semantic render must remain live");
        if self.content_transform(render_id).is_some()
            || matches!(render.object.kind(), RenderKind::Follower { .. })
        {
            child_world
        } else {
            world
        }
    }
}

fn semantic_clip_overlaps(clip: Rect, bounds: Rect) -> bool {
    clip.size.width > 0. && clip.size.height > 0. && clip.intersects(bounds)
}

/// How much of a visited element's semantics to rebuild.
#[derive(Clone, Copy)]
struct VisitScope {
    /// The element's own node is rebuilt in full.
    content: bool,
    /// Every element below is rebuilt in full.
    subtree_content: bool,
    /// The element's own bounds may have changed.
    own_bounds: bool,
    /// Every bound below may have changed.
    bounds_below: bool,
    /// A retained child list may need new spatial traversal ordering.
    descendant_bounds: bool,
}

/// Scroll actions for the current offset: forward only before the maximum,
/// backward only after the start (Flutter `ScrollPosition` semantics).
/// An action list with one spare slot for the `ShowOnScreen` action every
/// node inside a viewport gains, so it does not reallocate per node.
fn actions_with_room(actions: &[SemanticActionKind]) -> Vec<SemanticActionKind> {
    let mut list = Vec::with_capacity(actions.len() + 1);
    list.extend_from_slice(actions);
    list
}

fn scroll_edge_actions(controller: &ScrollController) -> Vec<SemanticActionKind> {
    let offset = controller.offset();
    let mut actions = Vec::with_capacity(2);
    if offset < controller.max_offset() {
        actions.push(SemanticActionKind::ScrollForward);
    }
    if offset > controller.min_offset() {
        actions.push(SemanticActionKind::ScrollBackward);
    }
    actions
}

/// The label an element declares for itself, without its descendants.
/// Borrowed from the element except for a button's gathered text.
fn element_own_label(entry: &Element) -> Option<Cow<'_, str>> {
    let properties = entry.widget.semantic_properties();
    properties
        .explicit
        .as_ref()
        .and_then(|explicit| explicit.label.as_deref())
        .or(properties.label.as_deref())
        .map(Cow::Borrowed)
        .or_else(|| match entry.widget.kind() {
            WidgetKind::Text { text, .. } | WidgetKind::SelectableText { text, .. } => {
                Some(Cow::Borrowed(text.as_str()))
            }
            WidgetKind::Button(_) => widget_text(&entry.widget).map(Cow::Owned),
            _ => None,
        })
}

fn element_label_attributes(entry: &Element) -> &[StringAttribute] {
    entry
        .widget
        .semantic_properties()
        .explicit
        .as_ref()
        .filter(|explicit| explicit.label.is_some())
        .and_then(|explicit| explicit.state.text_attributes.as_ref())
        .map_or(&[], |attributes| attributes.label.as_slice())
}

struct DragSemanticPhases<'a> {
    down: &'a Option<Rc<dyn Fn(incular_gestures::DragDownDetails)>>,
    start: &'a Option<Rc<dyn Fn(DragStartDetails)>>,
    update: &'a Option<Rc<dyn Fn(DragUpdateDetails)>>,
    end: &'a Option<Rc<dyn Fn(DragEndDetails)>>,
    has_cancel: bool,
}

impl<'a> DragSemanticPhases<'a> {
    fn for_axis(callbacks: &'a GestureCallbacks, axis: Option<Axis>) -> Self {
        match axis {
            Some(Axis::Horizontal) => Self {
                down: &callbacks.on_horizontal_drag_down,
                start: &callbacks.on_horizontal_drag_start,
                update: &callbacks.on_horizontal_drag_update,
                end: &callbacks.on_horizontal_drag_end,
                has_cancel: callbacks.on_horizontal_drag_cancel.is_some(),
            },
            Some(Axis::Vertical) => Self {
                down: &callbacks.on_vertical_drag_down,
                start: &callbacks.on_vertical_drag_start,
                update: &callbacks.on_vertical_drag_update,
                end: &callbacks.on_vertical_drag_end,
                has_cancel: callbacks.on_vertical_drag_cancel.is_some(),
            },
            None => Self {
                down: &callbacks.on_pan_down,
                start: &callbacks.on_pan_start,
                update: &callbacks.on_pan_update,
                end: &callbacks.on_pan_end,
                has_cancel: callbacks.on_pan_cancel.is_some(),
            },
        }
    }

    fn has_recognizer(&self) -> bool {
        self.down.is_some()
            || self.start.is_some()
            || self.update.is_some()
            || self.end.is_some()
            || self.has_cancel
    }

    fn bind(
        &self,
        start: DragStartDetails,
        update: DragUpdateDetails,
        end: DragEndDetails,
    ) -> impl Fn() + 'static + use<> {
        let (on_down, on_start, on_update, on_end) = (
            self.down.clone(),
            self.start.clone(),
            self.update.clone(),
            self.end.clone(),
        );
        move || {
            if let Some(callback) = &on_down {
                callback(incular_gestures::DragDownDetails {
                    global_position: start.global_position,
                    local_position: start.local_position,
                });
            }
            if let Some(callback) = &on_start {
                callback(start);
            }
            if let Some(callback) = &on_update {
                callback(update);
            }
            if let Some(callback) = &on_end {
                callback(end);
            }
        }
    }
}

fn semantic_drag_axis(action: SemanticActionKind) -> Option<(Axis, f32)> {
    match action {
        SemanticActionKind::ScrollLeft => Some((Axis::Horizontal, -1.)),
        SemanticActionKind::ScrollRight => Some((Axis::Horizontal, 1.)),
        SemanticActionKind::ScrollUp => Some((Axis::Vertical, -1.)),
        SemanticActionKind::ScrollDown => Some((Axis::Vertical, 1.)),
        _ => None,
    }
}

fn gesture_supports(
    callbacks: &GestureCallbacks,
    policy: Option<GestureSemanticPolicy>,
    action: SemanticActionKind,
) -> bool {
    policy.unwrap_or_default().allows(action)
        && match action {
            SemanticActionKind::Activate => callbacks.on_tap.is_some(),
            SemanticActionKind::LongPress => callbacks.on_long_press.is_some(),
            _ => semantic_drag_axis(action).is_some_and(|(axis, _)| {
                DragSemanticPhases::for_axis(callbacks, Some(axis)).has_recognizer()
                    || DragSemanticPhases::for_axis(callbacks, None).has_recognizer()
            }),
        }
}

fn gesture_can_execute(
    kind: &WidgetKind,
    policy: Option<GestureSemanticPolicy>,
    action: SemanticActionKind,
) -> bool {
    matches!(kind, WidgetKind::Gesture { callbacks, .. } if gesture_supports(callbacks, policy, action))
}

/// Looks up a semantic gesture against the live descriptor, even if a stale
/// accessibility projection is still showing a formerly available direction.
impl WidgetTree {
    pub(super) fn gesture_semantic_callback(
        &self,
        element: &Element,
        action: SemanticActionKind,
    ) -> Option<Rc<dyn Fn() + 'static>> {
        let WidgetKind::Gesture { callbacks, .. } = element.widget.kind() else {
            return None;
        };
        let policy = element
            .widget
            .semantic_properties()
            .callbacks
            .gesture_policy;
        if !gesture_supports(callbacks, policy, action) {
            return None;
        }
        match action {
            SemanticActionKind::Activate => callbacks.on_tap.clone(),
            SemanticActionKind::LongPress => callbacks.on_long_press.clone(),
            _ => {
                let (axis, sign) = semantic_drag_axis(action)?;
                let size = self.renders.get(element.render.0)?.size;
                let primary_delta =
                    axis.main_extent(size) * policy.unwrap_or_default().scroll_factor * sign;
                let delta = axis.offset(primary_delta, 0.);
                let world = self.render_world_transform(element.render);
                let local_position = Offset::new(size.width / 2., size.height / 2.);
                let global_position = world.transform_point(local_position);
                let start = DragStartDetails {
                    source_time: None,
                    local_position,
                    global_position,
                    kind: incular_core::PointerDeviceKind::Unknown,
                };
                let update = DragUpdateDetails {
                    source_time: None,
                    delta,
                    primary_delta: Some(primary_delta),
                    global_position,
                    // Flutter creates this update with only globalPosition.
                    local_position: global_position,
                    kind: incular_core::PointerDeviceKind::Unknown,
                };
                let local_end = local_position + delta;
                let end = DragEndDetails {
                    velocity: incular_gestures::Velocity::ZERO,
                    primary_velocity: Some(0.),
                    local_position: local_end,
                    global_position: world.transform_point(local_end),
                };
                let directional =
                    DragSemanticPhases::for_axis(callbacks, Some(axis)).bind(start, update, end);
                let pan = DragSemanticPhases::for_axis(callbacks, None).bind(
                    start,
                    update,
                    DragEndDetails {
                        primary_velocity: None,
                        ..end
                    },
                );
                Some(Rc::new(move || {
                    // Flutter dispatches the axis recognizer before the pan.
                    directional();
                    pan();
                }))
            }
        }
    }
}

/// Overlays the fields `overlay` sets (differs from the default) onto `base`.
/// Used when a role-less Semantics wrapper annotates a widget with its own role.
fn overlay_semantic_state(base: &mut SemanticState, overlay: &SemanticState) {
    let default = SemanticState::default();
    macro_rules! take {
        ($($field:ident),* $(,)?) => {
            $(if overlay.$field != default.$field { base.$field = overlay.$field.clone(); })*
        };
    }
    take!(
        enabled,
        focused,
        focusable,
        selected,
        checked,
        expanded,
        read_only,
        editable,
        obscured,
        multiline,
        selection,
        item_index,
        set_size,
        busy,
        invalid,
        required,
        numeric_value,
        numeric_min,
        numeric_max,
        numeric_step,
        heading_level,
        toggled,
        hidden,
        live_region,
        text_direction,
        text_attributes,
        tooltip,
        link_url,
        identifier,
        increased_value,
        decreased_value,
    );
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
        || matches!(
            entry.widget.kind(),
            WidgetKind::Opacity { alpha, controller, .. }
                if !entry.widget.semantic_properties().always_include_semantics
                    && opacity_is_transparent(controller.as_ref().map_or(*alpha, OpacityController::opacity))
        )
}

/// Flutter's `RenderOpacity` drops the child's semantics only when its alpha
/// rounds to zero on the 0..=255 scale.
fn opacity_is_transparent(alpha: f32) -> bool {
    (alpha * 255.0).round() == 0.0
}

/// The sort key a widget's explicit semantics declares, if any.
fn widget_sort_key(widget: &Widget) -> Option<&incular_semantics::OrdinalSortKey> {
    widget
        .semantic_properties()
        .explicit
        .as_ref()
        .and_then(|explicit| explicit.state.sort_key.as_ref())
}

/// Flutter's `SemanticsNode` sort-group compatibility: unkeyed keys agree
/// with each other, and keyed ones agree when they share a group name.
fn sort_keys_compatible(
    left: Option<&incular_semantics::OrdinalSortKey>,
    right: Option<&incular_semantics::OrdinalSortKey>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.name == right.name,
        _ => false,
    }
}

fn explicit_focus_block(entry: &Element) -> AccessibilityFocusBlockType {
    entry
        .widget
        .semantic_properties()
        .explicit
        .as_ref()
        .map_or(AccessibilityFocusBlockType::None, |explicit| {
            explicit.state.accessibility_focus_block_type
        })
}

/// Whether a merging element keeps its children as separate nodes
/// (Flutter's `explicitChildNodes`).
fn keeps_explicit_child_nodes(entry: &Element) -> bool {
    entry
        .widget
        .semantic_properties()
        .explicit
        .as_ref()
        .is_some_and(|explicit| explicit.state.explicit_child_nodes)
}

fn absorbs_descendant_semantics(entry: &Element) -> bool {
    entry.widget.semantic_properties().merge_descendants && !keeps_explicit_child_nodes(entry)
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
        SemanticActionKind::Focus | SemanticActionKind::Custom => false,
        // Advertised only inside a viewport; the dispatch reports whether a
        // viewport actually moved.
        SemanticActionKind::ShowOnScreen => true,
        SemanticActionKind::Activate => {
            matches!(
                kind,
                WidgetKind::Button(spec)
                    if spec.enabled && (spec.has_callback || spec.action != ActionId(0))
            ) || gesture_can_execute(kind, callbacks.gesture_policy, action)
        }
        SemanticActionKind::LongPress
        | SemanticActionKind::ScrollLeft
        | SemanticActionKind::ScrollRight
        | SemanticActionKind::ScrollUp
        | SemanticActionKind::ScrollDown => {
            gesture_can_execute(kind, callbacks.gesture_policy, action)
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
