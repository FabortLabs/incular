//! Text-field editing, selectable text, caret geometry, and static selection.

use super::*;
use crate::selection::{
    SelectedContentRange, SelectionGeometry, SelectionHandleType, SelectionPoint, SelectionStatus,
};
use incular_text::TextGeometry;

impl WidgetTree {
    #[must_use]
    pub fn text_field_at(&self, point: Offset) -> Option<ElementId> {
        let hit = self
            .hit_test(point)
            .and_then(|render| self.element_for_render(render))?;
        self.text_field_ancestor(hit)
    }
    /// [`Self::text_field_at`] for a hit test the caller already ran.
    #[must_use]
    pub fn text_field_for_hit(&self, hit: &PointerHitTest) -> Option<ElementId> {
        let hit = hit
            .target()
            .and_then(|render| self.element_for_render(render))?;
        self.text_field_ancestor(hit)
    }
    pub fn set_focused(
        &mut self,
        id: ElementId,
        focused: bool,
        now: Instant,
    ) -> Result<(), TreeError> {
        let interaction = self.elements.get(id.0).and_then(|element| {
            let WidgetKind::Button(spec) = element.widget.kind() else {
                return None;
            };
            spec.interaction.clone()
        });
        let render = self.render_id(id).ok_or(TreeError::MissingElement(id))?;
        let node = self.render_live_mut(render, "retained render must remain live");
        let mut repaint = false;
        if let Some(button) = node.button_state_mut() {
            let changed = button.focused != focused;
            button.focused = focused;
            button.visual = if button.pressed {
                ButtonState::Pressed
            } else if button.focused {
                ButtonState::Focused
            } else if button.hovered {
                ButtonState::Hovered
            } else {
                ButtonState::Normal
            };
            if changed {
                let snapshot = crate::internal::ActionInteractionState {
                    hovered: button.hovered,
                    pressed: button.pressed,
                    focused: button.focused,
                };
                repaint = true;
                if let Some(interaction) = interaction {
                    interaction.update(snapshot);
                }
            }
        }
        if node
            .text_field_state()
            .is_some_and(|state| state.focused != focused)
        {
            node.text_field_state_mut()
                .expect("text field state")
                .focused = focused;
            repaint = true;
            if focused && let RenderKind::TextField(field) = node.object.kind() {
                field.controller.reset_caret(now);
            }
        }
        if node
            .selectable_text_state()
            .is_some_and(|state| state.focused != focused)
        {
            node.selectable_text_state_mut()
                .expect("selectable text state")
                .focused = focused;
            repaint = true;
        }
        if repaint {
            self.mark_render_paint(render);
        }
        Ok(())
    }
    #[must_use]
    pub fn is_text_field(&self, id: ElementId) -> bool {
        self.elements
            .get(id.0)
            .is_some_and(|element| matches!(element.widget.kind(), WidgetKind::TextField(_)))
    }

    /// Returns whether the field is currently allowed to mutate its
    /// controller through keyboard/IME input.  Read-only fields intentionally
    /// remain focusable so selection and copy still work, while disabled
    /// fields are not editable or focusable.
    #[must_use]
    pub fn text_field_is_editable(&self, id: ElementId) -> bool {
        self.elements.get(id.0).is_some_and(|element| {
            matches!(element.widget.kind(), WidgetKind::TextField(spec) if spec.enabled && !spec.read_only)
        })
    }

    #[must_use]
    pub fn text_field_is_read_only(&self, id: ElementId) -> bool {
        self.elements.get(id.0).is_some_and(|element| {
            matches!(element.widget.kind(), WidgetKind::TextField(spec) if spec.enabled && spec.read_only)
        })
    }
    #[must_use]
    pub fn is_multiline_text_field(&self, id: ElementId) -> bool {
        self.elements.get(id.0).is_some_and(
            |element| matches!(element.widget.kind(), WidgetKind::TextField(spec) if spec.multiline),
        )
    }
    /// Flutter's `VerticalCaretMovementRun.moveNext`/`movePrevious` for a
    /// single line. Returns `false` when there is no adjacent line (single-line
    /// fields and the first/last line), where Flutter's vertical action moves
    /// the caret to the document edge instead.
    pub fn text_field_move_vertical(&mut self, id: ElementId, down: bool, extend: bool) -> bool {
        self.text_field_move_lines(id, down, 1, extend)
    }

    /// The number of lines `VerticalCaretMovementRun.moveByOffset` steps for a
    /// page: it keeps moving until the caret baseline has travelled the
    /// editable's height.
    #[must_use]
    pub fn text_field_page_lines(&self, id: ElementId) -> usize {
        let Some(node) = self
            .render_id(id)
            .and_then(|render| self.renders.get(render.0))
        else {
            return 1;
        };
        let Some(layout) = node.field_layout() else {
            return 1;
        };
        let line_height = layout.metrics().line_height;
        if line_height <= 0. {
            return 1;
        }
        ((node.size.height / line_height).ceil() as usize).max(1)
    }

