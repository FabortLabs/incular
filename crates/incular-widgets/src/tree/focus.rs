//! Keyboard focus discovery and traversal.

use std::{cell::RefCell, collections::BTreeSet};

use incular_gestures::KeyEventResult;

use super::*;

/// A directional traversal result for a runtime-owned tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectionalFocusResolution {
    Move { target: ElementId, forward: bool },
    LeaveView,
    Unchanged,
}

/// A sequential traversal result for a runtime-owned tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequentialFocusResolution {
    Move(ElementId),
    LeaveView,
    Unchanged,
}

#[derive(Clone, Copy, Default)]
struct TraversalScan {
    anchor: Option<ElementId>,
    excluded_focus: bool,
    excluded_traversal: bool,
}

pub(super) struct FocusChangeBinding {
    node: incular_gestures::FocusNode,
    callback: Rc<dyn Fn(bool)>,
    _subscription: incular_gestures::FocusNodeSubscription,
}

impl FocusChangeBinding {
    fn new(node: &incular_gestures::FocusNode, callback: &Rc<dyn Fn(bool)>) -> Self {
        let subscription = node.observe({
            let observed = node.clone();
            let callback = callback.clone();
            let previous = std::cell::Cell::new(node.has_focus());
            move || {
                let next = observed.has_focus();
                if previous.replace(next) != next {
                    callback(next);
                }
            }
        });
        Self {
            node: node.clone(),
            callback: callback.clone(),
            _subscription: subscription,
        }
    }
}

/// Track only annotated Focus nodes. Unrelated frames and widgets neither
/// scan the focus tree nor allocate a semantic observer.
#[derive(Default)]
pub(super) struct FocusSemanticsBindings {
    observers: BTreeMap<
        ElementId,
        (
            incular_gestures::FocusNode,
            incular_gestures::FocusNodeSubscription,
        ),
    >,
    dirty: Rc<RefCell<BTreeSet<ElementId>>>,
}

impl FocusSemanticsBindings {
    fn bind(&mut self, element: ElementId, node: &incular_gestures::FocusNode) {
        if self
            .observers
            .get(&element)
            .is_some_and(|(previous, _)| previous == node)
        {
            return;
        }
        let observe = node.clone();
        let dirty = Rc::clone(&self.dirty);
        let focus_semantics = |node: &incular_gestures::FocusNode| {
            (
                node.can_request_focus(),
                node.has_primary_focus(),
                node.descendants_are_focusable(),
            )
        };
        let previous = std::cell::Cell::new(focus_semantics(node));
        let subscription = node.observe(move || {
            let next = focus_semantics(&observe);
            if previous.replace(next) != next {
                dirty.borrow_mut().insert(element);
                incular_animation::scheduler::schedule_frame();
            }
        });
        self.observers.insert(element, (node.clone(), subscription));
    }

    pub(super) fn unbind(&mut self, element: ElementId) {
        self.observers.remove(&element);
        self.dirty.borrow_mut().remove(&element);
    }
}

pub(super) fn annotated_focus_node(kind: &WidgetKind) -> Option<&incular_gestures::FocusNode> {
    match kind {
        WidgetKind::Gesture { callbacks, .. }
            if callbacks.annotates_focus && callbacks.include_semantics =>
        {
            callbacks.focus_node.as_ref()
        }
        _ => None,
    }
}

fn focus_boundary_node(kind: &WidgetKind) -> Option<&incular_gestures::FocusNode> {
    match kind {
        WidgetKind::Gesture { callbacks, .. } if callbacks.annotates_focus => {
            callbacks.focus_node.as_ref()
        }
        _ => None,
    }
}

fn widget_autofocus_node(widget: &Widget) -> Option<&incular_gestures::FocusNode> {
    match widget.kind() {
        WidgetKind::Gesture { callbacks, .. } if callbacks.autofocus => {
            focus_boundary_node(widget.kind())
        }
        _ => None,
    }
}

pub(super) struct UpdatedAutofocus {
    id: ElementId,
    node: incular_gestures::FocusNode,
    scope: Option<incular_gestures::FocusScopeNode>,
}

fn external_node(kind: &WidgetKind) -> Option<&incular_gestures::FocusNode> {
    match kind {
        WidgetKind::Gesture { callbacks, .. } => callbacks.focus_node.as_ref(),
        WidgetKind::TextField(spec) => spec.focus_node.as_ref(),
        _ => None,
    }
}

pub(super) fn owned_focus_permission(widget: &Widget) -> Option<bool> {
    match widget.kind() {
        WidgetKind::Gesture { callbacks, .. } => callbacks.owned_focus_permission,
        _ => None,
    }
}

/// The declarative descriptor has no node to allocate on rebuild. Resolve
/// one for each actual mount, or keep the one from the compatible element.
pub(super) fn instantiate_owned_focus_node(widget: &mut Widget, previous: Option<&Widget>) {
    let Some(can_request) = owned_focus_permission(widget) else {
        return;
    };
    let creation = widget.semantic_properties().owned_focus_creation.clone();
    let is_group = widget.semantic_properties().owns_focus_group;
    let WidgetKind::Gesture { callbacks, .. } = widget.kind_mut() else {
        return;
    };
    let previous = previous
        .filter(|old| old.semantic_properties().owns_focus_group == is_group)
        .and_then(|previous| match previous.kind() {
        WidgetKind::Gesture { callbacks: old, .. }
            if old.owned_focus_permission.is_some()
                && old.owns_focus_scope == callbacks.owns_focus_scope =>
        {
            Some(old)
        }
        _ => None,
    });
    if callbacks.owns_focus_scope {
        let scope = previous.and_then(|old| old.focus_scope.clone()).unwrap_or_default();
        scope.set_can_request_focus(can_request);
        callbacks.focus_node = Some(scope.scope_focus_node());
        callbacks.focus_scope = Some(scope);
    } else {
        let retained = previous.and_then(|old| old.focus_node.clone());
        let notify = retained.is_none();
        let node = retained.unwrap_or_default();
        node.set_can_request_focus(can_request);
        if callbacks.owned_focus_debug_label.is_some()
            || previous.is_some_and(|old| old.owned_focus_debug_label.is_some())
        {
            node.set_debug_label(callbacks.owned_focus_debug_label.clone());
        }
        if notify && let Some(creation) = creation {
            (creation.0)(node.clone());
        }
        callbacks.focus_node = Some(node);
    }
}

impl WidgetTree {
    pub(super) fn flush_focus_semantics_changes(&mut self) {
        let Some(bindings) = &self.focus_semantics_bindings else {
            return;
        };
        if bindings.dirty.borrow().is_empty() {
            return;
        }
        let dirty = std::mem::take(&mut *bindings.dirty.borrow_mut());
        for element in dirty {
            if let Some(render) = self.render_id(element) {
                self.mark_render_semantics(render, SemanticsDirty::Subtree);
            }
        }
    }

