//! Display-list painting, compositing, hit testing, and scrollbar geometry.

use super::*;

mod behavior;

use super::rendering::{image_fit_rects, image_repeat_destinations};
use super::text::{caret_geometry, selection_rects, text_field_display};

#[derive(Default)]
struct RawHitResult {
    elements: Vec<ElementId>,
    subtree_hit: bool,
    blocks_siblings: bool,
}

impl WidgetTree {
    #[must_use]
    pub fn paint(&mut self) -> DisplayList {
        let _phase_guard = self.guard_phase_root(FramePhase::Paint);
        if let Some(root) = self.root.and_then(|id| self.render_id(id)) {
            self.paint_render(root);
        }
        self.sync_transient_surface_partitions();
        let mut output = self.compositor.flatten();
        self.paint_semantics_debugger_overlay(&mut output);
        output
    }
    #[must_use]
    pub fn hit_test(&self, point: Offset) -> Option<RenderObjectId> {
        // Native transient input arrives in the owning view's logical
        // coordinate space and may legitimately lie outside every ancestor's
        // layout bounds. Test visible transient roots first so Clip::None popup
        // content remains interactive without weakening ordinary ancestor hit
        // testing for the rest of the tree.
        for (_, _, _, stack, _, popup) in self.transient_portal_entries().into_iter().rev() {
            let Some(bounds) = self.element_bounds(popup) else {
                continue;
            };
            if !bounds.contains(point) {
                continue;
            }
            let Some(render) = self.render_id(popup) else {
                continue;
            };
            let origin = self
                .element_bounds(stack)
                .map_or(Offset::ZERO, |bounds| bounds.origin);
            if let Some(hit) = self.hit_test_render(render, point, origin) {
                return Some(hit);
            }
        }
        self.root
            .and_then(|id| self.render_id(id))
            .and_then(|id| self.hit_test_render(id, point, Offset::ZERO))
    }

    /// Returns raw-input annotations in hit-test order (deepest/frontmost
    /// first). Raw annotations deliberately use their own traversal: a
    /// translucent raw region must still see a sibling behind an opaque
    /// visual child, while ordinary retained hit testing continues to return
    /// one target for buttons and text editing.
    pub(super) fn raw_hit_elements(&self, point: Offset) -> Vec<ElementId> {
        for (_, _, _, stack, _, popup) in self.transient_portal_entries().into_iter().rev() {
            let Some(bounds) = self.element_bounds(popup) else {
                continue;
            };
            if !bounds.contains(point) {
                continue;
            }
            let Some(render) = self.render_id(popup) else {
                continue;
            };
            let origin = self
                .element_bounds(stack)
                .map_or(Offset::ZERO, |bounds| bounds.origin);
            let result = self.collect_raw_hit_render(render, point, origin);
            if result.subtree_hit || !result.elements.is_empty() {
                return result.elements;
            }
        }
        let Some(root) = self.root.and_then(|id| self.render_id(id)) else {
            return Vec::new();
        };
        self.collect_raw_hit_render(root, point, Offset::ZERO)
            .elements
    }

