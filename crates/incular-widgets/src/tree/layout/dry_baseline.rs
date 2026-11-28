//! Side-effect-free baseline measurement for shifted render boxes.

use super::*;
use crate::tree::widget::{aligned_child_offset, enforced_constraints, fractional_constraints};

type DryMetrics = (Size, Option<f32>);

fn shifted((child_size, baseline): DryMetrics, alignment: Alignment, size: Size) -> DryMetrics {
    (
        size,
        baseline.map(|baseline| baseline + aligned_child_offset(alignment, size, child_size).y),
    )
}

impl WidgetTree {
    /// Queries a built-in box's alphabetic baseline under the given bounds,
    /// without replacing the last laid-out size, offsets, or baseline.
    /// A box with no baseline, unsupported render kind, or stale ID
    /// returns `None`.
    #[must_use]
    pub fn dry_alphabetic_baseline(
        &self,
        id: RenderObjectId,
        constraints: Constraints,
    ) -> Option<f32> {
        self.dry_size_and_baseline(id, constraints, true)?.1
    }

    fn dry_size_and_baseline(
        &self,
        id: RenderObjectId,
        constraints: Constraints,
        measure_baseline: bool,
    ) -> Option<DryMetrics> {
        let node = self.renders.get(id.0)?;
        let child = |bounds: Constraints| {
            node.children
                .first()
                .map_or(Some((bounds.smallest(), None)), |&child| {
                    with_recursive_tree_stack(|| {
                        self.dry_size_and_baseline(child, bounds, measure_baseline)
                    })
                })
        };
        match node.object.kind() {
            RenderKind::Text { .. } if node.children.is_empty() => {
                self.dry_leaf_paragraph_metrics(id, constraints)
            }
            RenderKind::Constrained {
                constraints: bounds,
            } => child(enforced_constraints(constraints, *bounds)),
            RenderKind::Align {
                alignment,
                width_factor,
                height_factor,
            } => {
                let (child_size, baseline) = child(constraints.loosen())?;
                let (size, offset) = incular_layout::align_child(
                    constraints,
                    child_size,
                    incular_layout::Align {
                        alignment: *alignment,
                        width_factor: *width_factor,
                        height_factor: *height_factor,
                    },
                );
                Some((size, baseline.map(|baseline| baseline + offset.y)))
            }
            RenderKind::Padding { padding } => {
                let (size, baseline) =
                    child(constraints.deflate(padding.horizontal(), padding.vertical()))?;
                Some((
                    constraints.constrain(Size::new(
                        size.width + padding.horizontal(),
                        size.height + padding.vertical(),
                    )),
                    baseline.map(|baseline| baseline + padding.top),
                ))
            }
            RenderKind::Baseline { baseline } => {
                let (size, child_baseline) = child(constraints.loosen())?;
                let layout = incular_layout::layout_baseline(
                    constraints,
                    incular_layout::BaselineChild {
                        size,
                        baseline: child_baseline,
                    },
                    *baseline,
                );
                Some((layout.size, child_baseline.map(|_| *baseline)))
            }
            RenderKind::Overflow {
                min_width,
                max_width,
                min_height,
                max_height,
                alignment,
                fit,
            } => {
                if !measure_baseline && *fit == OverflowBoxFit::DeferToChild {
                    return child(constraints);
                }
                let min_width = min_width.unwrap_or(constraints.min_width());
                let min_height = min_height.unwrap_or(constraints.min_height());
                let inner = Constraints::new(
                    min_width,
                    max_width.unwrap_or(constraints.max_width()).max(min_width),
                    min_height,
                    max_height
                        .unwrap_or(constraints.max_height())
                        .max(min_height),
                );
                let result = child(inner)?;
                let size = match fit {
                    OverflowBoxFit::Max => Size::new(
                        if constraints.is_width_bounded() {
                            constraints.max_width()
                        } else {
                            result.0.width
                        },
                        if constraints.is_height_bounded() {
                            constraints.max_height()
                        } else {
                            result.0.height
                        },
                    ),
                    OverflowBoxFit::DeferToChild if inner == constraints => result.0,
                    OverflowBoxFit::DeferToChild => match node.children.first() {
                        Some(&child) => {
                            with_recursive_tree_stack(|| {
                                self.dry_size_and_baseline(child, constraints, false)
                            })?
                            .0
                        }
                        None => constraints.smallest(),
                    },
                };
                Some(shifted(result, *alignment, size))
            }
            RenderKind::SizedOverflow { size, alignment } => Some(shifted(
                child(constraints)?,
                *alignment,
                constraints.constrain(*size),
            )),
            RenderKind::Fractional {
                width_factor,
                height_factor,
                alignment,
            } => {
                let result = child(fractional_constraints(
                    constraints,
                    *width_factor,
                    *height_factor,
                ))?;
                Some(shifted(result, *alignment, constraints.constrain(result.0)))
            }
            kind if super::intrinsics::forwards_intrinsic_child(kind) => child(constraints),
            _ => Some((self.dry_layout_size(id, constraints)?, None)),
        }
    }
}
