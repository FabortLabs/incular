use super::super::*;

impl WidgetTree {
    /// Measures both natural widths without replacing the editor's viewport
    /// or scroll position. Only the maximum includes the caret reservation.
    #[must_use]
    pub fn editable_intrinsic_widths(
        &mut self,
        element: ElementId,
    ) -> Option<incular_text::TextIntrinsicWidths> {
        self.refresh_application_fonts();
        let field = self.editable_measurement_field(element)?;
        let mut widths = with_text_field_display(
            &field.controller,
            &field.placeholder,
            field.obscure_text.then_some(field.obscuring_character),
            |display| self.text_engine.intrinsic_widths(display, &field.style, field.text_direction),
        );
        let padding = field.content_padding.horizontal();
        widths.min += padding;
        widths.max += padding + crate::tree::text::editable_caret_margin(&field);
        Some(widths)
    }

    /// Measures the editable's minimum width, without the caret gap.
    #[must_use]
    pub fn editable_min_intrinsic_width(&mut self, element: ElementId) -> Option<f32> {
        self.editable_intrinsic_widths(element).map(|widths| widths.min)
    }

    /// Measures the editable's maximum width, including the caret gap.
    #[must_use]
    pub fn editable_max_intrinsic_width(&mut self, element: ElementId) -> Option<f32> {
        self.editable_intrinsic_widths(element).map(|widths| widths.max)
    }

    /// The minimum and maximum intrinsic heights of an editable coincide.
    /// Measures that height for a given logical width, ignoring a tight
    /// viewport height. Pass infinity to measure without soft wrapping.
    #[must_use]
    pub fn editable_intrinsic_height(&mut self, element: ElementId, width: f32) -> Option<f32> {
        if width.is_nan() || width < 0. {
            return None;
        }
        self.refresh_application_fonts();
        let field = self.editable_measurement_field(element)?;
        let padding = field.content_padding;
        let measure_width = (width.is_finite() && field.max_lines != Some(1)).then(|| {
            (width - padding.horizontal() - crate::tree::text::editable_caret_margin(&field)).max(0.)
        });
        let options = self.editable_measurement_options(&field, measure_width);
        let preferred_line_height = self.text_engine.layout_with_options(
            " ", &field.style, options.soft_wrap(false).max_lines(Some(1)),
        ).metrics.line_height;
        let min_lines = field.min_lines.or(field.max_lines).unwrap_or(0);
        let min_height = preferred_line_height * min_lines as f32;
        let height = if field.max_lines.is_some_and(|max| max > 1 && max == min_lines) {
            min_height
        } else {
            let content_height = with_text_field_display(
                &field.controller,
                &field.placeholder,
                field.obscure_text.then_some(field.obscuring_character),
                |display| {
                    if field.max_lines.is_none() && width.is_infinite() {
                        let lines = 1 + display.chars().filter(|&ch| {
                            matches!(ch, '\n' | '\u{85}' | '\u{b}' | '\u{c}' | '\u{2028}' | '\u{2029}')
                        }).count();
                        preferred_line_height * lines as f32
                    } else {
                        self.text_engine.layout_with_options(display, &field.style, options)
                            .metrics.size.height
                    }
                },
            );
            match field.max_lines {
                Some(1) => content_height,
                Some(max) => content_height.min(preferred_line_height * max as f32).max(min_height),
                None => content_height.max(min_height),
            }
        };
        Some(height + padding.vertical())
    }

    fn editable_measurement_field(&self, element: ElementId) -> Option<Box<TextFieldRender>> {
        let render = self.render_id(element)?;
        let RenderKind::TextField(field) = self.render_live(render, "mounted editable").object.kind()
        else {
            return None;
        };
        Some(field.clone())
    }

    fn editable_measurement_options(
        &mut self, field: &TextFieldRender, width: Option<f32>,
    ) -> TextLayoutOptions {
        let strut = field.strut.as_deref().map(|strut| {
            self.text_engine.resolve_strut(strut, &field.style)
        });
        TextLayoutOptions::new(
            width,
            rendering::physical_text_align(field.text_align, field.text_direction),
        )
        .text_direction(field.text_direction)
        .soft_wrap(field.multiline)
        .max_lines(if field.multiline { None } else { field.max_lines })
        .strut(strut)
    }

    fn sync_inline_children(
        &mut self,
        id: RenderObjectId,
        children: &[RenderObjectId],
        visible: Option<Box<[RenderObjectId]>>,
    ) {
        let node = self.render_live(id, "inline paragraph remains live");
        if node.inline_visible_children() == visible.as_deref() {
            return;
        }
        let layers = visible
            .as_deref()
            .unwrap_or(children)
            .iter()
            .filter_map(|&child| {
                let child = self.render_live(child, "inline child remains live");
                (!child.parked).then_some(child.object.layers.root)
            })
            .collect();
        let surface = node.object.layers.clone();
        let kind = node.object.shared_kind();
        self.render_live_mut(id, "inline paragraph remains live")
            .set_inline_visible_children(visible);
        surface.set_render_children(&mut self.compositor, &kind, layers);
        self.mark_render_semantics(id, SemanticsDirty::Subtree);
    }

