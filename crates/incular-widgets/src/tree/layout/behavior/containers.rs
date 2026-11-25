use super::super::*;
use crate::custom_layout::CustomLayoutDelegate;
use std::collections::HashMap;

impl WidgetTree {
    pub(super) fn layout_container_kind(
        &mut self,
        id: RenderObjectId,
        kind: &RenderKind,
        children: &[RenderObjectId],
        constraints: Constraints,
    ) -> Result<(Size, ChildOffsets), TreeError> {
        Ok(match *kind {
            RenderKind::Align {
                alignment,
                width_factor,
                height_factor,
            } => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints.loosen())?;
                    let child = self.render_live(child, "aligned child must remain live");
                    let (child_size, child_baseline) = (child.size, child.baseline);
                    let (size, offset) = incular_layout::align_child(
                        constraints,
                        child_size,
                        incular_layout::Align {
                            alignment,
                            width_factor,
                            height_factor,
                        },
                    );
                    self.render_live_mut(id, "align render must remain live")
                        .baseline = child_baseline.map(|local| local + offset.y);
                    (size, ChildOffsets::One(offset))
                } else {
                    self.render_live_mut(id, "align render must remain live")
                        .baseline = None;
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::Flex { flex, .. } => {
                let cross_max = match flex.direction {
                    Axis::Horizontal => constraints.max_height(),
                    Axis::Vertical => constraints.max_width(),
                };
                let main_max = match flex.direction {
                    Axis::Horizontal => constraints.max_width(),
                    Axis::Vertical => constraints.max_height(),
                };
                // Flutter's fillCrossAxis: stretch tightens the cross axis of
                // every child to the line's cross maximum (flex.dart:883, 913).
                let fill_cross = matches!(flex.cross_axis_alignment, CrossAxisAlignment::Stretch)
                    && cross_max.is_finite();
                let non_flex = match flex.direction {
                    Axis::Horizontal => Constraints::new(
                        0.,
                        f32::INFINITY,
                        if fill_cross { cross_max } else { 0. },
                        cross_max,
                    ),
                    Axis::Vertical => Constraints::new(
                        if fill_cross { cross_max } else { 0. },
                        cross_max,
                        0.,
                        f32::INFINITY,
                    ),
                };
                let flex_meta = children
                    .iter()
                    .map(|child| {
                        let render_node =
                            self.render_live(child, "retained render must remain live");
                        match render_node.object.kind() {
                            RenderKind::Flexible { flex: f, fit } => (*f, *fit),
                            _ => (0, FlexFit::Loose),
                        }
                    })
                    .collect::<Vec<_>>();

                let mut flex_children = Vec::with_capacity(children.len());
                let spacing =
                    f64::from(flex.spacing.max(0.0)) * children.len().saturating_sub(1) as f64;
                let mut occupied_non_flex = spacing;
                for (child, (f, _)) in children.iter().zip(&flex_meta) {
                    if *f == 0 || !main_max.is_finite() {
                        self.layout_render(*child, non_flex)?;
                        let render = self.render_live(child, "retained render must remain live");
                        let size = render.size;
                        // Baselines ride into the algorithm so
                        // cross-axis baseline alignment resolves instead
                        // of silently behaving as start.
                        let mut flex_child = incular_layout::FlexChild::new(size);
                        if let Some(baseline) = render.baseline {
                            flex_child = flex_child.with_baseline(baseline);
                        }
                        occupied_non_flex += f64::from(flex.direction.main_extent(size));
                        flex_children.push(flex_child);
                    } else {
                        flex_children.push(incular_layout::FlexChild::new(Size::ZERO));
                    }
                }

                let total_flex: u64 = flex_meta.iter().map(|(f, _)| u64::from(*f)).sum();
                if main_max.is_finite() && total_flex > 0 {
                    let space_per_flex =
                        (f64::from(main_max) - occupied_non_flex).max(0.0) / total_flex as f64;
                    for (i, (child, (f, fit))) in children.iter().zip(&flex_meta).enumerate() {
                        if *f > 0 {
                            let allocation = (space_per_flex * f64::from(*f)) as f32;
                            let child_constraints = match flex.direction {
                                Axis::Horizontal => Constraints::new(
                                    if *fit == FlexFit::Tight {
                                        allocation
                                    } else {
                                        0.0
                                    },
                                    allocation,
                                    if fill_cross { cross_max } else { 0.0 },
                                    cross_max,
                                ),
                                Axis::Vertical => Constraints::new(
                                    if fill_cross { cross_max } else { 0.0 },
                                    cross_max,
                                    if *fit == FlexFit::Tight {
                                        allocation
                                    } else {
                                        0.0
                                    },
                                    allocation,
                                ),
                            };
                            self.layout_render(*child, child_constraints)?;
                            let render =
                                self.render_live(child, "retained render must remain live");
                            let size = render.size;
                            // Placement uses measured sizes, without allocating the same
                            // flexible shares a second time from rounded inputs.
                            let mut flex_child = incular_layout::FlexChild::new(size);
                            if let Some(baseline) = render.baseline {
                                flex_child = flex_child.with_baseline(baseline);
                            }
                            flex_children[i] = flex_child;
                        }
                    }
                }

                let result = incular_layout::layout_flex(constraints, &flex_children, flex);
                let mut baseline: Option<f32> = None;
                let offsets = result
                    .children
                    .into_iter()
                    .map(|child| {
                        if let Some(local) = child.baseline {
                            let positioned = child.offset.y + local;
                            // Flutter uses the highest real baseline in a row
                            // and the first in logical child order in a column.
                            baseline = Some(match baseline {
                                None => positioned,
                                Some(current) if flex.direction == Axis::Horizontal => {
                                    current.min(positioned)
                                }
                                Some(current) => current,
                            });
                        }
                        child.offset
                    })
                    .collect();
                self.render_live_mut(id, "flex render must remain live")
                    .baseline = baseline;
                // Only unbounded non-flex children and spacing can exceed the
                // budget. Re-summing independently rounded flexible shares
                // can otherwise report overflow for children that fit.
                // Compare non-flex occupancy at the layout coordinate precision
                // before applying Flutter's precisionErrorTolerance.
                let overflow = main_max.is_finite()
                    && f64::from(occupied_non_flex as f32) - f64::from(main_max) > 1e-10;
                self.set_layout_overflow(id, overflow);
                (result.size, offsets)
            }
            RenderKind::CustomLayout { ref delegate } => {
                self.layout_custom(id, delegate, children, constraints)?
            }
            RenderKind::Flexible { .. }
            | RenderKind::Positioned { .. }
            | RenderKind::LayoutId { .. } => {
                // Flutter's Flexible and Positioned are parent-data widgets;
                // neither adds a render box that could hide a real baseline.
                let forwards_baseline = matches!(
                    kind,
                    RenderKind::Flexible { .. } | RenderKind::Positioned { .. }
                );
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints)?;
                    let child = self.render_live(child, "retained render must remain live");
                    let size = child.size;
                    if forwards_baseline {
                        let baseline = child.baseline;
                        self.render_live_mut(id, "retained render must remain live")
                            .baseline = baseline;
                    }
                    (constraints.constrain(size), ChildOffsets::One(Offset::ZERO))
                } else {
                    if forwards_baseline {
                        self.render_live_mut(id, "retained render must remain live")
                            .baseline = None;
                    }
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::ListBody { direction } => {
                let axis = direction.axis();
                let (main_max, cross) = match axis {
                    Axis::Horizontal => (constraints.max_width(), constraints.max_height()),
                    Axis::Vertical => (constraints.max_height(), constraints.max_width()),
                };
                if main_max.is_finite() {
                    return Err(TreeError::InvalidWidgetConfiguration {
                        widget: "ListBody",
                        reason: "RenderListBody must have unlimited space along its main axis. \
                                 Place it in a viewport with the same scrolling axis."
                            .into(),
                    });
                }
                if !cross.is_finite() {
                    return Err(TreeError::InvalidWidgetConfiguration {
                        widget: "ListBody",
                        reason:
                            "RenderListBody must have a bounded constraint for its cross axis. \
                                 Give it a finite extent perpendicular to its main axis."
                                .into(),
                    });
                }
                let child_constraints = match axis {
                    Axis::Horizontal => Constraints::new(0., f32::INFINITY, cross, cross),
                    Axis::Vertical => Constraints::new(cross, cross, 0., f32::INFINITY),
                };
                let mut extent = 0.0;
                for &child in children {
                    self.layout_render(child, child_constraints)?;
                    extent += axis
                        .main_extent(self.render_live(child, "list body child remains live").size);
                }

                let mut baseline = None;
                let mut cursor = 0.0;
                let offsets = children
                    .iter()
                    .map(|child| {
                        let child = self.render_live(child, "list body child remains live");
                        let child_extent = axis.main_extent(child.size);
                        let main = if direction.is_reversed() {
                            extent - (cursor + child_extent)
                        } else {
                            cursor
                        };
                        cursor += child_extent;
                        let offset = axis.offset(main, 0.0);
                        // RenderListBody uses the first real baseline in child
                        // order even if reverse puts another child above it.
                        if baseline.is_none() {
                            baseline = child.baseline.map(|local| local + offset.y);
                        }
                        offset
                    })
                    .collect();
                self.render_live_mut(id, "list body remains live").baseline = baseline;
                (constraints.constrain(axis.size(extent, cross)), offsets)
            }
            RenderKind::OverflowBar {
                spacing,
                overflow_spacing,
                overflow_alignment,
                alignment,
                overflow_direction,
                text_direction,
            } => {
                if children.is_empty() {
                    self.render_live_mut(id, "overflow bar remains live")
                        .baseline = None;
                    return Ok((constraints.smallest(), ChildOffsets::default()));
                }

                let child_constraints = constraints.loosen();
                let mut children_width = 0.0;
                let mut children_height = 0.0;
                let mut max_height: f32 = 0.0;
                for child in children {
                    self.layout_render(*child, child_constraints)?;
                    let child = self.render_live(child, "overflow bar child remains live");
                    children_width += child.size.width;
                    children_height += child.size.height;
                    max_height = max_height.max(child.size.height);
                }

                let count = children.len() as f32;
                let row_width = children_width + spacing * (count - 1.0);
                let max_width = constraints.max_width();
                let rtl = text_direction == TextDirection::Rtl;
                let mut baseline: Option<f32> = None;
                let mut placement = |child: &RenderObjectId, position: Offset| {
                    if let Some(value) = self
                        .render_live(child, "overflow bar child remains live")
                        .baseline
                    {
                        let value = value + position.y;
                        baseline = Some(baseline.map_or(value, |other| other.min(value)));
                    }
                    position
                };
                let (size, offsets) = if row_width > max_width {
                    let mut y = 0.0;
                    let mut position = |child| {
                        let size = self
                            .render_live(child, "overflow bar child remains live")
                            .size;
                        let remaining = max_width - size.width;
                        let x = match overflow_alignment {
                            WrapCrossAlignment::Center => remaining / 2.0,
                            WrapCrossAlignment::Start => {
                                if rtl {
                                    remaining
                                } else {
                                    0.0
                                }
                            }
                            WrapCrossAlignment::End => {
                                if rtl {
                                    0.0
                                } else {
                                    remaining
                                }
                            }
                        };
                        let offset = placement(child, Offset::new(x, y));
                        y += size.height + overflow_spacing;
                        offset
                    };
                    let offsets = if overflow_direction == VerticalDirection::Up {
                        // Reverse placement without changing the retained child, paint or hit order.
                        let mut offsets = children.iter().rev().map(position).collect::<Vec<_>>();
                        offsets.reverse();
                        offsets.into()
                    } else {
                        children.iter().map(&mut position).collect()
                    };
                    (
                        constraints.constrain(Size::new(
                            max_width,
                            children_height + overflow_spacing * (count - 1.0),
                        )),
                        offsets,
                    )
                } else {
                    if alignment.is_some() && !max_width.is_finite() {
                        return Err(TreeError::InvalidWidgetConfiguration {
                            widget: "OverflowBar",
                            reason: "explicit row alignment requires a bounded width".into(),
                        });
                    }
                    let desired_width = alignment.map_or(row_width, |_| max_width);
                    let size = constraints.constrain(Size::new(desired_width, max_height));
                    let remaining = size.width - row_width;
                    let distributed = size.width - children_width;
                    let (mut x, gap) = match alignment {
                        None | Some(MainAxisAlignment::Start) => (0.0, spacing),
                        Some(MainAxisAlignment::End) => (remaining, spacing),
                        Some(MainAxisAlignment::Center) => (remaining / 2.0, spacing),
                        Some(MainAxisAlignment::SpaceBetween) => (
                            0.0,
                            if children.len() == 1 {
                                0.0
                            } else {
                                distributed / (count - 1.0)
                            },
                        ),
                        Some(MainAxisAlignment::SpaceAround) => {
                            let gap = distributed / count;
                            (gap / 2.0, gap)
                        }
                        Some(MainAxisAlignment::SpaceEvenly) => {
                            let gap = distributed / (count + 1.0);
                            (gap, gap)
                        }
                    };
                    let offsets = children
                        .iter()
                        .map(|child| {
                            let child_size = self
                                .render_live(child, "overflow bar child remains live")
                                .size;
                            let left = if rtl {
                                size.width - x - child_size.width
                            } else {
                                x
                            };
                            x += child_size.width + gap;
                            placement(
                                child,
                                Offset::new(left, (max_height - child_size.height) / 2.0),
                            )
                        })
                        .collect();
                    (size, offsets)
                };
                self.render_live_mut(id, "overflow bar remains live")
                    .baseline = baseline;
                (size, offsets)
            }
            RenderKind::Wrap { wrap } => {
                // Flutter wrap.dart: children get only the main-axis maximum
                // (BoxConstraints(maxWidth:) or BoxConstraints(maxHeight:)), so
                // the cross axis stays unbounded.
                let child_constraints = match wrap.direction {
                    Axis::Horizontal => {
                        Constraints::new(0., constraints.max_width(), 0., f32::INFINITY)
                    }
                    Axis::Vertical => {
                        Constraints::new(0., f32::INFINITY, 0., constraints.max_height())
                    }
                };
                for child in children {
                    self.layout_render(*child, child_constraints)?;
                }
                let wrap_children = children
                    .iter()
                    .map(|child| {
                        incular_layout::WrapChild::new(
                            self.render_live(child, "retained render must remain live")
                                .size,
                        )
                    })
                    .collect::<Vec<_>>();

                let result = incular_layout::layout_wrap(constraints, &wrap_children, wrap);
                let baseline = children
                    .iter()
                    .zip(&result.children)
                    .filter_map(|(child, placement)| {
                        self.render_live(child, "wrapped child remains live")
                            .baseline
                            .map(|local| local + placement.offset.y)
                    })
                    .reduce(f32::min);
                self.render_live_mut(id, "wrap remains live").baseline = baseline;
                (
                    result.size,
                    result.children.into_iter().map(|c| c.offset).collect(),
                )
            }
            RenderKind::Table {
                columns,
                column_spacing,
                row_spacing,
                ref column_widths,
                default_column_width,
                default_vertical_alignment,
            } => {
                for child in children {
                    self.layout_render(*child, constraints.loosen())?;
                }
                let config = incular_layout::Table {
                    columns,
                    column_spacing,
                    row_spacing,
                    column_widths: column_widths.clone(),
                    default_column_width,
                    default_vertical_alignment,
                };
                let measured = children
                    .iter()
                    .map(|child| {
                        incular_layout::TableChild::new(
                            self.render_live(child, "retained render must remain live")
                                .size,
                        )
                    })
                    .collect::<Vec<_>>();
                // Flutter lays every cell out at its column's tight width, so
                // flex and fraction columns stretch the cell content. Column
                // widths come from the loose measurement, then each cell is
                // re-measured at its width.
                let widths = incular_layout::table_column_widths(constraints, &measured, &config);
                let columns_count = config.columns.max(1);
                for (index, child) in children.iter().enumerate() {
                    let width = widths[index % columns_count];
                    self.layout_render(*child, Constraints::new(width, width, 0., f32::INFINITY))?;
                }
                let cells = children
                    .iter()
                    .map(|child| {
                        incular_layout::TableChild::new(
                            self.render_live(child, "retained render must remain live")
                                .size,
                        )
                    })
                    .collect::<Vec<_>>();
                let result = incular_layout::layout_table(constraints, &cells, &config);
                (
                    result.size,
                    result
                        .children
                        .into_iter()
                        .map(|cell| cell.offset)
                        .collect(),
                )
            }
            RenderKind::Stack { .. } | RenderKind::IndexedStack { .. } if children.is_empty() => {
                // Flutter stack.dart: a childless stack takes the biggest size
                // its constraints allow, or the smallest when unbounded.
                let size = if constraints.is_width_bounded() && constraints.is_height_bounded() {
                    constraints.biggest()
                } else {
                    constraints.smallest()
                };
                self.set_layout_overflow(id, false);
                self.render_live_mut(id, "retained render must remain live")
                    .baseline = None;
                (size, ChildOffsets::default())
            }
            RenderKind::Stack { .. } | RenderKind::IndexedStack { .. } => {
                // Both stacks lay out every child. Plain Stack reports the
                // highest actual baseline; IndexedStack reads only its selection.
                let (stack, indexed, active_child) = match *kind {
                    RenderKind::Stack { stack } => (stack, false, None),
                    RenderKind::IndexedStack {
                        alignment,
                        fit,
                        clip_behavior,
                        index,
                    } => (
                        incular_layout::Stack {
                            alignment,
                            fit,
                            clip_behavior,
                        },
                        true,
                        index.and_then(|slot| children.get(slot)).copied(),
                    ),
                    _ => unreachable!("handled only stack kinds"),
                };
                let non_positioned_constraints = match stack.fit {
                    StackFit::Loose => constraints.loosen(),
                    StackFit::Expand => Constraints::tight(constraints.biggest()),
                    StackFit::Passthrough => constraints,
                };
                let mut max_non_pos_w: f32 = 0.0;
                let mut max_non_pos_h: f32 = 0.0;
                let mut has_non_positioned = false;
                // Positions are read again in the second pass instead of
                // being collected: stack layout allocates only its offsets.
                for child in children {
                    if self.stack_positioned(*child, constraints)?.is_none() {
                        has_non_positioned = true;
                        self.layout_render(*child, non_positioned_constraints)?;
                        let size = self
                            .render_live(child, "retained render must remain live")
                            .size;
                        max_non_pos_w = max_non_pos_w.max(size.width);
                        max_non_pos_h = max_non_pos_h.max(size.height);
                    }
                }
                // Flutter stack.dart: with no non-positioned child the stack
                // takes constraints.biggest, which positioned children see too.
                let stack_size = constraints.constrain(
                    if matches!(stack.fit, StackFit::Expand) || !has_non_positioned {
                        constraints.biggest()
                    } else {
                        Size::new(max_non_pos_w, max_non_pos_h)
                    },
                );
                let final_size = incular_layout::stack_size(
                    constraints,
                    &stack,
                    Size::new(max_non_pos_w, max_non_pos_h),
                    has_non_positioned,
                    children.len(),
                );

                let mut offsets = ChildOffsets::default();
                let mut has_visual_overflow = false;
                let mut baseline: Option<f32> = None;
                for child in children {
                    let position = self.stack_positioned(*child, constraints)?;
                    if let Some(pos) = position {
                        let incular_layout::Positioned {
                            left,
                            top,
                            right,
                            bottom,
                            width,
                            height,
                        } = pos;
                        // Share the positioning policy: a configured axis
                        // (explicit size or opposing edges) tightens the
                        // child, so the size it adopts equals the size the
                        // algorithm anchors with and right/bottom insets are
                        // exact. Unconfigured axes are unbounded, as
                        // Flutter's `RenderStack.layoutPositionedChild`
                        // starts from `BoxConstraints()`.
                        let child_w = incular_layout::positioned_axis_size(
                            stack_size.width,
                            left,
                            right,
                            width,
                        );
                        let child_h = incular_layout::positioned_axis_size(
                            stack_size.height,
                            top,
                            bottom,
                            height,
                        );
                        // An `OverlayPortal` popup is laid out against the
                        // overlay by the placement pass; here it stays within
                        // the stack.
                        let (free_w, free_h) = if self.is_transient_popup(*child) {
                            (stack_size.width, stack_size.height)
                        } else {
                            (f32::INFINITY, f32::INFINITY)
                        };
                        let (min_w, max_w) = child_w.map_or((0.0, free_w), |w| (w, w));
                        let (min_h, max_h) = child_h.map_or((0.0, free_h), |h| (h, h));
                        self.layout_render(*child, Constraints::new(min_w, max_w, min_h, max_h))?;
                    }
                    let child_node = self.render_live(child, "retained render must remain live");
                    let size = child_node.size;
                    let position = position.map_or(
                        incular_layout::StackPosition::NonPositioned,
                        incular_layout::StackPosition::Positioned,
                    );
                    let (_, offset) =
                        incular_layout::stack_child_geometry(final_size, &stack, size, position);
                    if (!indexed || active_child == Some(*child))
                        && let Some(local) = child_node.baseline
                    {
                        let candidate = local + offset.y;
                        baseline = Some(baseline.map_or(candidate, |old| old.min(candidate)));
                    }
                    if matches!(position, incular_layout::StackPosition::Positioned(_)) {
                        has_visual_overflow |= offset.x < 0.0
                            || offset.y < 0.0
                            || offset.x + size.width > final_size.width
                            || offset.y + size.height > final_size.height;
                    }
                    offsets.push(offset);
                }
                self.render_live_mut(id, "retained render must remain live")
                    .baseline = baseline;
                self.set_layout_overflow(id, has_visual_overflow);
                (final_size, offsets)
            }
            RenderKind::ClipRect { .. }
            | RenderKind::ClipRRect { .. }
            | RenderKind::ClipOval { .. }
            | RenderKind::ClipPath { .. } => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints)?;
                    let size = self
                        .render_live(child, "retained render must remain live")
                        .size;
                    (constraints.constrain(size), ChildOffsets::One(Offset::ZERO))
                } else {
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::LayoutBuilder => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints)?;
                    let child = self.render_live(child, "retained render must remain live");
                    let (size, baseline) = (child.size, child.baseline);
                    // LayoutBuilder is transparent to the child's actual
                    // baseline; without one its parent decides the fallback.
                    self.render_live_mut(id, "retained render must remain live")
                        .baseline = baseline;
                    (constraints.constrain(size), ChildOffsets::One(Offset::ZERO))
                } else {
                    // layout_builder.dart performLayout: no child sizes to biggest.
                    self.render_live_mut(id, "retained render must remain live")
                        .baseline = None;
                    (constraints.biggest(), ChildOffsets::default())
                }
            }
            RenderKind::Visibility {
                visible,
                maintain_size,
                ..
            } => {
                if let Some(&child) = children.first() {
                    // A proxy, like Flutter's `RenderOffstage` and `Visibility`.
                    self.layout_render(child, constraints)?;
                    let measured = self
                        .render_live(child, "retained render must remain live")
                        .size;
                    let size = constraints.constrain(if visible || maintain_size {
                        measured
                    } else {
                        Size::ZERO
                    });
                    (size, ChildOffsets::One(Offset::ZERO))
                } else {
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::AspectRatio { ratio } => {
                let mut size = if constraints.is_width_bounded() {
                    Size::new(constraints.max_width(), constraints.max_width() / ratio)
                } else if constraints.is_height_bounded() {
                    Size::new(constraints.max_height() * ratio, constraints.max_height())
                } else {
                    Size::ZERO
                };
                if size.height > constraints.max_height() {
                    size = Size::new(constraints.max_height() * ratio, constraints.max_height());
                }
                size = constraints.constrain(size);
                if let Some(&child) = children.first() {
                    self.layout_render(child, Constraints::tight(size))?;
                    (size, ChildOffsets::One(Offset::ZERO))
                } else {
                    (size, ChildOffsets::default())
                }
            }
            RenderKind::SizeObserver { ref observer } => {
                // A proxy box: the child's constraints and size, reported.
                let size = if let Some(&child) = children.first() {
                    self.layout_render(child, constraints)?;
                    self.render_live(child, "retained render must remain live")
                        .size
                } else {
                    constraints.constrain(Size::ZERO)
                };
                (observer.0)(size);
                (size, ChildOffsets::One(Offset::ZERO))
            }
            _ => unreachable!("container layout received a non-container render kind"),
        })
    }
}