    fn collect_raw_hit_render(
        &self,
        id: RenderObjectId,
        point: Offset,
        origin: Offset,
    ) -> RawHitResult {
        let Some(node) = self.renders.get(id.0) else {
            return RawHitResult::default();
        };
        let element = self.element_for_render(id);
        let kind = element
            .and_then(|element| self.elements.get(element.0))
            .map(|element| element.widget.kind());
        if matches!(kind, Some(WidgetKind::IgnorePointer { ignoring: true, .. }))
            || matches!(
                node.object.kind(),
                RenderKind::Visibility { visible: false, .. }
            )
        {
            return RawHitResult::default();
        }
        if matches!(node.object.kind(), RenderKind::Follower { .. })
            && !self.follower_content_visible(id)
        {
            return RawHitResult::default();
        }
        if matches!(
            kind,
            Some(WidgetKind::AbsorbPointer {
                absorbing: true,
                ..
            })
        ) {
            return RawHitResult {
                subtree_hit: true,
                blocks_siblings: true,
                ..RawHitResult::default()
            };
        }
        // A transform that opts out of hit-test conversion falls through
        // to the ordinary untransformed path below: hits land where the
        // child lays out, not where it paints. Painting and semantics
        // always follow the visual transform. Follower placement resolves
        // through the same shared projection, so hits land where the
        // content paints; hidden followers return above.
        if matches!(
            node.object.kind(),
            RenderKind::Transform {
                transform_hit_tests: true,
                ..
            } | RenderKind::Scale { .. }
                | RenderKind::Rotation { .. }
                | RenderKind::Follower { .. }
                | RenderKind::FittedBox { .. }
        ) {
            let current = origin + node.offset;
            let Some(local) = self
                .child_content_transform(id)
                .inverse_transform_point(point - current)
            else {
                return RawHitResult::default();
            };
            let child_size = node
                .children
                .first()
                .and_then(|child| self.renders.get(child.0))
                .map_or(node.size, |child| child.size);
            if !Rect::from_origin_size(Offset::ZERO, child_size).contains(local) {
                return RawHitResult::default();
            }
            let mut result = RawHitResult {
                subtree_hit: true,
                ..RawHitResult::default()
            };
            for child in node.children.iter().rev() {
                let child_result = self.collect_raw_hit_render(*child, local, Offset::ZERO);
                result.elements.extend(child_result.elements);
                result.blocks_siblings |= child_result.blocks_siblings;
                if child_result.blocks_siblings {
                    break;
                }
            }
            return result;
        }
        let current = self.hit_origin(id, node, origin);
        if !Rect::from_origin_size(current, node.size).contains(point) {
            return RawHitResult::default();
        }
        let child_origin = hit_child_origin(node.object.kind(), current);
        let hit_children = self.hit_children(id, node);
        let mut result = RawHitResult {
            subtree_hit: true,
            ..RawHitResult::default()
        };
        let mut child_hit = false;
        for child in hit_children.iter().rev() {
            let child_result = self.collect_raw_hit_render(*child, point, child_origin);
            child_hit |= child_result.subtree_hit;
            result.elements.extend(child_result.elements);
            if child_result.blocks_siblings {
                result.blocks_siblings = true;
                break;
            }
        }
        if let (Some(element), Some(WidgetKind::RawInput { kind, .. })) = (element, kind) {
            let include = kind.is_enabled()
                && (kind.hit_test_behavior() != crate::gestures::HitTestBehavior::DeferToChild
                    || child_hit);
            if include {
                result.elements.push(element);
                result.blocks_siblings =
                    kind.hit_test_behavior() == crate::gestures::HitTestBehavior::Opaque;
            }
        }
        result
    }
    pub(super) fn paint_render(&mut self, id: RenderObjectId) {
        with_recursive_tree_stack(|| {
            self.paint_render_inner(id);
        });
    }

    /// Repaints dirty pictures into their retained compositor layers. Clean
    /// subtrees cost one visit per node: placement and clipping live in the
    /// compositor, so this pass assembles no display list of its own.
    pub(super) fn paint_render_inner(&mut self, id: RenderObjectId) {
        let constraints = self.renders.get(id.0).and_then(|render| render.constraints);
        let _node_guard = self.guard_render(FramePhase::Paint, id, constraints);
        let node = self.render_live(id, "retained render must remain live");
        if matches!(
            node.object.kind(),
            RenderKind::Visibility { visible: false, .. }
        ) {
            return;
        }
        if node.dirty.contains(DirtyFlags::PAINT) {
            #[cfg(feature = "devtools")]
            let trace = self
                .element_for_render(id)
                .and_then(|element| self.devtools_trace_begin_element(element, TracePhase::Paint));
            self.repaint_pictures(id);
            #[cfg(feature = "devtools")]
            self.devtools_trace_end(trace);
        } else {
            self.diagnostics.display_lists_reused += 1;
        }
        for index in 0.. {
            let Some(&child) = self
                .render_live(id, "retained render must remain live")
                .children
                .get(index)
            else {
                break;
            };
            self.paint_render(child);
        }
    }