    pub(super) fn layout_text_kind(
        &mut self,
        id: RenderObjectId,
        kind: &RenderKind,
        children: &[RenderObjectId],
        constraints: Constraints,
    ) -> Result<(Size, ChildOffsets), TreeError> {
        Ok(match *kind {
            RenderKind::Text {
                ref text,
                ref style,
                align,
                text_direction,
                soft_wrap,
                max_lines,
                overflow,
                height_behavior,
                ref spans,
                ref inline,
                ref strut,
            } => {
                let width = constraints
                    .is_width_bounded()
                    .then_some(constraints.max_width());
                // RenderParagraph lays each `WidgetSpan` child out first, with the
                // paragraph's max width and unbounded height
                // (`BoxConstraints(maxWidth: constraints.maxWidth)`), and reserves
                // a box of its size and baseline.
                let mut placeholders = inline
                    .as_ref()
                    .map(|inline| inline.placeholders.to_vec())
                    .unwrap_or_default();
                if let Some(inline) = inline {
                    let child_constraints =
                        Constraints::new(0., constraints.max_width(), 0., f32::INFINITY);
                    for (&child, &slot) in children.iter().zip(inline.slots.iter()) {
                        self.layout_render(child, child_constraints)?;
                        let node = self.render_live(child, "retained render must remain live");
                        let (size, baseline) = (node.size, node.baseline);
                        if let Some(placeholder) = placeholders.get_mut(slot) {
                            placeholder.size = size;
                            placeholder.baseline = baseline;
                        }
                    }
                }
                let strut = strut
                    .as_ref()
                    .map(|strut| self.text_engine.resolve_strut(strut, style));
                let _external_call = self
                    .recursion_diagnostics
                    .external_text_call("TextEngine::layout_with_options", text);
                let options = |width| {
                    TextLayoutOptions::new(width, align)
                        .text_direction(text_direction)
                        .soft_wrap(soft_wrap)
                        .max_lines(max_lines)
                        .overflow(overflow)
                        .height_behavior(height_behavior)
                        .strut(strut)
                };
                // A rich paragraph shapes each span with its own style; a
                // plain one takes the retained single-style path.
                let mut layout = match spans {
                    Some(runs) => self.text_engine.layout_rich_with_options(
                        text,
                        runs,
                        &placeholders,
                        options(width),
                    ),
                    None => self
                        .text_engine
                        .layout_with_options(text, style, options(width)),
                };
                if let Some(shrunk) = shrink_wrapped_text_width(&layout, constraints) {
                    layout = match spans {
                        Some(runs) => self.text_engine.layout_rich_with_options(
                            text,
                            runs,
                            &placeholders,
                            options(Some(shrunk)),
                        ),
                        None => {
                            self.text_engine
                                .layout_with_options(text, style, options(Some(shrunk)))
                        }
                    };
                }
                let size = constraints.constrain(layout.metrics.size);
                let node = self.render_live_mut(id, "retained render must remain live");
                node.set_text_layout(layout.clone());
                node.baseline = Some(layout.metrics.baseline);
                // A placeholder with no laid-out box has no paint offset
                // in Flutter. Retain the widget and its geometry but exclude
                // it from paint, hit testing, and accessibility.
                let mut visible: Option<Vec<_>> = None;
                let offsets = match inline {
                    Some(inline) => inline
                        .slots
                        .iter()
                        .take(children.len())
                        .enumerate()
                        .map(|(child_index, &slot)| {
                            let index = placeholders.get(slot).map(|placeholder| placeholder.index);
                            let placement = layout.placeholders.iter()
                                .find(|placed| Some(placed.index) == index);
                            if let Some(placed) = placement {
                                if let Some(visible) = &mut visible {
                                    visible.push(children[child_index]);
                                }
                                return placed.rect.origin;
                            }
                            visible.get_or_insert_with(|| children[..child_index].to_vec());
                            Offset::ZERO
                        })
                        .collect(),
                    None => ChildOffsets::default(),
                };
                self.sync_inline_children(id, children, visible.map(Vec::into_boxed_slice));
                (size, offsets)
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
                let mut layout = self.text_engine.layout(text, style, width, align);
                if let Some(shrunk) = shrink_wrapped_text_width(&layout, constraints) {
                    layout = self.text_engine.layout(text, style, Some(shrunk), align);
                }
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
                let constraints = constraints.tighten(width, height);
                let size = image.as_ref().map_or_else(
                    || constraints.smallest(),
                    |image| {
                        let intrinsic = image.decoded();
                        constraints.constrain_preserving_aspect_ratio(Size::new(
                            intrinsic.width() as f32 / scale,
                            intrinsic.height() as f32 / scale,
                        ))
                    },
                );
                (size, ChildOffsets::default())
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
                    obscure_text,
                    obscuring_character,
                    paragraph_layout,
                    content_padding,
                    read_only,
                    ref misspelled_text_style,
                    ..
                } = **field;
                let paragraphs = paragraph_layout && multiline && max_lines.is_none();
                // Flutter `buildTextSpan`: an obscured field shows bullets, so
                // spell check styling never reaches it.
                let (spell_revision, spell_runs) = match misspelled_text_style {
                    Some(misspelled) if !paragraphs && !obscure_text => {
                        self.text_field_spell_runs(id, controller, style, misspelled, read_only)
                    }
                    _ => (0, None),
                };
                let intrinsic_width = if desired.width > 0.0 {
                    desired.width
                } else if constraints.is_width_bounded() {
                    constraints.max_width()
                } else {
                    260.0
                };
                // RenderEditable subtracts the cursor width
                // and one-pixel caret gap for alignment and wrapping in both
                // text directions and line modes.
                let width =
                    intrinsic_width.clamp(constraints.min_width(), constraints.max_width());
                let margin = crate::tree::text::editable_caret_margin(field);
                let width_for_text = (width - content_padding.horizontal() - margin).max(0.);
                // A multiline height cap restricts the viewport, not the
                // paragraph: all lines below it remain scrollable.
                let options = self.editable_measurement_options(field, Some(width_for_text));
                let mask = obscure_text.then_some(obscuring_character);
                let (metrics, layout, document) =
                    with_text_field_display(controller, placeholder, mask, |display| {
                        let _external_call = self
                            .recursion_diagnostics
                            .external_text_call("TextEngine::layout_with_options", display);
                        if paragraphs {
                            let document =
                                self.text_engine.layout_paragraphs(display, style, options);
                            (document.metrics, None, Some(document))
                        } else {
                            // Spell check runs style the field's own text,
                            // never a preedit or the placeholder.
                            let layout = match spell_runs
                                .as_ref()
                                .filter(|(text, _)| text.as_str() == display)
                            {
                                Some((_, runs)) => self
                                    .text_engine
                                    .layout_runs_with_options(display, runs, options),
                                None => self
                                    .text_engine
                                    .layout_with_options(display, style, options),
                            };
                            (layout.metrics, Some(layout), None)
                        }
                    });
                let decorations = text_field_decorations(
                    controller,
                    placeholder,
                    mask,
                    style,
                    spell_runs.as_ref(),
                );
                let line_height = metrics.line_height.max(1.0);
                // editable.dart _preferredHeight: minLines ?? maxLines. The
                // editor itself has no inset; a wrapper's content padding adds
                // to it.
                let minimum_lines = min_lines.or(max_lines).unwrap_or(1).max(1) as f32;
                let minimum_height = line_height * minimum_lines + content_padding.vertical();
                let intrinsic_height = if desired.height > 0.0 {
                    desired.height
                } else if expands && constraints.is_height_bounded() {
                    constraints.max_height()
                } else {
                    let content_height = metrics.size.height.max(line_height);
                    let preferred_height = if multiline {
                        max_lines.map_or(content_height, |lines| {
                            content_height.min(line_height * lines as f32)
                        })
                    } else {
                        content_height
                    };
                    preferred_height + content_padding.vertical()
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
                    state.spell_revision = spell_revision;
                    state.decorations = decorations;
                }
                self.render_live_mut(id, "retained render must remain live")
                    .baseline = Some(metrics.baseline);
                (size, ChildOffsets::default())
            }
            _ => unreachable!("text layout received a non-text render kind"),
        })
    }
}

