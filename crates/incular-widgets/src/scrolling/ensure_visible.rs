//! `Scrollable.ensureVisible`: scrolls every enclosing viewport so a mounted
//! element is revealed (Flutter `widgets/scrollable.dart:491-531` and
//! `ScrollPosition.ensureVisible`, `widgets/scroll_position.dart:812-878`).

use std::time::Duration;

use incular_animation::{Curve, Curves};
use incular_config::Axis;
use incular_core::{Offset, Rect, Size};
use incular_scroll::ScrollController;

use super::{Scrollable, SliverRevealGeometry};
use crate::tree::{ElementId, RenderKind, WidgetTree};

/// A viewport reveal calculation (Flutter `RevealedOffset`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RevealedOffset {
    /// The logical scroll offset that would reveal the requested rectangle.
    pub offset: f32,
    /// The rectangle's projected placement in viewport-local coordinates.
    pub rect: Rect,
}

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
#[derive(Clone, Debug, PartialEq)]
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

    fn reveal_sliver_for_rect(
        &self,
        viewport: &AncestorViewport,
        target: ElementId,
        rect: Rect,
    ) -> SliverRevealGeometry {
        let mut sliver = self.sliver_reveal_geometry(viewport.element, target);
        if !sliver.pins_at_edge {
            return sliver;
        }

        // Flutter only treats the target as pinned if its requested rect
        // does not protrude past the sliver's growth-leading edge.
        let child_bounds = self
            .viewport_child_containing(viewport.element, target)
            .and_then(|child| {
                let (_, size) = self.element_geometry(child)?;
                self.rect_in_viewport(
                    child,
                    Rect::from_origin_size(Offset::ZERO, size),
                    viewport.element,
                )
            });
        sliver.pins_at_edge = child_bounds.is_some_and(|child| {
            let (start, child_start) = match viewport.axis {
                Axis::Vertical => (rect.origin.y, child.origin.y),
                Axis::Horizontal => (rect.origin.x, child.origin.x),
            };
            if viewport.reverse ^ sliver.reverse_growth {
                start + viewport.axis.main_extent(rect.size)
                    <= child_start + viewport.axis.main_extent(child.size)
            } else {
                start >= child_start
            }
        });
        sliver
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
    sliver: SliverRevealGeometry,
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
    let leading_obstruction = if sliver.reverse_growth {
        0.
    } else {
        sliver.obstruction_before
    };
    let leading_scroll_offset = pixels + leading - leading_obstruction;
    if sliver.pins_at_edge {
        if !sliver.reverse_growth && alignment <= 0. {
            return pixels.max(leading_scroll_offset);
        }
        if sliver.reverse_growth && alignment >= 1. {
            // The reverse-growing header sticks to the trailing edge.
            // Its current logical trailing coordinate is the latest
            // offset at which the query must move toward it.
            return pixels.min(pixels + leading + extent);
        }
    }
    leading_scroll_offset - (viewport_extent - sliver.obstruction_before - extent) * alignment
}

impl Scrollable {
    /// Calculates where the nearest enclosing viewport would reveal a target,
    /// without scrolling it. Alignment 0 is the logical leading edge and
    /// alignment 1 the trailing edge. `rect` is in target-local coordinates;
    /// `None` uses the target's layout bounds.
    ///
    /// Use after layout. The offset is not clamped to the scroll range.
    /// Wheel and two-dimensional viewports are outside this query.
    #[must_use]
    pub fn get_offset_to_reveal(
        tree: &WidgetTree,
        target: ElementId,
        alignment: f32,
        rect: Option<Rect>,
    ) -> Option<RevealedOffset> {
        if !alignment.is_finite() {
            return None;
        }
        let viewport = tree.enclosing_viewport(target)?;
        let (_, target_size) = tree.element_geometry(target)?;
        let (_, viewport_size) = tree.element_geometry(viewport.element)?;
        let rect = tree.rect_in_viewport(
            target,
            rect.unwrap_or_else(|| Rect::from_origin_size(Offset::ZERO, target_size)),
            viewport.element,
        )?;
        let pixels = viewport.controller.offset();
        let offset = offset_to_reveal(
            rect,
            viewport_size,
            viewport.axis,
            viewport.reverse,
            pixels,
            alignment,
            tree.reveal_sliver_for_rect(&viewport, target, rect),
        );
        if !offset.is_finite() || !rect.origin.x.is_finite() || !rect.origin.y.is_finite() {
            return None;
        }
        let shift = pixels - offset;
        let shift = if viewport.reverse { -shift } else { shift };
        let translation = match viewport.axis {
            Axis::Vertical => Offset::new(0., shift),
            Axis::Horizontal => Offset::new(shift, 0.),
        };
        Some(RevealedOffset {
            offset,
            rect: Rect::from_origin_size(rect.origin + translation, rect.size),
        })
    }

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
    /// Call it after layout. A sequential sliver viewport accounts for
    /// its preceding pinned slivers. Wheel and two-dimensional viewports
    /// are not walked.
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
            let sliver = tree.reveal_sliver_for_rect(&viewport, object, rect);
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
                    sliver,
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
                controller.animate_to(target, options.duration, options.curve.clone());
            }
        }
        count
    }
}