    /// Moves the caret `lines` lines up or down, keeping the preferred caret
    /// x, clamped to the first/last line (`moveByOffset` stops where the
    /// layout ends). Returns whether the caret moved to another line.
    pub fn text_field_move_lines(
        &mut self,
        id: ElementId,
        down: bool,
        lines: usize,
        extend: bool,
    ) -> bool {
        let Some(render) = self.render_id(id) else {
            return false;
        };
        let Some(node) = self.renders.get(render.0) else {
            return false;
        };
        let (RenderKind::TextField(field), Some(layout)) =
            (node.object.kind(), node.field_layout())
        else {
            return false;
        };
        let (controller, multiline) = (&field.controller, field.multiline);
        if !multiline {
            return false;
        }
        let current = controller.selection().extent;
        let line = line_for_byte(&layout, current);
        let last = layout.line_count().saturating_sub(1);
        let target_line = if down {
            line.saturating_add(lines).min(last)
        } else {
            line.saturating_sub(lines)
        };
        if target_line == line || layout.get_line(target_line).is_none() {
            return false;
        }
        let x = controller.preferred_caret_x().unwrap_or_else(|| {
            line_caret_x_for_affinity(&layout, line, current, controller.caret_affinity())
        });
        let (target, affinity) = caret_for_line_position(&layout, target_line, x);
        controller.move_cursor_with_affinity(target, affinity, extend);
        controller.set_preferred_caret_x(x);
        self.mark_render_paint(render);
        true
    }

