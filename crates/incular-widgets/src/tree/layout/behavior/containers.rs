use super::super::*;

impl WidgetTree {
    pub(super) fn layout_container_kind(
        &mut self,
        _id: RenderObjectId,
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
                    let child_size = self
                        .render_live(child, "retained render must remain live")
                        .size;
                    let (size, offset) = incular_layout::align_child(
                        constraints,
                        child_size,
                        incular_layout::Align {
                            alignment,
                            width_factor,
                            height_factor,
                        },
                    );
                    (size, ChildOffsets::One(offset))
                } else {
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::Flex { flex } => {
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
                let mut occupied_non_flex = 0.0;
                for (child, (f, fit)) in children.iter().zip(&flex_meta) {
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
                        occupied_non_flex += flex.direction.main_extent(size);
                        flex_children.push(flex_child);
                    } else {
                        flex_children.push(incular_layout::FlexChild::flexible(
                            Size::ZERO,
                            *f,
                            *fit,
                        ));
                    }
                }

                let total_flex: u32 = flex_meta.iter().map(|(f, _)| *f).sum();
                let spacing = flex.spacing.max(0.0) * children.len().saturating_sub(1) as f32;
                if main_max.is_finite() && total_flex > 0 {
                    let available_for_flex = (main_max - occupied_non_flex - spacing).max(0.0);
                    for (i, (child, (f, fit))) in children.iter().zip(&flex_meta).enumerate() {
                        if *f > 0 {
                            let allocation = available_for_flex * *f as f32 / total_flex as f32;
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
                            let mut flex_child =
                                incular_layout::FlexChild::flexible(size, *f, *fit);
                            if let Some(baseline) = render.baseline {
                                flex_child = flex_child.with_baseline(baseline);
                            }
                            flex_children[i] = flex_child;
                        }
                    }
                }

                let result = incular_layout::layout_flex(constraints, &flex_children, flex);
                (
                    result.size,
                    result.children.into_iter().map(|c| c.offset).collect(),
                )
            }
            RenderKind::Flexible { .. } | RenderKind::Positioned { .. } => {
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
                (
                    result.size,
                    result.children.into_iter().map(|c| c.offset).collect(),
                )
            }
            RenderKind::Table {
                columns,
                column_spacing,
                row_spacing,
            } => {
                for child in children {
                    self.layout_render(*child, constraints.loosen())?;
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
                let config = incular_layout::Table {
                    columns,
                    column_spacing,
                    row_spacing,
                };
                let result = incular_layout::layout_table(constraints, &cells, config);
                (
                    result.size,
                    result
                        .children
                        .into_iter()
                        .map(|cell| cell.offset)
                        .collect(),
                )
            }
            RenderKind::Stack { .. } if children.is_empty() => {
                // Flutter stack.dart: a childless stack takes the biggest size
                // its constraints allow, or the smallest when unbounded.
                let size = if constraints.is_width_bounded() && constraints.is_height_bounded() {
                    constraints.biggest()
                } else {
                    constraints.smallest()
                };
                (size, ChildOffsets::default())
            }
            RenderKind::Stack { stack } => {
                let non_positioned_constraints = match stack.fit {
                    StackFit::Loose => constraints.loosen(),
                    StackFit::Expand => Constraints::tight(constraints.biggest()),
                    StackFit::Passthrough => constraints,
                };
                let mut max_non_pos_w: f32 = 0.0;
                let mut max_non_pos_h: f32 = 0.0;
                let mut has_non_positioned = false;
                let mut positions = Vec::with_capacity(children.len());
                for child in children {
                    positions.push(self.stack_positioned(*child, constraints)?);
                }
                for (child, position) in children.iter().zip(&positions) {
                    if position.is_none() {
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

                let mut stack_children = Vec::with_capacity(children.len());
                for (child, position) in children.iter().zip(positions) {
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
                        // exact. Unconfigured axes stay loosely bounded by
                        // the stack.
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
                        let (min_w, max_w) = child_w.map_or((0.0, stack_size.width), |w| (w, w));
                        let (min_h, max_h) = child_h.map_or((0.0, stack_size.height), |h| (h, h));
                        let child_constraints = Constraints::new(min_w, max_w, min_h, max_h);
                        self.layout_render(*child, child_constraints)?;
                        let size = self
                            .render_live(child, "retained render must remain live")
                            .size;
                        stack_children.push(incular_layout::StackChild::positioned(size, pos));
                    } else {
                        let size = self
                            .render_live(child, "retained render must remain live")
                            .size;
                        stack_children.push(incular_layout::StackChild::new(size));
                    }
                }

                let result = incular_layout::layout_stack(constraints, &stack_children, stack);
                (
                    result.size,
                    result.children.into_iter().map(|c| c.offset).collect(),
                )
            }
            RenderKind::IndexedStack { alignment, fit, .. } => {
                // Fit applies the established Stack policy to every child
                // (all children are measured so inactive ones keep state
                // and identity), then the widest/tallest drives the box.
                let child_constraints = match fit {
                    StackFit::Loose => constraints.loosen(),
                    StackFit::Expand => Constraints::tight(constraints.biggest()),
                    StackFit::Passthrough => constraints,
                };
                let mut natural = Size::ZERO;
                for child in children {
                    self.layout_render(*child, child_constraints)?;
                    let child_size = self
                        .render_live(child, "retained render must remain live")
                        .size;
                    natural = Size::new(
                        natural.width.max(child_size.width),
                        natural.height.max(child_size.height),
                    );
                }
                let size = constraints.constrain(if matches!(fit, StackFit::Expand) {
                    constraints.biggest()
                } else {
                    natural
                });
                let offsets = children
                    .iter()
                    .map(|child| {
                        alignment.within(
                            size,
                            self.render_live(child, "retained render must remain live")
                                .size,
                        )
                    })
                    .collect();
                (size, offsets)
            }
            RenderKind::SafeArea {
                minimum,
                left,
                top,
                right,
                bottom,
                maintain_bottom_view_padding,
            } => {
                // One shared policy feeds every construction path; the
                // retained arm only supplies the ambient signals.
                let insets = crate::SafeAreaPolicy {
                    minimum,
                    left,
                    top,
                    right,
                    bottom,
                    maintain_bottom_view_padding,
                }
                .padding(self.environment.safe_insets, self.environment.view_padding);
                let child_constraints = constraints.deflate(insets.horizontal(), insets.vertical());
                if let Some(&child) = children.first() {
                    self.layout_render(child, child_constraints)?;
                    let child_size = self
                        .render_live(child, "retained render must remain live")
                        .size;
                    let size = constraints.constrain(Size::new(
                        child_size.width + insets.horizontal(),
                        child_size.height + insets.vertical(),
                    ));
                    (
                        size,
                        ChildOffsets::One(Offset::new(insets.left, insets.top)),
                    )
                } else {
                    let size =
                        constraints.constrain(Size::new(insets.horizontal(), insets.vertical()));
                    (size, ChildOffsets::default())
                }
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
                    let size = self
                        .render_live(child, "retained render must remain live")
                        .size;
                    (constraints.constrain(size), ChildOffsets::One(Offset::ZERO))
                } else {
                    // layout_builder.dart performLayout: no child sizes to biggest.
                    (constraints.biggest(), ChildOffsets::default())
                }
            }
            RenderKind::Visibility {
                visible,
                maintain_size,
                ..
            } => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints.loosen())?;
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
    /// The [`Positioned`](crate::Positioned) data a stack child carries.
    ///
    /// Flutter applies a `Positioned` parent-data widget to the nearest
    /// render object below it, so stateful wrappers such as
    /// `AnimatedPositioned` stay positioned. Here the stack looks through
    /// constraint-independent builders, materializing them first, and
    /// animation tickers: both lay their only child out unchanged.
    fn stack_positioned(
        &mut self,
        child: RenderObjectId,
        constraints: Constraints,
    ) -> Result<Option<incular_layout::Positioned>, TreeError> {
        let mut current = child;
        loop {
            match *self
                .render_live(current, "retained render must remain live")
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
                    return Ok(Some(incular_layout::Positioned {
                        left,
                        top,
                        right,
                        bottom,
                        width,
                        height,
                    }));
                }
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
                        return Ok(None);
                    }
                    self.materialize_layout_builder(current, constraints)?;
                }
                _ => return Ok(None),
            }
            let [next] = self
                .render_live(current, "retained render must remain live")
                .children[..]
            else {
                return Ok(None);
            };
            current = next;
        }
    }
}
