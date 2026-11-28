//! The same two-pass flex allocation serves live and speculative layout.

use super::*;
use incular_layout::{ChildLayout, Flex, FlexChild};

pub(super) fn flex_axis_limits(axis: Axis, constraints: Constraints) -> (f32, f32) {
    if axis.is_horizontal() {
        (constraints.max_width(), constraints.max_height())
    } else {
        (constraints.max_height(), constraints.max_width())
    }
}

pub(super) fn invalid_unbounded_flex(
    flex: &Flex,
    main_max: f32,
    metadata: &[(u32, FlexFit)],
) -> Option<usize> {
    (cfg!(debug_assertions) && !main_max.is_finite())
        .then(|| {
            metadata.iter().position(|(factor, fit)| {
                *factor > 0 && (flex.main_axis_size == MainAxisSize::Max || *fit == FlexFit::Tight)
            })
        })
        .flatten()
}

impl WidgetTree {
    pub(super) fn flex_child_metadata(&self, id: RenderObjectId) -> (u32, FlexFit) {
        match self
            .render_live(&id, "flex child must remain live")
            .object
            .kind()
        {
            RenderKind::Flexible { flex, fit } => (*flex, *fit),
            _ => (0, FlexFit::Loose),
        }
    }
}

pub(super) fn measure_flex_children<E>(
    constraints: Constraints,
    flex: &Flex,
    metadata: &[(u32, FlexFit)],
    mut measure: impl FnMut(usize, Constraints) -> Result<(Size, Option<f32>), E>,
) -> Result<(Vec<FlexChild>, f64), E> {
    let (main_max, cross_max) = flex_axis_limits(flex.direction, constraints);
    let cross_min = if matches!(flex.cross_axis_alignment, CrossAxisAlignment::Stretch)
        && cross_max.is_finite()
    {
        cross_max
    } else {
        0.
    };
    let bounds = |main_min, main_max| match flex.direction {
        Axis::Horizontal => Constraints::new(main_min, main_max, cross_min, cross_max),
        Axis::Vertical => Constraints::new(cross_min, cross_max, main_min, main_max),
    };
    let non_flex = bounds(0., f32::INFINITY);
    let mut fixed = f64::from(flex.spacing.max(0.)) * metadata.len().saturating_sub(1) as f64;
    let mut units = 0_u64;
    let mut measured = Vec::with_capacity(metadata.len());
    let child = |size, baseline| FlexChild {
        size,
        baseline,
        ..FlexChild::new(size)
    };
    for (index, (factor, _)) in metadata.iter().copied().enumerate() {
        if factor != 0 && main_max.is_finite() {
            units += u64::from(factor);
            measured.push(FlexChild::new(Size::ZERO));
        } else {
            let (size, baseline) = measure(index, non_flex)?;
            fixed += f64::from(flex.direction.main_extent(size));
            measured.push(child(size, baseline));
        }
    }
    if units != 0 {
        let share = (f64::from(main_max) - fixed).max(0.) / units as f64;
        for (index, (factor, fit)) in metadata.iter().copied().enumerate() {
            if factor != 0 {
                let allocated = (share * f64::from(factor)) as f32;
                let minimum = if fit == FlexFit::Tight { allocated } else { 0. };
                let (size, baseline) = measure(index, bounds(minimum, allocated))?;
                // Measured size already carries the flexible share chosen by
                // its child. Placement must not allocate the share again.
                measured[index] = child(size, baseline);
            }
        }
    }
    Ok((measured, fixed))
}

pub(super) fn positioned_flex_baseline(direction: Axis, children: &[ChildLayout]) -> Option<f32> {
    let mut baselines = children
        .iter()
        .filter_map(|child| child.baseline.map(|baseline| child.offset.y + baseline));
    match direction {
        Axis::Horizontal => baselines.reduce(f32::min),
        Axis::Vertical => baselines.next(),
    }
}