    /// Moves a text-field caret by visual screen order. Logical UTF-8 order is
    /// not sufficient for bidi paragraphs because the next screen stop can
    /// have a smaller byte offset.
    pub fn text_field_move_horizontal(&mut self, id: ElementId, right: bool, extend: bool) -> bool {
        let Some(render) = self.render_id(id) else {
            return false;
        };
        let Some(node) = self.renders.get(render.0) else {
            return false;
        };
        let RenderKind::TextField(field) = node.object.kind() else {
            return false;
        };
        let controller = &field.controller;
        let Some(layout) = node.field_layout() else {
            return false;
        };
        if right {
            controller.move_right_visual(&layout, extend);
        } else {
            controller.move_left_visual(&layout, extend);
        }
        self.mark_render_paint(render);
        true
    }
    /// Flutter's `EditableTextState._scrollToDocumentBoundary`: brings the
    /// start or end of the text into view without moving the selection.
    pub fn text_field_scroll_to_document_boundary(&mut self, id: ElementId, forward: bool) -> bool {
        let Some(render) = self.render_id(id) else {
            return false;
        };
        let Some(node) = self.renders.get(render.0) else {
            return false;
        };
        let RenderKind::TextField(field) = node.object.kind() else {
            return false;
        };
        let Some(layout) = node.field_layout() else {
            return false;
        };
        let value = field.controller.value();
        let offset = if forward { value.text.len() } else { 0 };
        let (caret, caret_y, caret_height) = text_field_caret_geometry(
            &layout,
            offset,
            incular_text::TextAffinity::Downstream,
            value.text.is_empty(),
            field,
        );
        let size = node.size;
        let multiline = field.multiline;
        let available = (size.width - 8. - (field.cursor_width.max(0.) + 1.)).max(0.);
        let viewport = (size.height - 16.).max(0.);
        let state = self.text_field_state_live_mut(render);
        let (before_x, before_y) = (state.scroll_x, state.scroll_y);
        if multiline {
            if caret_y < state.scroll_y {
                state.scroll_y = caret_y;
            } else if caret_y + caret_height - state.scroll_y > viewport {
                state.scroll_y = caret_y + caret_height - viewport;
            }
        } else if caret - state.scroll_x > available {
            state.scroll_x = caret - available;
        } else if caret < state.scroll_x {
            state.scroll_x = caret;
        }
        let changed = state.scroll_x != before_x || state.scroll_y != before_y;
        if changed {
            self.mark_render_paint(render);
        }
        changed
    }
    /// Flutter's `EditableTextState._scroll` for a page `ScrollIntent`: a
    /// single-line field jumps to its maximum scroll extent; a multiline one
    /// scrolls by 0.8 of its viewport (`ScrollAction`'s page increment)
    /// toward `forward`, clamped to its extent. The selection stays.
    pub fn text_field_scroll_page(&mut self, id: ElementId, forward: bool) -> bool {
        let Some(render) = self.render_id(id) else {
            return false;
        };
        let Some(node) = self.renders.get(render.0) else {
            return false;
        };
        let RenderKind::TextField(field) = node.object.kind() else {
            return false;
        };
        let Some(layout) = node.field_layout() else {
            return false;
        };
        let size = node.size;
        let (target_x, target_y) = if field.multiline {
            let viewport = (size.height - 16.).max(0.);
            let max = (layout.metrics().size.height - viewport).max(0.);
            if max == 0. {
                return false;
            }
            let increment = 0.8 * viewport;
            let current = self.text_field_state_live(render).scroll_y;
            let target = if forward {
                current + increment
            } else {
                current - increment
            };
            (None, Some(target.clamp(0., max)))
        } else {
            let available = (size.width - 8. - (field.cursor_width.max(0.) + 1.)).max(0.);
            let (left, right) =
                layout
                    .lines()
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(left, right), line| {
                        (
                            left.min(line.offset()),
                            right.max(line.offset() + line.width()),
                        )
                    });
            let max = if right - left > available {
                right - available
            } else {
                0.
            };
            (Some(max), None)
        };
        let state = self.text_field_state_live_mut(render);
        let (before_x, before_y) = (state.scroll_x, state.scroll_y);
        if let Some(x) = target_x {
            state.scroll_x = x;
        }
        if let Some(y) = target_y {
            state.scroll_y = y;
        }
        let changed = state.scroll_x != before_x || state.scroll_y != before_y;
        if changed {
            self.mark_render_paint(render);
        }
        changed
    }
    /// The text field's current scroll offsets in content pixels.
    #[must_use]
    pub fn text_field_scroll_offset(&self, id: ElementId) -> Option<Offset> {
        let render = self.render_id(id)?;
        self.renders.get(render.0)?.text_field_state()?;
        let state = self.text_field_state_live(render);
        Some(Offset::new(state.scroll_x, state.scroll_y))
    }
    /// Flutter's `_moveToTextBoundary` over `LineBoundary`
    /// (`editable_text.dart`): the line start (backward) or end excluding a
    /// hard break (forward) of the line holding `offset`, as a position with
    /// its affinity. An upstream caret belongs to the grapheme before it.
    ///
    /// With `continues_at_wrap` (Home/End on Windows and Android) a caret
    /// sitting at a soft wrap first re-associates with the neighbouring line,
    /// so repeated presses keep moving. `None` means the field has no layout
    /// yet; callers then use the document edge.
    #[must_use]
    pub fn text_field_line_boundary(
        &self,
        id: ElementId,
        offset: usize,
        affinity: incular_text::TextAffinity,
        forward: bool,
        continues_at_wrap: bool,
    ) -> Option<(usize, incular_text::TextAffinity)> {
        use incular_text::TextAffinity::{Downstream, Upstream};
        let node = self.renders.get(self.render_id(id)?.0)?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let layout = node.field_layout()?;
        if layout.line_count() == 0 {
            return None;
        }
        let text = field.controller.text();
        // `Paragraph.getLineBoundary`: the line whose `[start, end)` holds the
        // offset, the last line past the end.
        let line_containing = |position: usize| layout.line(layout.line_for_offset(position));
        let mut affinity = affinity;
        if continues_at_wrap {
            let line = line_containing(offset);
            let byte = |at: usize| text.as_bytes().get(at).copied();
            if forward {
                if affinity == Upstream
                    && offset == line.caret_end()
                    && offset != text.len()
                    && byte(offset) != Some(b'\n')
                {
                    affinity = Downstream;
                }
            } else if affinity == Downstream
                && offset == line.start()
                && offset != 0
                && byte(offset - 1) != Some(b'\n')
            {
                affinity = Upstream;
            }
        }
        let caret_offset = match affinity {
            Upstream => {
                if offset < 1 && !forward {
                    return Some((0, Downstream));
                }
                // The grapheme before the caret.
                text[..offset.min(text.len())]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(index, _)| index)
            }
            Downstream => offset,
        };
        let line = line_containing(caret_offset);
        Some(if forward {
            (line.caret_end(), Upstream)
        } else {
            (line.start(), Downstream)
        })
    }

    pub fn text_field_move_line_edge(&mut self, id: ElementId, end: bool, extend: bool) -> bool {
        let Some(render) = self.render_id(id) else {
            return false;
        };
        let Some(node) = self.renders.get(render.0) else {
            return false;
        };
        let RenderKind::TextField(field) = node.object.kind() else {
            return false;
        };
        if !field.multiline {
            return false;
        }
        let controller = field.controller.clone();
        let selection = controller.selection();
        let Some((target, affinity)) = self.text_field_line_boundary(
            id,
            selection.extent,
            controller.caret_affinity(),
            end,
            false,
        ) else {
            return false;
        };
        controller.move_cursor_with_affinity(target, affinity, extend);
        self.mark_render_paint(render);
        true
    }
    pub fn text_field_set_caret(
        &mut self,
        id: ElementId,
        point: Offset,
        extend: bool,
        now: Instant,
    ) -> bool {
        let Some(render) = self.render_id(id) else {
            return false;
        };
        let (layout, scroll_x, scroll_y, multiline, controller) =
            match self.renders.get(render.0).map(|node| {
                let state = self.text_field_state_live(render);
                (
                    node.object.kind(),
                    node.field_layout(),
                    state.scroll_x,
                    state.scroll_y,
                )
            }) {
                Some((RenderKind::TextField(field), layout, scroll_x, scroll_y)) => (
                    layout,
                    scroll_x,
                    scroll_y,
                    field.multiline,
                    field.controller.clone(),
                ),
                _ => return false,
            };
        let local = self
            .render_world_transform(render)
            .inverse_transform_point(point)
            .unwrap_or(point);
        let x = local.x - 8. + scroll_x;
        let y = local.y - if multiline { 8. } else { 0. } + scroll_y;
        let text_len = controller.text().len();
        let (index, affinity) =
            layout
                .as_ref()
                .map_or((0, incular_text::TextAffinity::Downstream), |layout| {
                    let line_index = (y / layout.metrics().line_height).floor().max(0.) as usize;
                    let line_index = line_index.min(layout.line_count().saturating_sub(1));
                    layout
                        .get_line(line_index)
                        .map_or((0, incular_text::TextAffinity::Downstream), |_| {
                            caret_for_line_position(layout, line_index, x)
                        })
                });
        let index = index.min(text_len);
        let previous = controller.selection();
        controller.set_selection_with_affinity(
            if extend {
                TextSelection {
                    base: previous.base,
                    extent: index,
                }
            } else {
                TextSelection::collapsed(index)
            },
            affinity,
        );
        controller.reset_caret(now);
        self.mark_render_paint(render);
        true
    }
    /// Returns the selectable label under `point`, if it belongs to a selection area.
    #[must_use]
    pub fn selectable_text_at(&self, point: Offset) -> Option<ElementId> {
        let hit = self
            .hit_test(point)
            .and_then(|render| self.element_for_render(render))?;
        self.selectable_text_ancestor(hit)
    }
    /// [`Self::selectable_text_at`] for a hit test the caller already ran.
    #[must_use]
    pub fn selectable_text_for_hit(&self, hit: &PointerHitTest) -> Option<ElementId> {
        let hit = hit
            .target()
            .and_then(|render| self.element_for_render(render))?;
        self.selectable_text_ancestor(hit)
    }
    #[must_use]
    pub fn is_selectable_text(&self, id: ElementId) -> bool {
        self.elements.get(id.0).is_some_and(|element| {
            matches!(element.widget.kind(), WidgetKind::SelectableText { .. })
        })
    }
    /// Starts or extends read-only selection from a pointer position. The
    /// caret index is recovered from the cached Parley-backed layout; changing
    /// selection therefore never reshapes or rasterizes the text.
    pub fn selectable_text_set_selection(
        &mut self,
        id: ElementId,
        point: Offset,
        extend: bool,
    ) -> bool {
        let Some(area) = self.selection_area_ancestor(id) else {
            return false;
        };
        let Some(render) = self.render_id(id) else {
            return false;
        };
        let Some(layout) = self
            .renders
            .get(render.0)
            .and_then(RenderNode::text_layout_cloned)
        else {
            return false;
        };
        let local = self
            .render_world_transform(render)
            .inverse_transform_point(point)
            .unwrap_or(point);
        // Shaped stops resolve both edges: the walk below used raw
        // glyph order and an unshifted width cutoff, which mis-mapped
        // aligned, RTL, and mixed-direction clicks.
        let line_index = (local.y / layout.metrics().line_height).floor().max(0.) as usize;
        let line_index = line_index.min(layout.line_count().saturating_sub(1));
        let (byte, affinity) = layout
            .get_line(line_index)
            .map_or((0, incular_text::TextAffinity::Downstream), |_| {
                caret_for_line_position(&layout, line_index, local.x)
            });
        let point = StaticSelectionPoint {
            element: id,
            byte,
            affinity,
        };
        if extend
            && self
                .static_selections
                .get(&area)
                .is_some_and(|selection| selection.area != area)
        {
            return false;
        }
        let anchor = if extend {
            self.static_selections
                .get(&area)
                .copied()
                .map(|selection| selection.anchor)
                .unwrap_or(point)
        } else {
            point
        };
        self.static_selections.insert(
            area,
            StaticSelection {
                area,
                anchor,
                extent: point,
            },
        );
        self.sync_static_selection(area);
        true
    }
    /// Moves the active static-text selection by one grapheme cluster. Only
    /// selection state changes; there is no editable caret or IME route.
    pub fn selectable_text_move(&mut self, id: ElementId, right: bool, extend: bool) -> bool {
        let Some(area) = self.selection_area_ancestor(id) else {
            return false;
        };
        let Some(text) = self.selectable_text_value(id) else {
            return false;
        };
        let current = self
            .static_selections
            .get(&area)
            .copied()
            .filter(|selection| selection.extent.element == id)
            .map(|selection| selection.extent.byte)
            .unwrap_or(0);
        let byte = if right {
            next_grapheme_boundary(&text, current)
        } else {
            previous_grapheme_boundary(&text, current)
        };
        let point = StaticSelectionPoint {
            element: id,
            byte,
            affinity: incular_text::TextAffinity::Downstream,
        };
        let anchor = if extend {
            self.static_selections
                .get(&area)
                .copied()
                .map(|selection| selection.anchor)
                .unwrap_or(point)
        } else {
            point
        };
        self.static_selections.insert(
            area,
            StaticSelection {
                area,
                anchor,
                extent: point,
            },
        );
        self.sync_static_selection(area);
        true
    }
    pub fn selectable_text_move_to_edge(&mut self, id: ElementId, end: bool, extend: bool) -> bool {
        let Some(area) = self.selection_area_ancestor(id) else {
            return false;
        };
        let Some(text) = self.selectable_text_value(id) else {
            return false;
        };
        let point = StaticSelectionPoint {
            element: id,
            byte: if end { text.len() } else { 0 },
            affinity: incular_text::TextAffinity::Downstream,
        };
        let anchor = if extend {
            self.static_selections
                .get(&area)
                .copied()
                .map(|selection| selection.anchor)
                .unwrap_or(point)
        } else {
            point
        };
        self.static_selections.insert(
            area,
            StaticSelection {
                area,
                anchor,
                extent: point,
            },
        );
        self.sync_static_selection(area);
        true
    }
    pub fn selectable_text_select_all(&mut self, id: ElementId) -> bool {
        let Some(area) = self.selection_area_ancestor(id) else {
            return false;
        };
        let entries = self.selectable_texts_in_area(area);
        let Some(first) = entries.first().copied() else {
            return false;
        };
        let Some(last) = entries.last().copied() else {
            return false;
        };
        let last_len = self
            .selectable_text_value(last)
            .map_or(0, |text| text.len());
        self.static_selections.insert(
            area,
            StaticSelection {
                area,
                anchor: StaticSelectionPoint {
                    element: first,
                    byte: 0,
                    affinity: incular_text::TextAffinity::Downstream,
                },
                extent: StaticSelectionPoint {
                    element: last,
                    byte: last_len,
                    affinity: incular_text::TextAffinity::Downstream,
                },
            },
        );
        self.sync_static_selection(area);
        true
    }
    #[must_use]
    pub fn selectable_text_selected_text(&self, id: ElementId) -> Option<String> {
        let area = self.selection_area_ancestor(id)?;
        if let Some(controller) = self.selection_area_controller(area) {
            return Some(controller.selected_text());
        }
        let selection = self.static_selections.get(&area).copied()?;
        let entries = self.selectable_texts_in_area(area);
        Some(static_selection_text(self, &entries, selection))
    }
    pub(super) fn selectable_text_ancestor(&self, mut id: ElementId) -> Option<ElementId> {
        loop {
            if self.elements.get(id.0).is_some_and(|element| {
                matches!(element.widget.kind(), WidgetKind::SelectableText { .. })
            }) {
                return Some(id);
            }
            id = self.parent(id)?;
        }
    }
    pub(super) fn selection_area_ancestor(&self, mut id: ElementId) -> Option<ElementId> {
        let selectable = id;
        if !self.elements.get(id.0).is_some_and(|element| {
            matches!(element.widget.kind(), WidgetKind::SelectableText { .. })
        }) {
            return None;
        }
        loop {
            if let Some(policy) = self.selection_boundary_policy(id) {
                return policy.accepts_children().then_some(id);
            }
            let Some(parent) = self.parent(id) else {
                // A standalone SelectableText is its own one-label region.
                return Some(selectable);
            };
            id = parent;
        }
    }
    fn selection_boundary_policy(&self, id: ElementId) -> Option<SelectableChildPolicy> {
        match self.elements.get(id.0)?.widget.kind() {
            WidgetKind::SelectionArea { .. } => Some(SelectableChildPolicy::All),
            WidgetKind::SelectionContainer { delegate, .. }
            | WidgetKind::SelectionListener { delegate, .. } => Some(delegate.policy()),
            _ => None,
        }
    }
    pub(super) fn selection_area_controller(
        &self,
        id: ElementId,
    ) -> Option<SelectionAreaController> {
        match self.elements.get(id.0)?.widget.kind() {
            WidgetKind::SelectionArea { controller, .. } => Some(controller.clone()),
            WidgetKind::SelectionContainer { delegate, .. }
            | WidgetKind::SelectionListener { delegate, .. } => Some(delegate.controller()),
            _ => None,
        }
    }
    pub(super) fn selectable_text_value(&self, id: ElementId) -> Option<String> {
        let WidgetKind::SelectableText { text, .. } = self.elements.get(id.0)?.widget.kind() else {
            return None;
        };
        Some(text.clone())
    }
    pub(super) fn selectable_texts_in_area(&self, area: ElementId) -> Vec<ElementId> {
        let Some(policy) = self.selection_boundary_policy(area) else {
            // A standalone SelectableText owns its implicit one-item
            // selection region. It has no explicit boundary policy, but it
            // must still participate in select-all and copy operations.
            return self
                .elements
                .get(area.0)
                .is_some_and(|element| {
                    matches!(element.widget.kind(), WidgetKind::SelectableText { .. })
                })
                .then_some(vec![area])
                .unwrap_or_default();
        };
        if !policy.accepts_children() {
            return Vec::new();
        }
        let mut entries = Vec::new();
        if let Some(element) = self.elements.get(area.0) {
            for child in &element.children {
                self.collect_selectable_texts(*child, &mut entries);
            }
        }
        entries
    }
    pub(super) fn collect_selectable_texts(&self, id: ElementId, entries: &mut Vec<ElementId>) {
        let mut work = vec![id];
        while let Some(current) = work.pop() {
            let Some(element) = self.elements.get(current.0) else {
                continue;
            };
            if matches!(element.widget.kind(), WidgetKind::SelectableText { .. }) {
                entries.push(current);
                continue;
            }
            // A nested selection boundary owns its registrar subtree. Its
            // leaves must not be harvested by an ancestor container, even
            // when the nested boundary is disabled.
            if self.selection_boundary_policy(current).is_some() {
                continue;
            }
            work.extend(element.children.iter().rev().copied());
        }
    }
    pub(super) fn sync_static_selection(&mut self, area: ElementId) {
        let entries = self.selectable_texts_in_area(area);
        if let Some(controller) = self.selection_area_controller(area) {
            controller.set_registered_child_count(entries.len());
            if let Some(selection) = self.static_selections.get(&area).copied() {
                let selected = static_selection_text(self, &entries, selection);
                let geometry = self.selection_geometry(area, &entries, selection);
                let range = selection_content_range(self, &entries, selection);
                controller.set_selection_snapshot(selected, range, geometry);
            } else {
                controller.clear_selection();
            }
        }
        for element in entries {
            if let Some(render) = self.render_id(element) {
                self.mark_render_paint(render);
            }
        }
    }
    pub(super) fn static_selection_range(&self, element: ElementId) -> Option<TextRange> {
        let area = self.selection_area_ancestor(element)?;
        let selection = self.static_selections.get(&area).copied()?;
        let entries = self.selectable_texts_in_area(selection.area);
        static_selection_range(self, &entries, selection, element)
    }

    fn selection_geometry(
        &self,
        area: ElementId,
        entries: &[ElementId],
        selection: StaticSelection,
    ) -> SelectionGeometry {
        let has_content = !entries.is_empty();
        let status = if selection.anchor == selection.extent {
            SelectionStatus::Collapsed
        } else {
            SelectionStatus::Uncollapsed
        };
        let area_transform = self
            .render_id(area)
            .and_then(|render| self.render_world_transform(render).inverse())
            .unwrap_or(CoreTransform::IDENTITY);
        let mut rects = Vec::new();
        for element in entries {
            let Some(range) = static_selection_range(self, entries, selection, *element) else {
                continue;
            };
            let Some(render) = self.render_id(*element) else {
                continue;
            };
            let Some(layout) = self
                .renders
                .get(render.0)
                .and_then(RenderNode::text_layout_cloned)
            else {
                continue;
            };
            let transform = area_transform.then(self.render_world_transform(render));
            rects.extend(
                selection_rects(&layout, range, 0., 0., 0.)
                    .into_iter()
                    .map(|rect| transform.transform_rect_bbox(rect)),
            );
        }
        let start_selection_point = self.selection_point(
            area,
            entries,
            selection,
            if selection.anchor <= selection.extent {
                selection.anchor
            } else {
                selection.extent
            },
            SelectionHandleType::Left,
        );
        let end_selection_point = self.selection_point(
            area,
            entries,
            selection,
            if selection.anchor <= selection.extent {
                selection.extent
            } else {
                selection.anchor
            },
            SelectionHandleType::Right,
        );
        let (start_selection_point, end_selection_point) = if status == SelectionStatus::Collapsed {
            (
                self.selection_point(
                    area,
                    entries,
                    selection,
                    selection.anchor,
                    SelectionHandleType::Collapsed,
                ),
                None,
            )
        } else {
            (start_selection_point, end_selection_point)
        };
        SelectionGeometry {
            start_selection_point,
            end_selection_point,
            selection_rects: rects,
            status,
            has_content,
        }
    }

    fn selection_point(
        &self,
        area: ElementId,
        entries: &[ElementId],
        _selection: StaticSelection,
        point: StaticSelectionPoint,
        handle_type: SelectionHandleType,
    ) -> Option<SelectionPoint> {
        if !entries.contains(&point.element) {
            return None;
        }
        let render = self.render_id(point.element)?;
        let layout = self.renders.get(render.0)?.text_layout()?.as_ref();
        let (x, y, line_height) = caret_geometry(layout, point.byte, point.affinity);
        let area_transform = self
            .render_id(area)
            .and_then(|render| self.render_world_transform(render).inverse())
            .unwrap_or(CoreTransform::IDENTITY);
        let position = area_transform
            .then(self.render_world_transform(render))
            .transform_point(Offset::new(x, y));
        Some(SelectionPoint {
            local_position: position,
            line_height,
            handle_type,
        })
    }

    pub(super) fn refresh_selection_states(&mut self) {
        let areas = self
            .tracked
            .selection_boundaries
            .iter()
            .copied()
            .filter(|&id| {
                self.selection_boundary_policy(id).is_some()
                    && self.selection_area_controller(id).is_some()
            })
            .collect::<Vec<_>>();
        for area in areas {
            self.sync_static_selection(area);
        }
    }
    #[must_use]
    pub fn text_controller(&self, id: ElementId) -> Option<TextEditingController> {
        let render = self.render_id(id)?;
        match &self.renders.get(render.0)?.object.kind() {
            RenderKind::TextField(field) => Some(field.controller.clone()),
            _ => None,
        }
    }

    /// Returns the nearest [`UndoHistory`](crate::UndoHistory) capacity that
    /// wraps this text field. The value is kept on widget metadata so the
    /// renderer-neutral Widgets crate does not depend on Runtime.
    #[must_use]
    pub fn text_field_history_max_entries(&self, mut id: ElementId) -> Option<usize> {
        loop {
            let element = self.elements.get(id.0)?;
            if let Some(max_entries) = element
                .widget
                .semantic_properties()
                .undo_history_max_entries
            {
                return Some(max_entries);
            }
            id = element.parent?;
        }
    }

    /// Captures the current text-input client state after layout. The runtime
    /// turns this into a platform command; native window handles never enter
    /// the retained tree.
    #[must_use]
    pub fn text_field_input_snapshot(&self, id: ElementId) -> Option<TextFieldInputSnapshot> {
        let element = self.elements.get(id.0)?;
        let WidgetKind::TextField(spec) = element.widget.kind() else {
            return None;
        };
        let value = spec.controller.value();
        let bounds = self.element_bounds(id).unwrap_or_default();
        let caret_rect = self
            .render_id(id)
            .and_then(|render| self.renders.get(render.0).map(|node| (render, node)))
            .and_then(|(render, node)| {
                let layout = node.field_layout()?;
                let RenderKind::TextField(field) = node.object.kind() else {
                    return None;
                };
                let state = self.text_field_state_live(render);
                let (x, y, height) = text_field_caret_geometry(
                    &layout,
                    value.selection.extent,
                    spec.controller.caret_affinity(),
                    value.text.is_empty(),
                    field,
                );
                let top = if spec.multiline {
                    8.0
                } else {
                    ((node.size.height - height) / 2.0).max(0.0)
                };
                Some(self.render_world_transform(render).transform_rect_bbox(
                    Rect::from_origin_size(
                        Offset::new(x - state.scroll_x + 8.0, y - state.scroll_y + top),
                        Size::new(1.0, height.max(1.0)),
                    ),
                ))
            })
            .unwrap_or(bounds);
        Some(TextFieldInputSnapshot {
            client_id: (u64::from(id.0.generation()) << 32) | u64::from(id.0.index()),
            text: value.text,
            selection: value.selection,
            composing: value.composing,
            multiline: spec.multiline,
            enabled: spec.enabled,
            read_only: spec.read_only,
            obscure_text: spec.obscure_text,
            input_type: element
                .widget
                .semantic_properties()
                .text_input_type
                .unwrap_or_default(),
            input_action: element
                .widget
                .semantic_properties()
                .text_input_action
                .unwrap_or_default(),
            traits: spec.traits.clone(),
            bounds,
            caret_rect,
        })
    }
    pub fn submit_text_field(&self, id: ElementId) -> bool {
        let Some(element) = self.elements.get(id.0) else {
            return false;
        };
        let WidgetKind::TextField(spec) = element.widget.kind() else {
            return false;
        };
        if let Some(callback) = &spec.on_submit {
            callback(spec.controller.text());
        }
        true
    }

    pub(super) fn text_field_ancestor(&self, mut id: ElementId) -> Option<ElementId> {
        loop {
            if self.is_text_field(id) {
                return Some(id);
            }
            id = self.parent(id)?;
        }
    }
}

