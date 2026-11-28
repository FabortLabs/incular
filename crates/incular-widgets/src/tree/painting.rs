//! Display-list painting, compositing, hit testing, and scrollbar geometry.

use super::*;
use std::fmt::Write as _;

mod behavior;

use super::rendering::{image_fit_rects_scaled, image_repeat_destinations};
use super::text::{
    selection_rects, text_field_caret_geometry, text_field_content_top, text_field_display_offset,
    with_text_field_display,
};

/// RenderViewport paints only slivers with a positive main-axis paint
/// extent. Its cache may still contain children at a zero viewport extent.
pub(super) fn viewport_has_no_paint_extent(kind: &RenderKind, size: Size) -> bool {
    matches!(kind, RenderKind::SliverViewport { config } if config.axis.main_extent(size) <= 0.)
}

/// The paintable subset. Hidden viewport, IndexedStack and paragraph
/// children keep retained state and pending paint until reattached.
fn visible_paint_children(node: &RenderNode) -> &[RenderObjectId] {
    if viewport_has_no_paint_extent(node.object.kind(), node.size) {
        &[]
    } else if let Some(order) = node.flow_paint_order().or_else(|| node.inline_visible_children()) {
        order
    } else if let RenderKind::IndexedStack { index, .. } = node.object.kind() {
        index
            .and_then(|slot| node.children.get(slot))
            .map(std::slice::from_ref)
            .unwrap_or_default()
    } else {
        &node.children
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SpanPointerHit {
    pub(super) element: ElementId,
    pub(super) span: MouseSpanTarget,
    raw_before: usize,
}

pub(super) enum MouseHit<'a> {
    Region(ElementId),
    Span(&'a SpanPointerHit),
}

/// Both hit-test results for one pointer position, from one walk of the
/// retained tree: see [`WidgetTree::pointer_hit_test`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PointerHitTest {
    raw: Vec<ElementId>,
    pointer_spans: Vec<SpanPointerHit>,
    target: Option<RenderObjectId>,
    // Translucent siblings and pointer-only taps share the ordinary arena.
    coincident_gestures: Vec<ElementId>,
}

impl PointerHitTest {
    /// The retained hit target: the render [`WidgetTree::hit_test`] returns
    /// for the same position.
    #[must_use]
    pub const fn target(&self) -> Option<RenderObjectId> {
        self.target
    }

    /// Literal rich-text spans under this point, frontmost first. A hit
    /// follows a grapheme's whole advance, not just its visible ink.
    /// Mouse cursors and callbacks are not required for a span to be hit.
    pub fn text_span_targets(&self) -> impl Iterator<Item = &incular_text::TextSpan> {
        self.pointer_spans.iter().map(|hit| hit.span.text_span())
    }

    pub(super) fn coincident_gestures(&self) -> &[ElementId] {
        &self.coincident_gestures
    }

    pub(super) fn mouse_hits(&self) -> impl Iterator<Item = MouseHit<'_>> {
        let mut raw = self.raw.iter().copied().enumerate();
        let mut region = raw.next();
        let mut spans = self
            .pointer_spans
            .iter()
            .filter(|hit| hit.span.annotation().is_some())
            .peekable();
        std::iter::from_fn(move || {
            if let Some(span) = spans.peek()
                && region.is_none_or(|(index, _)| span.raw_before <= index)
            {
                return spans.next().map(MouseHit::Span);
            }
            let (_, element) = region?;
            region = raw.next();
            Some(MouseHit::Region(element))
        })
    }

    /// Raw-input annotations in hit-test order, deepest/frontmost first.
    pub(super) fn raw(&self) -> &[ElementId] {
        &self.raw
    }
}