/// The width a paragraph must be laid out again at when its box shrink-wraps
/// narrower than the width it was aligned in, as Flutter's `TextPainter`
/// does (`layout(minWidth, maxWidth)` relayouts at the clamped intrinsic
/// width). Without it a centered or end-aligned line keeps the offset it
/// got inside the wider span and paints past the shrunk box, where an
/// ellipsis or clip cuts it off. `None` when no line was shifted or the box
/// already spans the alignment width.
fn shrink_wrapped_text_width(
    layout: &incular_text::TextLayout,
    constraints: Constraints,
) -> Option<f32> {
    if !constraints.is_width_bounded() {
        return None;
    }
    let aligned_width = constraints.max_width();
    let shrunk = layout
        .metrics
        .size
        .width
        .clamp(constraints.min_width(), aligned_width);
    let shifted = layout.lines.iter().any(|line| line.offset > 0.0);
    (shifted && shrunk < aligned_width).then_some(shrunk)
}

impl WidgetTree {
    /// The spell check session of the text field rendered by `id`.
    pub(in crate::tree) fn text_field_spell_session(
        &self,
        id: RenderObjectId,
    ) -> Option<Rc<crate::spell_check::SpellCheckSession>> {
        let element = self.element_for_render(id)?;
        self.elements
            .get(element.0)?
            .auxiliary_state
            .as_ref()?
            .spell_check_session
            .clone()
    }