    /// Flutter's Focus annotation omits its focus action on iOS.
    #[doc(hidden)]
    pub fn set_focus_semantic_actions_enabled(&mut self, enabled: bool) {
        if self.focus_semantic_actions_enabled == enabled {
            return;
        }
        self.focus_semantic_actions_enabled = enabled;
        if let Some(bindings) = &self.focus_semantics_bindings {
            bindings
                .dirty
                .borrow_mut()
                .extend(bindings.observers.keys().copied());
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn can_dispatch_semantic_focus(&self, element: ElementId) -> bool {
        self.can_request_focus(element)
            && (self.focus_semantic_actions_enabled
                || self
                    .elements
                    .get(element.0)
                    .and_then(|entry| annotated_focus_node(entry.widget.kind()))
                    .is_none())
    }

    /// Returns the node installed by a mounted focus or editing widget.
    #[must_use]
    pub fn focus_node_at(&self, id: ElementId) -> Option<incular_gestures::FocusNode> {
        self.elements
            .get(id.0)
            .and_then(|element| external_node(element.widget.kind()))
            .cloned()
    }

    pub(crate) fn focus_boundary_at(&self, id: ElementId) -> Option<&incular_gestures::FocusNode> {
        self.elements
            .get(id.0)
            .and_then(|element| focus_boundary_node(element.widget.kind()))
    }

    pub(crate) fn scope_of_focus_boundary(
        &self,
        id: ElementId,
    ) -> Option<incular_gestures::FocusScopeNode> {
        let mut current = Some(id);
        while let Some(id) = current {
            if let Some(scope) = self.focus_boundary_at(id).and_then(|node| node.scope_owner()) {
                return Some(scope);
            }
            current = self.mounted_focus_parent(id);
        }
        None
    }

    /// Enables deferred FocusNode requests for a runtime-owned tree.
    #[doc(hidden)]
    pub fn enable_hosted_focus(&mut self) {
        debug_assert!(self.root.is_none(), "enable hosted focus before mounting");
        self.hosted_focus_nodes.get_or_insert_with(BTreeMap::new);
    }

    pub(super) fn sync_focus_node_binding(&mut self, id: ElementId) {
        let kind = self.elements.get(id.0).map(|element| element.widget.kind());
        if let Some(annotated) = kind.and_then(annotated_focus_node) {
            self.focus_semantics_bindings
                .get_or_insert_with(FocusSemanticsBindings::default)
                .bind(id, annotated);
        } else if let Some(bindings) = &mut self.focus_semantics_bindings {
            bindings.unbind(id);
        }
        let node = kind.and_then(external_node);
        if let (Some(node), Some(WidgetKind::Gesture { callbacks, .. })) = (node, kind) {
            if let Some(properties) = callbacks.focus_node_widget_properties {
                properties.apply(node);
            }
            if callbacks.focus_key_handler_owner == incular_gestures::FocusKeyHandlerOwner::FocusWidget {
                node.install_mounted_key_handlers(
                    callbacks.on_focus_key_event.as_ref(),
                    callbacks.on_focus_raw_key.as_ref(),
                );
            }
        }
        let callback = match kind {
            Some(WidgetKind::Gesture { callbacks, .. }) => callbacks.on_focus_change.as_ref(),
            _ => None,
        };
        let binding = node.zip(callback);
        let binding_changed = match (binding, self.focus_change_bindings.get(&id)) {
            (Some((node, callback)), Some(existing)) => {
                &existing.node != node || !Rc::ptr_eq(&existing.callback, callback)
            }
            (None, None) => false,
            _ => true,
        };
        if binding_changed {
            self.focus_change_bindings.remove(&id);
        }
        if let Some(hosted) = self.hosted_focus_nodes.as_mut() {
            let requested_parent = match (node, kind) {
                (Some(_), Some(WidgetKind::Gesture { callbacks, .. })) => {
                    callbacks.focus_parent_node.as_ref()
                }
                _ => None,
            };
            if self.requested_focus_parents.get(&id) != requested_parent {
                if let Some(parent) = requested_parent {
                    self.requested_focus_parents.insert(id, parent.clone());
                } else {
                    self.requested_focus_parents.remove(&id);
                }
                self.hosted_focus_chain.dirty = true;
                self.hosted_scope_membership_dirty = true;
            }
            if let Some(node) = node {
                if !hosted
                    .get(&id)
                    .is_some_and(|binding| binding.node() == node)
                {
                    hosted.insert(
                        id,
                        node.attach_to_focus_host(incular_animation::scheduler::schedule_frame),
                    );
                    if node.has_pending_focus_request() {
                        self.pending_focus_attachment_revision =
                            self.pending_focus_attachment_revision.wrapping_add(1);
                    }
                    self.hosted_focus_chain.dirty = true;
                    self.hosted_scope_membership_dirty = true;
                }
            } else if hosted.remove(&id).is_some() {
                self.hosted_focus_chain.dirty = true;
                self.hosted_scope_membership_dirty = true;
            }
        }
        if binding_changed && let Some((node, callback)) = binding {
            self.focus_change_bindings
                .insert(id, FocusChangeBinding::new(node, callback));
        }
        if self.hosted_focus_nodes.is_none() {
            if let Some(node) = node {
                node.bind_standalone_tree();
                self.standalone_focus_geometry
                    .get_or_insert_with(BTreeSet::new)
                    .insert(id);
            } else if let Some(nodes) = &mut self.standalone_focus_geometry {
                nodes.remove(&id);
            }
        }
        if node.is_some() {
            // Rebinding to a new node may not invalidate the existing layout.
            self.focus_geometry_stamp = None;
        }
    }

    pub(super) fn refresh_focus_geometry(&mut self, force: bool) {
        if self.hosted_focus_nodes.as_ref().is_none_or(BTreeMap::is_empty)
            && self.standalone_focus_geometry.as_ref().is_none_or(BTreeSet::is_empty)
        {
            return;
        }
        let stamp = (self.diagnostics.layouts, self.hover_hit_revision());
        if !force && self.focus_geometry_stamp == Some(stamp) {
            return;
        }
        self.focus_geometry_stamp = Some(stamp);
        self.refresh_follower_links_for_geometry();
        let nodes = self.hosted_focus_nodes.iter().flat_map(|nodes| nodes.keys())
            .chain(self.standalone_focus_geometry.iter().flatten());
        for &id in nodes {
            let Some(element) = self.elements.get(id.0) else {
                continue;
            };
            let Some(node) = external_node(element.widget.kind()) else {
                continue;
            };
            let render = self.render_live(element.render, "mounted focus render");
            node.set_mounted_geometry(render.size, self.render_world_transform(element.render));
        }
    }

    /// Changes only when a hosted attachment makes a queued request visible.
    #[doc(hidden)]
    #[must_use]
    pub fn pending_focus_attachment_revision(&self) -> u64 {
        self.pending_focus_attachment_revision
    }

    fn mounted_focus_parent(&self, element: ElementId) -> Option<ElementId> {
        self.resolved_focus_parents.get(&element).copied().or_else(|| self.parent(element))
    }

    pub(crate) fn custom_focus_policy_of(
        &self,
        element: ElementId,
    ) -> Option<crate::CustomFocusTraversalPolicy> {
        let mut current = self.parent(element);
        while let Some(id) = current {
            let properties = self.elements.get(id.0)?.widget.semantic_properties();
            if properties.owns_focus_group {
                return properties.custom_focus_policy.clone();
            }
            current = self.mounted_focus_parent(id);
        }
        None
    }

    /// Delegates a traversal request to the policy of the initiating focus
    /// group. If there is no origin, the target supplies the starting group.
    #[doc(hidden)]
    pub fn dispatch_custom_focus_request(
        &self,
        origin: Option<ElementId>,
        target: ElementId,
        alignment_policy: crate::ScrollPositionAlignmentPolicy,
    ) -> bool {
        let mut current = Some(origin.unwrap_or(target));
        while let Some(id) = current {
            let Some(entry) = self.elements.get(id.0) else { return false; };
            let properties = entry.widget.semantic_properties();
            if properties.owns_focus_group {
                let Some(callback) = &properties.custom_focus_request else {
                    return false;
                };
                (callback.0)(crate::FocusTraversalRequest {
                    node: self.focus_node_at(target),
                    alignment_policy,
                });
                return true;
            }
            current = self.mounted_focus_parent(id);
        }
        false
    }

    fn is_focus_descendant_or_self(&self, candidate: ElementId, ancestor: ElementId) -> bool {
        let mut current = Some(candidate);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.mounted_focus_parent(id);
        }
        false
    }

    fn refresh_explicit_focus_parents(&mut self) {
        self.resolved_focus_parents.clear();
        self.resolved_focus_children.clear();
        let Some(hosted) = &self.hosted_focus_nodes else {
            return;
        };
        for (&id, parent) in &self.requested_focus_parents {
            let Some(parent) = hosted.iter().find_map(|(&candidate, binding)| {
                (binding.node() == parent).then_some(candidate)
            }) else {
                continue;
            };
            let mut implicit = self.parent(id);
            while let Some(candidate) = implicit {
                if hosted.contains_key(&candidate) {
                    break;
                }
                implicit = self.parent(candidate);
            }
            if implicit != Some(parent) {
                self.resolved_focus_parents.insert(id, parent);
            }
        }
        // Decide from the entire prospective graph, not in mount order:
        // one override can make a separate override valid by moving that
        // branch away. Dropping a cyclic override may reveal another cycle
        // through its ordinary widget parent, so validate each fallback.
        loop {
            let cyclic = self
                .resolved_focus_parents
                .keys()
                .copied()
                .filter(|&origin| {
                    let mut visited = BTreeSet::new();
                    let mut current = self.mounted_focus_parent(origin);
                    while let Some(id) = current {
                        if id == origin {
                            return true;
                        }
                        if !visited.insert(id) {
                            break;
                        }
                        current = self.mounted_focus_parent(id);
                    }
                    false
                })
                .collect::<Vec<_>>();
            if cyclic.is_empty() {
                break;
            }
            for id in cyclic {
                self.resolved_focus_parents.remove(&id);
            }
        }
        for (&child, &parent) in &self.resolved_focus_parents {
            self.resolved_focus_children
                .entry(parent)
                .or_default()
                .push(child);
        }
    }

    /// Rebinds nearest external scopes after a mount or retained reparent.
    /// The focus hot path never scans the tree unless topology has changed.
    #[doc(hidden)]
    pub fn refresh_hosted_scope_membership(&mut self) {
        if !std::mem::take(&mut self.hosted_scope_membership_dirty) {
            return;
        }
        self.refresh_explicit_focus_parents();
        let Some(hosted) = &self.hosted_focus_nodes else {
            return;
        };
        let mut changed_parents = Vec::new();
        for (&id, binding) in hosted {
            let node = binding.node();
            let own_scope = node.scope_owner();
            let mut ancestor = self.mounted_focus_parent(id);
            let mut nearest = None;
            let mut scope = None;
            while let Some(id) = ancestor {
                if let Some(parent) = hosted.get(&id).map(|attachment| attachment.node())
                    && parent != node
                {
                    nearest.get_or_insert(parent);
                    if let Some(parent_scope) = parent.scope_owner()
                        && own_scope.as_ref() != Some(&parent_scope)
                    {
                        scope = Some(parent_scope);
                        break;
                    }
                }
                ancestor = self.mounted_focus_parent(id);
            }
            if node.parent().as_ref() != nearest {
                changed_parents.push((binding.mount_sequence(), node, nearest));
            }
            node.set_host_enclosing_scope(scope.as_ref());
        }
        // Arena slots are reused and are not necessarily in mount order.
        // Keep already linked siblings in place; append only the changes.
        changed_parents.sort_unstable_by_key(|(sequence, _, _)| *sequence);
        // Remove the outgoing edges first. Reversing a valid edge must
        // not be rejected as a cycle through the stale opposite link.
        for &(_, node, _) in &changed_parents {
            node.set_host_parent(None);
        }
        for (_, node, parent) in changed_parents {
            node.set_host_parent(parent);
        }
    }

    /// Applies all hosted focus roles before any node observer is notified.
    /// Rebuilds the chain only for a different target or retained ancestry.
    #[doc(hidden)]
    pub fn sync_hosted_focus_chain(&mut self, target: Option<ElementId>) {
        self.refresh_hosted_scope_membership();
        let Some(hosted) = &self.hosted_focus_nodes else {
            return;
        };
        if self.hosted_focus_chain.target == target && !self.hosted_focus_chain.dirty {
            return;
        }
        let mut next = Vec::new();
        let mut current = target;
        while let Some(id) = current {
            if let Some(binding) = hosted.get(&id)
                && !next.iter().any(|(node, _)| node == binding.node())
            {
                next.push((binding.node().clone(), Some(id) == target));
            }
            current = self.mounted_focus_parent(id);
        }
        // Each enclosing scope remembers the leaf or the nested scope
        // directly below it, not a descendant inside that nested scope.
        let mut focused_child = None;
        for (node, primary) in &next {
            if let Some(scope) = node.scope_owner() {
                if let Some(child) = focused_child {
                    scope.record_host_focused_child(child);
                }
                focused_child = Some(node);
            } else if *primary {
                focused_child = Some(node);
            }
        }
        let mut changed = Vec::new();
        for (node, _) in &self.hosted_focus_chain.entries {
            if !next.iter().any(|(retained, _)| retained == node)
                && node.set_host_focus_state_silent(false, false)
            {
                changed.push(node.clone());
            }
        }
        for (node, primary) in &next {
            if node.set_host_focus_state_silent(*primary, !*primary) {
                changed.push(node.clone());
            }
        }
        self.hosted_focus_chain = HostedFocusChain {
            target,
            entries: next,
            dirty: false,
        };
        for node in changed {
            node.notify_host_focus_state();
        }
    }

    #[must_use]
    pub fn can_request_focus(&self, id: ElementId) -> bool {
        let Some(element) = self.elements.get(id.0) else {
            return false;
        };
        let eligible = match element.widget.kind() {
            WidgetKind::Button(spec) => spec.enabled || spec.focusable_when_disabled,
            WidgetKind::TextField(spec) => {
                spec.enabled
                    && spec
                        .focus_node
                        .as_ref()
                        .is_none_or(|node| node.can_request_focus())
            }
            WidgetKind::SelectableText { .. } => true,
            // A `Text` registered with a selection area takes focus like a
            // `SelectableText` label so its selection keeps keyboard input.
            WidgetKind::Text { .. } => self.is_selection_label(id),
            WidgetKind::Gesture { callbacks, .. } => callbacks
                .focus_node
                .as_ref()
                .is_some_and(|node| node.can_request_focus()),
            _ => false,
        };
        if !eligible {
            return false;
        }
        let mut current = id;
        let has_overrides = !self.resolved_focus_parents.is_empty();
        let mut focus_ancestor_blocked = false;
        let mut reparented = false;
        loop {
            let Some(entry) = self.elements.get(current.0) else {
                return false;
            };
            reparented |= has_overrides && self.resolved_focus_parents.contains_key(&current);
            if entry.widget.semantic_properties().exclude_focus {
                return false;
            }
            let Some(parent) = entry.parent else {
                if self.root != Some(current) {
                    return false;
                }
                break;
            };
            let Some(ancestor) = self.elements.get(parent.0) else {
                return false;
            };
            match ancestor.widget.kind() {
                WidgetKind::IndexedStack { index, .. }
                    if index.and_then(|slot| ancestor.children.get(slot)) != Some(&current) =>
                {
                    return false;
                }
                WidgetKind::Gesture { callbacks, .. } => {
                    focus_ancestor_blocked |= callbacks
                        .focus_node
                        .as_ref()
                        .is_some_and(|node| !node.descendants_are_focusable());
                    if focus_ancestor_blocked && !has_overrides {
                        return false;
                    }
                }
                _ => {}
            }
            current = parent;
        }
        if !reparented {
            return !focus_ancestor_blocked;
        }
        let mut ancestor = self.mounted_focus_parent(id);
        while let Some(id) = ancestor {
            let Some(entry) = self.elements.get(id.0) else {
                return false;
            };
            if external_node(entry.widget.kind()).is_some_and(|node| !node.descendants_are_focusable())
            {
                return false;
            }
            ancestor = self.mounted_focus_parent(id);
        }
        true
    }

    #[must_use]
    pub fn focusable_elements(&self) -> Vec<ElementId> {
        with_recursive_tree_stack(|| self.focusable_elements_recursive(None, false))
    }

    /// The mounted focus nodes below `root`, in depth-first descendant order.
    ///
    /// With no root, inspects the whole mounted tree. Unlike the sorted Tab
    /// stops, this list includes eligible scope anchors. Skipping a node here
    /// does not skip its children; descendant restrictions do.
    #[must_use]
    pub fn focus_traversal_descendants(
        &self,
        root: Option<ElementId>,
    ) -> Vec<incular_gestures::FocusNode> {
        self.focus_traversal_snapshot(root, false)
    }

    /// Raw traversal candidates on the first focus layer below `root`.
    ///
    /// Non-focus widgets are transparent. A skipped focus node is omitted
    /// but its children are not promoted to the root's immediate children.
    #[must_use]
    pub fn focus_traversal_children(
        &self,
        root: Option<ElementId>,
    ) -> Vec<incular_gestures::FocusNode> {
        self.focus_traversal_snapshot(root, true)
    }

    fn focus_traversal_snapshot(
        &self,
        root: Option<ElementId>,
        children_only: bool,
    ) -> Vec<incular_gestures::FocusNode> {
        let Some(start) = root.or(self.root) else {
            return Vec::new();
        };
        let mut ancestor = if root.is_some() { Some(start) } else { None };
        while let Some(id) = ancestor {
            let Some(element) = self.elements.get(id.0) else {
                return Vec::new();
            };
            if element.widget.semantic_properties().exclude_focus_traversal
                || external_node(element.widget.kind())
                    .is_some_and(|node| !node.descendants_are_traversable())
            {
                return Vec::new();
            }
            ancestor = self.mounted_focus_parent(id);
        }

        let mut descendants = Vec::new();
        let mut pending = vec![(start, false)];
        while let Some((id, visited_children)) = pending.pop() {
            let Some(element) = self.elements.get(id.0) else {
                continue;
            };
            let node = external_node(element.widget.kind());
            if visited_children {
                if let Some(node) = node
                    && !node.skip_traversal()
                    && self.can_request_focus(id)
                {
                    descendants.push(node.clone());
                }
                continue;
            }
            if element.widget.semantic_properties().exclude_focus_traversal {
                continue;
            }
            if root != Some(id) && node.is_some() {
                pending.push((id, true));
                if children_only {
                    continue;
                }
            }
            if node.is_none_or(incular_gestures::FocusNode::descendants_are_traversable) {
                pending.extend(
                    self.mounted_focus_children(id, element)
                        .rev()
                        .map(|child| (child, false)),
                );
            }
        }
        descendants
    }

    /// Orders traversal candidates, keeping the explicitly focused node
    /// as the starting position even if traversal would not land on it.
    #[doc(hidden)]
    #[must_use]
    pub fn focus_traversal_elements(&self, anchor: Option<ElementId>) -> Vec<ElementId> {
        with_recursive_tree_stack(|| self.focusable_elements_recursive(anchor, true))
    }

    /// Resolves Tab traversal using mounted scope membership and its edge policy.
    #[doc(hidden)]
    pub fn resolve_sequential_focus(
        &self,
        focused: Option<ElementId>,
        reverse: bool,
    ) -> SequentialFocusResolution {
        let scope = focused.and_then(|id| self.scope_of_focus_boundary(id));
        self.resolve_sequential_focus_in_scope(focused, scope, reverse)
    }

    fn mounted_focus_element(&self, node: &incular_gestures::FocusNode) -> Option<ElementId> {
        self.hosted_focus_nodes.as_ref()?.iter().find_map(|(&id, binding)| {
            (binding.node() == node).then_some(id)
        })
    }

    /// Resolves a traversal invoked from a mounted node's scope, using its
    /// remembered child rather than treating the invoking node as a stop.
    #[doc(hidden)]
    pub fn resolve_node_sequential_focus(
        &self,
        origin: &incular_gestures::FocusNode,
        active: Option<ElementId>,
        reverse: bool,
    ) -> SequentialFocusResolution {
        self.resolve_node_sequential_focus_with_origin(origin, active, reverse).1
    }

    /// Resolves traversal together with the mounted policy origin.
    #[doc(hidden)]
    pub fn resolve_node_sequential_focus_with_origin(
        &self,
        origin: &incular_gestures::FocusNode,
        active: Option<ElementId>,
        reverse: bool,
    ) -> (Option<ElementId>, SequentialFocusResolution) {
        let Some(origin) = self.mounted_focus_element(origin) else {
            return (None, SequentialFocusResolution::Unchanged);
        };
        let scope = self.scope_of_focus_boundary(origin);
        let anchor = if let Some(scope) = &scope {
            scope.focused_child().and_then(|node| self.mounted_focus_element(&node))
        } else {
            // The implicit root has no stored ScopeNode. Its current child is
            // the outermost explicit scope (or the focused unscoped leaf).
            active.and_then(|id| {
                let Some(mut outer) = self.scope_of_focus_boundary(id) else {
                    return Some(id);
                };
                while let Some(parent) = outer.scope_focus_node().host_enclosing_scope() {
                    outer = parent;
                }
                self.mounted_focus_element(&outer.scope_focus_node())
            })
        };
        (Some(origin), self.resolve_sequential_focus_in_scope(anchor, scope, reverse))
    }

    fn resolve_sequential_focus_in_scope(
        &self,
        focused: Option<ElementId>,
        mut scope: Option<incular_gestures::FocusScopeNode>,
        reverse: bool,
    ) -> SequentialFocusResolution {
        use SequentialFocusResolution::{LeaveView, Move, Unchanged};
        use incular_gestures::TraversalEdgeBehavior;

        let mut ordered = self.focus_traversal_elements(focused);
        if reverse {
            ordered.reverse();
        }
        let remembered_or = |id, within: Option<&crate::FocusScopeNode>| {
            let mut nested = self.scope_of_focus_boundary(id);
            let mut child = None;
            while let Some(scope) = nested {
                if within == Some(&scope) {
                    break;
                }
                nested = scope.scope_focus_node().host_enclosing_scope();
                child = Some(scope);
            }
            let Some(scope) = child else { return id };
            let anchor = scope.scope_focus_node();
            let Some(owner) = self
                .mounted_focus_element(&anchor)
                .filter(|&id| self.can_request_focus(id))
            else {
                return id;
            };
            let retained = self.restore_scope_history(owner, scope, None);
            if retained == owner
                || self
                    .focus_boundary_at(retained)
                    .is_some_and(|n| n.scope_owner().is_some())
            {
                id
            } else {
                retained
            }
        };

        let mut departing_scope = None;
        loop {
            let local: Vec<_> = ordered
                .iter()
                .copied()
                .filter(|id| {
                    scope
                        .as_ref()
                        .is_none_or(|scope| self.is_in_focus_scope(*id, scope))
                })
                .collect();
            let Some(&first) = local.first() else {
                return Unchanged;
            };
            let anchor_scope = focused
                .and_then(|id| self.focus_boundary_at(id))
                .and_then(incular_gestures::FocusNode::scope_owner);
            let anchor = if let Some(departing) = &departing_scope {
                local.iter().rposition(|id| self.is_in_focus_scope(*id, departing))
            } else if let Some(anchor_scope) = &anchor_scope {
                // Enter an explicitly focused scope at its first stop. When
                // that scope is the remembered child of the requested scope,
                // step past its entire branch instead of re-entering it.
                if scope.as_ref() == Some(anchor_scope) {
                    None
                } else {
                    local
                        .iter()
                        .rposition(|id| self.is_in_focus_scope(*id, anchor_scope))
                }
            } else {
                focused.and_then(|id| local.iter().position(|candidate| *candidate == id))
            };
            let next = match anchor {
                Some(index) => local.get(index + 1).copied(),
                None => local
                    .iter()
                    .copied()
                    .find(|candidate| Some(*candidate) != focused),
            };
            if let Some(next) = next {
                return Move(remembered_or(next, scope.as_ref()));
            }
            match scope
                .as_ref()
                .map_or(TraversalEdgeBehavior::ClosedLoop, |scope| {
                    scope.traversal_edge_behavior()
                }) {
                TraversalEdgeBehavior::Stop => return Unchanged,
                TraversalEdgeBehavior::LeaveFlutterView => {
                    if let Some(scope) = &scope {
                        scope.clear_hosted_focus_history();
                    }
                    return LeaveView;
                }
                TraversalEdgeBehavior::ParentScope => {
                    if let Some(old) = &scope
                        && let Some(parent) = old.scope_focus_node().host_enclosing_scope()
                    {
                        old.clear_hosted_focus_history();
                        departing_scope = scope;
                        scope = Some(parent);
                        continue;
                    }
                }
                TraversalEdgeBehavior::ClosedLoop => {}
            }
            return if Some(first) == focused {
                // Parent-scope traversal already cleared the departing
                // scope's child history. With no other stop to enter,
                // leave the former child by focusing that scope itself.
                departing_scope
                    .as_ref()
                    .and_then(|scope| self.mounted_focus_element(&scope.scope_focus_node()))
                    .filter(|&id| self.can_request_focus(id))
                    .map_or(Unchanged, Move)
            } else {
                Move(remembered_or(first, scope.as_ref()))
            };
        }
    }

    fn focusable_elements_recursive(
        &self,
        anchor: Option<ElementId>,
        use_custom_policy: bool,
    ) -> Vec<ElementId> {
        let mut members = Vec::new();
        let mut sequence = 0;
        let scan = TraversalScan {
            anchor,
            ..TraversalScan::default()
        };
        if let Some(root) = self.root {
            if self.elements.get(root.0).is_some_and(|element| {
                element
                    .widget
                    .semantic_properties()
                    .focus_traversal_policy
                    .is_some()
            }) {
                self.collect_focus_group(
                    root,
                    FocusTraversalPolicyKind::ReadingOrder,
                    scan,
                    &mut members,
                    &mut sequence,
                );
            } else {
                self.append_focus_members(
                    root,
                    FocusTraversalPolicyKind::ReadingOrder,
                    scan,
                    &mut members,
                    &mut sequence,
                );
            }
        }
        let mut result = Vec::new();
        flatten_focus_group(
            self,
            FocusTraversalGroupMembers {
                owner: None,
                policy: FocusTraversalPolicyKind::ReadingOrder,
                order: None,
                members,
            },
            anchor,
            use_custom_policy,
            &mut result,
        );
        result
    }

    /// Dispatches a platform keyboard event through the nearest retained
    /// keyboard listeners surrounding the focused element. The tree owns the
    /// ordering; the runtime remains responsible for text-editing fallthrough
    /// when no listener consumes the event.
    #[must_use]
    pub fn dispatch_keyboard(&self, focused: Option<ElementId>, event: KeyboardEvent) -> bool {
        let lock_modes = event.modifiers;
        self.dispatch_keyboard_with_lock_modes(focused, event, lock_modes)
    }

    /// Routes shortcuts with lock state separate from event modifiers.
    #[must_use]
    pub fn dispatch_keyboard_with_lock_modes(
        &self,
        focused: Option<ElementId>,
        event: KeyboardEvent,
        lock_modes: incular_core::Modifiers,
    ) -> bool {
        self.route_keyboard_with_lock_modes(focused, event, lock_modes) == KeyEventResult::Handled
    }

    /// Reads the nearest mapping for an intent without invoking it.
    #[must_use]
    pub fn maybe_find_action_at<C: incular_gestures::CommandId>(
        &self,
        element: ElementId,
        intent: &incular_gestures::Intent<C>,
    ) -> Option<incular_gestures::Action<C>> {
        let mut current = Some(element);
        while let Some(id) = current {
            let entry = self.elements.get(id.0)?;
            if let WidgetKind::Gesture { callbacks, .. } = entry.widget.kind()
                && let Some(action) = callbacks.action_scope.as_ref().and_then(|s| s.action(intent))
            {
                return Some(action);
            }
            current = entry.parent;
        }
        None
    }

    /// Reads a required action mapping; an absent mapping is a typed error.
    pub fn find_action_at<C: incular_gestures::CommandId>(
        &self,
        element: ElementId,
        intent: &incular_gestures::Intent<C>,
    ) -> Result<incular_gestures::Action<C>, crate::ActionLookupError> {
        self.maybe_find_action_at(element, intent).ok_or(crate::ActionLookupError {
            command_type: std::any::type_name::<C>(),
        })
    }

    /// Captures a currently enabled action as a callback, using the caller's
    /// nearest dispatcher. An action that becomes disabled is not invoked.
    #[must_use]
    pub fn action_handler_at<C: incular_gestures::CommandId>(
        &self,
        element: ElementId,
        intent: &incular_gestures::Intent<C>,
    ) -> Option<Rc<dyn Fn()>> {
        let action = self.maybe_find_action_at(element, intent)?;
        let origin = self.element_address(element)?;
        let source = self.element_live(element, "handler caller is mounted").build_context.clone();
        let context = incular_gestures::ActionContext::new(&origin, &source);
        if !action.is_action_enabled_in_context(intent, Some(&context)) {
            return None;
        }
        let intent = intent.clone();
        let dispatcher = self.action_dispatcher_at(element);
        Some(Rc::new(move || {
            let context = incular_gestures::ActionContext::new(&origin, &source);
            let _ = incular_gestures::ActionDispatcher::new()
                .invoke_action_with_context(&action, &intent, dispatcher.clone(), Some(&context));
        }))
    }

    /// Finds the nearest explicit action dispatcher for this element.
    #[must_use]
    pub fn action_dispatcher_at(
        &self,
        element: ElementId,
    ) -> Option<Rc<incular_gestures::ScopedActionDispatcher>> {
        let mut current = Some(element);
        while let Some(id) = current {
            let entry = self.elements.get(id.0)?;
            if let WidgetKind::Gesture { callbacks, .. } = entry.widget.kind()
                && let Some(dispatcher) = callbacks.action_scope.as_ref().and_then(|s| s.dispatcher())
            {
                return Some(dispatcher);
            }
            current = entry.parent;
        }
        None
    }

    /// Invokes the closest action registered above a live element, without
    /// synthesizing a key event. A found but disabled action shadows a farther
    /// handler; `None` means no ancestor registered the intent.
    #[must_use]
    pub fn invoke_action_at<C: incular_gestures::CommandId>(
        &self,
        element: ElementId,
        intent: &incular_gestures::Intent<C>,
    ) -> Option<incular_gestures::ActionResult> {
        self.invoke_action_output_at(element, intent).map(|output| output.result())
    }

    /// Invokes an inherited action and returns any value produced by it.
    #[must_use]
    pub fn invoke_action_output_at<C: incular_gestures::CommandId>(
        &self,
        element: ElementId,
        intent: &incular_gestures::Intent<C>,
    ) -> Option<incular_gestures::ActionOutput> {
        let origin = self.element_address(element)?;
        let source = &self.elements.get(element.0)?.build_context;
        let context = incular_gestures::ActionContext::new(&origin, source);
        let intent = incular_gestures::ErasedIntent::from_intent(intent);
        let dismiss = intent.downcast_command::<crate::DismissIntent>().is_some();
        let scroll = intent.downcast_command::<crate::ScrollIntent>().copied();
        let mut current = Some(element);
        while let Some(id) = current {
            let entry = self.elements.get(id.0)?;
            if let WidgetKind::Gesture { callbacks, .. } = entry.widget.kind()
                && let Some(result) = callbacks
                    .action_scope
                    .as_ref()
                    .and_then(|scope| {
                        // Dispatch starts at the action's owner, not at the
                        // invoking descendant's closer unrelated dispatcher.
                        scope.invoke_in_context(&intent, || self.action_dispatcher_at(id), Some(&context))
                    })
            {
                return Some(result);
            }
            if dismiss
                && entry
                    .environment_override
                    .as_ref()
                    .and_then(|scope| scope.value.downcast_ref::<crate::raw_menu_anchor::AnchorScope>())
                    .is_some_and(crate::raw_menu_anchor::AnchorScope::dismiss_if_open)
            {
                return Some(incular_gestures::ActionResult::Handled.into());
            }
            current = entry.parent;
        }
        scroll
            .filter(|&intent| self.apply_scroll_intent(element, intent))
            .map(|_| incular_gestures::ActionResult::Handled.into())
    }

    fn dispatch_shortcut_action(
        &self,
        focused: ElementId,
        intent: &incular_gestures::ErasedIntent,
    ) -> KeyEventResult {
        let Some(origin) = self.element_address(focused) else {
            return KeyEventResult::Ignored;
        };
        let source = &self.element_live(focused, "shortcut caller is mounted").build_context;
        let context = incular_gestures::ActionContext::new(&origin, source);
        let mut current = Some(focused);
        while let Some(id) = current {
            let Some(entry) = self.elements.get(id.0) else {
                return KeyEventResult::Ignored;
            };
            if let WidgetKind::Gesture { callbacks, .. } = entry.widget.kind()
                && let Some(result) = callbacks.action_scope.as_ref().and_then(|scope| {
                    // Unlike direct Actions.invoke, Shortcuts gets the
                    // dispatcher from the original focused context.
                    scope.invoke_shortcut_in_context(intent, || self.action_dispatcher_at(focused), Some(&context))
                })
            {
                return result;
            }
            current = entry.parent;
        }
        KeyEventResult::Ignored
    }

    /// Routes keyboard input while preserving a Focus handler's request
    /// to stop framework handling without consuming the native event.
    #[must_use]
    pub fn route_keyboard_with_lock_modes(
        &self,
        focused: Option<ElementId>,
        event: KeyboardEvent,
        lock_modes: incular_core::Modifiers,
    ) -> KeyEventResult {
        self.route_keyboard_with_raw_payload(
            focused, event, lock_modes, incular_gestures::RawKeyDelivery::Mirror,
        )
    }

    /// Routes one key message while preserving whether it carried a raw event.
    #[doc(hidden)]
    #[must_use]
    pub fn route_keyboard_with_raw_payload(
        &self,
        focused: Option<ElementId>,
        event: KeyboardEvent,
        lock_modes: incular_core::Modifiers,
        raw_delivery: incular_gestures::RawKeyDelivery<'_>,
    ) -> KeyEventResult {
        self.dispatch_keyboard_where(focused, event, lock_modes, raw_delivery, None, |_| true, None)
    }

    /// Routes a key and returns a built-in focus intent for the runtime, if
    /// no application action already handled that shortcut.
    #[doc(hidden)]
    pub fn route_keyboard_with_focus_intent(
        &self,
        focused: Option<ElementId>,
        event: KeyboardEvent,
        lock_modes: incular_core::Modifiers,
        raw_delivery: incular_gestures::RawKeyDelivery<'_>,
    ) -> (KeyEventResult, Option<crate::DirectionalFocusIntent>) {
        self.route_keyboard_with_focus_intent_and_state(focused, event, lock_modes, raw_delivery, None)
    }

    #[doc(hidden)]
    pub fn route_keyboard_with_focus_intent_and_state(
        &self,
        focused: Option<ElementId>,
        event: KeyboardEvent,
        lock_modes: incular_core::Modifiers,
        raw_delivery: incular_gestures::RawKeyDelivery<'_>,
        keyboard: Option<&incular_core::HardwareKeyboardState>,
    ) -> (KeyEventResult, Option<crate::DirectionalFocusIntent>) {
        let mut intent = None;
        let result = self.dispatch_keyboard_where(
            focused, event, lock_modes, raw_delivery, keyboard, |_| true, Some(&mut intent),
        );
        (result, intent)
    }

    /// Shared ancestor walk behind keyboard dispatch. The `serve`
    /// predicate selects which listeners may handle the event: plain key
    /// routing serves every listener, while the semantic increment and
    /// decrement fallback serves only listeners sharing the advertisement
    /// predicate, so execution cannot outrun what semantics advertises.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn dispatch_keyboard_where(
        &self,
        focused: Option<ElementId>,
        event: KeyboardEvent,
        lock_modes: incular_core::Modifiers,
        raw_delivery: incular_gestures::RawKeyDelivery<'_>,
        keyboard: Option<&incular_core::HardwareKeyboardState>,
        mut serve: impl FnMut(&incular_gestures::GestureCallbacks) -> bool,
        mut directional_intent: Option<&mut Option<crate::DirectionalFocusIntent>>,
    ) -> KeyEventResult {
        self.notify_key_observers(focused, &event);
        let menu_escape = event.key == KeyboardKey::Named(NamedKey::Escape)
            && incular_gestures::SingleActivator::new(
                KeyboardKey::Named(NamedKey::Escape),
                incular_core::Modifiers::empty(),
            )
            .accepts_with_lock_modes(&event, lock_modes);
        let mut current = focused;
        while let Some(id) = current {
            let Some(element) = self.elements.get(id.0) else {
                break;
            };
            if let WidgetKind::Gesture { callbacks, .. } = element.widget.kind()
                && callbacks.has_keyboard_listener()
                && serve(callbacks)
            {
                let find_shortcut = || {
                    callbacks
                        .shortcut_scope
                        .as_ref()
                        .and_then(|scope| scope.find_with_keyboard(&event, keyboard))
                };
                let retained_match = (callbacks.on_shortcut.is_none()
                    && callbacks.on_shortcut_with_lock_modes.is_none()
                    && callbacks.on_shortcut_with_keyboard_state.is_none())
                    .then(find_shortcut);
                if let Some(Some(matched)) = &retained_match {
                    use incular_gestures::ShortcutMatch;
                    let result = match matched {
                        ShortcutMatch::Callback(callback) => {
                            callback();
                            KeyEventResult::Handled
                        }
                        ShortcutMatch::Intent(intent) => focused.map_or(KeyEventResult::Ignored, |focused| {
                            self.dispatch_shortcut_action(focused, intent)
                        }),
                    };
                    if result != KeyEventResult::Ignored {
                        return result;
                    }
                }
                let result = callbacks.route_keyboard_with_keyboard_state(
                    event.clone(), lock_modes, raw_delivery, keyboard,
                );
                if result != KeyEventResult::Ignored {
                    return result;
                }
                if let Some(slot) = directional_intent.as_deref_mut()
                    && let Some(incular_gestures::ShortcutMatch::Intent(matched)) =
                        retained_match.unwrap_or_else(find_shortcut)
                    && let Some(intent) = matched.downcast_command::<crate::DirectionalFocusIntent>()
                {
                    *slot = Some(*intent);
                    return KeyEventResult::Handled;
                }
            }
            if menu_escape
                && element
                    .environment_override
                    .as_ref()
                    .and_then(|scope| {
                        scope.value.downcast_ref::<crate::raw_menu_anchor::AnchorScope>()
                    })
                    .is_some_and(crate::raw_menu_anchor::AnchorScope::dismiss_if_open)
            {
                return KeyEventResult::Handled;
            }
            current = self.mounted_focus_parent(id);
        }
        KeyEventResult::Ignored
    }

