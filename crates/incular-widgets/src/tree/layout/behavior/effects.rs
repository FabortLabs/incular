use super::super::*;

impl WidgetTree {
    pub(super) fn layout_effect_kind(
        &mut self,
        id: RenderObjectId,
        kind: &RenderKind,
        children: &[RenderObjectId],
        constraints: Constraints,
    ) -> Result<(Size, ChildOffsets), TreeError> {
        Ok(match *kind {
            RenderKind::Translate { .. } => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints)?;
                    let size = constraints.constrain(
                        self.render_live(child, "retained render must remain live")
                            .size,
                    );
                    (size, ChildOffsets::One(Offset::ZERO))
                } else {
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::Transform { .. }
            | RenderKind::Scale { .. }
            | RenderKind::Affine { .. }
            | RenderKind::Rotation { .. } => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints)?;
                    let size = constraints.constrain(
                        self.render_live(child, "retained render must remain live")
                            .size,
                    );
                    (size, ChildOffsets::One(Offset::ZERO))
                } else {
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::FittedBox { fit, .. } => {
                if let Some(&child) = children.first() {
                    self.layout_render(
                        child,
                        Constraints::new(0., f32::INFINITY, 0., f32::INFINITY),
                    )?;
                    let child = self.render_live(child, "retained render must remain live");
                    let (child_size, baseline) = (child.size, child.baseline);
                    // Flutter asks the unconstrained child for its real
                    // baseline; the paint fit does not change this metric.
                    self.render_live_mut(id, "fitted render must remain live")
                        .baseline = baseline;
                    // ScaleDown fits against maxima only before applying
                    // the actual minimums, so it does not grow the child
                    // to satisfy a minimum in the other axis.
                    let size = if fit == ImageFit::ScaleDown {
                        constraints.constrain(
                            constraints
                                .loosen()
                                .constrain_preserving_aspect_ratio(child_size),
                        )
                    } else {
                        constraints.constrain_preserving_aspect_ratio(child_size)
                    };
                    (size, ChildOffsets::One(Offset::ZERO))
                } else {
                    self.render_live_mut(id, "fitted render must remain live")
                        .baseline = None;
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::Opacity { .. } => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints)?;
                    let size = constraints.constrain(
                        self.render_live(child, "retained render must remain live")
                            .size,
                    );
                    (size, ChildOffsets::One(Offset::ZERO))
                } else {
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::Blur { .. }
            | RenderKind::DropShadow { .. }
            | RenderKind::ColorFiltered { .. }
            | RenderKind::Blend { .. }
            | RenderKind::ShaderMask { .. }
            | RenderKind::BackdropFilter { .. }
            | RenderKind::AnnotatedRegion { .. }
            | RenderKind::Leader { .. }
            | RenderKind::Follower { .. } => {
                if let Some(&child) = children.first() {
                    self.layout_render(child, constraints)?;
                    let size = constraints.constrain(
                        self.render_live(child, "retained render must remain live")
                            .size,
                    );
                    (size, ChildOffsets::One(Offset::ZERO))
                } else {
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            _ => unreachable!("effect layout received a non-effect render kind"),
        })
    }
}