/// Runs `f` with the text a field displays. The common case (no preedit, not
/// obscured) borrows the controller's text or the placeholder without copying.
pub(super) fn with_text_field_display<R>(
    controller: &TextEditingController,
    placeholder: &str,
    obscure_text: bool,
    f: impl FnOnce(&str) -> R,
) -> R {
    // Keep one ASCII glyph per source byte.  Editing selections are
    // byte-indexed throughout the retained core, so this preserves
    // caret/selection geometry while still ensuring that the source
    // value never reaches the paint list.  For ordinary passwords
    // (the overwhelmingly common case) this is one mask glyph per
    // character; non-ASCII input remains safely obscured as well.
    let preedit = controller.preedit();
    controller.with_value(|value| {
        if let Some(preedit) = preedit {
            let range = value.selection.range();
            let mut text = value.text.clone();
            text.replace_range(range.as_range(), &preedit);
            if obscure_text {
                text = "*".repeat(text.len());
            }
            f(&text)
        } else if value.text.is_empty() {
            f(placeholder)
        } else if obscure_text {
            f(&"*".repeat(value.text.len()))
        } else {
            f(&value.text)
        }
    })
}

pub(super) fn valid_boundary(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    text.char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .take_while(|index| *index <= offset)
        .last()
        .unwrap_or(0)
}