    /// Routes a native accessibility increment/decrement through the same
    /// keyboard listener used by a slider. This keeps a control's value logic
    /// in its existing callback rather than creating a second action path.
    #[must_use]
    pub fn dispatch_semantic_increment(&self, element: ElementId, increment: bool) -> bool {
        let (key, code) = if increment {
            (
                KeyboardKey::Named(NamedKey::ArrowRight),
                incular_core::Code::ArrowRight,
            )
        } else {
            (
                KeyboardKey::Named(NamedKey::ArrowLeft),
                incular_core::Code::ArrowLeft,
            )
        };
        self.dispatch_keyboard_where(
            Some(element),
            KeyboardEvent::key_down(key, code),
            incular_core::Modifiers::empty(),
            incular_gestures::RawKeyDelivery::Absent,
            None,
            incular_gestures::GestureCallbacks::services_semantic_keys,
            None,
        ) == KeyEventResult::Handled
    }

    /// Returns whether `element` participates in menu-item keyboard navigation.
    ///
    /// Focus may land on the retained control beneath a transparent semantic
    /// wrapper, so walk outward until the first explicit semantic boundary.
    #[doc(hidden)]
    #[must_use]
    pub fn is_menu_item_focus(&self, mut element: ElementId) -> bool {
        loop {
            let Some(entry) = self.elements.get(element.0) else {
                return false;
            };
            if let Some(explicit) = entry.widget.semantic_properties().explicit.as_ref() {
                return explicit.role == SemanticRole::MenuItem;
            }
            let Some(parent) = entry.parent else {
                return false;
            };
            element = parent;
        }
    }

