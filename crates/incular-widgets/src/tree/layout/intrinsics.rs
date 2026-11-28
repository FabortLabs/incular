//! Non-mutating intrinsic queries for retained render boxes.

use super::*;

#[derive(Clone, Copy)]
pub(super) enum IntrinsicKind {
    Min,
    Max,
}

/// Paint and hit-test proxies have no intrinsic extent of their own. Keep
/// this classification shared by the dry API and the layout-time probe.
pub(super) fn forwards_intrinsic_child(kind: &RenderKind) -> bool {
    matches!(
        kind,
        RenderKind::Colored { .. }
            | RenderKind::Decorated { desired: None, .. }
            | RenderKind::Opacity { .. }
            | RenderKind::BackdropFilter { .. }
            | RenderKind::ProxyLayout {
                mode: ChildLayoutMode::SemanticAnnotation,
            }
            | RenderKind::ClipRect { .. }
            | RenderKind::Gesture
            | RenderKind::Flexible { .. }
            | RenderKind::RepaintBoundary
    )
}

impl WidgetTree {
    /// Returns the retained box's minimum intrinsic width at the given height.
    ///
    /// Use positive infinity for an unbounded height. Returns `None` for an
    /// invalid height, a stale render ID, or a render kind without dry
    /// intrinsic support. This does not execute layout callbacks.
    #[must_use]
    pub fn min_intrinsic_width(&self, id: RenderObjectId, height: f32) -> Option<f32> {
        self.dry_intrinsic_extent(id, Axis::Horizontal, IntrinsicKind::Min, height)
    }

    /// Returns the retained box's maximum intrinsic width at the given height.
    ///
    /// See [Self::min_intrinsic_width] for supported inputs and absence.
    #[must_use]
    pub fn max_intrinsic_width(&self, id: RenderObjectId, height: f32) -> Option<f32> {
        self.dry_intrinsic_extent(id, Axis::Horizontal, IntrinsicKind::Max, height)
    }

    /// Returns the retained box's minimum intrinsic height at the given width.
    ///
    /// Use positive infinity for an unbounded width. Returns `None` for an
    /// invalid width, a stale render ID, or a render kind without dry
    /// intrinsic support. This does not execute layout callbacks.
    #[must_use]
    pub fn min_intrinsic_height(&self, id: RenderObjectId, width: f32) -> Option<f32> {
        self.dry_intrinsic_extent(id, Axis::Vertical, IntrinsicKind::Min, width)
    }

    /// Returns the retained box's maximum intrinsic height at the given width.
    ///
    /// See [Self::min_intrinsic_height] for supported inputs and absence.
    #[must_use]
    pub fn max_intrinsic_height(&self, id: RenderObjectId, width: f32) -> Option<f32> {
        self.dry_intrinsic_extent(id, Axis::Vertical, IntrinsicKind::Max, width)
    }

