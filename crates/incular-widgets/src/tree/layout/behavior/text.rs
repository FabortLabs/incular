use super::super::*;

impl WidgetTree {
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
                // Each child sits in its placed box; a box the paragraph dropped
                // (clipped or ellipsized lines) leaves its child at the origin.
                let offsets = match inline {
                    Some(inline) => inline
                        .slots
                        .iter()
                        .take(children.len())
                        .map(|&slot| {
                            let index = placeholders.get(slot).map(|placeholder| placeholder.index);
                            layout
                                .placeholders
                                .iter()
                                .find(|placed| Some(placed.index) == index)
                                .map_or(Offset::ZERO, |placed| placed.rect.origin)
                        })
                        .collect(),
                    None => ChildOffsets::default(),
                };
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
                    text_align,
                    text_direction,
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
                let width_for_text = if !multiline && text_direction == TextDirection::Ltr {
                    let width =
                        intrinsic_width.clamp(constraints.min_width(), constraints.max_width());
                    // RenderEditable removes this tail from the text painter's
                    // alignment width; the caret stays inside even when the
                    // line is right-aligned and no scrolling is needed.
                    let margin = crate::tree::text::editable_caret_margin(field);
                    (width - content_padding.horizontal() - margin).max(0.)
                } else {
                    (intrinsic_width - content_padding.horizontal()).max(0.)
                };
                let options = TextLayoutOptions::new(
                    Some(width_for_text),
                    rendering::physical_text_align(text_align, text_direction),
                )
                .text_direction(text_direction)
                .soft_wrap(multiline)
                // A multiline height cap belongs to the viewport, not
                // the paragraph: lines below it must remain scrollable.
                .max_lines(if multiline { None } else { max_lines });
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
            incular_config::TargetPlatform::host(),
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