    /// Resolves desktop menu arrow-key focus using retained transient ownership.
    ///
    /// Vertical arrows cycle only inside the current menu surface. Horizontal
    /// arrows cross a visible submenu boundary in the text-direction-aware
    /// outward/inward direction. This remains renderer/platform neutral and is
    /// therefore identical for overlay and native transient presentation.
    #[doc(hidden)]
    #[must_use]
    pub fn menu_directional_focus_target(
        &self,
        focused: ElementId,
        code: incular_core::Code,
    ) -> Option<ElementId> {
        if !self.is_menu_item_focus(focused) {
            return None;
        }

        let entries = self.transient_portal_entries();
        let focusable = self.focusable_elements();
        let surface_for = |candidate: ElementId| {
            let mut current = Some(candidate);
            while let Some(element) = current {
                if let Some((id, _, _, _, _, _)) = entries
                    .iter()
                    .find(|(_, _, _, _, _, popup)| *popup == element)
                {
                    return Some(*id);
                }
                current = self.parent(element);
            }
            None
        };
        let current_surface = surface_for(focused)?;

        match code {
            incular_core::Code::ArrowDown | incular_core::Code::ArrowUp => {
                let candidates = focusable
                    .iter()
                    .copied()
                    .filter(|candidate| {
                        self.is_menu_item_focus(*candidate)
                            && surface_for(*candidate) == Some(current_surface)
                    })
                    .collect::<Vec<_>>();
                let current = candidates
                    .iter()
                    .position(|candidate| *candidate == focused)?;
                let next = if code == incular_core::Code::ArrowUp {
                    (current + candidates.len() - 1) % candidates.len()
                } else {
                    (current + 1) % candidates.len()
                };
                candidates.get(next).copied()
            }
            incular_core::Code::ArrowLeft | incular_core::Code::ArrowRight => {
                let outward = match self.environment.text_direction {
                    TextDirection::Ltr => incular_core::Code::ArrowRight,
                    TextDirection::Rtl => incular_core::Code::ArrowLeft,
                };
                if code == outward {
                    // The focused submenu trigger is the anchor of the child
                    // transient itself. Resolve that ownership directly rather
                    // than inferring it from the parent's surface id; this also
                    // remains correct through transparent retained wrappers.
                    let child_surface =
                        entries.iter().rev().find_map(|(id, _, _, _, anchor, _)| {
                            self.is_descendant_or_self(focused, *anchor).then_some(*id)
                        })?;
                    return focusable.iter().copied().find(|candidate| {
                        self.is_menu_item_focus(*candidate)
                            && surface_for(*candidate) == Some(child_surface)
                    });
                }

                let inward = match self.environment.text_direction {
                    TextDirection::Ltr => incular_core::Code::ArrowLeft,
                    TextDirection::Rtl => incular_core::Code::ArrowRight,
                };
                if code != inward {
                    return None;
                }
                let (_, _, parent, _, anchor, _) = entries
                    .iter()
                    .find(|(id, _, _, _, _, _)| *id == current_surface)?;
                parent.as_ref()?;
                focusable.iter().copied().find(|candidate| {
                    self.is_menu_item_focus(*candidate)
                        && self.is_descendant_or_self(*candidate, *anchor)
                })
            }
            _ => None,
        }
    }