    fn repaint_pictures(&mut self, id: RenderObjectId) {
        let node = self.render_live(id, "retained render must remain live");
        let (kind, size) = (node.object.shared_kind(), node.size);
        let (picture_layer, focus_layer) =
            (node.object.layers.picture, node.object.layers.focus_picture);
        let mut picture = DisplayList::new();
        let focus_ring = self.paint_kind(id, &kind, size, &mut picture);
        let bounds = Rect::from_origin_size(Offset::ZERO, size);
        self.render_live_mut(id, "retained render must remain live")
            .dirty
            .remove(DirtyFlags::PAINT);
        if let Some(layer) = picture_layer {
            self.compositor.update_picture(layer, picture, bounds);
        }
        if let Some(layer) = focus_layer {
            let mut focus = DisplayList::new();
            if let Some(focus_ring) = focus_ring.filter(|color| color.alpha > 0) {
                let inset = 1.0;
                let ring_size = Size::new(
                    (size.width - 2. * inset).max(0.),
                    (size.height - 2. * inset).max(0.),
                );
                focus.push(PaintCommand::Border {
                    rrect: RRect::uniform(
                        Rect::from_origin_size(Offset::new(inset, inset), ring_size),
                        4.,
                    ),
                    border: Border::new(2.0, focus_ring),
                });
            }
            self.compositor.update_picture(layer, focus, bounds);
        }
        self.diagnostics.paints += 1;
        #[cfg(feature = "devtools")]
        if let Some(element) = self.element_for_render(id)
            && let Some(element) = self.elements.get_mut(element.0)
        {
            element.dev.paints += 1;
        }
    }

    /// Paints the opt-in SemanticsDebugger over the already flattened child
    /// scene. Semantic bounds are world-space snapshots, so this overlay does
    /// not participate in the child's retained transform/clip layers and
    /// remains visible when the child picture is reused.
    fn paint_semantics_debugger_overlay(&mut self, output: &mut DisplayList) {
        let Some((label_style, requested_limit)) = self.semantics_debugger_config() else {
            return;
        };
        let Some(root) = self.semantics.root() else {
            return;
        };
        let limit =
            requested_limit.min(crate::semantics_debugger::DEFAULT_SEMANTICS_DEBUGGER_NODE_LIMIT);
        if limit == 0 {
            return;
        }
        let mut stack = vec![(root, 0usize)];
        let mut painted = 0usize;
        let colors = [
            Color::rgba(220, 40, 40, 230),
            Color::rgba(35, 115, 220, 230),
            Color::rgba(35, 155, 75, 230),
            Color::rgba(210, 125, 25, 230),
            Color::rgba(145, 65, 190, 230),
            Color::rgba(20, 155, 155, 230),
        ];
        while let Some((id, depth)) = stack.pop() {
            if painted >= limit {
                break;
            }
            let Some(node) = self.semantics.node(id).cloned() else {
                continue;
            };
            painted = painted.saturating_add(1);
            let color = colors[depth % colors.len()];
            let bounds = node.bounds;
            if bounds.size.width > 0.0 || bounds.size.height > 0.0 {
                output.push(PaintCommand::Border {
                    rrect: RRect::uniform(bounds, 2.0),
                    border: Border::new(1.0, color),
                });
            }
            let message = semantics_debug_message(&node);
            if !message.is_empty() {
                self.paint_semantics_debug_label(output, &label_style, bounds, &message);
            }
            for child in node.children.iter().rev() {
                stack.push((*child, depth.saturating_add(1)));
            }
        }
    }

    pub(super) fn semantics_debugger_config(&self) -> Option<(TextStyle, usize)> {
        self.tracked.semantics_debuggers.iter().find_map(|id| {
            let element = self.elements.get(id.0)?;
            if let WidgetKind::SemanticsDebugger {
                label_style,
                max_nodes,
                ..
            } = element.widget.kind()
            {
                let mut parent = element.parent;
                while let Some(ancestor) = parent {
                    if self.elements.get(ancestor.0).is_some_and(|element| {
                        matches!(element.widget.kind(), WidgetKind::SemanticsDebugger { .. })
                    }) {
                        return None;
                    }
                    parent = self
                        .elements
                        .get(ancestor.0)
                        .and_then(|element| element.parent);
                }
                Some((TextStyle::clone(label_style), *max_nodes))
            } else {
                None
            }
        })
    }