    pub(super) fn dry_intrinsic_extent(
        &self,
        id: RenderObjectId,
        axis: Axis,
        kind: IntrinsicKind,
        cross: f32,
    ) -> Option<f32> {
        if cross.is_nan() || cross < 0. {
            return None;
        }
        let node = self.renders.get(id.0)?;
        let query_child = |axis, kind, cross| match node.children.first() {
            Some(&child) => {
                with_recursive_tree_stack(|| self.dry_intrinsic_extent(child, axis, kind, cross))
            }
            None => Some(0.),
        };
        let child_extent = |cross| query_child(axis, kind, cross);
        match node.object.kind() {
            RenderKind::Box { desired, .. } | RenderKind::Shape { desired, .. } => {
                Some(axis.main_extent(*desired))
            }
            RenderKind::Flex { flex, .. } => {
                self.dry_flex_intrinsic_extent(&node.children, flex, axis, kind, cross)
            }
            RenderKind::Text {
                text,
                style,
                text_direction,
                spans,
                ..
            } if node.children.is_empty() => match axis {
                Axis::Horizontal if spans.is_none() => {
                    let mut engine = self.text_engine.borrow_mut();
                    if engine.has_pending_application_fonts() {
                        self.dry_shape_refreshed_fonts.set(true);
                    }
                    let widths = engine.intrinsic_widths(text, style, *text_direction);
                    Some(match kind {
                        IntrinsicKind::Min => widths.min,
                        IntrinsicKind::Max => widths.max,
                    })
                }
                Axis::Vertical => self
                    .dry_leaf_paragraph_metrics(id, Constraints::at_most(cross, f32::INFINITY))
                    .map(|(size, _)| size.height),
                _ => None,
            },
            RenderKind::CustomPaint { desired, .. } => {
                // Fill-to-constraints is an unbounded preferred size, not
                // an intrinsic width or height.
                Some(desired.map_or(0., |size| {
                    let extent = axis.main_extent(size);
                    if extent.is_finite() { extent } else { 0. }
                }))
            }
            RenderKind::Align {
                width_factor,
                height_factor,
                ..
            } => {
                let factor = if axis.is_horizontal() {
                    width_factor
                } else {
                    height_factor
                };
                // The other axis is unchanged; only the child's reported
                // intrinsic result is scaled.
                Some(child_extent(cross)? * factor.unwrap_or(1.))
            }
            RenderKind::Padding { padding } => {
                let (main_insets, cross_insets) = if axis.is_horizontal() {
                    (padding.horizontal(), padding.vertical())
                } else {
                    (padding.vertical(), padding.horizontal())
                };
                Some(child_extent((cross - cross_insets).max(0.))? + main_insets)
            }
            RenderKind::Constrained { constraints } => {
                let (min, max) = if axis.is_horizontal() {
                    (constraints.min_width(), constraints.max_width())
                } else {
                    (constraints.min_height(), constraints.max_height())
                };
                if min == max && max.is_finite() {
                    return Some(min);
                }
                let natural = child_extent(cross)?;
                Some(if min.is_finite() {
                    natural.clamp(min, max)
                } else {
                    natural
                })
            }
            RenderKind::Wrap { wrap, .. } if wrap.direction == axis => {
                let mut extent = 0.0f32;
                for &child in &node.children {
                    let value = with_recursive_tree_stack(|| {
                        self.dry_intrinsic_extent(child, axis, kind, f32::INFINITY)
                    })?;
                    extent = match kind {
                        IntrinsicKind::Min => extent.max(value),
                        IntrinsicKind::Max => extent + value,
                    };
                }
                Some(extent)
            }
            RenderKind::Wrap { wrap, .. } => {
                let constraints = match wrap.direction {
                    Axis::Horizontal => Constraints::at_most(cross, f32::INFINITY),
                    Axis::Vertical => Constraints::at_most(f32::INFINITY, cross),
                };
                self.dry_wrap_size(&node.children, wrap, constraints)
                    .map(|size| axis.main_extent(size))
            }
            RenderKind::AspectRatio { ratio } => incular_layout::AspectRatio::new(*ratio)
                .intrinsic_extent(axis, cross)
                .or_else(|| child_extent(cross)),
            RenderKind::ProxyLayout {
                mode: ChildLayoutMode::MaxIntrinsicSize { maximum, minimum },
            } => match kind {
                IntrinsicKind::Min => minimum
                    .map(|size| axis.main_extent(size))
                    .or_else(|| child_extent(cross).map(|min| min.min(axis.main_extent(*maximum)))),
                IntrinsicKind::Max => Some(axis.main_extent(*maximum)),
            },
            RenderKind::ProxyLayout {
                mode:
                    ChildLayoutMode::IntrinsicWidth {
                        step_width,
                        step_height,
                    },
            } => {
                use incular_layout::round_intrinsic_step;
                if axis.is_horizontal() {
                    let width = query_child(axis, IntrinsicKind::Max, cross)?;
                    return Some(round_intrinsic_step(width, *step_width));
                }

                let width = if cross.is_finite() {
                    cross
                } else {
                    let width = query_child(Axis::Horizontal, IntrinsicKind::Max, cross)?;
                    round_intrinsic_step(width, *step_width)
                };
                if !width.is_finite() {
                    return None;
                }
                child_extent(width).map(|height| round_intrinsic_step(height, *step_height))
            }
            RenderKind::ProxyLayout {
                mode: ChildLayoutMode::IntrinsicHeight,
            } => {
                if !axis.is_horizontal() {
                    return query_child(axis, IntrinsicKind::Max, cross);
                }

                let height = if cross.is_finite() {
                    cross
                } else {
                    query_child(Axis::Vertical, IntrinsicKind::Max, cross)?
                };
                if !height.is_finite() {
                    return None;
                }
                child_extent(height)
            }
            kind if forwards_intrinsic_child(kind) => child_extent(cross),
            _ => None,
        }
    }