    /// Flutter `DirectionalFocusIntent`: the focusable element a directional
    /// arrow key moves focus to from `focused` (or from the start of the tree
    /// when nothing is focused).
    ///
    /// Candidates come from the reading-ordered focusable elements. Candidates
    /// that share the focused element's nearest scrollable on the search axis
    /// are preferred, as Flutter does.
    #[must_use]
    pub fn directional_focus_target(
        &self,
        focused: Option<ElementId>,
        code: incular_core::Code,
    ) -> Option<ElementId> {
        let direction = traversal_direction_for(code, false)?;
        let axis = match direction {
            incular_gestures::TraversalDirection::Up
            | incular_gestures::TraversalDirection::Down => Axis::Vertical,
            incular_gestures::TraversalDirection::Left
            | incular_gestures::TraversalDirection::Right => Axis::Horizontal,
        };
        let current = focused.and_then(|id| self.element_bounds(id));
        let focused_owner = focused.and_then(|id| self.scroll_owner_on_axis(id, axis));
        let mut ids = Vec::new();
        let mut candidates = Vec::new();
        for id in self.focusable_elements() {
            if Some(id) == focused {
                continue;
            }
            let Some(rect) = self.element_bounds(id) else {
                continue;
            };
            let shares_scroll_owner = match focused_owner {
                None => true,
                Some(owner) => self.scroll_owner_on_axis(id, axis) == Some(owner),
            };
            ids.push(id);
            candidates.push(incular_gestures::DirectionalFocusCandidate {
                rect,
                shares_scroll_owner,
            });
        }
        incular_gestures::directional_focus_index(current, &candidates, direction)
            .map(|index| ids[index])
    }

    /// Resolves directional traversal within the current scope and applies
    /// the scope's edge policy only if it has no forward candidate.
    #[doc(hidden)]
    pub fn resolve_directional_focus(
        &self,
        focused: Option<ElementId>,
        code: incular_core::Code,
    ) -> DirectionalFocusResolution {
        let scope = focused.and_then(|id| self.scope_of_focus_boundary(id));
        self.resolve_directional_focus_in_scope(focused, code, scope)
    }