thread_local! {
    static SCROLL_VIEW_SCRATCH: std::cell::RefCell<Vec<RenderObjectId>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Owning iterator over scroll-view render ids that returns its buffer to
/// the thread-local pool when dropped.
pub(super) struct ScrollViewRenders {
    renders: Vec<RenderObjectId>,
    next: usize,
}

impl Iterator for ScrollViewRenders {
    type Item = RenderObjectId;
    fn next(&mut self) -> Option<RenderObjectId> {
        let render = self.renders.get(self.next).copied()?;
        self.next += 1;
        Some(render)
    }
}

impl Drop for ScrollViewRenders {
    fn drop(&mut self) {
        let mut renders = std::mem::take(&mut self.renders);
        renders.clear();
        let _ = SCROLL_VIEW_SCRATCH.try_with(|scratch| {
            let mut slot = scratch.borrow_mut();
            if slot.capacity() < renders.capacity() {
                *slot = renders;
            }
        });
    }
}

/// Whether a transient popup's raw hit result ends the search. An overlay
/// built from its layout info fills the view, so only its content counts:
/// a retained target or a raw annotation.
fn claims_overlay_hit(
    marker: &crate::transient::TransientPortalMarker,
    result: &RawHitResult,
) -> bool {
    if marker.layout_slot.is_some() {
        result.content_hit
    } else {
        result.subtree_hit || !result.elements.is_empty()
    }
}

/// Kinds whose hit-test behavior can claim their own bounds whatever their
/// children do.
fn claims_own_bounds(kind: Option<&WidgetKind>) -> bool {
    matches!(
        kind,
        Some(
            WidgetKind::Gesture { .. }
                | WidgetKind::RawInput { .. }
                | WidgetKind::Scroll { .. }
                | WidgetKind::SliverViewport { .. }
                | WidgetKind::ListWheelScrollView { .. }
        )
    )
}

/// Kinds that paint a surface of their own and therefore hit themselves even
/// where their children miss, like Flutter's `RenderDecoratedBox`,
/// `RenderColoredBox`, `RenderCustomPaint`, `RenderParagraph` and
/// `RenderImage`. Layout and proxy wrappers (`Stack`, `Align`, `Padding`,
/// `SizedBox`, transforms, effects, ...) are not among them: with children
/// they are hit only through those children, so a full-size wrapper around
/// a small child never swallows taps meant for the siblings behind it.
fn paints_own_hit_surface(kind: Option<&WidgetKind>) -> bool {
    // A transparent `Box` is the layout placeholder of a childless
    // `SizedBox`, `ConstrainedBox` or `Container` (Flutter's childless
    // `RenderConstrainedBox`, which never hits itself). ColoredBox is
    // separately opaque to hit testing regardless of its paint alpha.
    if let Some(WidgetKind::Box { color, .. }) = kind {
        return color.alpha > 0;
    }
    matches!(
        kind,
        Some(
            WidgetKind::Shape { .. }
                | WidgetKind::CustomPaint { .. }
                | WidgetKind::Colored { .. }
                | WidgetKind::Decorated { .. }
                | WidgetKind::Button(_)
                | WidgetKind::Text { .. }
                | WidgetKind::SelectableText { .. }
                | WidgetKind::Image { .. }
                | WidgetKind::TextField(_)
        )
    )
}

// Flutter Size.contains is half-open at the lower and right edges.
fn half_open_hit(size: Size, local: Offset) -> bool {
    local.x >= 0.0 && local.y >= 0.0 && local.x < size.width && local.y < size.height
}

fn hit_box_contains(
    kind: &RenderKind,
    widget: Option<&WidgetKind>,
    size: Size,
    origin: Offset,
    point: Offset,
) -> bool {
    if matches!(
        kind,
        RenderKind::Colored { .. } | RenderKind::CustomPaint { .. } | RenderKind::Image { .. }
    ) || matches!(widget, Some(WidgetKind::RawInput { kind, .. }) if kind.mouse_region_opaque().is_some())
    {
        // Flutter's RenderBox covers its half-open layout box, even if
        // its fill is transparent or the image has no decoded source.
        half_open_hit(size, point - origin)
    } else {
        Rect::from_origin_size(origin, size).contains(point)
    }
}

/// Whether a node whose bounds hold the point is hit itself once its
/// children declined it: only when its hit-test behavior claims its bounds
/// or it paints its own surface. A childless layout (an empty `Column`,
/// `Stack` or sized placeholder) is not hit, like Flutter's render boxes
/// whose `hitTestSelf` is false.
fn self_hit_after_children(kind: Option<&WidgetKind>, size: Size, local: Offset) -> bool {
    if let Some(WidgetKind::CustomPaint { painter, display_list, foreground, .. }) = kind {
        return painter.as_ref().map_or(!display_list.is_empty(), |painter| {
            painter.hit_test(local).unwrap_or(!foreground)
        });
    }
    if let Some(WidgetKind::Decorated {
        hit_decoration: Some(decoration),
        ..
    }) = kind
    {
        // RenderDecoratedBox.hitTestSelf is geometry, not a content clip.
        // A child can still accept a point outside the painted shape.
        return decoration.hit_test(size, local);
    }
    retained_self_hit(kind) && (claims_own_bounds(kind) || paints_own_hit_surface(kind))
}

#[derive(Default)]
pub(super) struct RawHitResult {
    elements: Vec<ElementId>,
    pointer_spans: Vec<SpanPointerHit>,
    coincident_gestures: Vec<ElementId>,
    subtree_hit: bool,
    /// A render target accepted the hit, not just a layout box containing
    /// the point or a translucent annotation adding itself to the path.
    accepts_hit: bool,
    /// Something in the subtree was hit itself: a leaf, or a widget whose
    /// hit-test behavior claims its bounds. A container's bounds alone do
    /// not count.
    pub(super) content_hit: bool,
    blocks_siblings: bool,
}

fn paragraph_pointer_annotation(
    node: &RenderNode,
    ranges: &[ParagraphMouseRange],
    local: Offset,
) -> Option<usize> {
    let layout = node.text_layout()?;
    let row = layout.lines.partition_point(|line| line.top <= local.y).checked_sub(1)?;
    let line = &layout.lines[row];
    let cluster = line.clusters.iter().find(|span| span.left <= local.x && local.x < span.right)?;
    let byte = cluster.start as usize;
    if byte >= layout.retained_source_end() {
        return None;
    }
    let index = ranges.partition_point(|span| span.source.end <= byte);
    let range = ranges.get(index)?;
    if !range.source.contains(&byte) {
        return None;
    }

    // Paragraph hit-testing uses glyph bounds, not the whole line box.
    // In particular, inter-line spacing should not trigger the cursor.
    let (top, bottom) = layout.font_run_at(byte).map_or(
        (line.top, line.top + line.height),
        |run| {
            let top = line.baseline - run.caret_ascent;
            (top, top + run.caret_height)
        },
    );
    (top <= local.y && local.y < bottom).then_some(index)
}

impl WidgetTree {
    /// Paints a runtime scene and checks Flutter's invalid nested link
    /// diagnostic in debug builds. Independent sibling links still use the
    /// compositor's order-independent publication.
    pub fn try_paint(&mut self) -> Result<DisplayList, TreeError> {
        let list = self.paint();
        if self.compositor.has_enclosed_linked_leader() {
            return Err(TreeError::LeaderAfterFollower);
        }
        Ok(list)
    }

    /// Toggles Flutter-style focus outlines in debug builds. Disabled by
    /// default; this never affects layout, focus or accessibility.
    pub fn set_debug_paint_focus_boxes(&mut self, enabled: bool) -> bool {
        #[cfg(debug_assertions)]
        {
            if self.debug_focus_boxes.as_ref().is_some_and(|state| state.enabled) != enabled {
                self.debug_focus_boxes.get_or_insert_with(Box::default).enabled = enabled;
                return true;
            }
        }
        #[cfg(not(debug_assertions))]
        let _ = enabled;
        false
    }

    #[must_use]
    pub fn paint(&mut self) -> DisplayList {
        let _phase_guard = self.guard_phase_root(FramePhase::Paint);
        self.flush_sliver_reclips();
        self.flush_custom_painter_repaints();
        self.flush_proxy_repaints();
        #[cfg(debug_assertions)]
        self.sync_debug_focus_boxes();
        if let Some(root) = self.root.and_then(|id| self.render_id(id)) {
            self.paint_render(root);
        }
        self.sync_transient_surface_partitions();
        let scene = self.compositor.flatten();
        // Open popups paint above everything after their anchors, as
        // entries of Flutter's `Overlay` do. Only a scene with a popup has
        // anything to lift, and an unchanged scene is lifted once.
        let mut output = if !self.compositor.has_surface_partitions() {
            self.lifted_scene = None;
            scene
        } else if let Some((source, lifted)) = &self.lifted_scene
            && source.shares_buffer(&scene)
        {
            lifted.clone()
        } else {
            let lifted = scene.clone().lift_surface_partitions();
            self.lifted_scene = Some((scene, lifted.clone()));
            lifted
        };
        self.paint_semantics_debugger_overlay(&mut output);
        if !self.view_transitions.is_empty() {
            let size = self
                .root
                .and_then(|root| self.element_bounds(root))
                .map_or(Size::ZERO, |bounds| bounds.size);
            crate::dx::view_transition::paint(
                &mut self.view_transitions,
                &mut self.compositor,
                &mut output,
                size,
            );
        }
        output
    }
    /// Repaints the shared paint region containing a non-painting proxy.
    /// Deduplicated at the boundary so two updates in one frame replay it
    /// only once; deeper explicit RepaintBoundaries keep their own pictures.
    pub(super) fn queue_proxy_repaint(&mut self, mut id: RenderObjectId) {
        while let Some(node) = self.renders.get(id.0) {
            if matches!(node.object.kind(), RenderKind::RepaintBoundary) {
                break;
            }
            let Some(parent) = node.parent else {
                break;
            };
            id = parent;
        }
        if self.proxy_repaint_regions.insert(id) {
            self.propagate_subtree_paint(id);
        }
    }

    fn flush_proxy_repaints(&mut self) {
        if self.proxy_repaint_regions.is_empty() {
            return;
        }
        let mut regions = std::mem::take(&mut self.proxy_repaint_regions);
        let mut work = Vec::new();
        for region in regions.drain() {
            work.push(region);
            while let Some(id) = work.pop() {
                let Some(node) = self.renders.get_mut(id.0) else {
                    continue;
                };
                if node.parked
                    || (id != region && matches!(node.object.kind(), RenderKind::RepaintBoundary))
                {
                    continue;
                }
                node.subtree_needs_paint = true;
                if node.object.layers.picture.is_some()
                    || node.object.layers.foreground_picture.is_some()
                {
                    node.dirty.insert(DirtyFlags::PAINT);
                }
                work.extend(visible_paint_children(node));
            }
        }
        self.proxy_repaint_regions = regions;
    }

    #[must_use]
    pub fn hit_test(&self, point: Offset) -> Option<RenderObjectId> {
        // Native transient input arrives in the owning view's logical
        // coordinate space and may legitimately lie outside every ancestor's
        // layout bounds. Test visible transient roots first so Clip::None popup
        // content remains interactive without weakening ordinary ancestor hit
        // testing for the rest of the tree.
        for (_, marker, _, stack, _, popup) in self.transient_portal_entries().into_iter().rev() {
            let Some(bounds) = self.element_bounds(popup) else {
                continue;
            };
            if !bounds.contains(point) {
                continue;
            }
            let Some(render) = self.render_id(popup) else {
                continue;
            };
            let origin = self.transient_hit_parent_origin(stack, render);
            if let Some(hit) = self.overlay_hit_target(&marker, render, point, origin) {
                return Some(hit);
            }
        }
        self.root
            .and_then(|id| self.render_id(id))
            .and_then(|id| self.hit_test_render(id, point, Offset::ZERO))
    }

    /// The retained target in a transient popup. An overlay built from its
    /// layout info fills the view, so it has one only where its content is
    /// hit.
    fn overlay_hit_target(
        &self,
        marker: &crate::transient::TransientPortalMarker,
        render: RenderObjectId,
        point: Offset,
        origin: Offset,
    ) -> Option<RenderObjectId> {
        if marker.layout_slot.is_none() {
            return self.hit_test_render(render, point, origin);
        }
        let (result, target) = self.collect_pointer_hit_render(render, point, origin, true, true);
        result.content_hit.then_some(target).flatten()
    }

    /// Hit-tests `point` once for both pointer routes: the raw-input
    /// annotations in hit-test order and the retained target that
    /// [`Self::hit_test`] returns. One pointer event shares the result
    /// between mouse regions, listeners, tap regions, raw recognizers, text
    /// editing and buttons instead of walking the tree for each of them.
    #[must_use]
    pub fn pointer_hit_test(&self, point: Offset) -> PointerHitTest {
        let mut raw = None;
        let mut pointer_spans = Vec::new();
        let mut target = None;
        let mut coincident_gestures = Vec::new();
        for (_, marker, _, stack, _, popup) in self.transient_portal_entries().into_iter().rev() {
            if raw.is_some() && target.is_some() {
                break;
            }
            let Some(bounds) = self.element_bounds(popup) else {
                continue;
            };
            if !bounds.contains(point) {
                continue;
            }
            let Some(render) = self.render_id(popup) else {
                continue;
            };
            let origin = self.transient_hit_parent_origin(stack, render);
            if raw.is_none() {
                let precise = marker.layout_slot.is_some();
                let (result, hit) = self.collect_pointer_hit_render(
                    render,
                    point,
                    origin,
                    target.is_none(),
                    precise,
                );
                if claims_overlay_hit(&marker, &result) {
                    raw = Some(result.elements);
                    pointer_spans = result.pointer_spans;
                    coincident_gestures = result.coincident_gestures;
                    target = target.or(hit);
                } else if !precise {
                    target = target.or(hit);
                }
            } else {
                target = self.overlay_hit_target(&marker, render, point, origin);
            }
        }
        if (raw.is_none() || target.is_none())
            && let Some(root) = self.root.and_then(|id| self.render_id(id))
        {
            if raw.is_none() {
                let (result, hit) = self.collect_pointer_hit_render(
                    root,
                    point,
                    Offset::ZERO,
                    target.is_none(),
                    false,
                );
                raw = Some(result.elements);
                pointer_spans = result.pointer_spans;
                coincident_gestures = result.coincident_gestures;
                target = target.or(hit);
            } else {
                target = self.hit_test_render(root, point, Offset::ZERO);
            }
        }
        PointerHitTest {
            raw: raw.unwrap_or_default(),
            pointer_spans,
            target,
            coincident_gestures,
        }
    }

    /// Whether the view's pointer hit path contains a native platform
    /// surface. A translucent target in front does not hide the answer.
    #[must_use]
    pub fn hit_test_contains_platform_view(&self, point: Offset) -> bool {
        self.raw_hit_elements(point).into_iter().any(|id| {
            let Some(element) = self.elements.get(id.0) else {
                return false;
            };
            let WidgetKind::RawInput { kind, .. } = element.widget.kind() else {
                return false;
            };
            matches!(
                &**kind,
                RawInputKind::MetaData { data, behavior }
                    if *behavior != crate::gestures::HitTestBehavior::DeferToChild
                        && data.is::<crate::platform_view::PlatformViewMarker>()
            )
        })
    }

    /// Flutter's `RenderMetaData` lookup on a hit-test result: the data of
    /// every [`MetaData`](crate::MetaData) under `point`, deepest and
    /// frontmost first, in the same order pointer listeners receive events.
    #[must_use]
    pub fn hit_test_meta_data(&self, point: Offset) -> Vec<Rc<dyn std::any::Any>> {
        self.raw_hit_elements(point)
            .into_iter()
            .filter_map(
                |element| match self.elements.get(element.0)?.widget.kind() {
                    WidgetKind::RawInput { kind, .. } => match &**kind {
                        RawInputKind::MetaData { data, .. } => Some(data.clone()),
                        _ => None,
                    },
                    _ => None,
                },
            )
            .collect()
    }

    /// Returns raw-input annotations in hit-test order (deepest/frontmost
    /// first). Raw annotations deliberately use their own traversal: a
    /// translucent raw region must still see a sibling behind an opaque
    /// visual child, while ordinary retained hit testing continues to return
    /// one target for buttons and text editing.
    pub(super) fn raw_hit_elements(&self, point: Offset) -> Vec<ElementId> {
        for (_, marker, _, stack, _, popup) in self.transient_portal_entries().into_iter().rev() {
            let Some(bounds) = self.element_bounds(popup) else {
                continue;
            };
            if !bounds.contains(point) {
                continue;
            }
            let Some(render) = self.render_id(popup) else {
                continue;
            };
            let origin = self.transient_hit_parent_origin(stack, render);
            let (result, _) = self.collect_pointer_hit_render(
                render,
                point,
                origin,
                false,
                marker.layout_slot.is_some(),
            );
            if claims_overlay_hit(&marker, &result) {
                return result.elements;
            }
        }
        let Some(root) = self.root.and_then(|id| self.render_id(id)) else {
            return Vec::new();
        };
        self.collect_pointer_hit_render(root, point, Offset::ZERO, false, false)
            .0
            .elements
    }

    /// Hit-tests a single render subtree in its own coordinates. Unlike a
    /// window-wide hit test, an overlapping sibling cannot drop this root.
    pub(super) fn raw_hit_elements_within(
        &self,
        root: ElementId,
        point: Offset,
    ) -> Option<Vec<ElementId>> {
        let render = self.render_id(root)?;
        let node = self.renders.get(render.0)?;
        let local = self
            .render_world_transform(render)
            .inverse_transform_point(point)?;
        if !Rect::from_origin_size(Offset::ZERO, node.size).contains(local) {
            return None;
        }
        // The point is already relative to this node's origin, so
        // cancel the layout offset the subtree traversal would add.
        Some(
            self.collect_pointer_hit_render(
                render,
                local,
                Offset::ZERO - node.offset,
                false,
                false,
            )
            .0
            .elements,
        )
    }

    /// Collects the raw annotations under `point` and, while `want_target`
    /// holds, the retained target in the same visit. The target is exactly
    /// what [`Self::hit_test_render`] returns for this subtree: where the raw
    /// route stops early (a blocking sibling, a hidden follower) the rest of
    /// the retained search continues on its own.
    pub(super) fn collect_pointer_hit_render(
        &self,
        id: RenderObjectId,
        point: Offset,
        origin: Offset,
        want_target: bool,
        precise: bool,
    ) -> (RawHitResult, Option<RenderObjectId>) {
        let Some(node) = self.renders.get(id.0) else {
            return (RawHitResult::default(), None);
        };
        if !precise && self.view_space_overlay_roots.contains(&id) {
            return (RawHitResult::default(), None);
        }
        if self.misses_outside_bounds(id, node, point, origin) {
            return (RawHitResult::default(), None);
        }
        // A view-filling overlay reached from an ancestor takes only hits on
        // its content, as when it is tested on its own.
        if !precise && self.layout_overlay_popups.contains(&id) {
            let (result, target) =
                self.collect_pointer_hit_render(id, point, origin, want_target, true);
            return if result.content_hit {
                (result, target)
            } else {
                (RawHitResult::default(), None)
            };
        }
        let element = self.element_for_render(id);
        let kind = element
            .and_then(|element| self.elements.get(element.0))
            .map(|element| element.widget.kind());
        if matches!(kind, Some(WidgetKind::IgnorePointer { ignoring: true, .. }))
            || matches!(
                node.object.kind(),
                RenderKind::Visibility {
                    visible: false,
                    interactive: false,
                    ..
                }
            )
            || flow_child_hidden(node.object.kind())
        {
            return (RawHitResult::default(), None);
        }
        if matches!(node.object.kind(), RenderKind::Follower { .. })
            && !self.follower_content_visible(id)
        {
            return (RawHitResult::default(), None);
        }
        if matches!(
            kind,
            Some(WidgetKind::AbsorbPointer {
                absorbing: true,
                ..
            })
        ) {
            if !half_open_hit(node.size, point - self.hit_origin(id, node, origin)) {
                return (RawHitResult::default(), None);
            }
            return (
                RawHitResult {
                    subtree_hit: true,
                    accepts_hit: true,
                    content_hit: true,
                    blocks_siblings: true,
                    ..RawHitResult::default()
                },
                want_target.then_some(id),
            );
        }
        // Positioned is parent data in Flutter, not an intervening hit
        // boundary. Its child may paint (and receive hits) outside the
        // nominal position while still inside the enclosing Stack.
        if matches!(node.object.kind(), RenderKind::Positioned { .. }) {
            let mut result = RawHitResult::default();
            let (_, target) = self.collect_pointer_hit_children(
                &node.children,
                point,
                origin + node.offset,
                want_target,
                precise,
                &mut result,
            );
            return (result, target);
        }
        if delegates_transformed_hits(node.object.kind()) {
            let Some((local, claims_bounds)) =
                self.transformed_hit_position(id, node, point, origin)
            else {
                return (RawHitResult::default(), None);
            };
            let mut result = RawHitResult {
                subtree_hit: claims_bounds,
                ..RawHitResult::default()
            };
            let (child_hit, target) = self.collect_pointer_hit_children(
                &node.children,
                local,
                Offset::ZERO,
                want_target,
                precise,
                &mut result,
            );
            result.subtree_hit |= child_hit;
            return (result, target);
        }
        let current = self.hit_origin(id, node, origin);
        if !hit_box_contains(node.object.kind(), kind, node.size, current, point) {
            return (RawHitResult::default(), None);
        }
        let child_origin = hit_child_origin(node.object.kind(), current);
        let hit_children = self.hit_children(id, node);
        let mut result = RawHitResult {
            subtree_hit: true,
            ..RawHitResult::default()
        };
        let (child_hit, mut target) = self.collect_pointer_hit_children(
            &hit_children,
            point,
            child_origin,
            want_target,
            precise,
            &mut result,
        );
        if !result.accepts_hit
            && let (Some(element), Some(WidgetKind::Text { pointer_spans: Some(ranges), .. })) =
                (element, kind)
            && let Some(index) = paragraph_pointer_annotation(node, &ranges.ranges, point - current)
        {
            result.pointer_spans.push(SpanPointerHit {
                element,
                span: MouseSpanTarget { paragraph: ranges.clone(), index },
                raw_before: result.elements.len(),
            });
            // A hit on a literal span is a real paragraph child hit.
            result.blocks_siblings = true;
        }
        let self_hit = self_hit_after_children(kind, node.size, point - current);
        if want_target && target.is_none() {
            target = self_hit.then_some(id);
        }
        if let (Some(element), Some(WidgetKind::RawInput { kind, .. })) = (element, kind) {
            use crate::gestures::HitTestBehavior::{DeferToChild, Opaque};
            let behavior = kind.hit_test_behavior();
            let annotation_child_hit = if kind.mouse_region_opaque().is_some()
                || matches!(
                    &**kind,
                    RawInputKind::TapRegionSurface
                        | RawInputKind::TapRegion { .. }
                        | RawInputKind::TextFieldTapRegion { .. }
                )
            {
                // These annotations defer to a child that actually
                // accepted the hit, not to a proxy that only occupies
                // the point over a non-hitting child.
                result.accepts_hit
            } else {
                child_hit
            };
            let include = kind.is_enabled() && (behavior != DeferToChild || annotation_child_hit);
            if include {
                result.elements.push(element);
                let hit_target = annotation_child_hit || behavior == Opaque;
                result.content_hit |= behavior != DeferToChild;
                result.blocks_siblings = match kind.mouse_region_opaque() {
                    // RenderMouseRegion.hitTest: `super.hitTest(..) && _opaque`.
                    // The region is always recorded, but only an opaque region
                    // absorbs the pointer for the siblings behind it.
                    Some(opaque) => hit_target && opaque,
                    None => behavior == Opaque,
                };
            }
            result.accepts_hit |= behavior == Opaque;
            if kind.mouse_region_opaque() == Some(false) {
                result.accepts_hit = false;
            }
        } else {
            use crate::gestures::HitTestBehavior::{Opaque, Translucent};
            if let (Some(element), Some(WidgetKind::Gesture { behavior, callbacks, .. })) =
                (element, kind)
                && (*behavior == Translucent || callbacks.on_any_tap.is_some())
            {
                result.coincident_gestures.push(element);
            }
            // An opaque gesture surface ends the sibling search for raw
            // listeners too. Translucent gestures still get their own
            // contender without blocking the hit behind an empty child.
            result.blocks_siblings |= matches!(
                kind, Some(WidgetKind::Gesture { behavior: Opaque, .. })
            );
            result.accepts_hit |= match kind {
                Some(WidgetKind::Gesture { behavior, .. })
                | Some(WidgetKind::Scroll {
                    hit_test_behavior: behavior,
                    ..
                }) => *behavior == Opaque,
                Some(WidgetKind::SliverViewport { config }) => config.hit_test_behavior == Opaque,
                _ => self_hit,
            };
        }
        // Flutter's rule for overlays: leaves hit themselves, and a widget
        // with children only through them unless its hit-test behavior
        // claims its own bounds.
        result.content_hit |= self_hit;
        (result, target)
    }

    /// Visits `children` frontmost first. Raw annotations stop at the first
    /// child that blocks its siblings; the retained target keeps searching
    /// the children behind it until one claims the point. Returns whether
    /// any child's bounds held the point, and the retained target.
    fn collect_pointer_hit_children(
        &self,
        children: &[RenderObjectId],
        point: Offset,
        origin: Offset,
        mut want_target: bool,
        precise: bool,
        result: &mut RawHitResult,
    ) -> (bool, Option<RenderObjectId>) {
        let mut child_hit = false;
        let mut target = None;
        let mut coincident_path_blocked = false;
        let mut children = children
            .iter()
            .rev()
            .filter(|child| !self.view_space_overlay_roots.contains(*child));
        for child in children.by_ref() {
            let (child_result, child_target) =
                self.collect_pointer_hit_render(*child, point, origin, want_target, precise);
            child_hit |= if precise {
                child_result.content_hit
            } else {
                child_result.subtree_hit
            };
            if !coincident_path_blocked {
                result.coincident_gestures.extend(child_result.coincident_gestures);
                coincident_path_blocked = child_result.accepts_hit;
            }
            result.content_hit |= child_result.content_hit;
            result.accepts_hit |= child_result.accepts_hit;
            let before = result.elements.len();
            result.pointer_spans.extend(child_result.pointer_spans.into_iter().map(|mut span| {
                span.raw_before += before;
                span
            }));
            result.elements.extend(child_result.elements);
            if child_target.is_some() {
                target = child_target;
                want_target = false;
            }
            if child_result.blocks_siblings {
                result.blocks_siblings = true;
                break;
            }
        }
        if want_target {
            target = children.find_map(|child| self.hit_test_render(*child, point, origin));
        }
        (child_hit, target)
    }
    pub(super) fn paint_render(&mut self, id: RenderObjectId) {
        with_recursive_tree_stack(|| {
            self.paint_render_inner(id);
        });
    }

    fn hides_render_paint(&self, id: RenderObjectId, kind: &RenderKind) -> bool {
        matches!(kind, RenderKind::Visibility { visible: false, .. })
            || flow_child_hidden(kind)
            || matches!(kind, RenderKind::Opacity { alpha, controller }
                if incular_rendering::opacity_is_transparent(
                    controller.as_ref().map_or(*alpha, OpacityController::opacity)
                ))
            || (matches!(kind, RenderKind::FittedBox { .. }) && !self.fitted_content_is_visible(id))
            || (matches!(
                kind,
                RenderKind::Transform { .. } | RenderKind::Affine { .. }
                    | RenderKind::Scale { .. } | RenderKind::Rotation { .. }
            ) && self.content_transform(id).is_some_and(|transform| !transform.has_drawable_area()))
    }

    /// Whether a render object paints the given direct child at its
    /// current state. A controlled opacity's child query is based on its
    /// animation value; raster work itself starts at the first nonzero byte.
    #[must_use]
    pub fn paints_child(&self, parent: RenderObjectId, child: RenderObjectId) -> bool {
        self.renders.get(parent.0).is_some_and(|node| {
            let visible = match node.object.kind() {
                RenderKind::Opacity { controller: Some(controller), .. } => controller.opacity() > 0.,
                kind => !self.hides_render_paint(parent, kind),
            };
            !node.parked && visible && visible_paint_children(node).contains(&child)
        })
    }

    /// Repaints dirty pictures into their retained compositor layers.
    /// Placement and clipping live in the compositor, so this pass assembles
    /// no display list of its own, and a subtree without a pending paint is
    /// skipped whole: its retained pictures stay attached as they are.
    pub(super) fn paint_render_inner(&mut self, id: RenderObjectId) {
        let node = self.render_live(id, "retained render must remain live");
        if node.parked || !node.subtree_needs_paint {
            self.diagnostics.display_lists_reused += 1;
            return;
        }
        let constraints = node.constraints;
        let _node_guard = self.guard_render(FramePhase::Paint, id, constraints);
        self.diagnostics.paint_visits += 1;
        let node = self.render_live(id, "retained render must remain live");
        if self.hides_render_paint(id, node.object.kind())
            || node.parent_sliver_layers.as_ref().is_some_and(|scopes| scopes.iter().any(|scope| scope.hides_paint()))
        {
            // Hidden or singular content keeps its pending paint until it is shown.
            return;
        }
        if node.dirty.contains(DirtyFlags::PAINT) {
            if node.flow_paint_order().is_some() {
                self.paint_flow(id);
            }
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
        // Invoke child painters in the same back-to-front order that the
        // compositor uses for their layers. Only mixed sliver viewports
        // whose children need reordering allocate a slot list.
        let slots = self.viewport_paint_slots(
            id,
            self.render_live(id, "retained render must remain live"),
        );
        for index in 0.. {
            let slot = match &slots {
                Some(slots) => match slots.get(index) {
                    Some(&slot) => slot,
                    None => break,
                },
                None => index,
            };
            let Some(&child) =
                visible_paint_children(self.render_live(id, "retained render must remain live"))
                    .get(slot)
            else {
                break;
            };
            self.paint_render(child);
        }
        // Read the children again rather than trusting their returns: paint
        // callbacks may request paint on a sibling that was already visited.
        let node = self.render_live(id, "retained render must remain live");
        let pending = node.dirty.contains(DirtyFlags::PAINT)
            || visible_paint_children(node).iter().any(|child| {
                self.renders
                    .get(child.0)
                    .is_some_and(|child| !child.parked && child.subtree_needs_paint)
            });
        self.render_live_mut(id, "retained render must remain live")
            .subtree_needs_paint = pending;
    }

    fn paint_paragraph_selection(
        &self,
        id: RenderObjectId,
        layout: &incular_text::TextLayout,
        cache: &mut DisplayList,
    ) {
        // A Text under a SelectionArea paints its registered range below
        // the glyphs and outside any TextOverflow::Fade isolation.
        if self.static_selections.is_empty() {
            return;
        }
        let Some(selection) = self
            .element_for_render(id)
            .and_then(|element| self.static_selection_range(element))
        else {
            return;
        };
        for rect in selection_rects(layout, selection, 0., 0., 0.) {
            cache.push(PaintCommand::Rect {
                rect,
                color: Color::rgba(72, 120, 220, 150),
            });
        }
    }

    fn repaint_pictures(&mut self, id: RenderObjectId) {
        let node = self.render_live(id, "retained render must remain live");
        let (kind, size) = (node.object.shared_kind(), node.size);
        let (picture_layer, foreground_layer) = (
            node.object.layers.picture,
            node.object.layers.foreground_picture,
        );
        // Taken rather than borrowed: paint callbacks may repaint another
        // node, which then records into a fresh buffer.
        let mut picture = std::mem::take(&mut self.paint_scratch);
        picture.clear();
        let focus_ring = self.paint_kind(id, &kind, size, &mut picture);
        // A picture may paint past its box (an offset box shadow, a stroke on
        // the edge); effect layers above it size their input from these
        // bounds, so they cover what was painted, as Flutter's layers do.
        let bounds = if matches!(&*kind, RenderKind::Image { .. }) {
            image_picture_bounds(picture.commands())
        } else {
            painted_bounds(picture.commands(), size)
        };
        self.render_live_mut(id, "retained render must remain live")
            .dirty
            .remove(DirtyFlags::PAINT);
        if let Some(layer) = picture_layer {
            self.compositor
                .update_picture_if_changed(layer, &picture, bounds);
        }
        let node = self.render_live(id, "retained render must remain live");
        if let Some(layer) = node.object.layers.paragraph_selection_picture()
            && let Some(layout) = node.text_layout()
        {
            let bounds = Rect::from_origin_size(Offset::ZERO, size);
            picture.clear();
            picture.push(PaintCommand::PushClip { rect: bounds });
            self.paint_paragraph_selection(id, layout, &mut picture);
            picture.push(PaintCommand::PopClip);
            // Skip an otherwise empty clip pair and its scene commands.
            if picture.commands().len() == 2 {
                picture.clear();
            }
            self.compositor.update_picture_if_changed(layer, &picture, bounds);
        }
        if let Some(layer) = foreground_layer {
            picture.clear();
            let mut foreground_bounds = bounds;
            if let RenderKind::Table {
                border: Some(border),
                ..
            } = &*kind
                && let crate::render_object::RenderFeatureState::TableBorder(geometry) = &self
                    .render_live(id, "bordered table remains live")
                    .object
                    .feature
            {
                // The grid height can be zero even though its explicit
                // border paints. In particular a square top edge extends
                // into the first pixel row and a divider can lie at y=0.
                let mut canvas = incular_rendering::Canvas::default();
                let rect = Rect::from_origin_size(Offset::ZERO, geometry.size);
                border.paint(&mut canvas, rect, &geometry.rows, &geometry.columns);
                picture.extend_from(&canvas.finish());
                foreground_bounds = painted_bounds(picture.commands(), size);
            } else if matches!(&*kind, RenderKind::RawScrollbar { .. }) {
                self.paint_explicit_scrollbar(id, size, &mut picture);
                foreground_bounds = painted_bounds(picture.commands(), size);
            } else if let Some(focus_ring) = focus_ring.filter(|color| color.alpha > 0) {
                let inset = 1.0;
                let ring_size = Size::new(
                    (size.width - 2. * inset).max(0.),
                    (size.height - 2. * inset).max(0.),
                );
                picture.push(PaintCommand::Border {
                    rrect: RRect::uniform(
                        Rect::from_origin_size(Offset::new(inset, inset), ring_size),
                        4.,
                    ),
                    border: Border::new(2.0, focus_ring),
                });
            }
            self.compositor
                .update_picture_if_changed(layer, &picture, foreground_bounds);
        }
        #[cfg(debug_assertions)]
        {
            let focus_color = self.debug_focus_color(id);
            let overflow = matches!(
                kind.as_ref(),
                RenderKind::Flex { .. } | RenderKind::Unconstrained { .. }
            );
            let existing = self
                .render_live(id, "retained render must remain live")
                .object
                .layers
                .has_debug_foreground_picture();
            if overflow || focus_color.is_some() || existing {
                picture.clear();
                if overflow {
                    self.paint_debug_overflow_indicator(id, &kind, size, &mut picture);
                }
                if let Some(color) = focus_color
                    && size.width > 0.0
                    && size.height > 0.0
                {
                    picture.push(PaintCommand::Border {
                        rrect: RRect::uniform(Rect::from_origin_size(Offset::ZERO, size), 0.),
                        border: Border::new(3.0, color),
                    });
                }
                let bounds = painted_bounds(picture.commands(), size);
                let node = self.renders.get_mut(id.0).expect("render remains live while painting");
                let resync = node.object.layers.update_debug_foreground_picture(
                    &mut self.compositor,
                    &picture,
                    bounds,
                );
                if resync && let Some(element) = self.element_for_render(id) {
                    self.sync_render_children(element);
                }
            }
        }
        self.paint_scratch = picture;
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
        let limit =
            requested_limit.min(crate::semantics_debugger::DEFAULT_SEMANTICS_DEBUGGER_NODE_LIMIT);
        if limit == 0 {
            return;
        }
        // Outline every root: the graph is a forest under layout-only widgets.
        let mut stack: Vec<_> = self
            .semantics
            .roots()
            .iter()
            .rev()
            .map(|&root| (root, 0usize))
            .collect();
        let mut painted = 0usize;
        let colors = [
            Color::rgba(220, 40, 40, 230),
            Color::rgba(35, 115, 220, 230),
            Color::rgba(35, 155, 75, 230),
            Color::rgba(210, 125, 25, 230),
            Color::rgba(145, 65, 190, 230),
            Color::rgba(20, 155, 155, 230),
        ];
        let mut message = String::new();
        while let Some((id, depth)) = stack.pop() {
            if painted >= limit {
                break;
            }
            let Some(node) = self.semantics.node(id) else {
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
            message.clear();
            semantics_debug_message(node, &mut message);
            for child in node.children.iter().rev() {
                stack.push((*child, depth.saturating_add(1)));
            }
            if !message.is_empty() {
                self.paint_semantics_debug_label(output, &label_style, bounds, &message);
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
            .external_text_call("TextEngine::layout_with_options", message);
        let layout = self.text_engine.get_mut().layout_with_options(
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
                    color: style.effective_foreground_color(),
                });
            }
        }
        output.push(PaintCommand::PopTransform);
    }
    /// Paints a scroll view's overscroll glow over its content. Only a
    /// scroll view with a glow indicator has one, so no other view pays for it.
    pub(super) fn paint_overscroll_glow(&self, id: RenderObjectId, cache: &mut DisplayList) {
        let Some(indicator) = self
            .element_for_render(id)
            .and_then(|element| self.scroll_overscroll_indicator(element))
        else {
            return;
        };
        let size = self.renders.get(id.0).map_or(Size::ZERO, |node| node.size);
        indicator.paint_glow(cache, size);
    }

    pub(super) fn paint_scrollbar(&self, id: RenderObjectId, cache: &mut DisplayList) {
        // The same geometry pointer handling uses: along the scroll axis, and
        // absent where the scroll behavior gives the view no scrollbar.
        let Some((controller, geometry)) = self.scrollbar_local_geometry(id) else {
            return;
        };
        let style = controller.scrollbar_style();
        if !geometry.visible {
            return;
        }
        // Flutter's ScrollbarPainter paints nothing at opacity 0 and scales
        // the track and thumb colors by the fade otherwise.
        let opacity = self.scroll_state_live(id).fade.opacity();
        if opacity <= 0.0 {
            return;
        }
        let faded = |color: Color| Color {
            alpha: (f32::from(color.alpha) * opacity).round() as u8,
            ..color
        };
        let thumb = faded(style.thumb_color);
        let track = faded(style.track_color);
        if track.alpha > 0 {
            cache.push(PaintCommand::RRect {
                rrect: RRect::uniform(geometry.track, style.width * 0.5),
                brush: track.into(),
            });
        }
        if thumb.alpha > 0 {
            cache.push(PaintCommand::RRect {
                rrect: RRect::uniform(geometry.thumb, style.width * 0.5),
                brush: thumb.into(),
            });
        }
    }

    fn paint_explicit_scrollbar(&self, id: RenderObjectId, size: Size, cache: &mut DisplayList) {
        let Some(scrollbar) = self
            .renders
            .get(id.0)
            .and_then(|node| node.raw_scrollbar_state())
            .and_then(|state| state.scrollbar.as_ref())
        else {
            return;
        };
        let geometry = scrollbar.geometry(size);
        if !geometry.visible {
            return;
        }
        let style = scrollbar.style();
        for (rect, mut color) in [
            (geometry.track, style.track_color),
            (geometry.thumb, style.thumb_color),
        ] {
            color.alpha = (f32::from(color.alpha) * geometry.opacity).round() as u8;
            if color.alpha > 0 {
                cache.push(PaintCommand::Rect { rect, color });
            }
        }
    }
    pub(super) fn hit_test_render(
        &self,
        id: RenderObjectId,
        point: Offset,
        origin: Offset,
    ) -> Option<RenderObjectId> {
        if self.view_space_overlay_roots.contains(&id) {
            return None;
        }
        let node = self.renders.get(id.0)?;
        if self.misses_outside_bounds(id, node, point, origin) {
            return None;
        }
        if self.layout_overlay_popups.contains(&id) {
            let (result, target) = self.collect_pointer_hit_render(id, point, origin, true, true);
            return result.content_hit.then_some(target).flatten();
        }
        let kind = self
            .element_for_render(id)
            .and_then(|element| self.elements.get(element.0))
            .map(|element| element.widget.kind());
        match kind {
            Some(WidgetKind::IgnorePointer { ignoring: true, .. }) => return None,
            Some(WidgetKind::AbsorbPointer {
                absorbing: true, ..
            }) => {
                return half_open_hit(node.size, point - self.hit_origin(id, node, origin))
                    .then_some(id);
            }
            _ => {}
        }
        if flow_child_hidden(node.object.kind()) {
            return None;
        }
        if matches!(node.object.kind(), RenderKind::Positioned { .. }) {
            return node
                .children
                .iter()
                .rev()
                .find_map(|child| self.hit_test_render(*child, point, origin + node.offset));
        }
        // A hidden follower has no input subtree. Check this before the
        // transform path (which otherwise falls back to layout placement).
        if matches!(node.object.kind(), RenderKind::Follower { .. })
            && !self.follower_content_visible(id)
        {
            return None;
        }
        if delegates_transformed_hits(node.object.kind()) {
            let (local, _) = self.transformed_hit_position(id, node, point, origin)?;
            for child in node.children.iter().rev() {
                if let Some(hit) = self.hit_test_render(*child, local, Offset::ZERO) {
                    return Some(hit);
                }
            }
            return None;
        }
        let current = self.hit_origin(id, node, origin);
        if !hit_box_contains(node.object.kind(), kind, node.size, current, point) {
            return None;
        }
        if matches!(
            node.object.kind(),
            RenderKind::Visibility {
                visible: false,
                interactive: false,
                ..
            }
        ) {
            return None;
        }
        let child_origin = hit_child_origin(node.object.kind(), current);
        let hit_children = self.hit_children(id, node);
        for child in hit_children.iter().rev() {
            if let Some(hit) = self.hit_test_render(*child, point, child_origin) {
                return Some(hit);
            }
        }
        self_hit_after_children(kind, node.size, point - current).then_some(id)
    }
    /// Whether `point` lies outside `node`'s box and that alone rules out a
    /// hit anywhere in its subtree. Most children of a long list are rejected
    /// here, before any widget lookup. Nodes that hit-test through a
    /// transform can place their content elsewhere. Gesture nodes defer
    /// their bounds check to the actual widget's hit behavior.
    fn misses_outside_bounds(
        &self,
        id: RenderObjectId,
        node: &RenderNode,
        point: Offset,
        origin: Offset,
    ) -> bool {
        if node.parked {
            return true;
        }
        if let Some(clips) = &node.parent_sliver_layers {
            let local = point - self.hit_origin(id, node, origin);
            if clips.iter().any(|clip| !clip.contains(local)) {
                return true;
            }
        }
        if matches!(
            node.object.kind(),
            RenderKind::Gesture | RenderKind::Positioned { .. }
        ) || delegates_transformed_hits(node.object.kind())
        {
            return false;
        }
        let current = self.hit_origin(id, node, origin);
        !Rect::from_origin_size(current, node.size).contains(point)
            || !node
                .object
                .layers
                .accepts_clip_hit(node.object.kind(), node.size, point - current)
    }

    fn transformed_hit_position(
        &self,
        id: RenderObjectId,
        node: &RenderNode,
        point: Offset,
        origin: Offset,
    ) -> Option<(Offset, bool)> {
        let kind = node.object.kind();
        let local = point - (origin + node.offset);
        let local = if hit_test_transforms_children(kind) {
            self.child_content_transform(id).inverse_transform_point(local)?
        } else {
            local
        };
        // Flutter RenderTransform forwards the hit without testing its
        // own box, even when transformHitTests is false. A child Transform
        // can move its real target outside that box. Other specialized
        // render objects keep their explicit box/overflow policy.
        if matches!(kind, RenderKind::Transform { .. }) {
            return Some((local, false));
        }
        let child_size = node
            .children
            .first()
            .and_then(|child| self.renders.get(child.0))
            .map_or(node.size, |child| child.size);
        Rect::from_origin_size(Offset::ZERO, child_size)
            .contains(local)
            .then_some((local, true))
    }

    /// The same coordinate path used by hit testing, not by paint.
    /// Accumulating upwards avoids allocating a path for each listener.
    pub(super) fn pointer_world_to_local(&self, id: RenderObjectId) -> Option<CoreTransform> {
        let mut node = self.renders.get(id.0)?;
        let mut local_to_world =
            CoreTransform::translation(self.hit_origin(id, node, Offset::ZERO));
        let mut current = id;
        while !self.view_space_overlay_roots.contains(&current) {
            let Some(parent) = node.parent else { break };
            current = parent;
            node = self.renders.get(parent.0)?;
            let origin = self.hit_origin(parent, node, Offset::ZERO);
            let to_parent = if hit_test_transforms_children(node.object.kind()) {
                CoreTransform::translation(origin).then(self.child_content_transform(parent))
            } else {
                CoreTransform::translation(hit_child_origin(node.object.kind(), origin))
            };
            local_to_world = to_parent.then(local_to_world);
        }
        incular_gestures::HitTestTransform::Paint(local_to_world).resolve()
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

    fn viewport_paint_slots(&self, id: RenderObjectId, node: &RenderNode) -> Option<Vec<usize>> {
        let RenderKind::SliverViewport { config } = node.object.kind() else {
            return None;
        };
        let element = self
            .element_for_render(id)
            .and_then(|element| self.elements.get(element.0))?;
        // Render children and sliver ids are parallel. sync_render_children
        // also uses these same slots to order their compositor layers.
        debug_assert!(
            node.children.len() == element.children.len()
                && node
                    .children
                    .iter()
                    .zip(&element.children)
                    .all(|(render, child)| self.render_id(*child) == Some(*render)),
            "viewport render children are parallel to its element's children"
        );
        crate::scrolling::sliver_paint_slots(element.sliver_child_ids(), config.paint_order)
    }

    /// A moving Flutter Scrollable can ignore pointer hits on its content
    /// without taking the viewport's own recognizer out of the hit path. A
    /// stop tap ends the old ballistic; that contact never hits a child.
    fn viewport_ignores_child_pointers(&self, id: RenderObjectId, kind: &RenderKind) -> bool {
        if self.input.scroll_flings.is_empty() {
            return false;
        }
        let controller = match kind {
            RenderKind::Scroll { controller, .. } => controller,
            RenderKind::SliverViewport { config } => &config.controller,
            RenderKind::ListWheelScrollView { config } => config.scroll_interaction().0,
            _ => return false,
        };
        self.input.scroll_flings.iter().any(|driver| {
            driver.ignores_child_pointers
                && driver.controller == *controller
                && driver.activity.is_current()
                && driver.expected_revision == controller.revision()
        }) && self
            .element_for_render(id)
            .is_some_and(|element| self.viewport_lease_live(element, controller))
    }

    /// Children in paint order, hit-tested in reverse. Sliver viewports paint
    /// whole slivers in their SliverPaintOrder; an indexed stack shows one child.
    fn hit_children<'a>(
        &self,
        id: RenderObjectId,
        node: &'a RenderNode,
    ) -> std::borrow::Cow<'a, [RenderObjectId]> {
        if viewport_has_no_paint_extent(node.object.kind(), node.size)
            || self.viewport_ignores_child_pointers(id, node.object.kind())
        {
            return std::borrow::Cow::Borrowed(&[]);
        }
        if let Some(slots) = self.viewport_paint_slots(id, node) {
            slots
                .into_iter()
                .filter_map(|slot| node.children.get(slot).copied())
                .collect::<Vec<_>>()
                .into()
        } else {
            visible_paint_children(node).into()
        }
    }
    pub(super) fn scrollbar_controller_and_geometry(
        &self,
        render: RenderObjectId,
    ) -> Option<(ScrollController, ScrollbarGeometry)> {
        let (controller, mut geometry) = self.scrollbar_local_geometry(render)?;
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
        let (controller, behavior, axis, reverse) = match node.object.kind() {
            RenderKind::Scroll {
                controller,
                behavior,
                axis,
                reverse,
                ..
            } => (controller.clone(), *behavior, *axis, *reverse),
            RenderKind::SliverViewport { config } => (
                config.controller.clone(),
                config.behavior,
                config.axis,
                config.reverse,
            ),
            _ => return None,
        };
        // The thumb runs along the scroll axis: a horizontal view gets a
        // horizontal thumb on its bottom edge, a reversed one starts at the
        // far end of the track.
        let mut geometry = scrollbar_geometry_for_axis(
            node.size,
            &controller,
            controller.scrollbar_style(),
            axis,
            reverse,
        );
        // Flutter's `ScrollBehavior.buildScrollbar`: only desktop platforms,
        // and only while the behavior keeps scrollbars, get one; Material's
        // gives horizontal scroll views none.
        geometry.visible &= behavior.scrollbar
            && (axis == incular_config::Axis::Vertical || behavior.horizontal_scrollbar);
        Some((controller, geometry))
    }
    pub(super) fn scrollbar_local_point(
        &self,
        render: RenderObjectId,
        point: Offset,
    ) -> Option<Offset> {
        self.render_world_transform(render)
            .inverse_transform_point(point)
    }
    /// Render objects of mounted scroll views, in render-arena order. The
    /// backing buffer is recycled through a thread-local pool, so a frame
    /// that walks the scroll views does not allocate.
    pub(super) fn scroll_view_renders(&self) -> ScrollViewRenders {
        let mut renders = SCROLL_VIEW_SCRATCH
            .try_with(|scratch| std::mem::take(&mut *scratch.borrow_mut()))
            .unwrap_or_default();
        renders.extend(
            self.tracked
                .scroll_views
                .iter()
                .filter_map(|id| self.elements.get(id.0).map(|element| element.render)),
        );
        renders.sort_unstable_by_key(|render| render.0);
        ScrollViewRenders { renders, next: 0 }
    }

    /// The scroll view whose scrollbar the pointer is over.
    ///
    /// Flutter's `ScrollbarPainter.hitTestInteractive`: touch and trackpad
    /// presses hit the track padded to `_kMinInteractiveSize` around the
    /// thumb, other kinds the track alone. A transparent scrollbar cannot
    /// be pressed, but a hovering mouse over the padded track brings it
    /// into view (`for_hover`).
    pub(super) fn scrollbar_at(
        &self,
        point: Offset,
        kind: crate::raw_input::PointerDeviceKind,
        for_hover: bool,
    ) -> Option<RenderObjectId> {
        use crate::raw_input::PointerDeviceKind;
        self.scroll_view_renders().into_iter().find(|&render| {
            let Some((_, geometry)) = self.scrollbar_local_geometry(render) else {
                return false;
            };
            if !geometry.visible {
                return false;
            }
            let Some(local) = self.scrollbar_local_point(render, point) else {
                return false;
            };
            let padded = scrollbar_min_interactive_rect(geometry.track, geometry.thumb);
            if self.scroll_state_live(render).fade.opacity() == 0.0 {
                return for_hover && kind == PointerDeviceKind::Mouse && padded.contains(local);
            }
            match kind {
                PointerDeviceKind::Touch | PointerDeviceKind::Trackpad => padded.contains(local),
                _ => geometry.track.contains(local),
            }
        })
    }
}

/// `rect` expanded to include the `_kMinInteractiveSize` (48) square centred
/// on the thumb, as in Flutter's scrollbar hit tests.
pub(super) fn scrollbar_min_interactive_rect(rect: Rect, thumb: Rect) -> Rect {
    const MIN_INTERACTIVE_SIZE: f32 = 48.0;
    let center = Offset::new(
        thumb.origin.x + thumb.size.width / 2.0,
        thumb.origin.y + thumb.size.height / 2.0,
    );
    let half = MIN_INTERACTIVE_SIZE / 2.0;
    let left = rect.origin.x.min(center.x - half);
    let top = rect.origin.y.min(center.y - half);
    let right = (rect.origin.x + rect.size.width).max(center.x + half);
    let bottom = (rect.origin.y + rect.size.height).max(center.y + half);
    Rect::from_origin_size(
        Offset::new(left, top),
        Size::new(right - left, bottom - top),
    )
}

/// Writes the debugger's one-line description of `node` into `out`.
fn semantics_debug_message(node: &SemanticNode, out: &mut String) {
    out.push_str(semantic_role_name(node.role));
    if let Some(label) = node.label.as_deref().filter(|label| !label.is_empty()) {
        let _ = write!(out, " label={:?}", bounded_debug_text(label, 120));
    }
    if let Some(value) = node.value.as_deref().filter(|value| !value.is_empty()) {
        let _ = write!(out, " value={:?}", bounded_debug_text(value, 120));
    }
    if let Some(description) = node
        .description
        .as_deref()
        .filter(|description| !description.is_empty())
    {
        let _ = write!(
            out,
            " description={:?}",
            bounded_debug_text(description, 120)
        );
    }
    if let Some(checked) = node.state.checked {
        let _ = write!(out, " checked={checked}");
    }
    if semantic_role_is_interactive(node.role) && !node.state.enabled {
        out.push_str(" disabled");
    }
    if node.state.read_only {
        out.push_str(" readOnly");
    }
    if let Some(index) = node.state.item_index {
        let position = index.saturating_add(1);
        let _ = match node.state.set_size {
            Some(size) => write!(out, " position={position}/{size}"),
            None => write!(out, " position={position}"),
        };
    }
    if !node.actions.is_empty() {
        let _ = write!(out, " actions={:?}", node.actions);
    }
}

fn semantic_role_name(role: SemanticRole) -> &'static str {
    match role {
        SemanticRole::Button => "button",
        SemanticRole::Text => "text",
        SemanticRole::TextField => "text field",
        SemanticRole::TextArea => "text area",
        SemanticRole::List => "list",
        SemanticRole::ListItem => "list item",
        SemanticRole::Table => "table",
        SemanticRole::Row => "row",
        SemanticRole::Cell => "cell",
        SemanticRole::ColumnHeader => "column header",
        SemanticRole::ScrollView => "scroll view",
        SemanticRole::GenericContainer => "container",
        SemanticRole::Checkbox => "checkbox",
        SemanticRole::Radio => "radio",
        SemanticRole::Switch => "switch",
        SemanticRole::Slider => "slider",
        SemanticRole::Menu => "menu",
        SemanticRole::Dialog => "dialog",
        SemanticRole::AlertDialog => "alert dialog",
        SemanticRole::Alert => "alert",
        SemanticRole::Status => "status",
        SemanticRole::Form => "form",
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

fn bounded_debug_text(text: &str, limit: usize) -> std::borrow::Cow<'_, str> {
    if text.chars().count() <= limit {
        return text.into();
    }
    let mut result = text.chars().take(limit).collect::<String>();
    result.push('…');
    result.into()
}

/// Whether a render none of whose children claimed the point is itself the
/// retained hit target.
fn retained_self_hit(kind: Option<&WidgetKind>) -> bool {
    match kind {
        Some(WidgetKind::Gesture { behavior, .. }) => {
            *behavior != crate::gestures::HitTestBehavior::DeferToChild
        }
        Some(WidgetKind::RawInput { kind, .. }) => {
            use crate::gestures::HitTestBehavior::{DeferToChild, Opaque};
            // MouseRegion adds an annotation even when it does not claim
            // the hit. A non-opaque region, or a translucent region with
            // no hit child, must not intercept the retained tap target.
            match kind.mouse_region_opaque() {
                Some(opaque) => opaque && kind.hit_test_behavior() == Opaque,
                None => kind.hit_test_behavior() != DeferToChild,
            }
        }
        // drag_target.dart: Draggable defaults to HitTestBehavior.deferToChild,
        // so it claims a hit only where its child is hit.
        Some(WidgetKind::Draggable { .. }) => false,
        // Flutter `Scrollable.hitTestBehavior` (default opaque).
        Some(WidgetKind::Scroll {
            hit_test_behavior, ..
        }) => *hit_test_behavior != crate::gestures::HitTestBehavior::DeferToChild,
        Some(WidgetKind::SliverViewport { config }) => {
            config.hit_test_behavior != crate::gestures::HitTestBehavior::DeferToChild
        }
        // Positioned supplies parent layout data; it has no independent
        // hit surface when its child (for example IgnorePointer) declines.
        Some(WidgetKind::Banner { .. } | WidgetKind::Positioned { .. }) => false,
        _ => true,
    }
}

fn delegates_transformed_hits(kind: &RenderKind) -> bool {
    matches!(kind, RenderKind::Transform { .. }) || hit_test_transforms_children(kind)
}

fn hit_test_transforms_children(kind: &RenderKind) -> bool {
    matches!(
        kind,
        RenderKind::Transform {
            transform_hit_tests: true,
            ..
        } | RenderKind::Scale { .. }
            | RenderKind::Affine { .. }
            | RenderKind::Rotation { .. }
            | RenderKind::Follower { .. }
            | RenderKind::FittedBox { .. }
    )
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

/// Draws one line's glyph runs. A paragraph layout positions runs within
/// their paragraph, so runs below the first paragraph are drawn as moved
/// copies.
fn push_line_runs(output: &mut DisplayList, line: incular_text::TextLineRef<'_>, color: Color) {
    let origin = line.paint_origin();
    for run in line.runs() {
        let run = if origin == Offset::ZERO {
            run.clone()
        } else {
            let mut moved = (**run).clone();
            moved.origin.x += origin.x;
            moved.origin.y += origin.y;
            Arc::new(moved)
        };
        output.push(PaintCommand::GlyphRun { run, color });
    }
}

/// A `Flow` child its delegate did not paint: neither drawn nor hit.
fn flow_child_hidden(kind: &RenderKind) -> bool {
    matches!(
        kind,
        RenderKind::Transform { slot: Some(slot), .. } if !slot.painted()
    )
}

/// Destination blending can replace the destination even for transparent
/// source pixels, but never in the space around a fitted image or tile.
fn image_picture_bounds(commands: &[PaintCommand]) -> Rect {
    let zero = Rect::from_origin_size(Offset::ZERO, Size::ZERO);
    let (images, clip) = match commands {
        [PaintCommand::PushClip { rect }, images @ .., PaintCommand::PopClip] => (images, Some(*rect)),
        _ => (commands, None),
    };
    let (images, transform) = match images {
        [PaintCommand::PushTransform { transform }, images @ .., PaintCommand::PopTransform] => {
            (images, Some(*transform))
        }
        _ => (images, None),
    };
    images
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Image { destination, .. } => {
                let destination = transform.map_or(*destination, |transform| {
                    transform.transform_rect_bbox(*destination)
                });
                match clip {
                    Some(clip) => destination.intersection(clip),
                    None => Some(destination),
                }
                .filter(|rect| rect.size.width > 0.0 && rect.size.height > 0.0)
            }
            _ => None,
        })
        .reduce(|a, b| {
            let left = a.origin.x.min(b.origin.x);
            let top = a.origin.y.min(b.origin.y);
            let right = (a.origin.x + a.size.width).max(b.origin.x + b.size.width);
            let bottom = (a.origin.y + a.size.height).max(b.origin.y + b.size.height);
            Rect::from_origin_size(Offset::new(left, top), Size::new(right - left, bottom - top))
        })
        .unwrap_or(zero)
}

