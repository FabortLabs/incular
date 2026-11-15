//! `Scrollable.ensureVisible`: scrolls every enclosing viewport so a mounted
//! element is revealed (Flutter `widgets/scrollable.dart:491-531` and
//! `ScrollPosition.ensureVisible`, `widgets/scroll_position.dart:812-878`).

use std::time::Duration;

use incular_animation::{Curve, Curves};
use incular_config::Axis;
use incular_core::{Offset, Rect, Size};
use incular_scroll::ScrollController;

use super::Scrollable;
use crate::tree::{ElementId, RenderKind, WidgetTree};

/// How [`Scrollable::ensure_visible`] chooses each viewport's target offset
/// (Flutter `ScrollPositionAlignmentPolicy`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollPositionAlignmentPolicy {
    /// Places the target at [`EnsureVisible::alignment`] within the viewport.
    #[default]
    Explicit,
    /// Scrolls only toward the end, until the target's trailing edge is
    /// visible (alignment 1.0, never moving back).
    KeepVisibleAtEnd,
    /// Scrolls only toward the start, until the target's leading edge is
    /// visible (alignment 0.0, never moving forward).
    KeepVisibleAtStart,
}

/// Options for [`Scrollable::ensure_visible`], with Flutter's defaults:
/// alignment 0.0, a zero duration (jump), `Curves.ease` and the explicit
/// alignment policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnsureVisible {
    alignment: f32,
    duration: Duration,
    curve: Curve,
    alignment_policy: ScrollPositionAlignmentPolicy,
}

impl Default for EnsureVisible {
    fn default() -> Self {
        Self {
            alignment: 0.0,
            duration: Duration::ZERO,
            curve: Curves::EASE,
            alignment_policy: ScrollPositionAlignmentPolicy::Explicit,
        }
    }
}

impl EnsureVisible {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Where the target lands along the main axis: 0.0 puts its leading edge
    /// at the viewport's leading edge, 1.0 its trailing edge at the trailing
    /// edge, 0.5 centres it.
    #[must_use]
    pub fn alignment(mut self, alignment: f32) -> Self {
        self.alignment = alignment;
        self
    }

    /// A non-zero duration animates each viewport with
    /// [`ScrollController::animate_to`]; zero jumps.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    #[must_use]
    pub fn alignment_policy(mut self, policy: ScrollPositionAlignmentPolicy) -> Self {
        self.alignment_policy = policy;
        self
    }
}

/// One enclosing viewport found by the ancestor walk.
struct AncestorViewport {
    element: ElementId,
    controller: ScrollController,
    axis: Axis,
    reverse: bool,
}

impl WidgetTree {
    fn enclosing_viewport(&self, element: ElementId) -> Option<AncestorViewport> {
        let mut cursor = self.parent(element);
        while let Some(candidate) = cursor {
            let kind = self
                .render_id(candidate)
                .and_then(|render| self.render_object_kind(render));
            let found = match kind {
                Some(RenderKind::Scroll {
                    controller,
                    axis,
                    reverse,
                    ..
                }) => Some((controller.clone(), *axis, *reverse)),
                Some(RenderKind::SliverViewport { config }) => {
                    Some((config.controller.clone(), config.axis, config.reverse))
                }
                _ => None,
            };
            if let Some((controller, axis, reverse)) = found {
                return Some(AncestorViewport {
                    element: candidate,
                    controller,
                    axis,
                    reverse,
                });
            }
            cursor = self.parent(candidate);
        }
        None
    }

    /// `rect` (in `target`'s coordinates) in `viewport`'s coordinates.
    fn rect_in_viewport(&self, target: ElementId, rect: Rect, viewport: ElementId) -> Option<Rect> {
        let (target_world, _) = self.element_geometry(target)?;
        let (viewport_world, _) = self.element_geometry(viewport)?;
        Some(
            viewport_world
                .inverse()?
                .then(target_world)
                .transform_rect_bbox(rect),
        )
    }
}

/// `RenderAbstractViewport.getOffsetToReveal` for a target whose rectangle is
/// `rect` in the viewport's coordinates while the viewport sits at `pixels`.
/// The scroll-space leading edge is measured from the axis direction's
/// leading side (`AxisDirection.up` and `left` measure from the far side,
/// `rendering/viewport.dart` `getOffsetToReveal`).
fn offset_to_reveal(
    rect: Rect,
    viewport: Size,
    axis: Axis,
    reverse: bool,
    pixels: f32,
    alignment: f32,
) -> f32 {
    let (start, extent, viewport_extent) = match axis {
        Axis::Vertical => (rect.origin.y, rect.size.height, viewport.height),
        Axis::Horizontal => (rect.origin.x, rect.size.width, viewport.width),
    };
    let leading = if reverse {
        viewport_extent - (start + extent)
    } else {
        start
    };
    let leading_scroll_offset = pixels + leading;
    leading_scroll_offset - (viewport_extent - extent) * alignment
}