    /// Finds the policy origin and current child of a node-scoped direction request.
    #[doc(hidden)]
    pub fn node_directional_origin(
        &self,
        origin: &incular_gestures::FocusNode,
        active: Option<ElementId>,
    ) -> Option<(ElementId, Option<ElementId>, Option<incular_gestures::FocusScopeNode>)> {
        let origin = self.mounted_focus_element(origin)?;
        let scope = self.scope_of_focus_boundary(origin);
        let anchor = if let Some(scope) = scope.as_ref() {
            active.filter(|id| self.is_in_focus_scope(*id, scope)).or_else(|| {
                let mut last = scope.scope_focus_node();
                while let Some(child) = last.scope_owner().and_then(|scope| scope.focused_child()) {
                    last = child;
                }
                self.mounted_focus_element(&last)
            })
        } else {
            active
        };
        Some((origin, anchor, scope))
    }

    #[doc(hidden)]
    pub fn resolve_directional_focus_in_scope(
        &self,
        focused: Option<ElementId>,
        code: incular_core::Code,
        current_scope: Option<incular_gestures::FocusScopeNode>,
    ) -> DirectionalFocusResolution {
        use DirectionalFocusResolution::{LeaveView, Move, Unchanged};
        use incular_gestures::{TraversalDirection, TraversalEdgeBehavior};
        let Some(direction) = traversal_direction_for(code, false) else {
            return Unchanged;
        };
        let axis = match direction {
            TraversalDirection::Up | TraversalDirection::Down => Axis::Vertical,
            TraversalDirection::Left | TraversalDirection::Right => Axis::Horizontal,
        };
        let current = focused
            .filter(|id| {
                self.focus_boundary_at(*id)
                    .is_none_or(|node| node.scope_owner().is_none())
            })
            .and_then(|id| self.element_bounds(id));
        let owner = focused.and_then(|id| self.scroll_owner_on_axis(id, axis));

        // Resolve candidates once. ParentScope may widen the search without
        // rescanning the mounted widget tree.
        let candidates: Vec<_> = self
            .focusable_elements()
            .into_iter()
            .filter(|id| Some(*id) != focused)
            .filter_map(|id| {
                let rect = self.element_bounds(id)?;
                let shares_scroll_owner =
                    owner.is_none_or(|owner| self.scroll_owner_on_axis(id, axis) == Some(owner));
                Some((
                    id,
                    incular_gestures::DirectionalFocusCandidate {
                        rect,
                        shares_scroll_owner,
                    },
                ))
            })
            .collect();

        let mut scope = current_scope.clone();
        loop {
            let (ids, geometry): (Vec<_>, Vec<_>) = candidates
                .iter()
                .filter(|(id, _)| {
                    scope
                        .as_ref()
                        .is_none_or(|scope| self.is_in_focus_scope(*id, scope))
                })
                .copied()
                .unzip();
            let find = |forward| {
                incular_gestures::directional_focus_index_in_phase(
                    current, &geometry, direction, forward,
                )
                .map(|index| ids[index])
            };
            if let Some(target) = find(true) {
                return Move {
                    target,
                    forward: current.is_some() && scope == current_scope,
                };
            }
            match scope.as_ref().map_or(TraversalEdgeBehavior::Stop, |scope| {
                scope.directional_traversal_edge_behavior()
            }) {
                TraversalEdgeBehavior::LeaveFlutterView => return LeaveView,
                TraversalEdgeBehavior::Stop => return Unchanged,
                TraversalEdgeBehavior::ParentScope => {
                    if let Some(parent) =
                        scope.and_then(|scope| scope.scope_focus_node().host_enclosing_scope())
                    {
                        scope = Some(parent);
                        continue;
                    }
                }
                TraversalEdgeBehavior::ClosedLoop => {}
            }
            return find(false).map_or(Unchanged, |target| Move {
                target,
                forward: false,
            });
        }
    }

    fn is_in_focus_scope(
        &self,
        element: ElementId,
        requested: &incular_gestures::FocusScopeNode,
    ) -> bool {
        let mut current = self.scope_of_focus_boundary(element);
        while let Some(scope) = current {
            if &scope == requested {
                return true;
            }
            current = scope.scope_focus_node().host_enclosing_scope();
        }
        false
    }

    /// Whether directional traversal may return to a previous focus stop.
    #[doc(hidden)]
    pub fn can_retrace_directional_focus(&self, current: ElementId, previous: ElementId) -> bool {
        self.can_request_focus(previous)
            && self.scope_of_focus_boundary(current) == self.scope_of_focus_boundary(previous)
            && self.scrollable_viewport_chain(current).first().map(|entry| entry.0)
                == self.scrollable_viewport_chain(previous).first().map(|entry| entry.0)
    }

    /// Flutter `ScrollAction` for `ScrollIntent`: scrolls the nearest scroll
    /// viewport of `focused` by one line (50 px) or one page (80% of the
    /// viewport) in `direction`, animating like Flutter's 100 ms `easeInOut`.
    /// Returns whether a viewport handled the request, even when it is already
    /// at the edge or lies on the other axis, since Flutter consumes the key.
    pub fn scroll_nearest_viewport(
        &self,
        focused: Option<ElementId>,
        code: incular_core::Code,
        page: bool,
    ) -> bool {
        use incular_config::AxisDirection;
        use incular_gestures::TraversalDirection;
        let Some(direction) = traversal_direction_for(code, true) else {
            return false;
        };
        let Some(focused) = focused else {
            return false;
        };
        let direction = match direction {
            TraversalDirection::Up => AxisDirection::Up,
            TraversalDirection::Down => AxisDirection::Down,
            TraversalDirection::Left => AxisDirection::Left,
            TraversalDirection::Right => AxisDirection::Right,
        };
        let command = crate::ScrollIntent::new(direction).increment_type(if page {
            crate::ScrollIncrementType::Page
        } else {
            crate::ScrollIncrementType::Line
        });
        self.invoke_action_at(focused, &incular_gestures::Intent::<crate::ScrollIntent>::from(command))
            == Some(incular_gestures::ActionResult::Handled)
    }

    fn apply_scroll_intent(&self, origin: ElementId, intent: crate::ScrollIntent) -> bool {
        use incular_config::AxisDirection;
        let target = self.scrollable_viewport_chain(origin).into_iter().next().or_else(|| {
            let primary = self.elements.get(origin.0)?.build_context.read::<ScrollController>()?;
            self.scroll_attachments.iter().find_map(|(element, leases)| {
                if !leases.iter().any(|lease| lease.controller() == &primary) {
                    return None;
                }
                self.scrollable_viewport_chain(*element).into_iter().find(|(_, controller, ..)| controller == &primary)
            })
        });
        let Some((viewport, controller, axis, reverse, physics)) = target else {
            return false;
        };
        // Use the retained viewport's physical direction after resolving
        // both the explicit reverse flag and inherited text direction.
        let axis_direction = match (axis, reverse) {
            (Axis::Vertical, false) => AxisDirection::Down,
            (Axis::Vertical, true) => AxisDirection::Up,
            (Axis::Horizontal, false) => AxisDirection::Right,
            (Axis::Horizontal, true) => AxisDirection::Left,
        };
        if intent.direction.axis() != axis
            || !physics.should_accept_user_offset(
                controller.offset(),
                controller.min_offset(),
                controller.max_offset(),
            )
        {
            return true;
        }
        let calculator = self.elements.get(viewport.0).and_then(|entry| {
            let direct = match entry.widget.kind() {
                WidgetKind::Scroll { increment_calculator, .. } => increment_calculator.as_ref(),
                WidgetKind::SliverViewport { config } => config.increment_calculator.as_ref(),
                _ => None,
            };
            direct.cloned().or_else(|| {
                let scope = entry.build_context.read::<crate::scrolling::ScopedScrollIncrement>()?;
                (scope.controller == controller).then_some(scope.calculator)
            })
        });
        let increment = if let Some(calculator) = calculator {
            calculator.calculate(crate::ScrollIncrementDetails {
                increment_type: intent.increment_type,
                metrics: incular_scroll::ScrollMetrics {
                    pixels: controller.offset(),
                    min_scroll_extent: controller.min_offset(),
                    max_scroll_extent: controller.max_offset(),
                    viewport_dimension: controller.viewport_extent(),
                    axis,
                    axis_direction,
                    device_pixel_ratio: self.environment.scale_factor as f32,
                    page_viewport_fraction: None,
                },
            })
        } else {
            match intent.increment_type {
                crate::ScrollIncrementType::Page => 0.8 * controller.viewport_extent(),
                crate::ScrollIncrementType::Line => 50.0,
            }
        };
        if !increment.is_finite() || increment == 0.0 {
            return true;
        }
        let signed = if intent.direction == axis_direction { increment } else { -increment };
        let target = (controller.offset() + signed)
            .clamp(controller.min_offset(), controller.max_offset().max(0.0));
        if target != controller.offset() {
            controller.animate_to(
                target,
                std::time::Duration::from_millis(100),
                incular_animation::Curve::EaseInOut,
            );
        }
        true
    }

    /// The element of the nearest scroll viewport that contains `element` on
    /// `axis`, used to group focus candidates the way Flutter's
    /// `Scrollable.maybeOf(context, axis:)` does.
    fn scroll_owner_on_axis(&self, element: ElementId, axis: Axis) -> Option<ElementId> {
        self.scrollable_viewport_chain(element)
            .into_iter()
            .find(|(_, _, viewport_axis, _, _)| *viewport_axis == axis)
            .map(|(owner, ..)| owner)
    }

    /// Finds the retained element associated with an externally managed focus
    /// node. This lets a `FocusScopeNode` request focus without coupling the
    /// gestures crate to runtime element IDs.
    #[must_use]
    pub fn focused_keyboard_element(&self) -> Option<ElementId> {
        self.focusable_elements().into_iter().find(|id| {
            self.elements
                .get(id.0)
                .and_then(|element| external_node(element.widget.kind()))
                .is_some_and(|node| node.has_primary_focus())
        })
    }

    /// Focus epoch of this thread's [`FocusNode`]s; it changes whenever any
    /// node gains or loses focus, so hosts can skip
    /// [`Self::external_focus_change`] while it is unchanged.
    #[must_use]
    pub fn focus_epoch(&self) -> u64 {
        incular_gestures::focus_epoch()
    }

    /// Drops requests queued before the host parks focus at the app root.
    #[doc(hidden)]
    pub fn clear_hosted_focus_requests(&self) {
        if let Some(hosted) = &self.hosted_focus_nodes {
            for binding in hosted.values() {
                binding.node().clear_pending_focus_request();
            }
        }
    }