/// The box of `size` grown to cover the geometry `commands` paint outside
/// it. Text and other commands without cheap bounds count as inside the box.
fn painted_bounds(commands: &[PaintCommand], size: Size) -> Rect {
    let mut bounds = Rect::from_origin_size(Offset::ZERO, size);
    // A fixed stack: this runs on every repaint and must not allocate.
    const MAX_DEPTH: usize = 32;
    let mut transforms = [incular_core::Transform::IDENTITY; MAX_DEPTH];
    let mut depth = 0usize;
    for command in commands {
        let transform = transforms[depth];
        let local = match command {
            PaintCommand::Rect { rect, .. } | PaintCommand::HardEdgeRect { rect, .. } => {
                Some(*rect)
            }
            PaintCommand::RRect { rrect, .. } | PaintCommand::Border { rrect, .. } => {
                Some(rrect.rect)
            }
            PaintCommand::FillPath { path, .. } => path.tight_bounds(),
            PaintCommand::StrokePath { path, stroke, .. } => path.tight_bounds().map(|rect| {
                let margin = stroke.width.max(0.) * 0.5 * stroke.miter_limit.max(1.);
                Rect::from_origin_size(
                    Offset::new(rect.origin.x - margin, rect.origin.y - margin),
                    Size::new(
                        rect.size.width + margin * 2.,
                        rect.size.height + margin * 2.,
                    ),
                )
            }),
            PaintCommand::PushTransform { transform: local } => {
                if depth + 1 == MAX_DEPTH {
                    // Too deep to track: keep the box alone.
                    return Rect::from_origin_size(Offset::ZERO, size);
                }
                depth += 1;
                transforms[depth] = transform.then(*local);
                None
            }
            PaintCommand::PopTransform => {
                depth = depth.saturating_sub(1);
                None
            }
            _ => None,
        };
        if let Some(local) = local.filter(|rect| {
            rect.origin.x.is_finite()
                && rect.origin.y.is_finite()
                && rect.size.width.is_finite()
                && rect.size.height.is_finite()
        }) {
            let world = transform.transform_rect_bbox(local);
            let left = bounds.origin.x.min(world.origin.x);
            let top = bounds.origin.y.min(world.origin.y);
            let right =
                (bounds.origin.x + bounds.size.width).max(world.origin.x + world.size.width);
            let bottom =
                (bounds.origin.y + bounds.size.height).max(world.origin.y + world.size.height);
            bounds = Rect::from_origin_size(
                Offset::new(left, top),
                Size::new(right - left, bottom - top),
            );
        }
    }
    bounds
}
