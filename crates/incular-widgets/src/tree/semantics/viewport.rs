use super::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct PaneChild {
    pub(super) visible_index: Option<usize>,
    pub(super) excluded: bool,
}

impl PaneChild {
    pub(super) fn from_state(
        state: &SemanticState,
        exclude_tag: &incular_semantics::SemanticsTag,
    ) -> Self {
        Self {
            visible_index: if state.hidden { None } else { state.item_index },
            excluded: state.tags.contains(exclude_tag),
        }
    }
}

impl WidgetTree {
    /// Assemble the viewport boundary separately from its scrolling pane.
    /// The outer ID stays attached to the parent; the inner ID serves
    /// scroll actions and survives changes to the visible child list.
    pub(super) fn assemble_viewport_semantic_panes(
        &mut self,
        element: ElementId,
        outer: SemanticNodeId,
        build: Collected,
        children: &[SemanticNodeId],
        changed_children: &FastHashMap<SemanticNodeId, PaneChild>,
        detached: &mut Vec<SemanticNodeId>,
    ) {
        let old_inner = self.viewport_semantic_panes.get(&element).copied();
        let old_content = old_inner.unwrap_or(outer);
        let role = match &build {
            Collected::Full(build) => build.role,
            Collected::Bounds { .. } => self.semantics.node(old_content).unwrap().role,
        };
        // ChildTagScope marks the first projected semantic node(s) under
        // this viewport. An empty viewport falls back to a single node.
        let split = role == SemanticRole::ScrollView && !children.is_empty();
        let exclude_tag = crate::Viewport::exclude_from_scrolling_tag();
        let pane_child = |id: SemanticNodeId| {
            changed_children.get(&id).copied().unwrap_or_else(|| {
                self.semantics
                    .node(id)
                    .map(|node| PaneChild::from_state(&node.state, exclude_tag))
                    .unwrap_or_default()
            })
        };
        let is_excluded = |id| pane_child(id).excluded;
        let mut excluded = Vec::new();
        let mut included = Vec::new();
        let children = if split && children.iter().copied().any(is_excluded) {
            for &id in children {
                if is_excluded(id) {
                    excluded.push(id);
                } else {
                    included.push(id);
                }
            }
            &included
        } else {
            children
        };
        let first_visible_index = if split {
            children.iter().find_map(|&id| pane_child(id).visible_index)
        } else {
            None
        };
        if let Some(old) = old_inner.and_then(|id| self.semantics.node(id)) {
            // The normal attachment sweep only examines the outer node.
            // An old inner descendant can move, or leave the graph.
            detached.extend(old.children.iter().copied());
        }
        let mut content = match build {
            Collected::Full(build) => SemanticNode {
                id: old_content,
                role: build.role,
                label: build.label,
                value: build.value,
                description: build.description,
                bounds: build.bounds,
                state: build.state,
                actions: build.actions,
                children: children.to_vec(),
            },
            Collected::Bounds { bounds, hidden, .. } => {
                let old_index = self
                    .semantics
                    .node(old_content)
                    .and_then(|node| node.state.scroll_metrics)
                    .and_then(|metrics| metrics.first_visible_index);
                if split == old_inner.is_some() && old_index == first_visible_index {
                    self.semantics.update_geometry_and_visibility(
                        old_content,
                        bounds,
                        children,
                        hidden,
                    );
                    if let Some(inner) = old_inner {
                        let mut outer_children = vec![inner];
                        outer_children.extend(excluded);
                        self.semantics.update_geometry_and_visibility(
                            outer,
                            bounds,
                            &outer_children,
                            hidden,
                        );
                    }
                    return;
                }
                let mut node = self.semantics.node(old_content).unwrap().clone();
                node.bounds = bounds;
                if let Some(hidden) = hidden {
                    node.state.hidden = hidden;
                }
                if old_inner.is_some() {
                    node.state.tags = self.semantics.node(outer).unwrap().state.tags.clone();
                    node.state.sort_key =
                        self.semantics.node(outer).unwrap().state.sort_key.clone();
                }
                node.children = children.to_vec();
                node
            }
        };
        if let Some(metrics) = &mut content.state.scroll_metrics {
            metrics.first_visible_index = first_visible_index;
        }
        if !split {
            if let Some(inner) = self.viewport_semantic_panes.remove(&element) {
                detached.push(inner);
            }
            content.id = outer;
            self.semantics.update(outer, content);
            return;
        }

        let bounds = content.bounds;
        let outer_state = SemanticState {
            hidden: content.state.hidden,
            tags: std::mem::take(&mut content.state.tags),
            sort_key: content.state.sort_key.take(),
            ..SemanticState::default()
        };
        let inner = if let Some(inner) = old_inner {
            content.id = inner;
            self.semantics.update(inner, content);
            inner
        } else {
            let inner = self.semantics.insert(content);
            self.viewport_semantic_panes.insert(element, inner);
            inner
        };
        let mut outer_children = vec![inner];
        outer_children.extend(excluded);
        self.semantics.update(
            outer,
            SemanticNode {
                id: outer,
                role: SemanticRole::GenericContainer,
                label: None,
                value: None,
                description: None,
                bounds,
                state: outer_state,
                actions: Vec::new(),
                children: outer_children,
            },
        );
    }
}