    /// Restores the most recent eligible target in the containing scope when
    /// a mounted subtree becomes explicitly excluded from focus.
    #[doc(hidden)]
    #[must_use]
    pub fn focus_fallback_after_exclusion(&self, focused: ElementId) -> Option<ElementId> {
        let hosted = self.hosted_focus_nodes.as_ref()?;
        let mut current = Some(focused);
        let mut excluded = false;
        let mut destination = None;
        while let Some(id) = current {
            let entry = self.elements.get(id.0)?;
            excluded |= entry.widget.semantic_properties().exclude_focus;
            if id != focused
                && let Some(scope) = hosted.get(&id).and_then(|binding| binding.node().scope_owner())
                && self.can_request_focus(id)
            {
                destination = Some((id, scope));
                break;
            }
            current = entry.parent;
        }
        // A gate higher than an otherwise eligible scope also makes that
        // scope ineligible, so we have already walked past the active gate.
        if !excluded {
            return None;
        }
        let (owner, scope) = destination?;
        Some(self.restore_scope_history(owner, scope, None))
    }

    /// Finds a new attachment for the exact same primary focus node after
    /// its former element was removed by a retained rebuild.
    #[doc(hidden)]
    #[must_use]
    pub fn reattached_primary_focus_target(
        &self,
        node: &incular_gestures::FocusNode,
    ) -> Option<ElementId> {
        self.hosted_focus_nodes
            .as_ref()?
            .iter()
            .find_map(|(&id, binding)| {
                (binding.node() == node && self.can_request_focus(id)).then_some(id)
            })
    }

    /// Finds the same external autofocus node after the layout widget that
    /// hosted it was replaced, provided an explicit focus attachment still
    /// connects it. A different newly mounted autofocus node cannot win
    /// this way, and the host settles explicit requests first.
    #[doc(hidden)]
    #[must_use]
    pub fn reattached_autofocus_target(
        &self,
        node: &incular_gestures::FocusNode,
    ) -> Option<ElementId> {
        let hosted = self.hosted_focus_nodes.as_ref()?;
        self.mounted_autofocus.iter().copied().find(|&id| {
            if hosted.get(&id).is_none_or(|binding| binding.node() != node)
                || !self.can_request_focus(id)
            {
                return false;
            }
            let mut current = Some(id);
            while let Some(id) = current {
                if self.resolved_focus_parents.contains_key(&id) {
                    return true;
                }
                current = self.parent(id);
            }
            false
        })
    }

    /// Flutter detaches a primary FocusNode with PreviouslyFocusedChild.
    /// Resolve its old scope path rather than the replacement element's
    /// ancestry: the new descriptor may have moved to a different scope.
    #[doc(hidden)]
    pub fn focus_fallback_after_node_detached(&self) -> Option<ElementId> {
        let hosted = self.hosted_focus_nodes.as_ref()?;
        let retired = self
            .hosted_focus_chain
            .entries
            .iter()
            .find_map(|(node, primary)| primary.then_some(node));
        for (anchor, primary) in &self.hosted_focus_chain.entries {
            if *primary {
                continue;
            }
            let Some(scope) = anchor.scope_owner() else {
                continue;
            };
            if let Some(owner) = hosted.iter().find_map(|(&id, binding)| {
                (binding.node() == anchor && self.can_request_focus(id)).then_some(id)
            }) {
                return Some(self.restore_scope_history(owner, scope, retired));
            }
        }
        None
    }

    fn restore_scope_history(
        &self,
        mut owner: ElementId,
        mut scope: crate::FocusScopeNode,
        retired: Option<&incular_gestures::FocusNode>,
    ) -> ElementId {
        let Some(hosted) = &self.hosted_focus_nodes else {
            return owner;
        };
        loop {
            let next = scope.restore_hosted_focus_child(|node| {
                if retired == Some(node) {
                    return None;
                }
                hosted.iter().find_map(|(&id, binding)| {
                    (id != owner && binding.node() == node
                        && self.is_focus_descendant_or_self(id, owner) && self.can_request_focus(id))
                        .then_some((id, node.scope_owner()))
                })
            });
            match next {
                Some((id, Some(inner_scope))) => {
                    owner = id;
                    scope = inner_scope;
                }
                Some((id, None)) => return id,
                None => return owner,
            }
        }
    }

    /// A focus change application code made through a [`FocusNode`] since the
    /// runtime last focused `current`: `Some(Some(element))` when another
    /// focusable node has an eligible pending request, `Some(None)`
    /// when `current`'s own node was unfocused, `None` when nothing changed.
    /// The runtime settles requests at its next input or frame boundary.
    /// A standalone tree retains direct focus behavior for existing users.
    #[must_use]
    pub fn external_focus_change(&self, current: Option<ElementId>) -> Option<Option<ElementId>> {
        if let Some(hosted) = &self.hosted_focus_nodes {
            let epoch = self.focus_epoch();
            let mut requests: Vec<_> = hosted
                .iter()
                .filter_map(|(&id, binding)| {
                    let order = binding.node().focus_request_order()?;
                    self.can_request_focus(id)
                        .then_some((epoch.wrapping_sub(order), id))
                })
                .collect();
            // The last eligible request owns primary focus; earlier
            // requests still update the scopes they touched.
            requests.sort_unstable_by_key(|(age, _)| std::cmp::Reverse(*age));
            for &(_, target) in &requests {
                self.record_hosted_focus_history(target);
            }
            let requested = requests.last().map(|&(_, target)| target);
            // Consume rejected requests as well. A branch that was hidden
            // at this boundary must not steal focus when it becomes visible.
            for binding in hosted.values() {
                binding.node().clear_pending_focus_request();
            }
            if let Some(requested) = requested {
                if Some(requested) != current {
                    return Some(Some(requested));
                }
                hosted[&requested].node().apply_host_focus(true);
                return None;
            }
            return current
                .and_then(|id| hosted.get(&id))
                .is_some_and(|binding| !binding.node().has_primary_focus())
                .then_some(None);
        }
        let node_of = |id: ElementId| {
            self.elements
                .get(id.0)
                .and_then(|element| external_node(element.widget.kind()))
        };
        if let Some(requested) = self
            .focusable_elements()
            .into_iter()
            .filter(|id| Some(*id) != current)
            .find(|id| {
                node_of(*id)
                    .is_some_and(|node| node.has_pending_focus_request() && node.has_focus())
            })
        {
            return Some(Some(requested));
        }
        let current = current?;
        node_of(current)
            .is_some_and(|node| !node.has_focus())
            .then_some(None)
    }

    /// Returns the first listener that requested autofocus and can currently
    /// receive focus.
    #[must_use]
    pub fn autofocus_element(&self) -> Option<ElementId> {
        // Autofocus is a direct request, not a Tab traversal. Walk declarations
        // in mounting order, including nodes excluded from traversal.
        let mut pending: Vec<_> = self.root.into_iter().collect();
        let mut autofocusing_scopes = Vec::new();
        while let Some(id) = pending.pop() {
            let Some(element) = self.elements.get(id.0) else {
                continue;
            };
            let requested = match element.widget.kind() {
                WidgetKind::Gesture { callbacks, .. } => callbacks.autofocus,
                WidgetKind::TextField(spec) => spec.autofocus,
                _ => false,
            };
            if requested && self.can_request_focus(id) {
                return Some(id);
            }
            if element.widget.semantic_properties().focus_scope_autofocus {
                autofocusing_scopes.push(id);
            }
            pending.extend(element.children.iter().rev().copied());
        }
        if autofocusing_scopes.is_empty() {
            return None;
        }
        // Scope-level autofocus without a designated node is an Incular
        // convenience: choose the first available traversal stop.
        let focusable = self.focusable_elements();
        autofocusing_scopes.into_iter().find_map(|scope| {
            focusable
                .iter()
                .copied()
                .find(|candidate| self.is_focus_descendant_or_self(*candidate, scope))
        })
    }

    pub(super) fn note_mounted_autofocus(&mut self, id: ElementId, widget: &Widget) {
        if self.hosted_focus_nodes.is_some()
            && (widget.semantic_properties().focus_scope_autofocus
                || match widget.kind() {
                    WidgetKind::Gesture { callbacks, .. } => callbacks.autofocus,
                    WidgetKind::TextField(spec) => spec.autofocus,
                    _ => false,
                })
        {
            self.mounted_autofocus.push(id);
            if widget_autofocus_node(widget).is_some() {
                self.element_live_mut(id, "new autofocus element must remain live")
                    .did_autofocus = true;
            }
        }
    }

    pub(super) fn note_updated_autofocus(&mut self, id: ElementId, old: &Widget, widget: &Widget) {
        if self.hosted_focus_nodes.is_none() || widget_autofocus_node(old).is_some() {
            return;
        }
        let Some(node) = widget_autofocus_node(widget) else {
            return;
        };
        let element = self.element_live_mut(id, "updated autofocus element must remain live");
        if std::mem::replace(&mut element.did_autofocus, true) {
            return;
        }
        self.updated_autofocus.push(UpdatedAutofocus {
            id,
            node: node.clone(),
            scope: crate::FocusScope::maybe_of(self, id),
        });
    }

    /// Drains this host's scope autofocus queue. Before rebuilding, only
    /// consume requests through the first candidate still awaiting attachment.
    /// After layout, process the remaining requests in submission order.
    #[doc(hidden)]
    pub fn take_scope_autofocus(
        &self,
        before_build: bool,
    ) -> Vec<(incular_gestures::FocusScopeNode, incular_gestures::FocusNode)> {
        let Some(hosted) = self.hosted_focus_nodes.as_ref() else {
            return Vec::new();
        };
        let contains = |node: &incular_gestures::FocusNode| {
            hosted.values().any(|attachment| attachment.node() == node)
        };
        let mut waiting_for_attachment = false;
        incular_gestures::take_hosted_scope_autofocus(|scope, target| {
            if !contains(&scope.scope_focus_node()) {
                return false;
            }
            if before_build {
                waiting_for_attachment |= !contains(target);
            }
            !waiting_for_attachment
        })
    }

    /// Remembers a focus request in its scopes without granting primary
    /// focus to that node or notifying its focus listeners.
    fn record_hosted_focus_history(&self, target: ElementId) {
        let Some(hosted) = &self.hosted_focus_nodes else {
            return;
        };
        let Some(mut child) = hosted.get(&target).map(|binding| binding.node()) else {
            return;
        };
        let mut current = self.mounted_focus_parent(target);
        while let Some(id) = current {
            if let Some(anchor) = hosted.get(&id).map(|binding| binding.node())
                && anchor != child
                && let Some(scope) = anchor.scope_owner()
            {
                scope.record_host_focused_child(child);
                child = anchor;
            }
            current = self.mounted_focus_parent(id);
        }
    }

