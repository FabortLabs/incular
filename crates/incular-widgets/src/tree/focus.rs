//! Keyboard focus discovery and traversal.

use super::*;

#[derive(Clone, Copy, Default)]
struct TraversalScan {
    anchor: Option<ElementId>,
    excluded_focus: bool,
    excluded_traversal: bool,
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
    let node = previous
        .filter(|previous| owned_focus_permission(previous).is_some())
        .and_then(|previous| external_node(previous.kind()))
        .cloned()
        .unwrap_or_default();
    node.set_can_request_focus(can_request);
    if let WidgetKind::Gesture { callbacks, .. } = widget.kind_mut() {
        callbacks.focus_node = Some(node);
    }
}

impl WidgetTree {
    /// Enables deferred FocusNode requests for a runtime-owned tree.
    #[doc(hidden)]
    pub fn enable_hosted_focus(&mut self) {
        debug_assert!(self.root.is_none(), "enable hosted focus before mounting");
        self.hosted_focus_nodes.get_or_insert_with(BTreeMap::new);
    }

    pub(super) fn sync_hosted_focus_node(&mut self, id: ElementId) {
        let Some(hosted) = self.hosted_focus_nodes.as_mut() else {
            return;
        };
        let node = self
            .elements
            .get(id.0)
            .and_then(|element| external_node(element.widget.kind()));
        if let Some(node) = node {
            if !hosted
                .get(&id)
                .is_some_and(|binding| binding.node() == node)
            {
                hosted.insert(id, node.attach_to_focus_host());
                self.hosted_focus_chain.dirty = true;
            }
        } else if hosted.remove(&id).is_some() {
            self.hosted_focus_chain.dirty = true;
        }
    }

    /// Applies all hosted focus roles before any node observer is notified.
    /// Rebuilds the chain only for a different target or retained ancestry.
    #[doc(hidden)]
    pub fn sync_hosted_focus_chain(&mut self, target: Option<ElementId>) {
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
            current = self.parent(id);
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
        loop {
            let Some(entry) = self.elements.get(current.0) else {
                return false;
            };
            if entry.widget.semantic_properties().exclude_focus {
                return false;
            }
            let Some(parent) = entry.parent else {
                return self.root == Some(current);
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
                WidgetKind::Gesture { callbacks, .. }
                    if callbacks
                        .focus_node
                        .as_ref()
                        .is_some_and(|node| !node.descendants_are_focusable()) =>
                {
                    return false;
                }
                _ => {}
            }
            current = parent;
        }
    }

    #[must_use]
    pub fn focusable_elements(&self) -> Vec<ElementId> {
        self.focus_traversal_elements(None)
    }

    /// Orders traversal candidates, keeping the explicitly focused node
    /// as the starting position even if traversal would not land on it.
    #[doc(hidden)]
    #[must_use]
    pub fn focus_traversal_elements(&self, anchor: Option<ElementId>) -> Vec<ElementId> {
        with_recursive_tree_stack(|| self.focusable_elements_recursive(anchor))
    }

    fn focusable_elements_recursive(&self, anchor: Option<ElementId>) -> Vec<ElementId> {
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
            FocusTraversalGroupMembers {
                policy: FocusTraversalPolicyKind::ReadingOrder,
                members,
            },
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
        self.dispatch_keyboard_where(focused, event, lock_modes, |_| true)
    }