impl WidgetTree {
    pub(super) fn dry_flex_intrinsic_extent(
        &self,
        children: &[RenderObjectId],
        flex: &Flex,
        axis: Axis,
        kind: super::intrinsics::IntrinsicKind,
        extent: f32,
    ) -> Option<f32> {
        use super::intrinsics::IntrinsicKind;
        if flex.direction == axis {
            let mut fixed = f64::from(flex.spacing) * children.len().saturating_sub(1) as f64;
            let mut units = 0_u64;
            let mut largest_fraction = 0_f64;
            for &child in children {
                let (factor, _) = self.flex_child_metadata(child);
                let contribution = with_recursive_tree_stack(|| {
                    self.dry_intrinsic_extent(child, axis, kind, extent)
                })?;
                if factor == 0 {
                    fixed += f64::from(contribution);
                } else {
                    units += u64::from(factor);
                    largest_fraction =
                        largest_fraction.max(f64::from(contribution) / f64::from(factor));
                }
            }
            return Some((fixed + largest_fraction * units as f64) as f32);
        }

        let constraints = if flex.direction.is_horizontal() {
            Constraints::at_most(extent, f32::INFINITY)
        } else {
            Constraints::at_most(f32::INFINITY, extent)
        };
        let metadata = children
            .iter()
            .map(|&id| self.flex_child_metadata(id))
            .collect::<Vec<_>>();
        let baseline_aligned = flex.direction.is_horizontal()
            && flex.cross_axis_alignment == CrossAxisAlignment::Baseline;
        let (measured, _) = measure_flex_children(constraints, flex, &metadata, |index, bounds| {
            let child = children[index];
            with_recursive_tree_stack(|| {
                let (allocated, _) = flex_axis_limits(flex.direction, bounds);
                let main = if allocated.is_finite() {
                    allocated
                } else {
                    self.dry_intrinsic_extent(
                        child,
                        flex.direction,
                        IntrinsicKind::Max,
                        f32::INFINITY,
                    )
                    .ok_or(())?
                };
                let cross = self
                    .dry_intrinsic_extent(child, axis, kind, main)
                    .ok_or(())?;
                let size = if flex.direction.is_horizontal() {
                    Size::maybe_new(main, cross)
                } else {
                    Size::maybe_new(cross, main)
                }
                .ok_or(())?;
                let baseline = if baseline_aligned {
                    self.dry_size_and_baseline(child, bounds, true).ok_or(())?.1
                } else {
                    None
                };
                Ok::<_, ()>((size, baseline))
            })
        })
        .ok()?;
        Some(
            flex.direction
                .cross_extent(incular_layout::layout_flex(constraints, &measured, flex).size),
        )
    }

    pub(super) fn dry_flex_metrics(
        &self,
        children: &[RenderObjectId],
        flex: &Flex,
        constraints: Constraints,
        report_baseline: bool,
    ) -> Option<(Size, Option<f32>)> {
        let metadata = children
            .iter()
            .map(|&id| self.flex_child_metadata(id))
            .collect::<Vec<_>>();
        let (main_max, _) = flex_axis_limits(flex.direction, constraints);
        if invalid_unbounded_flex(flex, main_max, &metadata).is_some() {
            return None;
        }
        let measure_baseline = report_baseline
            || (flex.direction.is_horizontal()
                && flex.cross_axis_alignment == CrossAxisAlignment::Baseline);
        let (measured, _) = measure_flex_children(constraints, flex, &metadata, |index, bounds| {
            let child = children[index];
            with_recursive_tree_stack(|| {
                self.dry_size_and_baseline(child, bounds, measure_baseline)
            })
            .ok_or(())
        })
        .ok()?;
        let positioned = incular_layout::layout_flex(constraints, &measured, flex);
        let baseline = if report_baseline {
            positioned_flex_baseline(flex.direction, &positioned.children)
        } else {
            None
        };
        Some((positioned.size, baseline))
    }
}