pub(super) fn previous_grapheme_boundary(text: &str, offset: usize) -> usize {
    GraphemeClusterSegmenter::new()
        .segment_str(text)
        .take_while(|index| *index < offset)
        .last()
        .unwrap_or(0)
}

pub(super) fn next_grapheme_boundary(text: &str, offset: usize) -> usize {
    GraphemeClusterSegmenter::new()
        .segment_str(text)
        .find(|index| *index > offset)
        .unwrap_or(text.len())
}
/// Byte-to-x fallback for lines without shaped caret stops (genuinely
/// empty or stop-free lines). Shaped lines resolve through
/// `line_caret_x_for_affinity` first; this keeps the alignment offset
/// for the degenerate case only and never substitutes for bidi geometry.
pub(super) fn line_caret_x(line: incular_text::TextLineRef<'_>, byte: usize) -> f32 {
    if byte >= line.caret_end() {
        // Glyph x positions carry the alignment offset, so the end-of-line
        // edge must add it too; `width` alone is the unshifted advance.
        return line.offset() + line.width();
    }
    // The walk starts at the line offset for the same reason: stored glyph
    // edges are already shifted, and byte zero sits at the aligned start.
    let mut x = line.offset();
    for glyph in line.glyphs() {
        if glyph.cluster as usize >= byte {
            break;
        }
        x = (glyph.offset.x + glyph.advance).max(x);
    }
    x
}
pub(super) fn line_caret_x_for_affinity(
    layout: &dyn TextGeometry,
    line: usize,
    byte: usize,
    affinity: incular_text::TextAffinity,
) -> f32 {
    layout
        .line_caret_positions(line)
        .iter()
        .find(|position| position.offset == byte && position.affinity == affinity)
        .or_else(|| {
            layout
                .line_caret_positions(line)
                .iter()
                .find(|position| position.offset == byte)
        })
        .map_or_else(
            || {
                layout
                    .get_line(line)
                    .map_or(0.0, |line| line_caret_x(line, byte))
            },
            |position| position.x,
        )
}
/// X-to-byte fallback reached only when a line carries no caret stops
/// (empty lines). All shaped pointer mapping goes through
/// `caret_for_line_position`.
pub(super) fn caret_for_line_x(line: incular_text::TextLineRef<'_>, x: f32) -> usize {
    if x >= line.offset() + line.width() {
        return line.caret_end();
    }
    let mut best = 0usize;
    for glyph in line.glyphs() {
        let midpoint = glyph.offset.x + glyph.advance / 2.;
        if x < midpoint {
            return glyph.cluster as usize;
        }
        best = (glyph.cluster as usize).max(best);
    }
    best.max(line.start())
}
pub(super) fn caret_for_line_position(
    layout: &dyn TextGeometry,
    line: usize,
    x: f32,
) -> (usize, incular_text::TextAffinity) {
    let Some(line_data) = layout.get_line(line) else {
        return (0, incular_text::TextAffinity::Downstream);
    };
    let positions = layout.line_caret_positions(line);
    positions
        .iter()
        .min_by(|left, right| {
            (left.x - x)
                .abs()
                .total_cmp(&(right.x - x).abs())
                .then_with(|| left.x.total_cmp(&right.x))
        })
        .map_or_else(
            || {
                (
                    caret_for_line_x(line_data, x),
                    incular_text::TextAffinity::Downstream,
                )
            },
            |position| (position.offset, position.affinity),
        )
}
pub(super) fn line_for_byte(layout: &dyn TextGeometry, byte: usize) -> usize {
    layout
        .lines()
        .position(|line| byte <= line.end())
        .unwrap_or_else(|| layout.line_count().saturating_sub(1))
}
pub(super) fn caret_geometry(
    layout: &dyn TextGeometry,
    byte: usize,
    affinity: incular_text::TextAffinity,
) -> (f32, f32, f32) {
    let index = line_for_byte(layout, byte);
    let line = layout.get_line(index);
    (
        line.map_or(0., |_| {
            line_caret_x_for_affinity(layout, index, byte, affinity)
        }),
        index as f32 * layout.metrics().line_height,
        layout.metrics().line_height,
    )
}
/// An empty editor's placeholder must not decide the Start/End caret edge.
/// Flutter TextPainter resolves the empty paragraph against textDirection.
pub(super) fn text_field_caret_geometry(
    layout: &dyn TextGeometry,
    byte: usize,
    affinity: incular_text::TextAffinity,
    text_is_empty: bool,
    field: &TextFieldRender,
) -> (f32, f32, f32) {
    let (mut x, y, height) = caret_geometry(layout, byte, affinity);
    if text_is_empty
        && field.controller.preedit().is_none()
        && matches!(field.text_align, TextAlign::Start | TextAlign::End)
        && let Some(line) = layout.get_line(0)
    {
        x = match rendering::physical_text_align(field.text_align, field.text_direction) {
            TextAlign::Left => line.offset(),
            TextAlign::Right => line.offset() + line.width(),
            _ => unreachable!("Start/End resolve to physical alignment"),
        };
    }
    (x, y, height)
}