    /// Shared ancestor walk behind keyboard dispatch. The `serve`
    /// predicate selects which listeners may handle the event: plain key
    /// routing serves every listener, while the semantic increment and
    /// decrement fallback serves only listeners sharing the advertisement
    /// predicate, so execution cannot outrun what semantics advertises.
    #[must_use]
    pub(super) fn dispatch_keyboard_where(
        &self,
        focused: Option<ElementId>,
        event: KeyboardEvent,
        lock_modes: incular_core::Modifiers,
        mut serve: impl FnMut(&incular_gestures::GestureCallbacks) -> bool,
    ) -> bool {
        self.notify_key_observers(focused, &event);
        let mut current = focused;
        while let Some(id) = current {
            if let Some(element) = self.elements.get(id.0)
                && let WidgetKind::Gesture { callbacks, .. } = element.widget.kind()
                && callbacks.has_keyboard_listener()
                && serve(callbacks)
                && callbacks.handle_keyboard_with_lock_modes(event.clone(), lock_modes)
            {
                return true;
            }
            current = self.parent(id);
        }
        false
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
            incular_gestures::GestureCallbacks::services_semantic_keys,
        )
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
        let Some(direction) = traversal_direction_for(code, true) else {
            return false;
        };
        let Some(focused) = focused else {
            return false;
        };
        let Some((_, controller, axis, reverse, _)) =
            self.scrollable_viewport_chain(focused).into_iter().next()
        else {
            return false;
        };
        // Flutter `getAxisDirectionFromAxisReverseAndDirectionality`.
        let rtl = self.environment.text_direction == TextDirection::Rtl;
        let axis_direction = match (axis, reverse, rtl) {
            (Axis::Vertical, false, _) => incular_gestures::TraversalDirection::Down,
            (Axis::Vertical, true, _) => incular_gestures::TraversalDirection::Up,
            (Axis::Horizontal, false, false) | (Axis::Horizontal, true, true) => {
                incular_gestures::TraversalDirection::Right
            }
            (Axis::Horizontal, _, _) => incular_gestures::TraversalDirection::Left,
        };
        let on_axis = matches!(
            (axis, direction),
            (
                Axis::Vertical,
                incular_gestures::TraversalDirection::Up
                    | incular_gestures::TraversalDirection::Down
            ) | (
                Axis::Horizontal,
                incular_gestures::TraversalDirection::Left
                    | incular_gestures::TraversalDirection::Right
            )
        );
        if !on_axis {
            return true;
        }
        let increment = if page {
            0.8 * controller.viewport_extent()
        } else {
            50.0
        };
        let signed = if direction == axis_direction {
            increment
        } else {
            -increment
        };
        let target = (controller.offset() + signed)
            .clamp(controller.min_offset(), controller.max_offset().max(0.0));
        controller.animate_to(
            target,
            std::time::Duration::from_millis(100),
            incular_animation::Curve::EaseInOut,
        );
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
            let requested = hosted
                .iter()
                .filter_map(|(&id, binding)| {
                    let order = binding.node().focus_request_order()?;
                    self.can_request_focus(id)
                        .then_some((epoch.wrapping_sub(order), id))
                })
                .min()
                .map(|(_, id)| id);
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
        let focusable = self.focusable_elements();
        if let Some(listener) = focusable.iter().find(|id| {
            self.elements
                .get(id.0)
                .is_some_and(|element| match element.widget.kind() {
                    WidgetKind::Gesture { callbacks, .. } => callbacks.autofocus,
                    WidgetKind::TextField(spec) => spec.autofocus,
                    _ => false,
                })
        }) {
            return Some(*listener);
        }
        self.elements
            .iter()
            .filter(|(_, element)| element.widget.semantic_properties().focus_scope_autofocus)
            .map(|(raw, _)| ElementId(raw))
            .find_map(|scope| {
                focusable
                    .iter()
                    .copied()
                    .find(|candidate| self.is_descendant_or_self(*candidate, scope))
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
            policy,
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
                    .is_some_and(|anchor| self.is_descendant_or_self(anchor, id))
            {
                continue;
            }
            scan.excluded_focus |= properties.exclude_focus;
            // A `Text` label takes focus from a selection gesture only; it is
            // not a traversal stop (Flutter traverses to the area, not to
            // each registered paragraph).
            let node = external_node(element.widget.kind());
            let traversal_candidate =
                !scan.excluded_traversal && node.is_none_or(|node| !node.skip_traversal());
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
                    node.set_rect(bounds);
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
            let focus_children: &[ElementId] = match element.widget.kind() {
                WidgetKind::IndexedStack { index, .. } => index
                    .and_then(|slot| element.children.get(slot))
                    .map_or(&[], std::slice::from_ref),
                _ => &element.children,
            };

            for &child in focus_children.iter().rev() {
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
