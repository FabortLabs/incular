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
    SemanticAnnotationFlags, SemanticValidationResult, SemanticsHitTestBehavior, SemanticsInputType,
    StringAttribute,
};
use std::{borrow::Cow, rc::Rc};

pub(super) mod paragraph;
pub(super) mod table;
mod traversal;
mod viewport;
use traversal::TraversalItem;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SemanticCollectionContext {
    item_index: Option<usize>,
    set_size: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SemanticTextMerge {
    #[default]
    None,
    CompatibleLabels,
    CompatibleControls,
    CompatibleActions,
    HintAndValue {
        take_value: bool,
    },
}

#[derive(Default)]
struct MergedSemanticFields {
    hint: String,
    hint_attributes: Vec<StringAttribute>,
    value: Option<String>,
    value_attributes: Vec<StringAttribute>,
    direction: Option<TextDirection>,
}

#[derive(Default)]
struct MergedSemanticSummary {
    label: String,
    label_attributes: Vec<StringAttribute>,
    direction: Option<TextDirection>,
    heading: Option<u8>,
    validation: SemanticValidationResult,
    annotation_flags: SemanticAnnotationFlags,
    input_type: SemanticsInputType,
    controls_nodes: Option<Arc<[String]>>,
    focus_block: AccessibilityFocusBlockType,
    custom_actions: Vec<CustomSemanticsActionId>,
    has_custom_callback: bool,
    standard_actions: u16,
    enabled: Option<bool>,
    hit_test: SemanticsHitTestBehavior,
    tags: Vec<incular_semantics::SemanticsTag>,
}

// Stable bit positions for explicitly supplied, argumentless widget callbacks.
const MERGEABLE_SEMANTIC_ACTIONS: [SemanticActionKind; 11] = [
    SemanticActionKind::Activate,
    SemanticActionKind::LongPress,
    SemanticActionKind::Dismiss,
    SemanticActionKind::Increment,
    SemanticActionKind::Decrement,
    SemanticActionKind::ScrollForward,
    SemanticActionKind::ScrollBackward,
    SemanticActionKind::ScrollLeft,
    SemanticActionKind::ScrollRight,
    SemanticActionKind::ScrollUp,
    SemanticActionKind::ScrollDown,
];

impl MergedSemanticSummary {
    fn add_label(
        &mut self,
        label: &str,
        direction: Option<TextDirection>,
        attributes: &[StringAttribute],
    ) {
        append_semantic_text(
            &mut self.label,
            &mut self.label_attributes,
            &mut self.direction,
            label,
            direction,
            attributes,
        );
    }

    fn add_tags(&mut self, tags: &[incular_semantics::SemanticsTag]) {
        for tag in tags {
            if !self.tags.contains(tag) {
                self.tags.push(tag.clone());
            }
        }
    }

    fn add_custom_actions(&mut self, callbacks: &SemanticCallbacks) {
        for binding in &callbacks.custom {
            self.custom_actions.push(binding.action.get_identifier());
            self.has_custom_callback |= binding.callback.is_some();
        }
    }
}

/// Paint and cache bounds of the owning sliver viewport, in world space.
fn append_semantic_text(
    output: &mut String,
    output_attributes: &mut Vec<StringAttribute>,
    output_direction: &mut Option<TextDirection>,
    text: &str,
    direction: Option<TextDirection>,
    attributes: &[StringAttribute],
) {
    if output_direction.is_none() {
        *output_direction = direction;
    }
    if text.is_empty() {
        return;
    }
    if !output.is_empty() {
        output.push('\n');
    }
    let embedded = direction.filter(|direction| Some(*direction) != *output_direction);
    if let Some(direction) = embedded {
        output.push(match direction {
            TextDirection::Ltr => '\u{202a}',
            TextDirection::Rtl => '\u{202b}',
        });
    }
    if !attributes.is_empty() {
        let offset = output.encode_utf16().count();
        output_attributes.extend(attributes.iter().map(|attribute| {
            let range = attribute.range();
            attribute.with_range((offset + range.start)..(offset + range.end))
        }));
    }
    output.push_str(text);
    if embedded.is_some() {
        output.push('\u{202c}');
    }
}

#[derive(Clone, Copy)]
struct SemanticSliverWindow {
    visible: Option<Rect>,
    cache: Rect,
}

struct ChildTagScope {
    tag: incular_semantics::SemanticsTag,
    outer: Option<Rc<Self>>,
}

impl ChildTagScope {
    fn extend(
        outer: Option<Rc<Self>>,
        tag: Option<&incular_semantics::SemanticsTag>,
    ) -> Option<Rc<Self>> {
        match tag {
            Some(tag) => Some(Rc::new(Self {
                tag: tag.clone(),
                outer,
            })),
            None => outer,
        }
    }

    fn for_children(
        outer: Option<Rc<Self>>,
        entry: &Element,
        emitted_node: bool,
    ) -> Option<Rc<Self>> {
        let viewport_tag = matches!(entry.widget.kind(), WidgetKind::SliverViewport { .. })
            .then(crate::Viewport::use_two_pane_semantics_tag);
        let outer = Self::extend(outer, viewport_tag);
        let outer = Self::extend(
            outer,
            (!emitted_node && entry.widget.semantic_properties().sliver_scroll_excluded)
                .then(crate::Viewport::exclude_from_scrolling_tag),
        );
        Self::extend(
            outer,
            entry.widget.semantic_properties().tag_for_children.as_ref(),
        )
    }

    fn apply(&self, state: &mut SemanticState) {
        let mut stack = Vec::new();
        let mut scope = Some(self);
        while let Some(current) = scope {
            stack.push(&current.tag);
            scope = current.outer.as_deref();
        }
        let mut tags = std::mem::take(&mut state.tags).into_vec();
        for tag in stack.into_iter().rev() {
            if !tags.contains(tag) {
                tags.push(tag.clone());
            }
        }
        state.tags = tags.into_boxed_slice();
    }
}

/// One element waiting to be visited by the semantics pass.
#[derive(Clone)]
pub(super) struct SemanticVisit {
    element: ElementId,
    /// Index in `builds` of the nearest collected semantic ancestor, or
    /// `TOP_LEVEL`.
    parent: u32,
    collection: Option<SemanticCollectionContext>,
    /// Most recent locale annotation on the traversal path.
    locale_scope: Option<ElementId>,
    inherited_tags: Option<Rc<ChildTagScope>>,
    /// Collect the full content of every element below, whatever its marks
    /// say.
    force_content: bool,
    /// Every bound below may have moved, although content is unchanged.
    force_bounds: bool,
    /// The visual ancestor set an accessibility subtree block.
    subtree_focus_blocked: bool,
    /// Compatible text fragments may join an established semantic node.
    parent_text_merge: SemanticTextMerge,
    /// Plain text belongs to an enclosing Focus annotation until a
    /// descendant independent semantics boundary starts its own group.
    merges_focus_labels: bool,
    /// The parent's world transform followed by its child content
    /// transform; this element's own offset is not applied yet.
    parent_world: CoreTransform,
    /// Visibility clip inherited from ancestor slivers. Measured bounds
    /// and actions remain intact for cached, hidden semantic nodes.
    viewport_clip: Option<Rect>,
    /// Approximate accessibility geometry from ordinary paint clips
    /// and sliver clippers. Unlike the scrolling viewport's visibility
    /// clip, it constrains the projected node's actual bounds.
    semantic_bounds_clip: Option<Rect>,
    /// Window to apply to the immediate children of a sliver viewport.
    sliver_window: Option<SemanticSliverWindow>,
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

#[derive(Default)]
struct SyntheticSemanticChanges {
    pub(super) kept: Vec<SemanticNodeId>,
    pub(super) dropped: Vec<SemanticNodeId>,
    pub(super) detached: Vec<SemanticNodeId>,
}

impl SyntheticSemanticChanges {
    pub(super) fn clear(&mut self) {
        self.kept.clear();
        self.dropped.clear();
        self.detached.clear();
    }
}

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
    /// Synthetic nodes created by tables and rich paragraph ranges.
    synthetic_changes: SyntheticSemanticChanges,
    /// Pane-relevant state changes from the current collection pass.
    changed_viewport_children: FastHashMap<SemanticNodeId, viewport::PaneChild>,
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
            for (_, pane) in self.viewport_semantic_panes.drain() {
                let _ = self.semantics.remove(pane);
            }
            for (_, wrappers) in self.table_semantic_wrappers.drain() {
                for id in wrappers.into_ids() {
                    let _ = self.semantics.remove(id);
                }
            }
            for (_, parts) in self.paragraph_semantic_nodes.drain() {
                for id in parts {
                    let _ = self.semantics.remove(id);
                }
            }
            self.semantics.set_root(None);
            self.semantic_point_order.clear();
            // Marks recorded while the graph is released are not kept up to
            // date, so the next pass collects everything.
            self.semantics_full_pass = true;
        }
    }

    #[must_use]
    pub const fn semantics_enabled(&self) -> bool {
        self.semantics_enabled
    }

    fn geometry_stamp(&self) -> (u64, u64, u64) {
        (
            self.diagnostics.layouts,
            self.diagnostics.compositor_only_updates,
            self.diagnostics.scroll_offset_updates,
        )
    }

    /// Publishes leader transforms before hover, focus, or semantics use
    /// the current frame's follower geometry. Share the pre-paint pass.
    #[doc(hidden)]
    pub fn refresh_follower_links_for_geometry(&mut self) {
        if self.tracked.followers.is_empty() {
            return;
        }
        let stamp = self.geometry_stamp();
        if self.follower_link_geometry_stamp != Some(stamp) {
            self.compositor.publish_leader_links();
            self.follower_link_geometry_stamp = Some(stamp);
        }
    }

    pub fn update_semantics(&mut self) {
        if !self.semantics_enabled && self.tracked.semantics_debuggers.is_empty() {
            return;
        }
        self.flush_sliver_reclips();
        self.flush_focus_semantics_changes();
        let stamp = self.geometry_stamp();
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
                self.refresh_follower_links_for_geometry();
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
            self.semantic_point_order.clear();
            return;
        };
        if !full
            && !self
                .render_live(root_render, "semantic root render must remain live")
                .subtree_needs_semantics
        {
            return;
        }
        self.refresh_semantics_block_reach();
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
            locale_scope: None,
            inherited_tags: None,
            force_content: full,
            force_bounds: full,
            subtree_focus_blocked: false,
            parent_text_merge: SemanticTextMerge::None,
            merges_focus_labels: false,
            parent_world: CoreTransform::IDENTITY,
            viewport_clip: None,
            semantic_bounds_clip: None,
            sliver_window: None,
        });
        // Set when a node below a retained child list turns out to have
        // appeared or disappeared. Marks never announce that (see
        // `SemanticsDirty::is_content_scope`), so it is only a guard: the
        // graph is then collected whole instead of being left inconsistent.
        let mut inconsistent = false;
        while let Some(visit) = visits.pop() {
            let entry =
                self.element_live(visit.element, "semantic traversal element must remain live");
            let block = effective_focus_block(entry, visit.subtree_focus_blocked);
            let can_own_hints = can_own_hint_value(entry, block);
            let can_own_controls = can_own_control_relations(entry);
            let can_own_sliver_labels = self.sliver_label_annotation_candidate(visit.element, entry);
            let can_own_validation = can_own_validation_labels(entry);
            let merges_child_fragments = can_own_hints
                || can_own_controls
                || can_own_sliver_labels
                || can_own_validation
                || can_own_blocked_labels(entry, block)
                || super::focus::annotated_focus_node(entry.widget.kind()).is_some();
            let render = entry.render;
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
                content: visit.force_content
                    || dirty.any_content()
                    || merges_child_fragments && dirty.has_descendant_content(),
                subtree_content: visit.force_content
                    || dirty.subtree_content()
                    // A value becoming conflicting affects clean siblings as well.
                    || (can_own_hints
                        || can_own_sliver_labels
                        || can_own_controls
                        || can_own_validation)
                        && dirty.has_descendant_content(),
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
            if self.semantics_hidden(entry) {
                continue;
            }
            stack.extend(self.semantic_children(element, entry).iter().rev());
        }
    }

    fn refresh_semantics_block_reach(&mut self) {
        if self.tracked.sibling_blockers.is_empty() && self.semantics_block_reach.is_empty() {
            return;
        }
        let mut next = FastHashSet::default();
        for &blocker in &self.tracked.sibling_blockers {
            let mut current = blocker;
            while let Some(entry) = self.elements.get(current.0) {
                if !self.can_contribute_merged_semantics(entry) || !next.insert(current) {
                    break;
                }
                let Some(parent) = entry.parent else {
                    break;
                };
                let Some(parent_entry) = self.elements.get(parent.0) else {
                    break;
                };
                let props = parent_entry.widget.semantic_properties();
                // The block still applies to siblings immediately inside
                // this container. It must not suppress siblings outside it.
                if props
                    .explicit
                    .as_ref()
                    .is_some_and(|explicit| explicit.state.container)
                    || absorbs_descendant_semantics(parent_entry)
                    || self.semantics_hidden(parent_entry)
                    || self.fitted_children_have_no_semantic_geometry(parent_entry)
                    || matches!(parent_entry.widget.kind(), WidgetKind::Button(_) | WidgetKind::Table { .. })
                    || matches!(parent_entry.widget.kind(), WidgetKind::IndexedStack { index, .. }
                        if index.and_then(|slot| parent_entry.children.get(slot)) != Some(&current))
                {
                    break;
                }
                current = parent;
            }
        }
        let previous = std::mem::replace(&mut self.semantics_block_reach, next);
        let changed_parents = previous
            .symmetric_difference(&self.semantics_block_reach)
            .filter_map(|child| self.elements.get(child.0)?.parent)
            .filter_map(|parent| self.elements.get(parent.0).map(|entry| entry.render))
            .collect::<FastHashSet<_>>();
        for parent in changed_parents {
            // For example, a visibility change nested in an ordinary padding
            // can change the accessible child list of its outer parent.
            self.mark_render_semantics(parent, SemanticsDirty::Subtree);
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
        if let Some(visible) = self
            .render_live(entry.render, "semantic render remains live")
            .inline_visible_children()
        {
            let mut filtered: Vec<_> = entry
                .children
                .iter()
                .copied()
                .filter(|child| {
                    self.elements
                        .get(child.0)
                        .is_some_and(|entry| visible.contains(&entry.render))
                })
                .collect();
            let start = filtered
                .iter()
                .rposition(|child| self.semantics_block_reach.contains(child))
                .unwrap_or(0);
            filtered.drain(..start);
            return Cow::Owned(filtered);
        }
        if self.semantics_block_reach.is_empty() {
            return Cow::Borrowed(&entry.children);
        }
        let first_visible_child = entry
            .children
            .iter()
            .rposition(|child| self.semantics_block_reach.contains(child))
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
            synthetic_changes,
            changed_viewport_children,
            detached,
            ..
        } = scratch;
        // Allocate stable IDs for every new node before wiring any edges, so
        // a parent's child list can name children collected after it. Native
        // accessibility must receive a coherent tree on the same frame a
        // transient opens, not one semantic pass later.
        // A viewport parent is assembled before changed descendants.
        // Snapshot their pane-relevant metadata from this frame's builds.
        let exclude_tag = crate::Viewport::exclude_from_scrolling_tag();
        changed_viewport_children.clear();
        #[cfg(feature = "devtools")]
        let mut inserted = HashSet::new();
        for build in builds.iter() {
            if let Some(&id) = self.semantic_ids.get(&build.element())
                && let Some(old) = self.semantics.node(id)
            {
                let before = viewport::PaneChild::from_state(&old.state, exclude_tag);
                let now = match build {
                    Collected::Full(build) => {
                        viewport::PaneChild::from_state(&build.state, exclude_tag)
                    }
                    Collected::Bounds { hidden: Some(hidden), .. } => viewport::PaneChild {
                        visible_index: if *hidden {
                            None
                        } else {
                            old.state.item_index
                        },
                        ..before
                    },
                    _ => before,
                };
                if now != before {
                    changed_viewport_children.insert(id, now);
                }
                continue;
            }
            let Collected::Full(build) = build else { continue };
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
        synthetic_changes.clear();
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
                let pane = self.viewport_semantic_panes.get(&element);
                detached.extend(
                    old.children
                        .iter()
                        .filter(|child| Some(*child) != pane && !is_attached(attached, **child)),
                );
            }
            let is_table = match &build {
                Collected::Full(build) => build.role == SemanticRole::Table,
                Collected::Bounds { .. } => self.semantics.node(id).is_some_and(|node| node.role == SemanticRole::Table),
            };
            let table_widget = matches!(
                self.element_live(element, "semantic element remains live").widget.kind(),
                WidgetKind::Table { .. }
            );
            if table_widget {
                if is_table {
                    self.assemble_semantic_table(
                        element,
                        &edges[start..cursor],
                        child_ids,
                        synthetic_changes,
                    );
                } else {
                    // An explicit role can replace the widget's table role.
                    // Its previous synthetic nodes leave via the same
                    // detached child sweep as all other old nodes.
                    self.table_semantic_wrappers.remove(&element);
                }
            }

            if matches!(
                self.element_live(element, "semantic element remains live").widget.kind(),
                WidgetKind::Text { .. }
            ) {
                self.assemble_paragraph_semantics(
                    element, id, &edges[start..cursor], child_ids, synthetic_changes,
                );
            }

            #[cfg(feature = "devtools")]
            let semantic_revision_before = self.semantics.revision();
            if matches!(
                self.element_live(element, "semantic element remains live").widget.kind(),
                WidgetKind::SliverViewport { .. } | WidgetKind::Scroll { .. }
            ) {
                self.assemble_viewport_semantic_panes(
                    element,
                    id,
                    build,
                    (child_ids, &edges[start..cursor]),
                    changed_viewport_children,
                    detached,
                );
            } else {
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
            }
            self.remember_semantic_point_order(Some(id), &edges[start..cursor]);
            if let Some(pane) = self.viewport_semantic_panes.get(&element).copied() {
                self.remember_semantic_point_order(Some(pane), &edges[start..cursor]);
            }
            #[cfg(feature = "devtools")]
            if (inserted.contains(&element)
                || self.semantics.revision() != semantic_revision_before)
                && let Some(element) = self.elements.get_mut(element.0)
            {
                element.dev.semantic_updates = element.dev.semantic_updates.saturating_add(1);
            }
        }
        // Synthetic table wrappers and paragraph ranges have no element edges.
        // Keep the IDs they reused out of the detached-subtree sweep.
        if !synthetic_changes.dropped.is_empty() {
            synthetic_changes.dropped.sort_unstable_by_key(|id| id.0);
            attached.retain(|id| !is_attached(&synthetic_changes.dropped, *id));
            detached.append(&mut synthetic_changes.dropped);
        }
        if !synthetic_changes.kept.is_empty() {
            attached.append(&mut synthetic_changes.kept);
            attached.sort_unstable_by_key(|id| id.0);
        }
        detached.append(&mut synthetic_changes.detached);
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
        self.remember_semantic_point_order(None, top_level);
        // A detached node takes its semantic subtree with it, except for any
        // node this pass attached somewhere else.
        let mut removed_any = false;
        while let Some(id) = detached.pop() {
            if is_attached(attached, id) {
                continue;
            }
            if let Some(node) = self.semantics.remove(id) {
                self.semantic_point_order.remove(&Some(id));
                removed_any = true;
                detached.extend(node.children);
            }
        }
        if removed_any {
            let semantics = &self.semantics;
            self.semantic_ids
                .retain(|_, node| semantics.node(*node).is_some());
            self.table_semantic_wrappers
                .retain(|element, _| self.semantic_ids.contains_key(element));
            self.paragraph_semantic_nodes
                .retain(|element, _| self.semantic_ids.contains_key(element));
            self.viewport_semantic_panes.retain(|element, pane| {
                self.semantic_ids.contains_key(element) && semantics.node(*pane).is_some()
            });
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
            locale_scope,
            inherited_tags,
            parent_world,
            subtree_focus_blocked,
            parent_text_merge,
            merges_focus_labels,
            mut viewport_clip,
            mut semantic_bounds_clip,
            mut sliver_window,
            ..
        } = visit;
        let _node_guard = self.guard_element(FramePhase::Semantics, element);
        let entry = self.element_live(element, "semantic traversal element must remain live");
        if self.semantics_hidden(entry) {
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
        if let Some(clips) = &render.parent_sliver_layers {
            for placement in clips.iter() {
                if placement.hides_semantics() {
                    return;
                }
                let Some(rect) = placement.approximate_clip() else {
                    continue;
                };
                let clip = project_semantic_rect(world, rect);
                semantic_bounds_clip = Some(semantic_bounds_clip.map_or(clip, |outer| {
                    outer.intersection(clip).unwrap_or_default()
                }));
            }
        }
        let direct_sliver = entry
            .parent
            .and_then(|parent| self.elements.get(parent.0))
            .filter(|parent| matches!(parent.widget.kind(), WidgetKind::SliverViewport { .. }))
            .and_then(|parent| {
                parent
                    .children
                    .iter()
                    .position(|child| *child == element)
                    .map(|slot| (parent, slot))
            });
        let cached_bounds = (sliver_window.is_some() || semantic_bounds_clip.is_some())
            .then(|| self.semantic_bounds(render_id, world, child_world));
        let mut outside_semantic_scope = false;
        if let Some(mut window) = sliver_window {
            let layout_box =
                project_semantic_rect(world, Rect::from_origin_size(Offset::ZERO, render.size));
            let in_window = semantic_clip_overlaps(window.cache, layout_box);
            if !in_window && entry.widget.semantic_properties().ensures_sliver_semantics {
                // An explicitly ensured off-cache sliver must keep its
                // descendants, not just its direct wrapper.
                sliver_window = None;
            } else {
                if !in_window && direct_sliver.is_some() {
                    // The direct sliver itself has no viewport/cache coverage.
                    return;
                }
                if window.visible.is_some()
                    && let Some((parent, slot)) = direct_sliver
                    && let Some(clip) = self.sliver_semantic_paint_clip(parent, slot, parent_world)
                {
                    window.visible = Some(clip);
                    sliver_window = Some(window);
                }
                if let Some(visible) = window.visible
                    && viewport_clip != Some(visible)
                {
                    viewport_clip = Some(viewport_clip.map_or(visible, |ancestor| {
                        ancestor.intersection(visible).unwrap_or_default()
                    }));
                }
                // For boxes below the sliver, cull the projected node, not
                // the branch. A later transform can move a descendant back
                // into the cache even when this wrapper's box is outside it.
                outside_semantic_scope = cached_bounds
                    .is_some_and(|bounds| !semantic_clip_overlaps(window.cache, bounds));
            }
        }
        outside_semantic_scope |= semantic_bounds_clip.is_some_and(|clip| {
            cached_bounds.is_some_and(|bounds| !semantic_clip_overlaps(clip, bounds))
        });
        let semantic_bounds = || {
            let bounds = cached_bounds
                .unwrap_or_else(|| self.semantic_bounds(render_id, world, child_world));
            semantic_bounds_clip.map_or(bounds, |clip| {
                clip.intersection(bounds).unwrap_or_default()
            })
        };
        let child_bounds_clip = if let Some(local_clip) = render
            .object
            .layers
            .approximate_box_paint_clip(render.object.kind(), render.size)
        {
            let clip = project_semantic_rect(world, local_clip);
            Some(semantic_bounds_clip.map_or(clip, |ancestor| {
                ancestor.intersection(clip).unwrap_or_default()
            }))
        } else {
            semantic_bounds_clip
        };
        let child_sliver_window = match render.object.kind() {
            RenderKind::SliverViewport { config } => {
                let visible = (config.clip_behavior != incular_config::Clip::None).then(|| {
                    project_semantic_rect(world, Rect::from_origin_size(Offset::ZERO, render.size))
                });
                let cache = project_semantic_rect(world, sliver_semantics_cache_clip(
                    config.axis,
                    render.size,
                    config.cache_extent,
                ));
                let cache = sliver_window.map_or(cache, |outer| {
                    outer.cache.intersection(cache).unwrap_or_default()
                });
                Some(SemanticSliverWindow { visible, cache })
            }
            // A transformed descendant can leave the cache even while
            // its sliver's own layout box remains inside. Evaluate the
            // same world-space window throughout the subtree.
            _ => sliver_window,
        };
        let child_viewport_clip = match render.object.kind() {
            RenderKind::Scroll { clip_behavior, .. }
                if *clip_behavior != incular_config::Clip::None =>
            {
                let bounds =
                    project_semantic_rect(world, Rect::from_origin_size(Offset::ZERO, render.size));
                Some(viewport_clip.map_or(bounds, |outer| {
                    outer.intersection(bounds).unwrap_or_default()
                }))
            }
            _ => viewport_clip,
        };
        let locale_scope = entry
            .widget
            .semantic_properties()
            .locale_for_subtree
            .as_ref()
            .map(|_| element)
            .or(locale_scope);
        let own_focus_block = effective_focus_block(entry, subtree_focus_blocked);
        let child_focus_blocked = own_focus_block == AccessibilityFocusBlockType::BlockSubtree;
        let retained_id = self.semantic_ids.get(&element).copied();
        let joins_text_merge = parent_text_merge != SemanticTextMerge::None
            && self.can_merge_text_fragment(element, entry, parent_text_merge);
        let action_fragments = if joins_text_merge || outside_semantic_scope {
            None
        } else {
            self.compatible_action_fragments(element, entry)
        };
        let own_text_merge = if joins_text_merge || outside_semantic_scope {
            SemanticTextMerge::None
        } else if action_fragments.is_some() {
            SemanticTextMerge::CompatibleActions
        } else if can_own_blocked_labels(entry, own_focus_block)
            || can_own_validation_labels(entry)
            || self.is_sliver_label_annotation_owner(element, entry)
        {
            SemanticTextMerge::CompatibleLabels
        } else if can_own_control_relations(entry) {
            SemanticTextMerge::CompatibleControls
        } else if can_own_hint_value(entry, own_focus_block) {
            let owns_value = entry
                .widget
                .semantic_properties()
                .explicit
                .as_ref()
                .and_then(|data| data.value.as_deref())
                .is_some_and(|value| !value.is_empty());
            let take_value = !owns_value
                && if let Some(retained) = retained_id.filter(|_| !scope.content) {
                    // Geometry alone cannot introduce or remove a value
                    // conflict. The owner already records an absorbed value.
                    self.semantics
                        .node(retained)
                        .and_then(|node| node.value.as_deref())
                        .is_some_and(|value| !value.is_empty())
                } else {
                    self.count_compatible_values(element, entry) < 2
                };
            SemanticTextMerge::HintAndValue { take_value }
        } else {
            SemanticTextMerge::None
        };
        let child_text_merge = if joins_text_merge {
            parent_text_merge
        } else {
            own_text_merge
        };
        let focus_annotation = (!outside_semantic_scope)
            .then(|| super::focus::annotated_focus_node(entry.widget.kind()))
            .flatten();
        let joins_focus_annotation =
            merges_focus_labels && !self.focus_semantics_boundary(element, entry);
        let child_merges_focus_labels = focus_annotation.is_some() || joins_focus_annotation;
        let retained = retained_id.is_some();
        if !scope.content && retained && !outside_semantic_scope {
            let next_tag_scope = ChildTagScope::for_children(None, entry, true);
            // Nothing but geometry changed here or below, so the retained
            // node keeps its role, text, state and actions. A node with a
            // retained id always emitted a node, so its descendants inherit
            // no collection.
            if semantic_parent != RETAINED_LIST {
                edges.push((semantic_parent, edges.len() as u32, element));
            }
            let regroups = self.regroups_sliver_semantics(entry);
            let table = matches!(entry.widget.kind(), WidgetKind::Table { .. });
            let paragraph_parts = entry.widget.semantic_properties().paragraph_parts.is_some();
            let reorder = scope.descendant_bounds
                && self.semantic_reading_direction(element).is_some()
                && self
                    .semantic_ids
                    .get(&element)
                    .and_then(|id| self.semantics.node(*id))
                    .is_some_and(|node| node.children.len() > 1);
            if !scope.own_bounds && !regroups && !reorder && !table && !paragraph_parts {
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
                            locale_scope,
                            inherited_tags: next_tag_scope.clone(),
                            force_content: false,
                            force_bounds: false,
                            subtree_focus_blocked: child_focus_blocked,
                            parent_text_merge: child_text_merge,
                            merges_focus_labels: child_merges_focus_labels,
                            viewport_clip: child_viewport_clip,
                            semantic_bounds_clip: child_bounds_clip,
                            sliver_window: if index < entry.children.len() {
                                child_sliver_window
                            } else {
                                sliver_window
                            },
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
            let bounds = semantic_bounds();
            out.push(Collected::Bounds {
                element,
                bounds,
                hidden: viewport_clip.map(|clip| {
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
                        locale_scope,
                        inherited_tags: next_tag_scope.clone(),
                        force_content: regroups,
                        force_bounds: scope.bounds_below || regroups,
                        subtree_focus_blocked: child_focus_blocked,
                        parent_text_merge: child_text_merge,
                        merges_focus_labels: child_merges_focus_labels,
                        viewport_clip: child_viewport_clip,
                        semantic_bounds_clip: child_bounds_clip,
                        sliver_window: if index < entry.children.len() {
                            child_sliver_window
                        } else {
                            sliver_window
                        },
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
        let explicit_index = match (entry.widget.kind(), entry.widget.key()) {
            (WidgetKind::IndexedSemantics { index, .. }, _) => Some(*index),
            // Grid rows are only a materialization detail. Each retained
            // tile boundary already carries its absolute logical index.
            (_, Some(Key::GridCell(index))) if collection.is_some() => Some(*index),
            _ => None,
        };
        if let Some(index) = explicit_index {
            collection = Some(SemanticCollectionContext {
                item_index: Some(index),
                set_size: collection.and_then(|context| context.set_size),
            });
        } else if collection.is_none()
            && let Some((parent, slot)) = direct_sliver
            && let WidgetKind::SliverViewport { config } = parent.widget.kind()
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
        let properties = entry.widget.semantic_properties();
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
                    has_enabled_state: true,
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
            WidgetKind::Table { .. } => (
                Some(SemanticRole::Table),
                None,
                None,
                SemanticState { container: true, ..SemanticState::default() },
                Vec::new(),
            ),
            WidgetKind::Text { .. } if properties.paragraph_parts.is_some() => (
                Some(SemanticRole::GenericContainer),
                None,
                None,
                SemanticState {
                    container: true,
                    explicit_child_nodes: true,
                    ..SemanticState::default()
                },
                Vec::new(),
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
                let selection = (spec.enabled && spec.keyboard_selection_enabled)
                    .then(|| spec.controller.selection_if_set())
                    .flatten();
                let mut actions = actions_with_room(&[SemanticActionKind::Focus]);
                if focused && spec.enabled && spec.keyboard_selection_enabled {
                    actions.push(SemanticActionKind::SetSelection);
                }
                if let Some(selection) = selection {
                    if selection.extent > 0 {
                        actions.push(SemanticActionKind::MoveCursorBackwardByCharacter);
                        actions.push(SemanticActionKind::MoveCursorBackwardByWord);
                    }
                    if selection.extent < value.text.len() {
                        actions.push(SemanticActionKind::MoveCursorForwardByCharacter);
                        actions.push(SemanticActionKind::MoveCursorForwardByWord);
                    }
                }
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
                    Some(if spec.obscure_text {
                        super::text::with_text_field_display(
                            &spec.controller,
                            "",
                            Some(spec.obscuring_character),
                            str::to_owned,
                        )
                    } else {
                        value.text
                    }),
                    SemanticState {
                        enabled: spec.enabled,
                        focused,
                        focusable: spec.enabled,
                        editable: spec.enabled && !spec.read_only,
                        input_type: match entry.widget.semantic_properties().text_input_type {
                            Some(incular_platform::TextInputType::Phone) => SemanticsInputType::Phone,
                            Some(incular_platform::TextInputType::Email) => SemanticsInputType::Email,
                            Some(incular_platform::TextInputType::Url) => SemanticsInputType::Url,
                            _ => SemanticsInputType::Text,
                        },
                        multiline: spec.multiline,
                        obscured: spec.obscure_text,
                        read_only: spec.read_only,
                        text_direction: match render.object.kind() {
                            RenderKind::TextField(field) => Some(field.text_direction),
                            _ => None,
                        },
                        selection: selection.map(|selection| SemanticTextSelection {
                            base: selection.base,
                            extent: selection.extent,
                        }),
                        ..SemanticState::default()
                    },
                    actions,
                )
            }
            WidgetKind::Scroll { controller, .. } => (
                Some(SemanticRole::ScrollView),
                None,
                None,
                semantic_scroll_state(render.object.kind()),
                scroll_edge_actions(controller, render.object.kind()),
            ),
            WidgetKind::SliverViewport { config } => (
                Some(SemanticRole::ScrollView),
                None,
                None,
                semantic_scroll_state(render.object.kind()),
                scroll_edge_actions(&config.controller, render.object.kind()),
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
        // An action-only annotation also creates a node.
        if role.is_none()
            && (properties.merge_descendants || !properties.callbacks.custom.is_empty())
        {
            role = Some(SemanticRole::GenericContainer);
        }
        let mut merged = MergedSemanticSummary::default();
        merged.add_custom_actions(&properties.callbacks);
        if let Some(node) = focus_annotation {
            for &child in self.visible_semantic_children(element, entry).iter() {
                self.push_focus_label_fragments(child, &mut merged);
            }
            if self.can_request_focus(element) || !merged.label.is_empty() {
                role.get_or_insert(SemanticRole::GenericContainer);
            }
            state.focused = node.has_primary_focus();
        } else if joins_focus_annotation && matches!(entry.widget.kind(), WidgetKind::Text { .. }) {
            role = None;
        }
        let explicitly_merges = absorbs_descendant_semantics(entry);
        if (explicitly_merges || own_text_merge == SemanticTextMerge::CompatibleLabels)
            && focus_annotation.is_none()
        {
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
                if explicitly_merges {
                    self.push_merged_semantics(child, &mut merged);
                } else {
                    self.push_compatible_labels(child, &mut merged);
                }
            }
        }
        if own_text_merge == SemanticTextMerge::CompatibleControls {
            for &child in self.visible_semantic_children(element, entry).iter() {
                self.push_compatible_controls(child, &mut merged.controls_nodes);
            }
        }
        if let Some(fragments) = action_fragments {
            if let Some(own) = element_own_label(entry) {
                merged.add_label(
                    &own,
                    self.semantic_label_direction(element, entry),
                    element_label_attributes(entry),
                );
            }
            for (child, mask) in fragments {
                let source =
                    self.element_live(child, "compatible action fragment must remain live");
                if let Some(label) = element_own_label(source) {
                    merged.add_label(
                        &label,
                        self.semantic_label_direction(child, source),
                        element_label_attributes(source),
                    );
                }
                if !self.pointer_actions_blocked(child) {
                    merged.standard_actions |= mask;
                }
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
                let enabled = explicit.state.enabled_state().or(state.enabled_state());
                state = explicit.state.clone();
                state.set_enabled_state(enabled);
                actions = actions_with_room(&explicit.actions);
            }
        }
        if properties.label.is_none()
            && properties.explicit.as_ref().is_none_or(|explicit| explicit.label.is_none())
            && let Some(attributes) = &properties.implicit_text_attributes
        {
            match &mut state.text_attributes {
                Some(current) => Arc::make_mut(current).label.clone_from(&attributes.label),
                None => state.text_attributes = Some(attributes.clone()),
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
        let mut description = properties.description.clone().or_else(|| {
            properties
                .explicit
                .as_ref()
                .and_then(|explicit| explicit.description.clone())
        });
        state.text_direction = state.text_direction.or(merged.direction).or_else(|| {
            let has_text = default_label.is_some()
                || merged_label.is_some()
                || properties.label.is_some()
                || value.is_some()
                || description.is_some()
                || state.tooltip.is_some()
                || state.increased_value.is_some()
                || state.decreased_value.is_some();
            has_text
                .then(|| self.semantic_label_direction(element, entry))
                .flatten()
        });
        if let SemanticTextMerge::HintAndValue { .. } = own_text_merge {
            let mut fields = MergedSemanticFields {
                direction: state.text_direction,
                ..MergedSemanticFields::default()
            };
            if let Some(hint) = description.as_deref() {
                fields.hint.push_str(hint);
                if let Some(attributes) = &state.text_attributes {
                    fields.hint_attributes.clone_from(&attributes.hint);
                }
            }
            for &child in self.visible_semantic_children(element, entry).iter() {
                self.push_compatible_hint_value(child, own_text_merge, &mut fields);
            }
            if !fields.hint.is_empty() {
                description = Some(fields.hint);
            }
            if let Some(taken) = fields.value {
                value = Some(taken);
                if !fields.value_attributes.is_empty() || state.text_attributes.is_some() {
                    Arc::make_mut(state.text_attributes.get_or_insert_with(Arc::default)).value =
                        fields.value_attributes;
                }
            }
            if !fields.hint_attributes.is_empty() || state.text_attributes.is_some() {
                Arc::make_mut(state.text_attributes.get_or_insert_with(Arc::default)).hint =
                    fields.hint_attributes;
            }
            state.text_direction = fields.direction;
        }
        // Flutter absorption preserves the parent's level if present.
        // Otherwise the first descendant with a heading supplies it.
        state.set_enabled_state(merge_enabled_state(state.enabled_state(), merged.enabled));
        if state.hit_test_behavior == SemanticsHitTestBehavior::Defer {
            state.hit_test_behavior = merged.hit_test;
        }
        if !merged.tags.is_empty() {
            let mut tags = std::mem::take(&mut state.tags).into_vec();
            for tag in merged.tags {
                if !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
            state.tags = tags.into_boxed_slice();
        }
        let mut metadata = incular_semantics::SemanticAnnotationMetadata {
            heading_level: state.heading_level,
            validation_result: state.validation_result,
        };
        metadata.absorb(incular_semantics::SemanticAnnotationMetadata {
            heading_level: merged.heading,
            validation_result: merged.validation,
        });
        state.heading_level = metadata.heading_level;
        state.validation_result = metadata.validation_result;
        state.annotation_flags = state.annotation_flags.merge(merged.annotation_flags);
        if state.input_type == SemanticsInputType::None {
            state.input_type = merged.input_type;
        }
        if let Some(controls) = &merged.controls_nodes {
            SemanticState::absorb_controls_nodes(&mut state.controls_nodes, controls);
        }
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
            SemanticActionKind::SetText,
            SemanticActionKind::SetSelection,
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
        // Flutter's owner can reveal a node through the renderer even
        // when ShowOnScreen is absent from its explicit action list.
        state.implicit_show_on_screen = role.is_some() && self.has_scroll_ancestor(element);
        let focus_blocked = state.accessibility_focus_block_type.blocks_focus();
        let keyboard_target = !focus_blocked && self.can_request_focus(element);
        let declared_focusable = state.declared_input_focusable();
        state.focusable = !focus_blocked
            && declared_focusable.unwrap_or(
                keyboard_target || state.focusable || (focus_annotation.is_none() && state.focused),
            );
        if let Some(focused) = state.declared_focused {
            state.focused = focused;
        }
        if !state.focusable
            || (focus_annotation.is_some() && !keyboard_target && declared_focusable.is_none())
        {
            // The independent retained keyboard focus is not changed.
            state.focused = false;
        }
        let bound_focus = !focus_blocked
            && entry.widget.semantic_properties().callbacks.supports(SemanticActionKind::Focus);
        let focus_action = bound_focus
            || (keyboard_target
                && declared_focusable != Some(false)
                && (focus_annotation.is_none() || self.focus_semantic_actions_enabled));
        let has_user_actions = focus_action
            || state.implicit_show_on_screen
            || !actions.is_empty()
            || merged.standard_actions != 0
            || !merged.custom_actions.is_empty()
            || absorbs_descendant_semantics(entry);
        if has_user_actions && self.pointer_actions_blocked(element) {
            // Flutter's blocking configuration masks user actions, not the
            // node or its focusable/focused flags. Accessibility focus
            // lifecycle notifications are separate from our user actions.
            actions.retain(|action| {
                action.is_accessibility_focus_notification()
                    && entry.widget.semantic_properties().callbacks.supports(*action)
            });
            state.custom_actions = Box::default();
            state.implicit_show_on_screen = false;
        } else {
            if focus_action && !actions.contains(&SemanticActionKind::Focus) {
                actions.push(SemanticActionKind::Focus);
            }
            actions.retain(|action| {
                if *action == SemanticActionKind::Focus {
                    return focus_action;
                }
                semantic_action_is_executable(
                    entry.widget.kind(),
                    *action,
                    &entry.widget.semantic_properties().callbacks,
                )
            });
            if absorbs_descendant_semantics(entry)
                && !actions.contains(&SemanticActionKind::Activate)
                && self.semantic_activation_target(element).is_some()
            {
                actions.push(SemanticActionKind::Activate);
            }
            for (bit, kind) in MERGEABLE_SEMANTIC_ACTIONS.into_iter().enumerate() {
                if merged.standard_actions & (1 << bit) != 0 && !actions.contains(&kind) {
                    actions.push(kind);
                }
            }
            merged.custom_actions.sort_unstable_by_key(|id| id.0);
            merged.custom_actions.dedup();
            state.custom_actions = merged.custom_actions.into_boxed_slice();
            if merged.has_custom_callback {
                actions.push(SemanticActionKind::Custom);
            }
        }
        if joins_text_merge || outside_semantic_scope {
            role = None;
        }
        if semantic_parent == RETAINED_LIST && role.is_some() != retained {
            // The parent's retained child list no longer describes this
            // node; see `collect_semantics`.
            *inconsistent = true;
            return;
        }
        let this_parent = if let Some(role) = role {
            let mut state = state;
            if let Some(tags) = inherited_tags.as_deref() {
                tags.apply(&mut state);
            }
            if entry.widget.semantic_properties().sliver_scroll_excluded {
                state.add_tag(crate::Viewport::exclude_from_scrolling_tag().clone());
            }
            if let Some(scope) = locale_scope {
                state.locale.clone_from(
                    &self
                        .element_live(scope, "semantic locale ancestor must remain live")
                        .widget
                        .semantic_properties()
                        .locale_for_subtree,
                );
            }
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
            let bounds = semantic_bounds();
            state.hidden |= viewport_clip.is_some_and(|clip| !semantic_clip_overlaps(clip, bounds));
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
                description,
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
        let next_tag_scope = ChildTagScope::for_children(
            if role.is_none() { inherited_tags } else { None },
            entry,
            role.is_some(),
        );
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
                    locale_scope,
                    inherited_tags: next_tag_scope.clone(),
                    force_content: scope.subtree_content || regroups,
                    force_bounds: scope.bounds_below || regroups,
                    subtree_focus_blocked: child_focus_blocked,
                    parent_text_merge: child_text_merge,
                    merges_focus_labels: child_merges_focus_labels,
                    viewport_clip: child_viewport_clip,
                    semantic_bounds_clip: child_bounds_clip,
                    sliver_window: if index < entry.children.len() {
                        child_sliver_window
                    } else {
                        sliver_window
                    },
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
                    && !self.elements.get(parent.0).is_some_and(|entry| {
                        entry.widget.semantic_properties().preserves_semantic_child_order
                    })
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
    fn element_sort_key(&self, element: ElementId) -> Option<TraversalSortKey<'_>> {
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
                    a.compare_to(b).expect("compatible semantic sort run")
                });
            }
            run_start = run_end;
        }
    }

    /// Returns the laid-out sliver viewport's local semantics cache clip.
    /// This includes cached content on both sides of the visible viewport.
    #[must_use]
    pub fn viewport_semantics_cache_clip(&self, element: ElementId) -> Option<Rect> {
        let render = self.renders.get(self.render_id(element)?.0)?;
        let RenderKind::SliverViewport { config } = render.object.kind() else {
            return None;
        };
        if render.dirty.contains(DirtyFlags::LAYOUT) {
            return None;
        }
        Some(sliver_semantics_cache_clip(
            config.axis,
            render.size,
            config.cache_extent,
        ))
    }

    /// The current viewport's physical paint clip for an overlapping sliver,
    /// transformed from the viewport's local space, not the scrolled child's.
    fn sliver_semantic_paint_clip(
        &self,
        viewport: &Element,
        child_slot: usize,
        child_parent_world: CoreTransform,
    ) -> Option<Rect> {
        let render = self.renders.get(viewport.render.0)?;
        let RenderKind::SliverViewport { config } = render.object.kind() else {
            return None;
        };
        let child = *viewport.sliver_child_ids().get(child_slot)?;
        let geometry = config.delegate.reveal_geometry(child);
        let clip = config.overlap_paint_clip(render.constraints?, render.size, geometry)?;
        let unscroll = self.child_content_transform(viewport.render).inverse()?;
        let viewport_world = child_parent_world.then(unscroll);
        Some(project_semantic_rect(viewport_world, clip))
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
        self.reveal_semantic_bounds(element, node.bounds)
    }

    fn reveal_semantic_bounds(&self, element: ElementId, bounds: Rect) -> bool {
        let chain = self.scrollable_viewport_chain(element);
        let mut candidates = chain
            .into_iter()
            .filter(|(_, _, _, _, physics)| physics.allows_implicit_scrolling());
        let Some(first) = candidates.next() else {
            return false;
        };
        // A shrink-wrapped inner list can allow implicit scrolling but have
        // nowhere to scroll. Continue outward to a viewport that can reveal
        // its content instead of treating the inner list as the only target.
        let (viewport, controller, axis, reverse, _) = if first.1.max_offset() > first.1.min_offset() {
            first
        } else {
            candidates
                .find(|(_, controller, _, _, _)| controller.max_offset() > controller.min_offset())
                .unwrap_or(first)
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
                bounds.origin.y,
                bounds.origin.y + bounds.size.height,
                view.bounds.origin.y,
                view.bounds.origin.y + view.bounds.size.height,
            ),
            Axis::Horizontal => (
                bounds.origin.x,
                bounds.origin.x + bounds.size.width,
                view.bounds.origin.x,
                view.bounds.origin.x + view.bounds.size.width,
            ),
        };
        let sliver = self.sliver_reveal_geometry(viewport, element);
        let (view_start, view_end) = if reverse {
            let (leading, trailing) = sliver.reserve(-view_end, -view_start);
            (-trailing, -leading)
        } else {
            sliver.reserve(view_start, view_end)
        };
        // Flutter showInViewport clamps the current position between
        // both aligned offsets, regardless of which is smaller. When the
        // target is larger than the viewport, a viewport already within
        // that interval keeps seeing the same useful part of the target.
        let align_start = start - view_start;
        let align_end = end - view_end;
        let first = align_start.min(align_end);
        let last = align_start.max(align_end);
        let delta = if first > 0. {
            first
        } else if last < 0. {
            last
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

    fn focus_semantics_boundary(&self, element: ElementId, entry: &Element) -> bool {
        let properties = entry.widget.semantic_properties();
        if properties.explicit.is_some()
            || properties.paragraph_parts.is_some()
            || properties.merge_descendants
            || !properties.callbacks.custom.is_empty()
        {
            return true;
        }
        match entry.widget.kind() {
            WidgetKind::Text { .. } => self.is_selection_label(element),
            WidgetKind::Button(_)
            | WidgetKind::Table { .. }
            | WidgetKind::SelectableText { .. }
            | WidgetKind::TextField(_)
            | WidgetKind::Scroll { .. }
            | WidgetKind::SliverViewport { .. } => true,
            WidgetKind::Image { .. } => properties.label.is_some(),
            WidgetKind::Gesture { callbacks, .. } => {
                super::focus::annotated_focus_node(entry.widget.kind()).is_some()
                    || GestureSemanticPolicy::ACTIONS.into_iter().any(|action| {
                        gesture_supports(callbacks, properties.callbacks.gesture_policy, action)
                    })
            }
            _ => false,
        }
    }

    fn push_focus_label_fragments(&self, element: ElementId, merged: &mut MergedSemanticSummary) {
        let Some(entry) = self.elements.get(element.0) else {
            return;
        };
        if !self.can_contribute_merged_semantics(entry)
            || self.focus_semantics_boundary(element, entry)
        {
            return;
        }
        if let WidgetKind::Text { text, .. } = entry.widget.kind() {
            merged.add_label(
                entry
                    .widget
                    .semantic_properties()
                    .label
                    .as_deref()
                    .unwrap_or(text),
                self.semantic_label_direction(element, entry),
                element_label_attributes(entry),
            );
        }
        with_recursive_tree_stack(|| {
            for &child in self.visible_semantic_children(element, entry).iter() {
                self.push_focus_label_fragments(child, merged);
            }
        });
    }

    /// A heading, ordered, or validating sliver annotation owns compatible
    /// text. This keeps the label on the node with its semantic properties
    /// without absorbing independent actions or explicit child nodes.
    fn sliver_label_annotation_candidate(&self, element: ElementId, entry: &Element) -> bool {
        let properties = entry.widget.semantic_properties();
        properties.explicit.as_ref().is_some_and(|annotation| {
            !annotation.state.explicit_child_nodes
                && (annotation.role == SemanticRole::Heading
                    || (annotation.role == SemanticRole::GenericContainer
                        && (annotation.state.sort_key.is_some()
                            || annotation.state.custom_sort_key.is_some()
                            || annotation.state.effective_validation_result()
                                != SemanticValidationResult::None
                            || ((annotation.state.container || annotation.state.toggled.is_some())
                                && annotation.label.is_none()
                                && annotation.value.is_none()
                                && annotation.description.is_none()
                                && properties.tag_for_children.is_none()
                                && properties.locale_for_subtree.is_none()))))
        }) && entry.parent.is_some_and(|viewport| {
            self.elements.get(viewport.0).is_some_and(|parent| {
                matches!(parent.widget.kind(), WidgetKind::SliverViewport { .. })
            }) && self
                .viewport_sliver_child(viewport, element)
                .is_some_and(|(_, id)| crate::scrolling::is_semantics_anchor(id))
        })
    }

    fn is_sliver_label_annotation_owner(&self, element: ElementId, entry: &Element) -> bool {
        if !self.sliver_label_annotation_candidate(element, entry) {
            return false;
        }
        let annotation = entry.widget.semantic_properties().explicit.as_ref().unwrap();
        if annotation.role == SemanticRole::Heading
            || annotation.state.sort_key.is_some()
            || annotation.state.custom_sort_key.is_some()
            || annotation.state.effective_validation_result() != SemanticValidationResult::None
        {
            return true;
        }
        // A plain sliver boundary absorbs compatible Text, but that must
        // not prevent a different boundary from merging hint/value only.
        self.visible_semantic_children(element, entry)
            .iter()
            .any(|&child| self.has_compatible_label(child))
    }

    fn has_compatible_label(&self, element: ElementId) -> bool {
        let Some(entry) = self.elements.get(element.0) else {
            return false;
        };
        if !self.can_merge_text_fragment(element, entry, SemanticTextMerge::CompatibleLabels) {
            return false;
        }
        element_own_label(entry).is_some() || with_recursive_tree_stack(|| {
            self.visible_semantic_children(element, entry)
                .iter()
                .any(|&child| self.has_compatible_label(child))
        })
    }

    /// Forms a merge from simple annotation fragments at a box or sliver
    /// boundary. Count actions before suppression: two tap handlers
    /// conflict even if one is currently blocked from accessibility.
    fn compatible_action_fragments(
        &self,
        element: ElementId,
        entry: &Element,
    ) -> Option<Vec<(ElementId, u16)>> {
        let properties = entry.widget.semantic_properties();
        let explicit = properties.explicit.as_ref()?;
        if explicit.role != SemanticRole::GenericContainer
            || !explicit.state.container
            || explicit.state.explicit_child_nodes
            || properties.merge_descendants
            || properties.callbacks != SemanticCallbacks::default()
        {
            return None;
        }
        let parent_viewport = entry.parent.filter(|parent| {
            self.elements.get(parent.0).is_some_and(|entry| {
                matches!(entry.widget.kind(), WidgetKind::SliverViewport { .. })
            })
        });
        let eligible = if let Some(viewport) = parent_viewport {
            self.viewport_sliver_child(viewport, element)
                .is_some_and(|(_, id)| crate::scrolling::is_semantics_anchor(id))
        } else {
            can_own_hint_value(entry, effective_focus_block(entry, false))
        };
        if !eligible {
            return None;
        }

        let mut work = self.visible_semantic_children(element, entry).into_owned();
        work.reverse();
        let mut fragments = Vec::new();
        let mut actions = 0;
        let mut labels = 0;
        while let Some(child) = work.pop() {
            let source = self.elements.get(child.0)?;
            if !self.can_contribute_merged_semantics(source) {
                continue;
            }
            let mask = self.compatible_action_fragment_mask(child, source)?;
            if actions & mask != 0 {
                return None;
            }
            actions |= mask;
            if element_own_label(source).is_some() {
                labels += 1;
                fragments.push((child, mask));
            }
            work.extend(
                self.visible_semantic_children(child, source)
                    .iter()
                    .rev()
                    .copied(),
            );
        }
        (labels >= 2 && actions != 0).then_some(fragments)
    }

    fn compatible_action_fragment_mask(&self, element: ElementId, entry: &Element) -> Option<u16> {
        let properties = entry.widget.semantic_properties();
        if !self.can_contribute_merged_semantics(entry)
            || properties.merge_descendants
            || properties.paragraph_parts.is_some()
            || properties.tag_for_children.is_some()
            || properties.locale_for_subtree.is_some()
            || properties.description.is_some()
            || !properties.callbacks.custom.is_empty()
            || !properties.callbacks.bound.is_empty()
            || matches!(entry.widget.kind(), WidgetKind::IndexedSemantics { .. })
        {
            return None;
        }
        let Some(explicit) = properties.explicit.as_deref() else {
            return (!self.focus_semantics_boundary(element, entry)).then_some(0);
        };
        if explicit.role != SemanticRole::GenericContainer
            || explicit.label.is_none()
            || explicit.value.is_some()
            || explicit.description.is_some()
            || explicit.actions.iter().any(|action| {
                !MERGEABLE_SEMANTIC_ACTIONS.contains(action)
                    || !properties.callbacks.supports(*action)
            })
            || explicit.state
                != (SemanticState {
                    text_direction: explicit.state.text_direction,
                    text_attributes: explicit.state.text_attributes.clone(),
                    ..SemanticState::default()
                })
            || matches!(
                entry.widget.kind(),
                WidgetKind::Button(_)
                    | WidgetKind::Image { .. }
                    | WidgetKind::Gesture { .. }
                    | WidgetKind::RawInput { .. }
                    | WidgetKind::TextField(_)
                    | WidgetKind::SelectableText { .. }
                    | WidgetKind::Scroll { .. }
                    | WidgetKind::SliverViewport { .. }
                    | WidgetKind::Table { .. }
            )
        {
            return None;
        }
        Some(
            MERGEABLE_SEMANTIC_ACTIONS
                .into_iter()
                .enumerate()
                .fold(0, |mask, (bit, action)| {
                    mask | if properties.callbacks.supports(action) {
                        1 << bit
                    } else {
                        0
                    }
                }),
        )
    }

    fn merged_compatible_action_source(
        &self,
        element: ElementId,
        action: SemanticActionKind,
    ) -> Option<ElementId> {
        let bit = MERGEABLE_SEMANTIC_ACTIONS
            .iter()
            .position(|&candidate| candidate == action)?;
        let entry = self.elements.get(element.0)?;
        self.compatible_action_fragments(element, entry)?
            .into_iter()
            .find_map(|(child, mask)| {
                (mask & (1 << bit) != 0 && !self.pointer_actions_blocked(child)).then_some(child)
            })
    }

    /// Resolve the current mounted callback rather than caching its pointer
    /// in the projected node. Handler-only replacements remain graph no-ops.
    pub(super) fn merged_compatible_action_callback(
        &self,
        element: ElementId,
        action: SemanticActionKind,
    ) -> Option<Rc<dyn Fn() + 'static>> {
        let child = self.merged_compatible_action_source(element, action)?;
        self.elements
            .get(child.0)?
            .widget
            .semantic_properties()
            .callbacks
            .callback(action)
    }

    fn can_merge_text_fragment(
        &self,
        element: ElementId,
        entry: &Element,
        policy: SemanticTextMerge,
    ) -> bool {
        if policy == SemanticTextMerge::CompatibleActions {
            return self.compatible_action_fragment_mask(element, entry).is_some();
        }
        // A controls-only merge does not absorb labels or their roles.
        if policy == SemanticTextMerge::CompatibleControls && element_own_label(entry).is_some() {
            return false;
        }
        let properties = entry.widget.semantic_properties();
        if !self.can_contribute_merged_semantics(entry)
            || properties.merge_descendants
            || properties.tag_for_children.is_some()
            || properties.locale_for_subtree.is_some()
            || properties.description.is_some()
            || properties.callbacks != SemanticCallbacks::default()
            || matches!(entry.widget.kind(), WidgetKind::IndexedSemantics { .. })
        {
            return false;
        }
        let kind = entry.widget.kind();
        match properties.explicit.as_deref() {
            Some(explicit) => {
                // Incompatible annotations keep their independent node.
                let state = &explicit.state;
                if policy == SemanticTextMerge::CompatibleControls {
                    return explicit.role == SemanticRole::GenericContainer
                        && explicit.label.is_none()
                        && explicit.value.is_none()
                        && explicit.description.is_none()
                        && explicit.actions.is_empty()
                        && state.controls_nodes.is_some()
                        && *state == (SemanticState {
                            controls_nodes: state.controls_nodes.clone(),
                            ..SemanticState::default()
                        })
                        && !matches!(kind, WidgetKind::Button(_) | WidgetKind::TextField(_)
                            | WidgetKind::Gesture { .. } | WidgetKind::RawInput { .. }
                            | WidgetKind::Image { .. } | WidgetKind::Scroll { .. }
                            | WidgetKind::SliverViewport { .. } | WidgetKind::Table { .. });
                }
                let merges_validation = policy == SemanticTextMerge::CompatibleLabels;
                let validation = state.effective_validation_result();
                let text_is_compatible = match policy {
                    SemanticTextMerge::None
                    | SemanticTextMerge::CompatibleActions
                    | SemanticTextMerge::CompatibleControls => false,
                    SemanticTextMerge::CompatibleLabels => {
                        (explicit.label.is_some() || validation != SemanticValidationResult::None)
                            && explicit.value.is_none()
                            && explicit.description.is_none()
                            && state.text_attributes.as_ref().is_none_or(|attributes| {
                                attributes.hint.is_empty() && attributes.value.is_empty()
                            })
                    }
                    SemanticTextMerge::HintAndValue { take_value } => {
                        let has_value = explicit
                            .value
                            .as_deref()
                            .is_some_and(|value| !value.is_empty());
                        explicit.label.is_none()
                            && (explicit.description.is_some() || explicit.value.is_some())
                            && (take_value || !has_value)
                            && !state.accessibility_focus_block_type.blocks_focus()
                            && state
                                .text_attributes
                                .as_ref()
                                .is_none_or(|attributes| attributes.label.is_empty())
                    }
                };
                text_is_compatible
                    && explicit.role == SemanticRole::GenericContainer
                    && explicit.actions.is_empty()
                    && !matches!(
                        kind,
                        WidgetKind::Button(_)
                            | WidgetKind::Image { .. }
                            | WidgetKind::Gesture { .. }
                            | WidgetKind::RawInput { .. }
                            | WidgetKind::TextField(_)
                            | WidgetKind::SelectableText { .. }
                            | WidgetKind::Scroll { .. }
                            | WidgetKind::SliverViewport { .. }
                            | WidgetKind::Table { .. }
                    )
                    && *state
                        == SemanticState {
                            text_direction: state.text_direction,
                            text_attributes: state.text_attributes.clone(),
                            accessibility_focus_block_type: state.accessibility_focus_block_type,
                            invalid: merges_validation && state.invalid,
                            validation_result: if merges_validation {
                                state.validation_result
                            } else {
                                SemanticValidationResult::None
                            },
                            ..SemanticState::default()
                        }
            }
            None => {
                !self.focus_semantics_boundary(element, entry)
                    && (!matches!(policy, SemanticTextMerge::HintAndValue { .. })
                        || element_own_label(entry).is_none())
            }
        }
    }

    fn push_compatible_labels(&self, element: ElementId, merged: &mut MergedSemanticSummary) {
        let Some(entry) = self.elements.get(element.0) else {
            return;
        };
        if !self.can_merge_text_fragment(element, entry, SemanticTextMerge::CompatibleLabels) {
            return;
        }
        if let Some(explicit) = &entry.widget.semantic_properties().explicit {
            merged.validation = merged
                .validation
                .merge(explicit.state.effective_validation_result());
        }
        if let Some(label) = element_own_label(entry) {
            merged.add_label(
                &label,
                self.semantic_label_direction(element, entry),
                element_label_attributes(entry),
            );
        }
        with_recursive_tree_stack(|| {
            for &child in self.visible_semantic_children(element, entry).iter() {
                self.push_compatible_labels(child, merged);
            }
        });
    }

    fn push_compatible_controls(
        &self,
        element: ElementId,
        controls: &mut Option<Arc<[String]>>,
    ) {
        let Some(entry) = self.elements.get(element.0) else {
            return;
        };
        if !self.can_merge_text_fragment(element, entry, SemanticTextMerge::CompatibleControls) {
            return;
        }
        if let Some(own) = entry
            .widget
            .semantic_properties()
            .explicit
            .as_ref()
            .and_then(|explicit| explicit.state.controls_nodes.as_ref())
        {
            SemanticState::absorb_controls_nodes(controls, own);
        }
        with_recursive_tree_stack(|| {
            for &child in self.visible_semantic_children(element, entry).iter() {
                self.push_compatible_controls(child, controls);
            }
        });
    }

    fn count_compatible_values(&self, element: ElementId, entry: &Element) -> u8 {
        let mut count = 0;
        for &child in self.visible_semantic_children(element, entry).iter() {
            let Some(entry) = self.elements.get(child.0) else {
                continue;
            };
            if !self.can_merge_text_fragment(
                child,
                entry,
                SemanticTextMerge::HintAndValue { take_value: true },
            ) {
                continue;
            }
            if entry
                .widget
                .semantic_properties()
                .explicit
                .as_ref()
                .and_then(|data| data.value.as_deref())
                .is_some_and(|value| !value.is_empty())
            {
                count += 1;
            }
            if count < 2 {
                count += with_recursive_tree_stack(|| self.count_compatible_values(child, entry));
            }
            // Two or more usable values conflict. No need to count farther.
            if count >= 2 {
                return 2;
            }
        }
        count
    }

    fn push_compatible_hint_value(
        &self,
        element: ElementId,
        policy: SemanticTextMerge,
        fields: &mut MergedSemanticFields,
    ) {
        let Some(entry) = self.elements.get(element.0) else {
            return;
        };
        if !self.can_merge_text_fragment(element, entry, policy) {
            return;
        }
        if let Some(explicit) = &entry.widget.semantic_properties().explicit {
            let direction = self.semantic_label_direction(element, entry);
            if fields.direction.is_none() {
                fields.direction = direction;
            }
            let attributes = explicit.state.text_attributes.as_deref();
            if let Some(hint) = &explicit.description {
                append_semantic_text(
                    &mut fields.hint,
                    &mut fields.hint_attributes,
                    &mut fields.direction,
                    hint,
                    direction,
                    attributes.map_or(&[], |attributes| attributes.hint.as_slice()),
                );
            }
            if let Some(value) = explicit.value.as_ref().filter(|value| !value.is_empty()) {
                fields.value = Some(value.clone());
                if let Some(attributes) = attributes {
                    fields.value_attributes.clone_from(&attributes.value);
                }
            }
        }
        with_recursive_tree_stack(|| {
            for &child in self.visible_semantic_children(element, entry).iter() {
                self.push_compatible_hint_value(child, policy, fields);
            }
        });
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
        let callbacks = &entry.widget.semantic_properties().callbacks;
        if !callbacks.custom.is_empty() && !self.pointer_actions_blocked(element) {
            merged.add_custom_actions(callbacks);
        }
        if let Some(explicit) = &entry.widget.semantic_properties().explicit {
            merged.validation = merged
                .validation
                .merge(explicit.state.effective_validation_result());
            merged.annotation_flags = merged.annotation_flags.merge(explicit.state.annotation_flags);
            if merged.input_type == SemanticsInputType::None {
                merged.input_type = explicit.state.input_type;
            }
            if let Some(controls) = &explicit.state.controls_nodes {
                SemanticState::absorb_controls_nodes(&mut merged.controls_nodes, controls);
            }
            merged.add_tags(&explicit.state.tags);
            if merged.hit_test == SemanticsHitTestBehavior::Defer {
                merged.hit_test = explicit.state.hit_test_behavior;
            }
        }
        let enabled = entry
            .widget
            .semantic_properties()
            .explicit
            .as_ref()
            .and_then(|explicit| explicit.state.enabled_state())
            .or_else(|| match entry.widget.kind() {
                WidgetKind::Button(spec) => Some(spec.enabled),
                WidgetKind::TextField(spec) if !spec.enabled => Some(false),
                _ => None,
            });
        merged.enabled = merge_enabled_state(merged.enabled, enabled);
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

    fn semantics_hidden(&self, entry: &Element) -> bool {
        semantics_hidden(entry)
            || self
                .renders
                .get(entry.render.0)
                .is_some_and(|render| render.parked)
    }

    fn can_contribute_merged_semantics(&self, entry: &Element) -> bool {
        !self.semantics_hidden(entry)
            && (!matches!(
                entry.widget.kind(),
                WidgetKind::CompositedTransformFollower { .. }
            ) || self.follower_content_visible(entry.render))
    }

    /// Resolves activation to the first live handler in a merged subtree.
    /// An ordinary node may activate only itself. Excluded or pointer-
    /// blocked descendants cannot take the merged node's action.
    #[doc(hidden)]
    #[must_use]
    pub fn semantic_activation_target(&self, element: ElementId) -> Option<ElementId> {
        let entry = self.elements.get(element.0)?;
        self.find_semantic_activation_target(element, absorbs_descendant_semantics(entry))
            .or_else(|| self.merged_compatible_action_source(element, SemanticActionKind::Activate))
    }

    fn find_semantic_activation_target(
        &self,
        element: ElementId,
        search_descendants: bool,
    ) -> Option<ElementId> {
        let entry = self.elements.get(element.0)?;
        if !self.can_contribute_merged_semantics(entry) || self.pointer_actions_blocked(element) {
            return None;
        }
        if semantic_action_is_executable(
            entry.widget.kind(),
            SemanticActionKind::Activate,
            &entry.widget.semantic_properties().callbacks,
        ) {
            return Some(element);
        }
        if !search_descendants {
            return None;
        }
        with_recursive_tree_stack(|| {
            self.visible_semantic_children(element, entry)
                .iter()
                .find_map(|&child| self.find_semantic_activation_target(child, true))
        })
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
        if !self.can_contribute_merged_semantics(entry) || self.pointer_actions_blocked(element) {
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

    fn remember_semantic_point_order(
        &mut self,
        parent: Option<SemanticNodeId>,
        traversal_edges: &[(u32, u32, ElementId)],
    ) {
        if !traversal_edges.windows(2).any(|pair| pair[0].1 > pair[1].1) {
            if !self.semantic_point_order.is_empty() {
                self.semantic_point_order.remove(&parent);
            }
            return;
        }
        let published = match parent {
            Some(id) => self.semantics.node(id).map_or(&[][..], |node| node.children.as_slice()),
            None => self.semantics.roots(),
        };
        // Viewports, tables and rich paragraphs can replace collected
        // children with synthetic ranges or groups. Never bypass one to use a flat permutation.
        if published.len() != traversal_edges.len()
            || published
                .iter()
                .zip(traversal_edges)
                .any(|(node, edge)| self.semantic_ids.get(&edge.2) != Some(node))
        {
            self.semantic_point_order.remove(&parent);
            return;
        }
        let order = self.semantic_point_order.entry(parent).or_default();
        order.clear();
        order.extend(
            traversal_edges
                .iter()
                .zip(published)
                .map(|(edge, &node)| (edge.1, node)),
        );
        order.sort_unstable_by_key(|&(sequence, _)| sequence);
    }

    /// The child order used by semantic point dispatch, before reversing it
    /// to test the topmost child first.
    #[doc(hidden)]
    #[must_use]
    pub fn semantic_inverse_hit_test_children(&self, parent: SemanticNodeId) -> Option<Vec<SemanticNodeId>> {
        let node = self.semantics.node(parent)?;
        Some(self.semantic_point_order.get(&Some(parent)).map_or_else(
            || node.children.clone(),
            |order| order.iter().map(|&(_, child)| child).collect(),
        ))
    }

    /// Selects a retained semantic action target at a logical window point.
    #[doc(hidden)]
    #[must_use]
    pub fn semantic_action_target_at(
        &self,
        position: Offset,
        action: &SemanticAction,
    ) -> Option<SemanticNodeId> {
        self.semantic_action_in_children(None, self.semantics.roots(), position, action)
    }

    fn semantic_action_in_children(
        &self,
        parent: Option<SemanticNodeId>,
        children: &[SemanticNodeId],
        position: Offset,
        action: &SemanticAction,
    ) -> Option<SemanticNodeId> {
        if let Some(paint_order) = self.semantic_point_order.get(&parent) {
            return paint_order.iter().rev().find_map(|&(_, child)| {
                self.semantic_action_in_subtree(child, position, action)
            });
        }
        children.iter().rev().find_map(|&child| {
            self.semantic_action_in_subtree(child, position, action)
        })
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
        let Some(element) = self.element_for_semantic_node(node) else {
            // A synthetic semantics-only container has no local render box
            // or action handler; its published bounds still clip its children.
            return self.semantic_action_in_children(Some(node), &semantic.children, position, action);
        };
        let render_id = self.render_id(element)?;
        let render = self.renders.get(render_id.0)?;
        let world = self.render_world_transform(render_id);
        let child_world = world.then(self.child_content_transform(render_id));
        let local = self
            .semantic_bounds_transform(render_id, world, child_world)
            .inverse_transform_point(position)?;
        // Like Flutter's Rect.contains, a semantic box excludes its right
        // and bottom edges. The pointer path has separate edge semantics.
        let bounds = self.semantic_local_bounds(render_id, render.size);
        if !(local.x >= bounds.origin.x
            && local.x < bounds.origin.x + bounds.size.width
            && local.y >= bounds.origin.y
            && local.y < bounds.origin.y + bounds.size.height)
        {
            return None;
        }

        if let Some(child) =
            self.semantic_action_in_children(Some(node), &semantic.children, position, action)
        {
            return Some(child);
        }
        if !semantic.supports_action(action.kind()) {
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
        project_semantic_rect(
            self.semantic_bounds_transform(render_id, world, child_world),
            self.semantic_local_bounds(render_id, render.size),
        )
    }

    /// Insets operate in local coordinates before the retained world transform.
    /// An over-deflated axis is empty rather than an invalid Size.
    fn semantic_local_bounds(&self, render: RenderObjectId, size: Size) -> Rect {
        let unmodified = Rect::from_origin_size(Offset::ZERO, size);
        if self.semantic_bounds_insets.is_empty() {
            return unmodified;
        }
        let Some(insets) = self.semantic_bounds_insets.get(&render) else {
            return unmodified;
        };
        let finite = |value: f32| if value.is_finite() { value } else { 0. };
        let left = finite(insets.left);
        let top = finite(insets.top);
        let width = (size.width - finite(insets.right) - left).max(0.);
        let height = (size.height - finite(insets.bottom) - top).max(0.);
        let size = Size::maybe_new(width, height).unwrap_or(Size::ZERO);
        Rect::from_origin_size(Offset::new(left, top), size)
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

fn sliver_semantics_cache_clip(
    axis: Axis,
    size: Size,
    cache_extent: crate::ScrollCacheExtent,
) -> Rect {
    let main = axis.main_extent(size);
    let extension = cache_extent.resolve(main);
    Rect::from_origin_size(
        axis.offset(-extension, 0.),
        axis.size(main + 2. * extension, axis.cross_extent(size)),
    )
}

fn project_semantic_rect(world: CoreTransform, rect: Rect) -> Rect {
    // Keep annotation/clip geometry safe even before paint rejects an
    // invalid or unrepresentable projection.
    world.try_transform_rect_bbox(rect).unwrap_or_default()
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

fn semantic_scroll_state(kind: &RenderKind) -> SemanticState {
    use incular_semantics::{SemanticScrollAxis, SemanticScrollMetrics};
    let (controller, axis, reverse, child_count) = match kind {
        RenderKind::Scroll {
            controller,
            axis,
            reverse,
            ..
        } => (controller, *axis, *reverse, None),
        RenderKind::SliverViewport { config } => (
            &config.controller,
            config.axis,
            config.reverse,
            config.semantic_child_count,
        ),
        _ => return SemanticState::default(),
    };
    SemanticState {
        has_implicit_scrolling: semantic_scroll_allows_implicit(kind),
        scroll_metrics: Some(SemanticScrollMetrics {
            axis: match axis {
                Axis::Horizontal => SemanticScrollAxis::Horizontal,
                Axis::Vertical => SemanticScrollAxis::Vertical,
            },
            reverse,
            offset: f64::from(controller.offset()),
            minimum: f64::from(controller.min_offset()),
            maximum: f64::from(controller.max_offset()),
            child_count,
            first_visible_index: None,
        }),
        ..SemanticState::default()
    }
}

fn semantic_scroll_allows_implicit(kind: &RenderKind) -> bool {
    match kind {
        RenderKind::Scroll { physics, .. } => physics.allows_implicit_scrolling(),
        RenderKind::SliverViewport { config } => config.physics.allows_implicit_scrolling(),
        _ => false,
    }
}

/// Flutter names its semantics actions for the swipe direction. A normal
/// downwards viewport advances when its contents move upwards, for example.
pub(super) fn semantic_scroll_directions(
    axis: Axis,
    reverse: bool,
) -> (SemanticActionKind, SemanticActionKind) {
    use SemanticActionKind::{ScrollDown, ScrollLeft, ScrollRight, ScrollUp};
    match (axis, reverse) {
        (Axis::Vertical, false) => (ScrollUp, ScrollDown),
        (Axis::Vertical, true) => (ScrollDown, ScrollUp),
        (Axis::Horizontal, false) => (ScrollLeft, ScrollRight),
        (Axis::Horizontal, true) => (ScrollRight, ScrollLeft),
    }
}

fn scroll_edge_actions(
    controller: &ScrollController,
    kind: &RenderKind,
) -> Vec<SemanticActionKind> {
    let (axis, reverse, physics) = match kind {
        RenderKind::Scroll {
            axis,
            reverse,
            physics,
            ..
        } => (*axis, *reverse, *physics),
        RenderKind::SliverViewport { config } => (config.axis, config.reverse, config.physics),
        _ => return Vec::new(),
    };
    let offset = controller.offset();
    let (min, max) = (controller.min_offset(), controller.max_offset());
    let mut actions = Vec::with_capacity(5);
    if physics.allows_implicit_scrolling() && max > min {
        actions.push(SemanticActionKind::ScrollToOffset);
    }
    if physics.should_accept_user_offset(offset, min, max) {
        let (forward, backward) = semantic_scroll_directions(axis, reverse);
        if offset < max {
            actions.extend([forward, SemanticActionKind::ScrollForward]);
        }
        if offset > min {
            actions.extend([backward, SemanticActionKind::ScrollBackward]);
        }
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
            WidgetKind::Text { .. } if properties.paragraph_parts.is_some() => None,
            WidgetKind::Text { text, .. } | WidgetKind::SelectableText { text, .. } => {
                Some(Cow::Borrowed(text.as_str()))
            }
            WidgetKind::Button(_) => widget_text(&entry.widget).map(Cow::Owned),
            _ => None,
        })
}

fn element_label_attributes(entry: &Element) -> &[StringAttribute] {
    let properties = entry.widget.semantic_properties();
    let attributes = if let Some(explicit) =
        properties.explicit.as_ref().filter(|explicit| explicit.label.is_some())
    {
        explicit.state.text_attributes.as_ref()
    } else if properties.label.is_none() {
        properties.implicit_text_attributes.as_ref()
    } else {
        None
    };
    attributes.map_or(&[], |attributes| attributes.label.as_slice())
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
                    kind: None,
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

fn merge_enabled_state(first: Option<bool>, second: Option<bool>) -> Option<bool> {
    // Flutter SemanticsFlags.merge: true > false > unspecified.
    match (first, second) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), _) | (_, Some(false)) => Some(false),
        (None, None) => None,
    }
}

/// Overlays the fields `overlay` sets (differs from the default) onto `base`.
/// Used when a role-less Semantics wrapper annotates a widget with its own role.
fn overlay_semantic_state(base: &mut SemanticState, overlay: &SemanticState) {
    let old_alternates = if base.increased_value.is_some() || base.decreased_value.is_some() {
        base.text_attributes.clone()
    } else {
        None
    };
    let default = SemanticState::default();
    if let Some(enabled) = overlay.enabled_state() {
        base.set_enabled_state(Some(enabled));
    }
    base.validation_result = base.validation_result.merge(overlay.validation_result);
    base.annotation_flags.overlay(overlay.annotation_flags);
    if !overlay.tags.is_empty() {
        base.tags.clone_from(&overlay.tags);
    }
    macro_rules! take {
        ($($field:ident),* $(,)?) => {
            $(if overlay.$field != default.$field { base.$field = overlay.$field.clone(); })*
        };
    }
    if overlay.sort_key.is_some() || overlay.custom_sort_key.is_some() {
        base.sort_key.clone_from(&overlay.sort_key);
        base.custom_sort_key.clone_from(&overlay.custom_sort_key);
    }
    take!(
        focused,
        focusable,
        declared_focused,
        declared_focusable,
        hit_test_behavior,
        selected,
        checked,
        expanded,
        read_only,
        editable,
        input_type,
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
        controls_nodes,
        increased_value,
        decreased_value,
    );
    let alternate_attributes = |replace: bool| {
        if replace {
            overlay.text_attributes.as_deref()
        } else {
            old_alternates.as_deref()
        }
    };
    for (increased, attributes) in [
        (true, alternate_attributes(overlay.increased_value.is_some())),
        (false, alternate_attributes(overlay.decreased_value.is_some())),
    ] {
        let selected = attributes.map_or(&[][..], |data| {
            if increased { data.increased_value.as_slice() } else { data.decreased_value.as_slice() }
        });
        let current = base.text_attributes.as_deref().map_or(&[][..], |data| {
            if increased { data.increased_value.as_slice() } else { data.decreased_value.as_slice() }
        });
        if selected != current {
            let data = Arc::make_mut(base.text_attributes.get_or_insert_with(Arc::default));
            let destination = if increased { &mut data.increased_value } else { &mut data.decreased_value };
            destination.clear();
            destination.extend_from_slice(selected);
            if data.is_empty() {
                base.text_attributes = None;
            }
        }
    }
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
                    && incular_rendering::opacity_is_transparent(controller.as_ref().map_or(*alpha, OpacityController::opacity))
        )
}

type TraversalSortKey<'a> = incular_semantics::SemanticsSortKeyRef<'a>;

/// The sort key a widget's explicit semantics declares, if any.
fn widget_sort_key(widget: &Widget) -> Option<TraversalSortKey<'_>> {
    let state = &widget.semantic_properties().explicit.as_ref()?.state;
    state
        .custom_sort_key
        .as_ref()
        .map(TraversalSortKey::Custom)
        .or_else(|| state.sort_key.as_ref().map(TraversalSortKey::Ordinal))
}

/// Adjacent keys share a sort run only when their concrete type and name
/// agree. Unkeyed siblings retain their existing order.
fn sort_keys_compatible(
    left: Option<TraversalSortKey<'_>>,
    right: Option<TraversalSortKey<'_>>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.shares_group_with(right),
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

fn effective_focus_block(entry: &Element, inherited: bool) -> AccessibilityFocusBlockType {
    if inherited {
        AccessibilityFocusBlockType::BlockSubtree
    } else {
        explicit_focus_block(entry)
    }
}

fn can_own_blocked_labels(entry: &Element, block: AccessibilityFocusBlockType) -> bool {
    block == AccessibilityFocusBlockType::BlockSubtree
        && entry.widget.semantic_properties().explicit.is_some()
        && !keeps_explicit_child_nodes(entry)
}

/// A non-boundary annotation can absorb the label whose validity it
/// describes. Explicit child boundaries and independent actions stay separate.
fn can_own_validation_labels(entry: &Element) -> bool {
    let properties = entry.widget.semantic_properties();
    let Some(explicit) = properties.explicit.as_deref() else {
        return false;
    };
    let state = &explicit.state;
    !entry.children.is_empty()
        && explicit.role == SemanticRole::GenericContainer
        && explicit.label.is_none()
        && explicit.value.is_none()
        && explicit.description.is_none()
        && explicit.actions.is_empty()
        && !properties.merge_descendants
        && properties.callbacks == SemanticCallbacks::default()
        && state.effective_validation_result() != SemanticValidationResult::None
        && *state == (SemanticState {
            text_direction: state.text_direction,
            invalid: state.invalid,
            validation_result: state.validation_result,
            ..SemanticState::default()
        })
}

fn can_own_control_relations(entry: &Element) -> bool {
    entry
        .widget
        .semantic_properties()
        .explicit
        .as_ref()
        .is_some_and(|data| data.state.controls_nodes.is_some() && !data.state.explicit_child_nodes)
}

fn can_own_hint_value(entry: &Element, block: AccessibilityFocusBlockType) -> bool {
    let properties = entry.widget.semantic_properties();
    block == AccessibilityFocusBlockType::None
        && !entry.children.is_empty()
        && !properties.merge_descendants
        && !keeps_explicit_child_nodes(entry)
        && properties
            .explicit
            .as_ref()
            .is_some_and(|data| data.state.container && data.role == SemanticRole::GenericContainer)
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
        SemanticActionKind::Focus
        | SemanticActionKind::Custom
        | SemanticActionKind::Dismiss
        | SemanticActionKind::Copy
        | SemanticActionKind::Cut
        | SemanticActionKind::Paste
        | SemanticActionKind::Expand
        | SemanticActionKind::Collapse
        | SemanticActionKind::DidGainAccessibilityFocus
        | SemanticActionKind::DidLoseAccessibilityFocus => false,
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
        SemanticActionKind::LongPress => {
            gesture_can_execute(kind, callbacks.gesture_policy, action)
        }
        SemanticActionKind::ScrollLeft
        | SemanticActionKind::ScrollRight
        | SemanticActionKind::ScrollUp
        | SemanticActionKind::ScrollDown => {
            matches!(
                kind,
                WidgetKind::Scroll { .. } | WidgetKind::SliverViewport { .. }
            ) || gesture_can_execute(kind, callbacks.gesture_policy, action)
        }
        SemanticActionKind::SetText
        | SemanticActionKind::SetSelection
        | SemanticActionKind::MoveCursorForwardByCharacter
        | SemanticActionKind::MoveCursorBackwardByCharacter
        | SemanticActionKind::MoveCursorForwardByWord
        | SemanticActionKind::MoveCursorBackwardByWord => {
            matches!(kind, WidgetKind::TextField(_))
        }
        SemanticActionKind::ScrollForward
        | SemanticActionKind::ScrollBackward
        | SemanticActionKind::ScrollToOffset => {
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