pub(super) fn selection_rects(
    layout: &dyn TextGeometry,
    selection: TextRange,
    scroll_x: f32,
    scroll_y: f32,
    top: f32,
) -> Vec<Rect> {
    layout
        .lines()
        .enumerate()
        .flat_map(|(index, line)| {
            let start = selection.start.max(line.start());
            let end = selection.end.min(line.end());
            // Direction geometry lives in Text: project the stored
            // cluster spans and frame each visual segment. This keeps
            // paint, hit testing, and selection areas on one policy.
            let y = index as f32 * layout.metrics().line_height - scroll_y + top;
            line.selection_spans(start, end)
                .into_iter()
                .map(move |(left, right)| {
                    Rect::from_origin_size(
                        Offset::new(left - scroll_x + 8., y),
                        Size::new((right - left).max(1.), layout.metrics().line_height),
                    )
                })
        })
        .collect()
}

pub(super) fn static_selection_range(
    tree: &WidgetTree,
    entries: &[ElementId],
    selection: StaticSelection,
    element: ElementId,
) -> Option<TextRange> {
    let anchor_index = entries
        .iter()
        .position(|entry| *entry == selection.anchor.element)?;
    let extent_index = entries
        .iter()
        .position(|entry| *entry == selection.extent.element)?;
    let element_index = entries.iter().position(|entry| *entry == element)?;
    let (first_index, last_index, first_byte, last_byte) = if anchor_index <= extent_index {
        (
            anchor_index,
            extent_index,
            selection.anchor.byte,
            selection.extent.byte,
        )
    } else {
        (
            extent_index,
            anchor_index,
            selection.extent.byte,
            selection.anchor.byte,
        )
    };
    if !(first_index..=last_index).contains(&element_index) {
        return None;
    }
    // A backward drag within one element must select the same bytes as
    // the forward drag: normalize the endpoints like selection_content_range
    // does across elements, or right-to-left drags select nothing.
    let (first_byte, last_byte) = if first_index == last_index {
        (first_byte.min(last_byte), first_byte.max(last_byte))
    } else {
        (first_byte, last_byte)
    };
    let text = tree.selectable_text_value(element)?;
    let start = if element_index == first_index {
        valid_boundary(&text, first_byte)
    } else {
        0
    };
    let end = if element_index == last_index {
        valid_boundary(&text, last_byte)
    } else {
        text.len()
    };
    (start < end).then_some(TextRange::new(start, end))
}