    /// Flutter's `buildTextSpan` with spell check results: the session's
    /// answer revision and, when results were received, the field's text
    /// with its misspelled words styled, as styled runs.
    fn text_field_spell_runs(
        &self,
        id: RenderObjectId,
        controller: &TextEditingController,
        style: &TextStyle,
        misspelled: &TextStyle,
        read_only: bool,
    ) -> (u64, Option<(String, Vec<incular_text::StyledTextRun>)>) {
        let Some(session) = self.text_field_spell_session(id) else {
            return (0, None);
        };
        let revision = session.revision();
        let Some(results) = session.received() else {
            return (revision, None);
        };
        let value = controller.value();
        if value.text.is_empty() {
            return (revision, None);
        }
        // `withComposing = !readOnly && _hasFocus`; the composing region
        // counts only when it lies inside the text.
        let focused = self.text_field_state_live(id).focused;
        let composing_valid = value
            .composing
            .is_some_and(|range| range.start <= range.end && range.end <= value.text.len());
        let out_of_range = !composing_valid || read_only || !focused;
        let span = crate::build_text_span_with_spell_check_suggestions(
            &value,
            out_of_range,
            Some(style),
            misspelled,
            &results,
            incular_config::TargetPlatform::current(),
        );
        let runs = span.flatten(style);
        (revision, Some((value.text, runs)))
    }
}

/// The runs of a text field's laid-out text that carry a decoration: the
/// spell check runs when they styled the layout, else the field's own
/// style over its whole text. A field showing its placeholder has none.
fn text_field_decorations(
    controller: &TextEditingController,
    placeholder: &str,
    mask: Option<char>,
    style: &TextStyle,
    spell_runs: Option<&(String, Vec<incular_text::StyledTextRun>)>,
) -> Arc<[incular_text::StyledTextRun]> {
    let decorated = |text_style: &TextStyle| {
        text_style
            .decoration
            .is_some_and(|decoration| !decoration.is_empty())
    };
    with_text_field_display(controller, placeholder, mask, |display| {
        if controller.with_value(|value| value.text.is_empty()) {
            return Arc::from([]);
        }
        if let Some((text, runs)) = spell_runs
            && text.as_str() == display
        {
            return runs
                .iter()
                .filter(|run| decorated(&run.style))
                .cloned()
                .collect();
        }
        if !decorated(style) {
            return Arc::from([]);
        }
        Arc::from([incular_text::StyledTextRun {
            text: display.to_owned(),
            style: style.clone(),
            range: 0..display.len(),
        }])
    })
}

impl WidgetTree {
    /// Whether this field currently has an active spell-check service.
    #[doc(hidden)]
    #[must_use]
    pub fn text_field_spell_check_enabled(&self, id: ElementId) -> bool {
        self.elements
            .get(id.0)
            .and_then(|element| element.auxiliary_state.as_ref())
            .is_some_and(|auxiliary| auxiliary.spell_check_session.is_some())
    }

    /// Flutter's `findSuggestionSpanAtCursorIndex` for this mounted editor.
    /// The caret offset is in UTF-8 bytes, like the editing controller.
    #[must_use]
    pub fn text_field_suggestion_span_at_cursor_index(
        &self,
        element: ElementId,
        cursor_index: usize,
    ) -> Option<incular_platform::SuggestionSpan> {
        self.elements
            .get(element.0)?
            .auxiliary_state
            .as_ref()?
            .spell_check_session
            .as_ref()?
            .received()?
            .suggestion_span_at(cursor_index)
            .cloned()
    }

    /// The latest spell check results of the text field, Flutter's
    /// `EditableTextState.spellCheckResults`: `None` before the first answer
    /// or without spell checking.
    #[doc(hidden)]
    #[must_use]
    pub fn text_field_spell_check_results(
        &self,
        element: ElementId,
    ) -> Option<incular_platform::SpellCheckResults> {
        self.elements
            .get(element.0)?
            .auxiliary_state
            .as_ref()?
            .spell_check_session
            .as_ref()?
            .results()
    }
}
