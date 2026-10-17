use super::super::*;

impl WidgetTree {
    pub(super) fn layout_text_kind(
        &mut self,
        id: RenderObjectId,
        kind: &RenderKind,
        _children: &[RenderObjectId],
        constraints: Constraints,
    ) -> Result<(Size, ChildOffsets), TreeError> {
        Ok(match *kind {
            RenderKind::Text {
                ref text,
                ref style,
                align,
                soft_wrap,
                max_lines,
                overflow,
            } => {
                let width = constraints
                    .is_width_bounded()
                    .then_some(constraints.max_width());
                let _external_call = self
                    .recursion_diagnostics
                    .external_text_call("TextEngine::layout_with_options", text);
                let layout = self.text_engine.layout_with_options(
                    text,
                    style,
                    TextLayoutOptions::new(width, align)
                        .soft_wrap(soft_wrap)
                        .max_lines(max_lines)
                        .overflow(overflow),
                );
                let size = constraints.constrain(layout.metrics.size);
                let node = self.render_live_mut(id, "retained render must remain live");
                node.set_text_layout(layout.clone());
                node.baseline = Some(layout.metrics.baseline);
                (size, ChildOffsets::default())
            }
            RenderKind::SelectableText {
                ref text,
                ref style,
                align,
            } => {
                let width = constraints
                    .is_width_bounded()
                    .then_some(constraints.max_width());
                let _external_call = self
                    .recursion_diagnostics
                    .external_text_call("TextEngine::layout", text);
                let layout = self.text_engine.layout(text, style, width, align);
                let size = constraints.constrain(layout.metrics.size);
                let node = self.render_live_mut(id, "retained render must remain live");
                node.set_text_layout(layout.clone());
                node.baseline = Some(layout.metrics.baseline);
                (size, ChildOffsets::default())
            }
            RenderKind::Image {
                ref image,
                width,
                height,
                scale,
                ..
            } => {
                // `scale` maps source pixels to intrinsic logical size:
                // logical = decoded / scale. Explicit width/height win when
                // present. The source handle is untouched, so scale changes
                // never re-decode.
                let intrinsic = image.decoded();
                let natural_w = intrinsic.width() as f32 / scale;
                let natural_h = intrinsic.height() as f32 / scale;
                let ratio = natural_w / natural_h;
                let natural = match (width, height) {
                    (Some(w), Some(h)) => Size::new(w, h),
                    (Some(w), None) => Size::new(w, w / ratio),
                    (None, Some(h)) => Size::new(h * ratio, h),
                    (None, None) => Size::new(natural_w, natural_h),
                };
                (constraints.constrain(natural), ChildOffsets::default())
            }
            RenderKind::TextField(ref field) => {
                let TextFieldRender {
                    ref controller,
                    desired,
                    ref style,
                    ref placeholder,
                    multiline,
                    min_lines,
                    max_lines,
                    expands,
                    text_align,
                    text_direction,
                    obscure_text,
                    paragraph_layout,
                    ..
                } = **field;
                let intrinsic_width = if desired.width > 0.0 {
                    desired.width
                } else if constraints.is_width_bounded() {
                    constraints.max_width()
                } else {
                    260.0
                };
                let width_for_text = (intrinsic_width - 16.).max(0.);
                let options = TextLayoutOptions::new(
                    Some(width_for_text),
                    rendering::physical_text_align(text_align, text_direction),
                )
                .text_direction(text_direction)
                .soft_wrap(multiline)
                .max_lines(max_lines);
                let (metrics, layout, document) =
                    with_text_field_display(controller, placeholder, obscure_text, |display| {
                        let _external_call = self
                            .recursion_diagnostics
                            .external_text_call("TextEngine::layout_with_options", display);
                        if paragraph_layout && multiline && max_lines.is_none() {
                            let document =
                                self.text_engine.layout_paragraphs(display, style, options);
                            (document.metrics, None, Some(document))
                        } else {
                            let layout = self
                                .text_engine
                                .layout_with_options(display, style, options);
                            (layout.metrics, Some(layout), None)
                        }
                    });
                let line_height = metrics.line_height.max(1.0);
                // editable.dart _preferredHeight: minLines ?? maxLines.
                let minimum_lines = min_lines.or(max_lines).unwrap_or(1).max(1) as f32;
                let minimum_height = (line_height * minimum_lines + 16.0).max(32.0);
                let intrinsic_height = if desired.height > 0.0 {
                    desired.height
                } else if expands && constraints.is_height_bounded() {
                    constraints.max_height()
                } else {
                    metrics.size.height.max(line_height) + 16.0
                };
                let size = constraints.constrain(Size::new(
                    intrinsic_width,
                    intrinsic_height.max(minimum_height),
                ));
                let (revision, visual_revision) = controller.revisions();
                {
                    let state = self.text_field_state_live_mut(id);
                    state.text.layout = layout;
                    state.document = document;
                    state.content_revision = revision;
                    state.visual_revision = visual_revision;
                }
                self.render_live_mut(id, "retained render must remain live")
                    .baseline = Some(metrics.baseline);
                (size, ChildOffsets::default())
            }
            _ => unreachable!("text layout received a non-text render kind"),
        })
    }
}