    fn paint_semantics_debug_label(
        &mut self,
        output: &mut DisplayList,
        style: &TextStyle,
        bounds: Rect,
        message: &str,
    ) {
        let max_width = bounds.size.width.clamp(180.0, 480.0);
        let _external_call = self
            .recursion_diagnostics
            .external_call(text_call_label("TextEngine::layout_with_options", message));
        let layout = self.text_engine.layout_with_options(
            message,
            style,
            TextLayoutOptions::new(Some(max_width), TextAlign::Start)
                .soft_wrap(true)
                .max_lines(Some(3))
                .overflow(TextOverflow::Clip),
        );
        let size = Size::new(
            (layout.metrics.size.width + 4.0).min(max_width),
            (layout.metrics.size.height + 4.0).max(style.size + 4.0),
        );
        let origin = Offset::new(bounds.origin.x + 2.0, bounds.origin.y + 2.0);
        output.push(PaintCommand::Rect {
            rect: Rect::from_origin_size(origin, size),
            color: Color::rgba(255, 255, 255, 220),
        });
        output.push(PaintCommand::PushTransform {
            transform: CoreTransform::translation(Offset::new(origin.x + 2.0, origin.y + 2.0)),
        });
        for line in layout.lines.iter() {
            for run in line.runs.iter() {
                output.push(PaintCommand::GlyphRun {
                    run: run.clone(),
                    color: style.color,
                });
            }
        }
        output.push(PaintCommand::PopTransform);
    }
    pub(super) fn paint_scrollbar(
        &self,
        id: RenderObjectId,
        size: Size,
        controller: &ScrollController,
        cache: &mut DisplayList,
    ) {
        let style = controller.scrollbar_style();
        let geometry = scrollbar_geometry(size, controller, style);
        if !geometry.visible {
            return;
        }
        let scroll = self.scroll_state_live(id);
        if !controller.scrollbar_thumb_visibility() && !scroll.hovered && !scroll.dragging {
            return;
        }
        let thumb = style.thumb_color;
        if style.track_color.alpha > 0 {
            cache.push(PaintCommand::RRect {
                rrect: RRect::uniform(geometry.track, style.width * 0.5),
                brush: style.track_color.into(),
            });
        }
        if thumb.alpha > 0 {
            cache.push(PaintCommand::RRect {
                rrect: RRect::uniform(geometry.thumb, style.width * 0.5),
                brush: thumb.into(),
            });
        }
    }
    pub(super) fn hit_test_render(
        &self,
        id: RenderObjectId,
        point: Offset,
        origin: Offset,
    ) -> Option<RenderObjectId> {
        let node = self.renders.get(id.0)?;
        match self
            .element_for_render(id)
            .and_then(|element| self.elements.get(element.0))
            .map(|element| element.widget.kind())
        {
            Some(WidgetKind::IgnorePointer { ignoring: true, .. }) => return None,
            Some(WidgetKind::AbsorbPointer {
                absorbing: true, ..
            }) => return Some(id),
            _ => {}
        }
        if matches!(
            node.object.kind(),
            RenderKind::Transform {
                transform_hit_tests: true,
                ..
            } | RenderKind::Scale { .. }
                | RenderKind::Rotation { .. }
                | RenderKind::Follower { .. }
                | RenderKind::FittedBox { .. }
        ) {
            let current = origin + node.offset;
            let local = self
                .child_content_transform(id)
                .inverse_transform_point(point - current)?;
            let child_size = node
                .children
                .first()
                .and_then(|child| self.renders.get(child.0))
                .map_or(node.size, |child| child.size);
            if !Rect::from_origin_size(Offset::ZERO, child_size).contains(local) {
                return None;
            }
            for child in node.children.iter().rev() {
                if let Some(hit) = self.hit_test_render(*child, local, Offset::ZERO) {
                    return Some(hit);
                }
            }
            return None;
        }
        let current = self.hit_origin(id, node, origin);
        if !Rect::from_origin_size(current, node.size).contains(point) {
            return None;
        }
        if matches!(
            node.object.kind(),
            RenderKind::Visibility { visible: false, .. }
        ) {
            return None;
        }
        if matches!(node.object.kind(), RenderKind::Follower { .. })
            && !self.follower_content_visible(id)
        {
            return None;
        }
        let child_origin = hit_child_origin(node.object.kind(), current);
        let hit_children = self.hit_children(id, node);
        for child in hit_children.iter().rev() {
            if let Some(hit) = self.hit_test_render(*child, point, child_origin) {
                return Some(hit);
            }
        }
        match self
            .element_for_render(id)
            .and_then(|element| self.elements.get(element.0))
            .map(|element| element.widget.kind())
        {
            Some(WidgetKind::Gesture { behavior, .. })
                if *behavior == crate::gestures::HitTestBehavior::DeferToChild =>
            {
                None
            }
            Some(WidgetKind::RawInput { kind, .. })
                if kind.hit_test_behavior() == crate::gestures::HitTestBehavior::DeferToChild =>
            {
                None
            }
            // Positioned supplies parent layout data; it has no independent
            // hit surface when its child (for example IgnorePointer) declines.
            Some(WidgetKind::Banner { .. } | WidgetKind::Positioned { .. }) => None,
            _ => Some(id),
        }
    }
    /// Position of `node` in its parent's hit-test space, including the
    /// translations that place translated and persistent-header content.
    fn hit_origin(&self, id: RenderObjectId, node: &RenderNode, origin: Offset) -> Offset {
        match node.object.kind() {
            RenderKind::Translate { controller } => origin + node.offset + controller.offset(),
            RenderKind::PersistentHeader {
                controller,
                axis,
                reverse,
                pinned,
            } => {
                origin
                    + node.offset
                    + self.persistent_header_translation(id, controller, *axis, *reverse, *pinned)
            }
            _ => origin + node.offset,
        }
    }