/// Converts a retained endpoint pair into the byte range of the flattened
/// selected content. Newlines use the same one-byte separator as
/// `static_selection_text`, so listeners can use the range to address the
/// text they receive from the clipboard path.
pub(super) fn selection_content_range(
    tree: &WidgetTree,
    entries: &[ElementId],
    selection: StaticSelection,
) -> Option<SelectedContentRange> {
    fn content_offset(
        tree: &WidgetTree,
        entries: &[ElementId],
        point: StaticSelectionPoint,
    ) -> Option<usize> {
        let index = entries.iter().position(|entry| *entry == point.element)?;
        let mut offset: usize = 0;
        for entry in &entries[..index] {
            offset = offset.saturating_add(tree.selectable_text_value(*entry)?.len());
            offset = offset.saturating_add(1);
        }
        let text = tree.selectable_text_value(point.element)?;
        Some(offset.saturating_add(valid_boundary(&text, point.byte)))
    }

    let anchor = content_offset(tree, entries, selection.anchor)?;
    let extent = content_offset(tree, entries, selection.extent)?;
    Some(SelectedContentRange::new(
        anchor.min(extent),
        anchor.max(extent),
    ))
}

pub(super) fn static_selection_text(
    tree: &WidgetTree,
    entries: &[ElementId],
    selection: StaticSelection,
) -> String {
    entries
        .iter()
        .filter_map(|element| {
            let range = static_selection_range(tree, entries, selection, *element)?;
            let text = tree.selectable_text_value(*element)?;
            Some(text[range.start..range.end].to_owned())
        })
        .collect::<Vec<_>>()
        .join("\n")
}