impl Scrollable {
    /// Scrolls every viewport enclosing `element` so it becomes visible
    /// (Flutter `Scrollable.ensureVisible`, `widgets/scrollable.dart:491-531`).
    ///
    /// Viewports are visited innermost first. The innermost reveals the
    /// element's whole box; each outer viewport reveals the enclosing inner
    /// viewport, aiming at the part of it where the element sits
    /// (`ScrollPosition.ensureVisible`'s `targetRenderObject`, which keeps the
    /// first target as visible as possible; flutter/flutter#65100). As in
    /// Flutter, every target is computed from the geometry of the last
    /// completed layout, before any viewport moves.
    ///
    /// Each target is clamped to the viewport's scroll range. A viewport
    /// already at its target is left alone; otherwise it jumps (zero
    /// duration) or animates with [`ScrollController::animate_to`]. Returns
    /// how many viewports were asked to move.
    ///
    /// Call it after layout. Pinned slivers do not reduce the revealing
    /// extent (Flutter subtracts `maxScrollObstructionExtentBefore`), wheel
    /// and two-dimensional viewports are not walked, and the minimum scroll
    /// extent is 0.
    pub fn ensure_visible(tree: &WidgetTree, element: ElementId, options: EnsureVisible) -> usize {
        let Some((_, target_size)) = tree.element_geometry(element) else {
            return 0;
        };
        let mut moves = Vec::new();
        let mut object = element;
        let mut first_target: Option<(ElementId, Size)> = None;
        while let Some(viewport) = tree.enclosing_viewport(object) {
            let Some((_, viewport_size)) = tree.element_geometry(viewport.element) else {
                break;
            };
            let Some((_, object_size)) = tree.element_geometry(object) else {
                break;
            };
            // `ScrollPosition.ensureVisible`: with a recorded first target the
            // revealed rectangle is that target's box, clipped to the size of
            // the object being revealed (`object.paintBounds.intersect(
            // targetRenderObject.paintBounds)`), carried into the object.
            let rect = match first_target {
                Some((target, size)) => tree.rect_in_viewport(
                    target,
                    Rect::from_origin_size(
                        Offset::ZERO,
                        Size::new(
                            size.width.min(object_size.width),
                            size.height.min(object_size.height),
                        ),
                    ),
                    viewport.element,
                ),
                None => tree.rect_in_viewport(
                    object,
                    Rect::from_origin_size(Offset::ZERO, object_size),
                    viewport.element,
                ),
            };
            let Some(rect) = rect else {
                break;
            };
            let pixels = viewport.controller.offset();
            let min = viewport.controller.min_offset().min(0.0);
            let max = viewport.controller.max_offset().max(0.0);
            let reveal = |alignment| {
                offset_to_reveal(
                    rect,
                    viewport_size,
                    viewport.axis,
                    viewport.reverse,
                    pixels,
                    alignment,
                )
                .clamp(min, max)
            };
            // `_applyAxisDirectionToAlignmentPolicy`: reversed axes swap the
            // keep-visible policies.
            let policy = match (viewport.reverse, options.alignment_policy) {
                (true, ScrollPositionAlignmentPolicy::KeepVisibleAtEnd) => {
                    ScrollPositionAlignmentPolicy::KeepVisibleAtStart
                }
                (true, ScrollPositionAlignmentPolicy::KeepVisibleAtStart) => {
                    ScrollPositionAlignmentPolicy::KeepVisibleAtEnd
                }
                (_, policy) => policy,
            };
            let target = match policy {
                ScrollPositionAlignmentPolicy::Explicit => reveal(options.alignment),
                ScrollPositionAlignmentPolicy::KeepVisibleAtEnd => reveal(1.0).max(pixels),
                ScrollPositionAlignmentPolicy::KeepVisibleAtStart => reveal(0.0).min(pixels),
            };
            if target != pixels {
                moves.push((viewport.controller.clone(), target));
            }
            first_target.get_or_insert((element, target_size));
            object = viewport.element;
        }
        let count = moves.len();
        for (controller, target) in moves {
            if options.duration.is_zero() {
                controller.jump_to(target);
            } else {
                controller.animate_to(target, options.duration, options.curve);
            }
        }
        count
    }
}