    /// Children in paint order, hit-tested in reverse. Sliver viewports paint
    /// pinned overlays above flow children; an indexed stack shows one child.
    fn hit_children<'a>(
        &self,
        id: RenderObjectId,
        node: &'a RenderNode,
    ) -> std::borrow::Cow<'a, [RenderObjectId]> {
        match node.object.kind() {
            RenderKind::SliverViewport { .. } => {
                let overlays = self
                    .element_for_render(id)
                    .and_then(|element| self.elements.get(element.0))
                    .and_then(|element| element.sliver_overlay_ids().cloned())
                    .unwrap_or_default();
                let mut ordered = node.children.clone();
                ordered.sort_by_key(|child| {
                    let child_element = self.element_for_render(*child);
                    child_element
                        .and_then(|element| self.elements.get(element.0))
                        .and_then(|element| element.parent)
                        .and_then(|parent| self.elements.get(parent.0))
                        .and_then(|parent| {
                            parent
                                .children
                                .iter()
                                .position(|candidate| Some(*candidate) == child_element)
                                .and_then(|slot| parent.sliver_child_ids().get(slot))
                        })
                        .map_or(0, |child_id| usize::from(overlays.contains(child_id)))
                });
                ordered.into()
            }
            RenderKind::IndexedStack { index, .. } => node
                .children
                .get(*index)
                .map(std::slice::from_ref)
                .unwrap_or_default()
                .into(),
            _ => node.children.as_slice().into(),
        }
    }
    pub(super) fn scrollbar_controller_and_geometry(
        &self,
        render: RenderObjectId,
    ) -> Option<(ScrollController, ScrollbarGeometry)> {
        let node = self.renders.get(render.0)?;
        let controller = match node.object.kind() {
            RenderKind::Scroll { controller, .. } => controller.clone(),
            RenderKind::SliverViewport { config } => config.controller.clone(),
            _ => return None,
        };
        let mut geometry = scrollbar_geometry(node.size, &controller, controller.scrollbar_style());
        let origin = self.render_viewport_origin(render);
        geometry.track.origin = geometry.track.origin + origin;
        geometry.thumb.origin = geometry.thumb.origin + origin;
        Some((controller, geometry))
    }
    pub(super) fn scrollbar_local_geometry(
        &self,
        render: RenderObjectId,
    ) -> Option<(ScrollController, ScrollbarGeometry)> {
        let node = self.renders.get(render.0)?;
        let controller = match node.object.kind() {
            RenderKind::Scroll { controller, .. } => controller.clone(),
            RenderKind::SliverViewport { config } => config.controller.clone(),
            _ => return None,
        };
        Some((
            controller.clone(),
            scrollbar_geometry(node.size, &controller, controller.scrollbar_style()),
        ))
    }
    pub(super) fn scrollbar_local_point(
        &self,
        render: RenderObjectId,
        point: Offset,
    ) -> Option<Offset> {
        self.render_world_transform(render)
            .inverse_transform_point(point)
    }
    /// Render objects of mounted scroll views, in render-arena order.
    pub(super) fn scroll_view_renders(&self) -> Vec<RenderObjectId> {
        let mut renders: Vec<_> = self
            .tracked
            .scroll_views
            .iter()
            .filter_map(|id| self.elements.get(id.0).map(|element| element.render))
            .collect();
        renders.sort_unstable_by_key(|render| render.0);
        renders
    }

    pub(super) fn scrollbar_at(&self, point: Offset) -> Option<RenderObjectId> {
        self.scroll_view_renders().into_iter().find(|&render| {
            self.scrollbar_local_geometry(render)
                .is_some_and(|(_, geometry)| {
                    self.scrollbar_local_point(render, point)
                        .is_some_and(|point| geometry.visible && geometry.track.contains(point))
                })
        })
    }
}

