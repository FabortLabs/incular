//! Non-mutating intrinsic queries for retained render boxes.

use super::*;

#[derive(Clone, Copy)]
enum IntrinsicKind {
    Min,
    Max,
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

    fn dry_intrinsic_extent(
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
            _ => None,
        }
    }
}
