use super::super::*;
use crate::tree::widget::aligned_child_offset;

impl WidgetTree {
    pub(super) fn layout_core_kind(
        &mut self,
        id: RenderObjectId,
        kind: &RenderKind,
        children: &[RenderObjectId],
        constraints: Constraints,
    ) -> Result<(Size, ChildOffsets), TreeError> {
        Ok(match *kind {
            RenderKind::Box { desired, .. } | RenderKind::Shape { desired, .. } => {
                (constraints.constrain(desired), ChildOffsets::default())
            }
            // Flutter `RenderCustomPaint`: no preferred size fills the constraints.
            RenderKind::CustomPaint { desired, .. } => (
                desired.map_or_else(
                    || constraints.biggest(),
                    |desired| constraints.constrain(desired),
                ),
                ChildOffsets::default(),
            ),
            RenderKind::Decorated { desired, .. } => {
                if let Some(&child) = children.first() {
                    // Flutter's `RenderDecoratedBox` is a proxy: the child gets the
                    // box's constraints, so a tight parent sizes the child too.
                    // Only a sized surface lets a smaller child sit inside it.
                    let child_constraints = if desired.is_some() {
                        constraints.loosen()
                    } else {
                        constraints
                    };
                    self.layout_render(child, child_constraints)?;
                    let child_size = self
                        .render_live(child, "retained render must remain live")
                        .size;
                    let wanted = desired.unwrap_or(child_size);
                    let size = constraints.constrain(Size::new(
                        wanted.width.max(child_size.width),
                        wanted.height.max(child_size.height),
                    ));
                    (size, ChildOffsets::One(Offset::ZERO))
                } else {
                    (
                        constraints.constrain(desired.unwrap_or(Size::ZERO)),
                        ChildOffsets::default(),
                    )
                }
            }
            RenderKind::Banner {
                ref message,
                ref text_style,
                ..
            } => {
                let (size, offsets) = if let Some(&child) = children.first() {
                    self.layout_render(child, constraints)?;
                    let child_size = self
                        .render_live(child, "retained render must remain live")
                        .size;
                    (
                        constraints.constrain(child_size),
                        ChildOffsets::One(Offset::ZERO),
                    )
                } else {
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                };
                let _external_call = self
                    .recursion_diagnostics
                    .external_text_call("TextEngine::layout_with_options", message);
                let text_layout = self.text_engine.layout_with_options(
                    message,
                    text_style,
                    TextLayoutOptions::new(Some(80.0), TextAlign::Center),
                );
                let node = self.render_live_mut(id, "retained render must remain live");
                node.set_text_layout(text_layout.clone());
                node.baseline = Some(text_layout.metrics.baseline);
                (size, offsets)
            }
            RenderKind::Button { desired, .. } => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints.loosen())?;
                    let child_size = self
                        .render_live(child, "retained render must remain live")
                        .size;
                    let size = constraints.constrain(Size::new(
                        desired.width.max(child_size.width),
                        desired.height.max(child_size.height),
                    ));
                    (
                        size,
                        ChildOffsets::One(Offset::new(
                            (size.width - child_size.width) / 2.0,
                            (size.height - child_size.height) / 2.0,
                        )),
                    )
                } else {
                    (constraints.constrain(desired), ChildOffsets::default())
                }
            }
            RenderKind::Padding { padding } => {
                let child_constraints =
                    constraints.deflate(padding.horizontal(), padding.vertical());
                if let Some(&child) = children.first() {
                    self.layout_render(child, child_constraints)?;
                    let child = self.render_live(child, "padding child must remain live");
                    let (s, baseline) = (child.size, child.baseline);
                    self.render_live_mut(id, "padding render must remain live")
                        .baseline = baseline.map(|local| local + padding.top);
                    (
                        constraints.constrain(Size::new(
                            s.width + padding.horizontal(),
                            s.height + padding.vertical(),
                        )),
                        ChildOffsets::One(Offset::new(padding.left, padding.top)),
                    )
                } else {
                    self.render_live_mut(id, "padding render must remain live")
                        .baseline = None;
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::Constrained {
                constraints: additional,
            } => {
                let child_constraints = enforced_constraints(constraints, additional);
                if let Some(&child) = children.first() {
                    self.layout_render(child, child_constraints)?;
                    let child = self.render_live(child, "retained render must remain live");
                    let (size, baseline) = (child.size, child.baseline);
                    // RenderConstrainedBox proxies its child without a placement offset.
                    self.render_live_mut(id, "constrained render must remain live")
                        .baseline = baseline;
                    (constraints.constrain(size), ChildOffsets::One(Offset::ZERO))
                } else {
                    self.render_live_mut(id, "constrained render must remain live")
                        .baseline = None;
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::Limited {
                max_width,
                max_height,
            } => {
                let child_constraints = Constraints::new(
                    constraints.min_width(),
                    if constraints.max_width().is_infinite() {
                        max_width
                    } else {
                        constraints.max_width()
                    },
                    constraints.min_height(),
                    if constraints.max_height().is_infinite() {
                        max_height
                    } else {
                        constraints.max_height()
                    },
                );
                if let Some(&child) = children.first() {
                    self.layout_render(child, child_constraints)?;
                    let size = self
                        .render_live(child, "retained render must remain live")
                        .size;
                    (constraints.constrain(size), ChildOffsets::One(Offset::ZERO))
                } else {
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::Overflow {
                min_width,
                max_width,
                min_height,
                max_height,
                alignment,
                fit,
            } => {
                let child_constraints = Constraints::new(
                    min_width.unwrap_or(constraints.min_width()),
                    max_width
                        .unwrap_or(constraints.max_width())
                        .max(min_width.unwrap_or(constraints.min_width())),
                    min_height.unwrap_or(constraints.min_height()),
                    max_height
                        .unwrap_or(constraints.max_height())
                        .max(min_height.unwrap_or(constraints.min_height())),
                );
                let fit_size = |child_size| {
                    let child_size = constraints.constrain(child_size);
                    match fit {
                        OverflowBoxFit::Max => Size::new(
                            if constraints.is_width_bounded() {
                                constraints.max_width()
                            } else {
                                child_size.width
                            },
                            if constraints.is_height_bounded() {
                                constraints.max_height()
                            } else {
                                child_size.height
                            },
                        ),
                        OverflowBoxFit::DeferToChild => child_size,
                    }
                };
                if let Some(&child) = children.first() {
                    self.layout_render(child, child_constraints)?;
                    let child = self.render_live(child, "overflow child must remain live");
                    let child_size = child.size;
                    let size = fit_size(child_size);
                    let offset = aligned_child_offset(alignment, size, child_size);
                    let baseline = child.baseline.map(|local| offset.y + local);
                    self.render_live_mut(id, "overflow render must remain live")
                        .baseline = baseline;
                    (size, ChildOffsets::One(offset))
                } else {
                    self.render_live_mut(id, "overflow render must remain live")
                        .baseline = None;
                    (fit_size(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::SizedOverflow { size, alignment } => {
                let size = constraints.constrain(size);
                if let Some(&child) = children.first() {
                    // Flutter lays out the child with the unchanged incoming
                    // bounds rather than the wrapper's requested size.
                    self.layout_render(child, constraints)?;
                    let child = self.render_live(child, "sized overflow child must remain live");
                    let offset = aligned_child_offset(alignment, size, child.size);
                    let baseline = child.baseline.map(|local| offset.y + local);
                    self.render_live_mut(id, "sized overflow render must remain live")
                        .baseline = baseline;
                    (size, ChildOffsets::One(offset))
                } else {
                    self.render_live_mut(id, "sized overflow render must remain live")
                        .baseline = None;
                    (size, ChildOffsets::default())
                }
            }
            RenderKind::Unconstrained {
                constrained_axis,
                alignment,
                ref transform,
                clip_behavior,
            } => {
                let child_constraints = if let Some(transform) = transform {
                    (transform.0)(constraints).map_err(|error| {
                        TreeError::InvalidWidgetConfiguration {
                            widget: "ConstraintsTransformBox",
                            reason: format!(
                                "transformed constraints are not normalized: {error:?}"
                            ),
                        }
                    })?
                } else {
                    unconstrained_constraints(constraints, constrained_axis)
                };
                let measured = if let Some(&child) = children.first() {
                    self.layout_render(child, child_constraints)?;
                    let child = self.render_live(child, "retained render must remain live");
                    Some((child.size, child.baseline))
                } else {
                    None
                };
                let node = self.render_live_mut(id, "overflow proxy must remain live");
                if let Some((child_size, baseline)) = measured {
                    let size = constraints.constrain(child_size);
                    let offset = aligned_child_offset(alignment, size, child_size);
                    let overflows = offset.x < 0.
                        || offset.y < 0.
                        || offset.x + child_size.width > size.width
                        || offset.y + child_size.height > size.height;
                    node.baseline = baseline.map(|baseline| baseline + offset.y);
                    node.object.layers.set_layout_overflow(overflows);
                    if overflows
                        && clip_behavior == Clip::None
                        && transform.is_some()
                        && cfg!(debug_assertions)
                    {
                        self.diagnostics.unclipped_constraint_overflows += 1;
                    }
                    (size, ChildOffsets::One(offset))
                } else {
                    node.baseline = None;
                    node.object.layers.set_layout_overflow(false);
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::Fractional {
                width_factor,
                height_factor,
                alignment,
            } => {
                let child_constraints =
                    fractional_constraints(constraints, width_factor, height_factor);
                if let Some(&child) = children.first() {
                    self.layout_render(child, child_constraints)?;
                    let child = self.render_live(child, "fractional child must remain live");
                    let (child_size, child_baseline) = (child.size, child.baseline);
                    // `RenderFractionallySizedOverflowBox`: the box takes the
                    // constrained child size and `alignChild` places the child.
                    let size = constraints.constrain(child_size);
                    let offset = Offset::new(
                        (size.width - child_size.width) * (alignment.x + 1.) / 2.,
                        (size.height - child_size.height) * (alignment.y + 1.) / 2.,
                    );
                    self.render_live_mut(id, "fractional render must remain live")
                        .baseline = child_baseline.map(|local| local + offset.y);
                    (size, ChildOffsets::One(offset))
                } else {
                    self.render_live_mut(id, "fractional render must remain live")
                        .baseline = None;
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::Baseline { baseline } => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints.loosen())?;
                    let child = self.render_live(child, "retained render must remain live");
                    let result = incular_layout::layout_baseline(
                        constraints,
                        incular_layout::BaselineChild {
                            size: child.size,
                            baseline: child.baseline,
                        },
                        baseline,
                    );
                    let offset = result.children[0].offset;
                    // The bottom-edge fallback positions a child without
                    // inventing an actual baseline for its ancestors.
                    let actual_baseline = child.baseline.map(|local| local + offset.y);
                    self.render_live_mut(id, "baseline render must remain live")
                        .baseline = actual_baseline;
                    (result.size, ChildOffsets::One(offset))
                } else {
                    self.render_live_mut(id, "baseline render must remain live")
                        .baseline = None;
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            // A ticker is a frame source, not a render object in Flutter: its
            // child lays out exactly as if the ticker were absent.
            RenderKind::AnimationTicker { .. } => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints)?;
                    let size = constraints.constrain(
                        self.render_live(child, "retained render must remain live")
                            .size,
                    );
                    (size, ChildOffsets::One(Offset::ZERO))
                } else {
                    (constraints.smallest(), ChildOffsets::default())
                }
            }
            RenderKind::ProxyLayout { mode } => {
                let Some(&child) = children.first() else {
                    return Ok((constraints.constrain(Size::ZERO), ChildOffsets::default()));
                };
                match mode {
                    ChildLayoutMode::IntrinsicWidth {
                        step_width,
                        step_height,
                    } => {
                        // Derive each stepped extent from its own probe: the
                        // orthogonal axis keeps the incoming upper bound. Then
                        // issue the final constraints to the child itself.
                        let width = if constraints.min_width() == constraints.max_width() {
                            None
                        } else {
                            let probe =
                                Constraints::at_most(f32::INFINITY, constraints.max_height());
                            self.layout_render(child, probe)?;
                            let natural = self
                                .render_live(child, "intrinsic child must remain live")
                                .size
                                .width;
                            Some(incular_layout::round_intrinsic_step(natural, step_width))
                        };
                        let height = if step_height.is_some()
                            && constraints.min_height() != constraints.max_height()
                        {
                            let probe =
                                Constraints::at_most(constraints.max_width(), f32::INFINITY);
                            self.layout_render(child, probe)?;
                            let natural = self
                                .render_live(child, "intrinsic child must remain live")
                                .size
                                .height;
                            Some(incular_layout::round_intrinsic_step(natural, step_height))
                        } else {
                            None
                        };
                        self.layout_render(child, constraints.tighten(width, height))?;
                        let child = self.render_live(child, "intrinsic child must remain live");
                        let (size, baseline) = (child.size, child.baseline);
                        self.render_live_mut(id, "intrinsic render must remain live")
                            .baseline = baseline;
                        (size, ChildOffsets::One(Offset::ZERO))
                    }
                    ChildLayoutMode::IntrinsicHeight => {
                        // `RenderIntrinsicHeight`: the child keeps the incoming
                        // width constraints and is tightened to its intrinsic
                        // height. A tight incoming height is used unchanged.
                        let child_constraints =
                            if constraints.min_height() == constraints.max_height() {
                                constraints
                            } else {
                                self.layout_render(
                                    child,
                                    Constraints::new(
                                        constraints.min_width(),
                                        constraints.max_width(),
                                        0.,
                                        f32::INFINITY,
                                    ),
                                )?;
                                let natural = self
                                    .render_live(child, "retained render must remain live")
                                    .size
                                    .height;
                                constraints.tighten(None, Some(natural))
                            };
                        self.layout_render(child, child_constraints)?;
                        let child = self.render_live(child, "intrinsic child must remain live");
                        let (size, baseline) = (child.size, child.baseline);
                        self.render_live_mut(id, "intrinsic render must remain live")
                            .baseline = baseline;
                        (size, ChildOffsets::One(Offset::ZERO))
                    }
                    ChildLayoutMode::IgnoreBaseline => {
                        self.layout_render(child, constraints)?;
                        let size = self
                            .render_live(child, "baseline-ignored child must remain live")
                            .size;
                        self.render_live_mut(id, "IgnoreBaseline must remain live")
                            .baseline = None;
                        (size, ChildOffsets::One(Offset::ZERO))
                    }
                    ChildLayoutMode::QuarterTurns(turns) if turns.rem_euclid(2) == 0 => {
                        self.layout_render(child, constraints)?;
                        let size = self
                            .render_live(child, "retained render must remain live")
                            .size;
                        (size, ChildOffsets::One(Offset::ZERO))
                    }
                    ChildLayoutMode::QuarterTurns(_) => {
                        // `RenderRotatedBox`: odd turns lay the child out with
                        // flipped constraints and report swapped dimensions. The
                        // child is centered so its rotation pivot is the box center.
                        let flipped = Constraints::new(
                            constraints.min_height(),
                            constraints.max_height(),
                            constraints.min_width(),
                            constraints.max_width(),
                        );
                        self.layout_render(child, flipped)?;
                        let child_size = self
                            .render_live(child, "retained render must remain live")
                            .size;
                        let size =
                            constraints.constrain(Size::new(child_size.height, child_size.width));
                        (
                            size,
                            ChildOffsets::One(aligned_child_offset(
                                Alignment::CENTER,
                                size,
                                child_size,
                            )),
                        )
                    }
                }
            }
            RenderKind::RepaintBoundary
            | RenderKind::Gesture
            | RenderKind::SelectionArea
            | RenderKind::SelectionContainer
            | RenderKind::SelectionListener
            | RenderKind::IndexedSemantics
            | RenderKind::SemanticsDebugger { .. } => {
                if let Some(&child) = children.first() {
                    // Proxies pass their constraints through, as Flutter's
                    // `RenderProxyBox` does.
                    self.layout_render(child, constraints)?;
                    let size = constraints.constrain(
                        self.render_live(child, "retained render must remain live")
                            .size,
                    );
                    (size, ChildOffsets::One(Offset::ZERO))
                } else {
                    let expands = self
                        .element_for_render(id)
                        .and_then(|element| self.elements.get(element.0))
                        .is_some_and(|element| {
                            matches!(
                                element.widget.kind(),
                                WidgetKind::RawInput { child: None, .. }
                            )
                        });
                    if expands {
                        let width = if constraints.max_width().is_finite() {
                            constraints.max_width()
                        } else {
                            constraints.min_width()
                        };
                        let height = if constraints.max_height().is_finite() {
                            constraints.max_height()
                        } else {
                            constraints.min_height()
                        };
                        (
                            constraints.constrain(Size::new(width, height)),
                            ChildOffsets::default(),
                        )
                    } else {
                        (constraints.constrain(Size::ZERO), ChildOffsets::default())
                    }
                }
            }
            RenderKind::PersistentHeader { .. } => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints.loosen())?;
                    let size = constraints.constrain(
                        self.render_live(child, "retained render must remain live")
                            .size,
                    );
                    (size, ChildOffsets::One(Offset::ZERO))
                } else {
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            _ => unreachable!("core layout received a non-core render kind"),
        })
    }
}