fn semantics_debug_message(node: &SemanticNode) -> String {
    let mut fields = vec![semantic_role_name(node.role).to_owned()];
    if let Some(label) = node.label.as_deref().filter(|label| !label.is_empty()) {
        fields.push(format!("label={:?}", bounded_debug_text(label, 120)));
    }
    if let Some(value) = node.value.as_deref().filter(|value| !value.is_empty()) {
        fields.push(format!("value={:?}", bounded_debug_text(value, 120)));
    }
    if let Some(description) = node
        .description
        .as_deref()
        .filter(|description| !description.is_empty())
    {
        fields.push(format!(
            "description={:?}",
            bounded_debug_text(description, 120)
        ));
    }
    if let Some(checked) = node.state.checked {
        fields.push(format!("checked={checked}"));
    }
    if semantic_role_is_interactive(node.role) && !node.state.enabled {
        fields.push("disabled".to_owned());
    }
    if node.state.read_only {
        fields.push("readOnly".to_owned());
    }
    if let Some(index) = node.state.item_index {
        let position = index.saturating_add(1);
        fields.push(match node.state.set_size {
            Some(size) => format!("position={position}/{size}"),
            None => format!("position={position}"),
        });
    }
    if !node.actions.is_empty() {
        fields.push(format!("actions={:?}", node.actions));
    }
    fields.join(" ")
}

fn semantic_role_name(role: SemanticRole) -> &'static str {
    match role {
        SemanticRole::Button => "button",
        SemanticRole::Text => "text",
        SemanticRole::TextField => "text field",
        SemanticRole::TextArea => "text area",
        SemanticRole::List => "list",
        SemanticRole::ListItem => "list item",
        SemanticRole::ScrollView => "scroll view",
        SemanticRole::GenericContainer => "container",
        SemanticRole::Checkbox => "checkbox",
        SemanticRole::Radio => "radio",
        SemanticRole::Switch => "switch",
        SemanticRole::Slider => "slider",
        SemanticRole::Menu => "menu",
        SemanticRole::Dialog => "dialog",
        SemanticRole::Image => "image",
        SemanticRole::Heading => "heading",
        SemanticRole::Link => "link",
        SemanticRole::Group => "group",
        SemanticRole::Tab => "tab",
        SemanticRole::TabList => "tab list",
        SemanticRole::TabPanel => "tab panel",
        SemanticRole::MenuItem => "menu item",
        SemanticRole::ProgressBar => "progress bar",
        SemanticRole::Meter => "meter",
        SemanticRole::SearchField => "search field",
    }
}

fn semantic_role_is_interactive(role: SemanticRole) -> bool {
    matches!(
        role,
        SemanticRole::Button
            | SemanticRole::TextField
            | SemanticRole::TextArea
            | SemanticRole::Checkbox
            | SemanticRole::Radio
            | SemanticRole::Switch
            | SemanticRole::Slider
            | SemanticRole::Link
            | SemanticRole::Tab
            | SemanticRole::MenuItem
            | SemanticRole::SearchField
    )
}

fn bounded_debug_text(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut result = text.chars().take(limit).collect::<String>();
    result.push('…');
    result
}

/// Origin of a node's children in hit-test space: scroll views translate
/// their content by the scroll offset.
fn hit_child_origin(kind: &RenderKind, current: Offset) -> Offset {
    match kind {
        RenderKind::Scroll {
            controller,
            axis,
            reverse,
            ..
        } => current + scroll_translation(controller, *axis, *reverse),
        RenderKind::SliverViewport { config } => {
            current + scroll_translation(&config.controller, config.axis, config.reverse)
        }
        _ => current,
    }
}