    /// Earlier eligible requests fill their scope and its ancestors before
    /// the next request is tried.
    #[doc(hidden)]
    pub fn finish_hosted_scope_autofocus(
        &self,
        pending: Vec<(incular_gestures::FocusScopeNode, incular_gestures::FocusNode)>,
    ) -> Option<ElementId> {
        let hosted = self.hosted_focus_nodes.as_ref()?;
        let find = |node: &incular_gestures::FocusNode| {
            hosted
                .iter()
                .find_map(|(&id, attachment)| (attachment.node() == node).then_some(id))
        };
        let mut chosen = None;
        for (scope, node) in pending {
            let Some(owner) = find(&scope.scope_focus_node()) else {
                continue;
            };
            if scope.focused_child().is_some() {
                continue;
            }
            let Some(target) = find(&node).filter(|&id| {
                id != owner && self.is_focus_descendant_or_self(id, owner) && self.can_request_focus(id)
            }) else {
                continue;
            };
            let target = node.scope_owner().map_or(target, |child_scope| {
                self.restore_scope_history(target, child_scope, None)
            });
            self.record_hosted_focus_history(target);
            chosen = Some(target);
        }
        chosen
    }

    /// Resolves retained Focus updates after direct focus requests and layout.
    #[doc(hidden)]
    pub fn finish_updated_autofocus(&mut self, focused: Option<ElementId>) -> Option<ElementId> {
        let requests = std::mem::take(&mut self.updated_autofocus);
        requests.into_iter().find_map(|request| {
            (self.focus_boundary_at(request.id) == Some(&request.node)
                && crate::FocusScope::maybe_of(self, request.id) == request.scope
                && request.scope.as_ref().map_or_else(
                    || focused.is_none(),
                    |scope| {
                        scope.focused_child().is_none()
                            && focused.is_none_or(|focused| {
                                crate::FocusScope::maybe_of(self, focused).as_ref() != Some(scope)
                            })
                    },
                )
                && self.can_request_focus(request.id))
            .then_some(request.id)
        })
    }

    /// Drains mount-time autofocus requests; detached, hidden or otherwise
    /// ineligible requests do not survive to a later frame.
    #[doc(hidden)]
    pub fn finish_mounted_autofocus(&mut self, use_requests: bool) -> Option<ElementId> {
        if !use_requests {
            self.mounted_autofocus.clear();
            return None;
        }
        let requests = std::mem::take(&mut self.mounted_autofocus);
        requests.into_iter().find_map(|id| {
            let element = self.elements.get(id.0)?;
            if match element.widget.kind() {
                WidgetKind::Gesture { callbacks, .. } => callbacks.autofocus,
                WidgetKind::TextField(spec) => spec.autofocus,
                _ => false,
            } && self.can_request_focus(id)
            {
                return Some(id);
            }
            element
                .widget
                .semantic_properties()
                .focus_scope_autofocus
                .then(|| {
                    self.focusable_elements()
                        .into_iter()
                        .find(|candidate| self.is_focus_descendant_or_self(*candidate, id))
                })
                .flatten()
        })
    }

    /// Notifies focus-within callbacks and mirrors standalone tree focus
    /// onto the listener's node. Runtime trees synchronize their full path.
    pub fn set_keyboard_focus(&self, id: ElementId, focused: bool) {
        self.notify_focus_within(id, focused);
        if self.hosted_focus_nodes.is_some() {
            return;
        }
        let Some(element) = self.elements.get(id.0) else {
            return;
        };
        let Some(node) = external_node(element.widget.kind()) else {
            return;
        };
        if focused && !node.can_request_focus() {
            return;
        }
        node.apply_host_focus(focused);
    }

    fn collect_focus_group(
        &self,
        id: ElementId,
        inherited_policy: FocusTraversalPolicyKind,
        scan: TraversalScan,
        out: &mut Vec<FocusTraversalMember>,
        sequence: &mut usize,
    ) {
        let Some(element) = self.elements.get(id.0) else {
            return;
        };
        let policy = element
            .widget
            .semantic_properties()
            .focus_traversal_policy
            .unwrap_or(inherited_policy);
        let mut members = Vec::new();
        self.append_focus_members(id, policy, scan, &mut members, sequence);
        if members.is_empty() {
            return;
        }
        out.push(FocusTraversalMember::Group(FocusTraversalGroupMembers {
            owner: Some(id),
            policy,
            order: element.widget.semantic_properties().focus_traversal_order.clone(),
            members,
        }));
    }

    fn append_focus_members(
        &self,
        id: ElementId,
        policy: FocusTraversalPolicyKind,
        scan: TraversalScan,
        out: &mut Vec<FocusTraversalMember>,
        sequence: &mut usize,
    ) {
        enum FocusWork {
            Node(ElementId, FocusTraversalPolicyKind, TraversalScan),
            Group(ElementId, FocusTraversalPolicyKind, TraversalScan),
        }

        let mut work = vec![FocusWork::Node(id, policy, scan)];
        while let Some(next) = work.pop() {
            let FocusWork::Node(id, policy, mut scan) = next else {
                let FocusWork::Group(id, inherited_policy, scan) = next else {
                    unreachable!();
                };
                self.collect_focus_group(id, inherited_policy, scan, out, sequence);
                continue;
            };
            let Some(element) = self.elements.get(id.0) else {
                continue;
            };
            let properties = element.widget.semantic_properties();
            scan.excluded_traversal |= properties.exclude_focus_traversal;
            if scan.excluded_traversal
                && !scan
                    .anchor
                    .is_some_and(|anchor| self.is_focus_descendant_or_self(anchor, id))
            {
                continue;
            }
            scan.excluded_focus |= properties.exclude_focus;
            // A `Text` label takes focus from a selection gesture only; it is
            // not a traversal stop (Flutter traverses to the area, not to
            // each registered paragraph).
            let node = external_node(element.widget.kind());
            let traversal_candidate =
                !scan.excluded_traversal
                    && node.is_none_or(|node| node.scope_owner().is_none() && !node.skip_traversal());
            if !scan.excluded_focus
                && (traversal_candidate || scan.anchor == Some(id))
                && !matches!(element.widget.kind(), WidgetKind::Text { .. })
                && self.can_request_focus(id)
            {
                let bounds = self.element_bounds(id).unwrap_or_default();
                let order = element
                    .widget
                    .semantic_properties()
                    .focus_traversal_order
                    .clone();
                if let Some(node) = node {
                    node.set_focus_order(order.clone());
                }
                out.push(FocusTraversalMember::Candidate(FocusCandidate {
                    id,
                    bounds,
                    order,
                    sequence: *sequence,
                }));
                *sequence = sequence.saturating_add(1);
            }
            let (descendants_are_focusable, descendants_are_traversable) =
                match element.widget.kind() {
                    WidgetKind::Gesture { callbacks, .. } => {
                        callbacks.focus_node.as_ref().map_or((true, true), |node| {
                            (
                                node.descendants_are_focusable(),
                                node.descendants_are_traversable(),
                            )
                        })
                    }
                    _ => (true, true),
                };
            scan.excluded_traversal |= !descendants_are_traversable;
            if scan.excluded_traversal && scan.anchor.is_none() {
                continue;
            }
            scan.excluded_focus |= !descendants_are_focusable;
            for child in self.mounted_focus_children(id, element).rev() {
                let child_policy = self
                    .elements
                    .get(child.0)
                    .and_then(|child| child.widget.semantic_properties().focus_traversal_policy);
                if child_policy.is_some() {
                    work.push(FocusWork::Group(child, policy, scan));
                } else {
                    work.push(FocusWork::Node(child, policy, scan));
                }
            }
        }
    }
    fn mounted_focus_children<'a>(
        &'a self,
        id: ElementId,
        element: &'a Element,
    ) -> impl DoubleEndedIterator<Item = ElementId> + 'a {
        let children: &[ElementId] = match element.widget.kind() {
            WidgetKind::IndexedStack { index, .. } => index
                .and_then(|slot| element.children.get(slot))
                .map_or(&[], std::slice::from_ref),
            _ => element.children.as_slice(),
        };
        children
            .iter()
            .copied()
            .filter(|child| !self.resolved_focus_parents.contains_key(child))
            .chain(
                self.resolved_focus_children
                    .get(&id)
                    .into_iter()
                    .flatten()
                    .copied(),
            )
    }

    pub(super) fn is_descendant_or_self(&self, candidate: ElementId, ancestor: ElementId) -> bool {
        let mut current = Some(candidate);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.parent(id);
        }
        false
    }
}

/// Maps a default focus-or-scroll key to its direction. Arrow keys always
/// map; PageUp and PageDown map only for scrolling.
fn traversal_direction_for(
    code: incular_core::Code,
    scroll: bool,
) -> Option<incular_gestures::TraversalDirection> {
    use incular_gestures::TraversalDirection;
    match code {
        incular_core::Code::ArrowUp => Some(TraversalDirection::Up),
        incular_core::Code::ArrowDown => Some(TraversalDirection::Down),
        incular_core::Code::ArrowLeft => Some(TraversalDirection::Left),
        incular_core::Code::ArrowRight => Some(TraversalDirection::Right),
        incular_core::Code::PageUp if scroll => Some(TraversalDirection::Up),
        incular_core::Code::PageDown if scroll => Some(TraversalDirection::Down),
        _ => None,
    }
}

impl WidgetTree {
    /// The `on_focus_within` observers on `id` and its ancestors, nearest
    /// first. Collected before any call so observers run without borrows.
    fn focus_within_observers(&self, id: ElementId) -> Vec<Rc<dyn Fn(bool)>> {
        let mut observers = Vec::new();
        let mut current = Some(id);
        while let Some(element_id) = current {
            if let Some(element) = self.elements.get(element_id.0)
                && let WidgetKind::Gesture { callbacks, .. } = element.widget.kind()
                && let Some(observer) = &callbacks.on_focus_within
            {
                observers.push(observer.clone());
            }
            current = self.parent(element_id);
        }
        observers
    }

    fn notify_focus_within(&self, id: ElementId, focused: bool) {
        for observer in self.focus_within_observers(id) {
            observer(focused);
        }
    }

    /// Passive key observers around the focused element see every key event
    /// first, consumed or not, except while a text field is focused.
    fn notify_key_observers(&self, focused: Option<ElementId>, event: &KeyboardEvent) {
        let Some(focused) = focused else {
            return;
        };
        if self.is_text_field(focused) {
            return;
        }
        let mut observers: Vec<Rc<dyn Fn(KeyboardEvent)>> = Vec::new();
        let mut current = Some(focused);
        while let Some(id) = current {
            if let Some(element) = self.elements.get(id.0)
                && let WidgetKind::Gesture { callbacks, .. } = element.widget.kind()
                && let Some(observer) = &callbacks.on_key_observe
            {
                observers.push(observer.clone());
            }
            current = self.parent(id);
        }
        for observer in observers {
            observer(event.clone());
        }
    }
}