impl WidgetTree {
    /// The render object a parent-data widget (`Positioned`, `LayoutId`)
    /// under `child` applies to.
    ///
    /// Flutter applies parent data to the nearest render object below the
    /// widget, so stateful wrappers such as `AnimatedPositioned` keep it.
    /// Here the parent looks through constraint-independent builders,
    /// materializing them first, and animation tickers: both lay their only
    /// child out unchanged.
    fn parent_data_render(
        &mut self,
        child: RenderObjectId,
        constraints: Constraints,
    ) -> Result<RenderObjectId, TreeError> {
        let mut current = child;
        loop {
            match *self
                .render_live(current, "retained render must remain live")
                .object
                .kind()
            {
                RenderKind::AnimationTicker { .. } => {}
                RenderKind::LayoutBuilder => {
                    let constraint_dependent =
                        self.element_for_render(current).is_none_or(|element| {
                            matches!(
                                self.element_live(element, "builder element must remain live")
                                    .widget
                                    .kind(),
                                WidgetKind::LayoutBuilder {
                                    constraint_dependent: true,
                                    ..
                                }
                            )
                        });
                    if constraint_dependent {
                        return Ok(current);
                    }
                    self.materialize_layout_builder(current, constraints)?;
                }
                _ => return Ok(current),
            }
            let [next] = self
                .render_live(current, "retained render must remain live")
                .children[..]
            else {
                return Ok(current);
            };
            current = next;
        }
    }