    /// Computes a built-in box's dry size without laying out the mounted
    /// tree, changing existing geometry, or invoking a painter. Unsupported
    /// render kinds and stale IDs return no result rather than a wet estimate.
    #[must_use]
    pub fn dry_layout_size(&self, id: RenderObjectId, constraints: Constraints) -> Option<Size> {
        let node = self.renders.get(id.0)?;
        let child = |constraints: Constraints| match node.children.first() {
            Some(&id) => with_recursive_tree_stack(|| self.dry_layout_size(id, constraints)),
            None => Some(constraints.smallest()),
        };
        match node.object.kind() {
            RenderKind::Box { desired, .. } | RenderKind::Shape { desired, .. } => {
                Some(constraints.constrain(*desired))
            }
            RenderKind::Constrained {
                constraints: additional,
            } => child(crate::tree::widget::enforced_constraints(
                constraints,
                *additional,
            )),
            RenderKind::Flex { flex, .. } => self
                .dry_flex_metrics(&node.children, flex, constraints, false)
                .map(|(size, _)| size),
            RenderKind::AspectRatio { ratio } => incular_layout::aspect_ratio_size(
                constraints,
                incular_layout::AspectRatio::new(*ratio),
            ),
            RenderKind::Wrap { wrap, .. } => self.dry_wrap_size(&node.children, wrap, constraints),
            RenderKind::Text { .. } if node.children.is_empty() => self
                .dry_leaf_paragraph_metrics(id, constraints)
                .map(|(size, _)| size),
            RenderKind::ProxyLayout {
                mode: ChildLayoutMode::QuarterTurns(turns),
            } => {
                if turns.rem_euclid(2) == 0 {
                    return child(constraints);
                }
                let flipped = Constraints::new(
                    constraints.min_height(),
                    constraints.max_height(),
                    constraints.min_width(),
                    constraints.max_width(),
                );
                let size = child(flipped)?;
                Some(constraints.constrain(Size::new(size.height, size.width)))
            }
            RenderKind::Transform { .. } => child(constraints),
            kind if forwards_intrinsic_child(kind) => child(constraints),
            _ => None,
        }
    }

    pub(super) fn dry_leaf_paragraph_metrics(
        &self,
        id: RenderObjectId,
        constraints: Constraints,
    ) -> Option<(Size, Option<f32>)> {
        let node = self.renders.get(id.0)?;
        let RenderKind::Text { inline, .. } = node.object.kind() else {
            return None;
        };
        let placeholders = inline
            .as_ref()
            .map_or(&[][..], |inline| &inline.placeholders);
        let mut engine = self.text_engine.borrow_mut();
        if engine.has_pending_application_fonts() {
            // Shaping syncs the collection. The next frame must still
            // invalidate previously measured text geometry.
            self.dry_shape_refreshed_fonts.set(true);
        }
        let layout = super::behavior::shape_render_paragraph(
            &mut engine,
            node.object.kind(),
            placeholders,
            constraints,
        )?;
        Some((
            constraints.constrain(layout.metrics.size),
            Some(layout.metrics.baseline),
        ))
    }

    fn dry_wrap_size(
        &self,
        children: &[RenderObjectId],
        wrap: &incular_layout::Wrap,
        constraints: Constraints,
    ) -> Option<Size> {
        let axis = wrap.direction;
        let child_constraints = match axis {
            Axis::Horizontal => Constraints::at_most(constraints.max_width(), f32::INFINITY),
            Axis::Vertical => Constraints::at_most(f32::INFINITY, constraints.max_height()),
        };
        let limit = f64::from(match axis {
            Axis::Horizontal => child_constraints.max_width(),
            Axis::Vertical => child_constraints.max_height(),
        });
        let spacing = f64::from(wrap.spacing);
        let run_spacing = f64::from(wrap.run_spacing);
        let (mut main, mut cross) = (0.0f64, 0.0f64);
        let (mut run_main, mut run_cross) = (0.0f64, 0.0f64);
        let mut run_has_child = false;
        for &child in children {
            let size =
                with_recursive_tree_stack(|| self.dry_layout_size(child, child_constraints))?;
            let item_main = f64::from(axis.main_extent(size));
            let item_cross = f64::from(axis.cross_extent(size));
            // Flutter's dry measurement uses a strict limit; live layout's
            // placement tolerance intentionally does not apply here.
            if run_has_child && run_main + spacing + item_main > limit {
                main = main.max(run_main);
                cross += run_cross + run_spacing;
                run_main = 0.;
                run_cross = 0.;
                run_has_child = false;
            }
            run_main += if run_has_child {
                spacing + item_main
            } else {
                item_main
            };
            run_cross = run_cross.max(item_cross);
            run_has_child = true;
        }
        let (main, cross) = (main.max(run_main), cross + run_cross);
        let (width, height) = match axis {
            Axis::Horizontal => (main, cross),
            Axis::Vertical => (cross, main),
        };
        let clamp = |value: f64, minimum, maximum| {
            value.clamp(f64::from(minimum), f64::from(maximum)) as f32
        };
        Size::maybe_new(
            clamp(width, constraints.min_width(), constraints.max_width()),
            clamp(height, constraints.min_height(), constraints.max_height()),
        )
    }
}
