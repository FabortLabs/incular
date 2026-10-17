use super::super::*;

impl WidgetTree {
    pub(super) fn layout_effect_kind(
        &mut self,
        _id: RenderObjectId,
        kind: &RenderKind,
        children: &[RenderObjectId],
        constraints: Constraints,
    ) -> Result<(Size, ChildOffsets), TreeError> {
        Ok(match *kind {
            RenderKind::Translate { .. } => {
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
            RenderKind::Transform { .. }
            | RenderKind::Scale { .. }
            | RenderKind::Rotation { .. } => {
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
            RenderKind::FittedBox { .. } => {
                if let Some(&child) = children.first() {
                    self.layout_render(
                        child,
                        Constraints::new(0., f32::INFINITY, 0., f32::INFINITY),
                    )?;
                    let child_size = self
                        .render_live(child, "retained render must remain live")
                        .size;
                    (
                        constraints.constrain(child_size),
                        ChildOffsets::One(Offset::ZERO),
                    )
                } else {
                    (constraints.constrain(Size::ZERO), ChildOffsets::default())
                }
            }
            RenderKind::Opacity { .. } => {
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
            _ => unreachable!("effect layout received a non-effect render kind"),
        })
    }
}