    fn set_layout_overflow(&self, id: RenderObjectId, enabled: bool) {
        self.render_live(id, "overflow render must remain live")
            .object
            .layers
            .set_layout_overflow(enabled);
    }

    /// The [`Positioned`](crate::Positioned) data a stack child carries.
    fn stack_positioned(
        &mut self,
        child: RenderObjectId,
        constraints: Constraints,
    ) -> Result<Option<incular_layout::Positioned>, TreeError> {
        let render = self.parent_data_render(child, constraints)?;
        Ok(
            match *self
                .render_live(render, "retained render must remain live")
                .object
                .kind()
            {
                RenderKind::Positioned {
                    left,
                    top,
                    right,
                    bottom,
                    width,
                    height,
                } => {
                    let position = incular_layout::Positioned {
                        left,
                        top,
                        right,
                        bottom,
                        width,
                        height,
                    };
                    position.is_positioned().then_some(position)
                }
                _ => None,
            },
        )
    }

    /// `RenderCustomSingleChildLayoutBox`, `RenderCustomMultiChildLayoutBox`
    /// and `RenderFlow`.
    fn layout_custom(
        &mut self,
        id: RenderObjectId,
        delegate: &CustomLayoutDelegate,
        children: &[RenderObjectId],
        constraints: Constraints,
    ) -> Result<(Size, ChildOffsets), TreeError> {
        match delegate {
            CustomLayoutDelegate::Single(delegate) => {
                let delegate = delegate.get();
                let size = constraints.constrain(delegate.get_size(constraints));
                let Some(&child) = children.first() else {
                    return Ok((size, ChildOffsets::default()));
                };
                self.layout_render(child, delegate.get_constraints_for_child(constraints))?;
                let child_size = self
                    .render_live(child, "retained render must remain live")
                    .size;
                let offset = delegate.get_position_for_child(size, child_size);
                Ok((size, ChildOffsets::One(offset)))
            }
            CustomLayoutDelegate::ChildSized(layout) => {
                let Some(&child) = children.first() else {
                    self.set_layout_overflow(id, false);
                    self.render_live_mut(id, "child-sized layout remains live")
                        .baseline = None;
                    let (size, _) = layout.layout(constraints, Size::ZERO);
                    return Ok((constraints.constrain(size), ChildOffsets::default()));
                };
                self.layout_render(child, constraints)?;
                let child_size = self
                    .render_live(child, "retained render must remain live")
                    .size;
                let (size, offset) = layout.layout(constraints, child_size);
                let size = constraints.constrain(size);
                if layout.reflows() && size != child_size {
                    self.layout_render(child, Constraints::tight(size))?;
                    self.set_layout_overflow(id, false);
                } else {
                    self.set_layout_overflow(
                        id,
                        layout.overflow_clip().is_some()
                            && (child_size.width > size.width || child_size.height > size.height),
                    );
                }
                let baseline = self
                    .render_live(child, "laid-out child remains live")
                    .baseline
                    .map(|local| local + offset.y);
                self.render_live_mut(id, "child-sized layout remains live")
                    .baseline = baseline;
                Ok((size, ChildOffsets::One(offset)))
            }
            CustomLayoutDelegate::Multi(delegate) => {
                let delegate = delegate.get();
                let size = constraints.constrain(delegate.get_size(constraints));
                let mut ids = HashMap::with_capacity(children.len());
                for (index, &child) in children.iter().enumerate() {
                    let render = self.parent_data_render(child, constraints)?;
                    let RenderKind::LayoutId { id } = self
                        .render_live(render, "retained render must remain live")
                        .object
                        .kind()
                    else {
                        return Err(TreeError::InvalidWidgetConfiguration {
                            widget: "CustomMultiChildLayout",
                            reason: format!("child at position {index} has no LayoutId"),
                        });
                    };
                    if ids.insert(id.clone(), index).is_some() {
                        return Err(TreeError::InvalidWidgetConfiguration {
                            widget: "CustomMultiChildLayout",
                            reason: format!("LayoutId {id:?} is assigned to multiple children"),
                        });
                    }
                }
                let mut error = None;
                let offsets = crate::custom_layout::perform_multi_child_layout(
                    delegate,
                    size,
                    &ids,
                    children.len(),
                    &mut |index, child_constraints| {
                        let child = children[index];
                        match self.layout_render(child, child_constraints) {
                            Ok(()) => {
                                self.render_live(child, "retained render must remain live")
                                    .size
                            }
                            Err(failure) => {
                                error.get_or_insert(failure);
                                Size::ZERO
                            }
                        }
                    },
                );
                if let Some(error) = error {
                    return Err(error);
                }
                let offsets = offsets.map_err(|missing| TreeError::InvalidWidgetConfiguration {
                    widget: "CustomMultiChildLayout",
                    reason: format!(
                        "delegate did not lay out {} child slot(s): {missing:?}",
                        missing.len()
                    ),
                })?;
                Ok((size, offsets.into_iter().collect()))
            }
            CustomLayoutDelegate::Flow(delegate) => {
                let delegate = delegate.get();
                let size = constraints.constrain(delegate.get_size(constraints));
                let mut sizes = Vec::with_capacity(children.len());
                for (index, &child) in children.iter().enumerate() {
                    self.layout_render(
                        child,
                        delegate.get_constraints_for_child(index, constraints),
                    )?;
                    sizes.push(
                        self.render_live(child, "retained render must remain live")
                            .size,
                    );
                }
                // `RenderFlow.paint` runs the delegate at paint time; its
                // transforms only need the sizes known here.
                let mut context = crate::custom_layout::FlowPaintingContext::new(size, &sizes);
                delegate.paint_children(&mut context);
                for (&child, painted) in children.iter().zip(context.into_painted()) {
                    self.apply_flow_slot(child, painted);
                }
                Ok((size, children.iter().map(|_| Offset::ZERO).collect()))
            }
        }
    }

    /// Writes a flow child's paint call into its slot and pushes the new
    /// transform and opacity to the compositor.
    fn apply_flow_slot(&mut self, child: RenderObjectId, painted: Option<(CoreTransform, f32)>) {
        let RenderKind::Transform {
            slot: Some(slot), ..
        } = self
            .render_live(child, "retained render must remain live")
            .object
            .kind()
        else {
            return;
        };
        let slot = slot.clone();
        let was_painted = slot.painted();
        slot.set(painted);
        let (content, opacity_child) = {
            let node = self.render_live(child, "retained render must remain live");
            (node.object.layers.content(), node.children.first().copied())
        };
        if let (Some(content), Some(transform)) = (content, self.content_transform(child)) {
            self.compositor.update_transform(content, transform);
        }
        if let Some(opacity_child) = opacity_child
            && let Some(layer) = self
                .render_live(opacity_child, "retained render must remain live")
                .object
                .layers
                .opacity()
        {
            self.compositor
                .update_opacity(layer, slot.opacity().opacity());
        }
        self.mark_render_semantics(child, SemanticsDirty::Bounds);
        if was_painted != slot.painted() {
            self.mark_render_paint(child);
        }
    }
}
