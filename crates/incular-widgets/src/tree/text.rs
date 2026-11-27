//! Text-field editing, selectable text, caret geometry, and static selection.

use super::*;
use crate::selection::{
    SelectedContentRange, SelectionGeometry, SelectionHandleType, SelectionPoint, SelectionStatus,
    TextSelectionPoint,
};
use incular_text::{LineBoundary, TextGeometry};

#[derive(Clone, Copy)]
enum EditableCaret {
    Visual,
    ImeAnchor,
}

/// The application callback that failed during a text input event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEditingCallback {
    EditingComplete,
    Submitted,
    AppPrivateCommand,
    ContentInserted,
    SelectionChanged,
}

/// A caught Rust panic from an editor callback, preserving its message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextEditingCallbackError {
    pub callback: TextEditingCallback,
    pub message: String,
}

fn run_editing_callback(
    kind: TextEditingCallback,
    callback: impl FnOnce(),
    report: &mut impl FnMut(TextEditingCallbackError),
) {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    if let Err(payload) = catch_unwind(AssertUnwindSafe(callback)) {
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic payload")
            .to_owned();
        report(TextEditingCallbackError { callback: kind, message });
    }
}

/// The cursor and the one-pixel gap after it are part of the paragraph's
/// scrollable tail, but not its alignment width.
pub(super) fn editable_caret_margin(field: &TextFieldRender) -> f32 {
    field.cursor_width + 1.
}

/// Multiline editing scrolls vertically, so its horizontally wrapped
/// paragraph keeps the caret inside the content width even when the text
/// shaper reports advances for unbroken trailing spaces past that edge.
/// Single-line caret positions remain scrollable on the horizontal axis.
pub(super) fn editable_caret_x(field: &TextFieldRender, width: f32, x: f32) -> f32 {
    if field.multiline {
        let limit = (width - field.content_padding.horizontal() - editable_caret_margin(field))
            .max(0.);
        x.clamp(0., limit)
    } else {
        x
    }
}

/// Includes the invisible one-pixel gap and caret width in the scrollable
/// content, even if the glyphs alone fit.
pub(super) fn editable_horizontal_max_scroll(
    field: &TextFieldRender,
    layout: &impl TextGeometry,
    viewport: f32,
) -> f32 {
    let width = layout.get_line(0).map_or(0., |line| line.width());
    (width + editable_caret_margin(field) - viewport).max(0.)
}

/// The cursor's painted size and vertical alignment. The IME anchor also
/// uses these dimensions, even when the visual caret is not painted.
pub(super) fn editable_cursor_rect(field: &TextFieldRender, origin: Offset, line_height: f32) -> Rect {
    let height = field.cursor_height.unwrap_or(line_height).max(0.);
    // When the caret is taller than its line, the signed offset evenly
    // distributes the overflow above and below the line's center.
    let top = origin.y + (line_height - height) * 0.5;
    Rect::from_origin_size(
        Offset::new(origin.x, top),
        Size::new(field.cursor_width.max(0.), height),
    )
}

pub(super) fn editable_vertical_max_scroll(layout: &impl TextGeometry, viewport: f32) -> f32 {
    (layout.metrics().size.height - viewport).max(0.)
}

pub(super) fn reveal_horizontal_caret(
    current: f32,
    caret_left: f32,
    caret_width: f32,
    viewport: f32,
) -> f32 {
    if caret_width >= viewport {
        // A caret wider than the viewport cannot be fully visible; center it.
        caret_left + (caret_width - viewport) * 0.5
    } else {
        current.clamp(caret_left + (caret_width - viewport), caret_left)
    }
}

impl TextFieldRender {
    /// Resolves the externally controlled displacement without changing the
    /// automatic viewport's saved scroll position.
    pub(in crate::tree) fn scroll_offset(&self, x: f32, y: f32) -> Offset {
        match self.fixed_scroll_offset {
            Some(offset) if self.multiline => Offset::new(0., offset),
            Some(offset) => Offset::new(offset, 0.),
            None => Offset::new(x, y),
        }
    }
}

pub(super) fn editable_content_clip(
    field: &TextFieldRender,
    layout: &impl TextGeometry,
    size: Size,
    scroll: Offset,
) -> Option<Rect> {
    if field.clip_behavior == Clip::None {
        return None;
    }
    let padding = field.content_padding;
    let width = (size.width - padding.horizontal()).max(0.);
    let overflowing = if field.multiline {
        editable_vertical_max_scroll(layout, (size.height - padding.vertical()).max(0.)) > 0.
    } else {
        editable_horizontal_max_scroll(field, layout, width) > 0.
    };
    (overflowing || scroll != Offset::ZERO)
        .then(|| Rect::from_origin_size(Offset::new(padding.left, 0.), Size::new(width, size.height)))
}

/// Finds the first or last visual selection box without allocating boxes for
/// the entire highlight. In a visually contiguous RTL run the logical start
/// is its right edge and the logical end is its left edge.
fn editor_selection_edge(
    layout: &impl TextGeometry,
    start: usize,
    end: usize,
    first: bool,
) -> Option<(f32, f32, f32, TextDirection)> {
    use incular_text::{TextClusterSpan, TextLineRef};

    fn line_edge(
        line: TextLineRef<'_>,
        spans: impl Iterator<Item = TextClusterSpan>,
        start: usize,
        end: usize,
        first: bool,
    ) -> Option<(f32, f32, f32, TextDirection)> {
        let mut spans = spans.filter(|span| {
            let range = span.range();
            range.start >= start && range.end <= end && span.left < span.right
        });
        let edge = spans.next()?;
        let mut x = match (first, edge.rtl) {
            (true, false) | (false, true) => edge.left,
            _ => edge.right,
        };
        // For RTL the outer edge of the box is the opposite edge of one
        // cluster, so continue through the remaining contiguous clusters.
        if edge.rtl {
            for next in spans {
                let (adjacent, outward) = if first {
                    (next.left, next.right)
                } else {
                    (next.right, next.left)
                };
                if !next.rtl || (adjacent - x).abs() > 1e-3 {
                    break;
                }
                x = outward;
            }
        }
        let height = line.shaped().height;
        let direction = if edge.rtl { TextDirection::Rtl } else { TextDirection::Ltr };
        Some((x, line.top() + height, height, direction))
    }

    if start >= end {
        return None;
    }
    let indices = layout.line_for_offset(start)..=layout.line_for_offset(end - 1);
    if first {
        indices.filter_map(|index| layout.get_line(index)).find_map(|line| {
            line_edge(line, line.clusters(), start, end, true)
        })
    } else {
        indices.rev().filter_map(|index| layout.get_line(index)).find_map(|line| {
            line_edge(line, line.clusters().rev(), start, end, false)
        })
    }
}

pub(super) fn editable_cursor_alpha(base_alpha: u8, opacity: f32) -> u8 {
    f32::from(base_alpha).min(opacity * 255.0).round() as u8
}

/// Local editable geometry used by the native floating-caret gesture.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct FloatingCaretGeometry {
    pub caret: Rect,
    pub preferred_height: f32,
    pub minimum: Offset,
    pub maximum: Offset,
    pub world: incular_core::Transform,
}

impl FloatingCaretGeometry {
    /// Returns the floating prototype at a viewport-local drag offset.
    /// Unlike the ordinary caret, this position is not translated by scroll.
    #[must_use]
    pub fn floating_rect_at(&self, offset: Offset, inflation: f32) -> Rect {
        let size = self.caret.size;
        // RenderEditable paints from the platform's raw prototype, not
        // from the resolved regular caret rectangle. Inflation then adds
        // half a pixel on each horizontal edge and one on each vertical edge.
        let (prototype_top, prototype_height) = if incular_config::TargetPlatform::current().is_apple() {
            (0., size.height + 2.)
        } else {
            (2., size.height - 4.)
        };
        Rect::from_origin_size(
            offset + Offset::new(-0.5 * inflation, prototype_top - inflation),
            Size::new(
                size.width + inflation,
                (prototype_height + 2. * inflation).max(0.),
            ),
        )
    }
}

impl WidgetTree {
    #[doc(hidden)]
    pub fn floating_caret_geometry(
        &self,
        id: ElementId,
        offset: usize,
        affinity: incular_text::TextAffinity,
    ) -> Option<FloatingCaretGeometry> {
        let (render, caret) =
            self.text_field_local_offset_rect_with(id, offset, affinity, EditableCaret::ImeAnchor)?;
        let node = self.renders.get(render.0)?;
        let layout = node.field_layout()?;
        let metrics = layout.metrics();
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let padding = field.content_padding;
        let top = text_field_content_top(
            field.multiline,
            node.size.height,
            metrics.line_height,
            padding,
        );
        let minimum = Offset::new(padding.left - 4., top - 4.);
        let viewport_width = (node.size.width - padding.horizontal()).max(0.);
        // A force-line editor lays out its paragraph to the viewport minus
        // the caret margin, even when the visible glyphs occupy less space.
        // Ignoring that minimum cuts off the trailing whitespace at a wrap.
        let paragraph_width = metrics
            .size
            .width
            .max((viewport_width - editable_caret_margin(field)).max(0.));
        let viewport_height = (node.size.height - top - padding.bottom).max(0.);
        Some(FloatingCaretGeometry {
            caret,
            preferred_height: metrics.line_height,
            minimum,
            maximum: Offset::new(
                padding.left + viewport_width.min(paragraph_width) + 4.,
                (top + viewport_height.min(metrics.size.height) - metrics.line_height + 5.)
                    .max(minimum.y),
            ),
            world: self.render_world_transform(render),
        })
    }

    #[doc(hidden)]
    pub fn set_floating_caret(&mut self, id: ElementId, rect: Option<Rect>) {
        self.set_floating_caret_with_anchor(id, rect, None);
    }

    #[doc(hidden)]
    pub fn set_floating_caret_with_anchor(
        &mut self,
        id: ElementId,
        rect: Option<Rect>,
        anchor: Option<(usize, incular_text::TextAffinity)>,
    ) {
        let cursor = rect.map(|rect| crate::render_object::FloatingCaretPaint { rect, anchor });
        let Some(render) = self.render_id(id) else { return; };
        let Some(state) = self.renders.get(render.0).and_then(|node| node.text_field_state()) else {
            return;
        };
        if state.floating_cursor != cursor {
            self.text_field_state_live_mut(render).floating_cursor = cursor;
            self.mark_render_paint(render);
        }
    }

    /// Sets the editor's host autocorrection prompt using exact UTF-16 offsets.
    #[doc(hidden)]
    pub fn set_autocorrection_prompt(
        &mut self,
        id: ElementId,
        start: usize,
        end: usize,
    ) -> bool {
        let Some(render) = self.render_id(id) else {
            return false;
        };
        let Some(controller) = self.text_controller(id) else {
            return false;
        };
        let range = controller.with_value(|value| {
            let byte_at = |units| {
                let byte = incular_text::byte_offset_at_utf16_boundary(&value.text, units);
                (utf16_units(&value.text[..byte]) == units).then_some(byte)
            };
            let (start, end) = (byte_at(start)?, byte_at(end)?);
            (start <= end).then(|| TextRange::new(start, end))
        });
        let Some(range) = range else {
            return false;
        };
        let revision = controller.revisions().0;
        let prompt = Some((controller, revision, range));
        if self.text_field_state_live(render).autocorrection_prompt != prompt {
            self.text_field_state_live_mut(render).autocorrection_prompt = prompt;
            self.mark_render_paint(render);
        }
        true
    }

    /// Samples the focused editor's caret before paint, returning the next
    /// deadline only while blinking. No paragraph layout is invalidated.
    #[doc(hidden)]
    pub fn sample_focused_cursor(
        &mut self,
        id: ElementId,
        now: Instant,
        deterministic: bool,
    ) -> Option<Instant> {
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let state = node.text_field_state()?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let previous = state.cursor_frame;
        let eligible = state.focused
            && state.floating_cursor.is_none()
            && field.show_cursor
            && field
                .controller
                .selection_if_set()
                .is_some_and(TextSelection::is_collapsed)
            && !self.animation_muted(id);
        let (frame, deadline) = if !eligible {
            (CursorFrame::Hidden, None)
        } else if deterministic {
            (CursorFrame::Steady(field.cursor_color.alpha), None)
        } else {
            if !matches!(previous, CursorFrame::Blinking(_)) {
                field.controller.restart_caret_clock(now);
            }
            let (opacity, deadline) = field
                .controller
                .caret_blink_sample(now, field.cursor_opacity_animates);
            (
                CursorFrame::Blinking(editable_cursor_alpha(field.cursor_color.alpha, opacity)),
                deadline,
            )
        };
        if frame != previous {
            self.text_field_state_live_mut(render).cursor_frame = frame;
            if frame.sampled_alpha() != previous.sampled_alpha() {
                self.mark_render_paint(render);
            }
        }
        deadline
    }

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
        let mut changed_editor_focus = false;
        let mut editor_composition_changed = false;
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
            let state = node.text_field_state_mut().expect("text field state");
            state.focused = focused;
            state.cursor_frame = CursorFrame::Unmanaged;
            if !focused {
                state.autocorrection_prompt = None;
            }
            repaint = true;
            changed_editor_focus = true;
            if let RenderKind::TextField(field) = node.object.kind() {
                editor_composition_changed = !field.read_only
                    && !field.obscure_text
                    && (field.controller.with_value(|value| value.composing.is_some())
                        || field.controller.preedit().is_some());
                if focused {
                    // EditableText._handleFocusChanged: gaining focus with an
                    // invalid selection collapses it at the end of the text.
                    field.controller.collapse_unset_selection();
                    field.controller.reset_caret(now);
                }
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
        if changed_editor_focus {
            self.set_editor_sliver_keep_alive(id, focused);
        }
        if editor_composition_changed {
            self.mark_render_dirty(render, DirtyFlags::LAYOUT | DirtyFlags::PAINT, true);
        } else if repaint {
            self.mark_render_paint(render);
        }
        Ok(())
    }
    /// Mounted editors, including unfocused retained fields.
    #[doc(hidden)]
    pub fn text_field_elements(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.tracked.text_fields.iter().copied()
    }

    /// The retained editor set or its attached controller can change
    /// only when mounting or rebuilding. Offset-only frames need not
    /// visit every editor.
    #[doc(hidden)]
    #[must_use]
    pub fn text_field_history_epoch(&self) -> (u64, u64) {
        (self.diagnostics.mounts, self.diagnostics.rebuilds)
    }

    #[must_use]
    pub fn is_text_field(&self, id: ElementId) -> bool {
        self.elements
            .get(id.0)
            .is_some_and(|element| matches!(element.widget.kind(), WidgetKind::TextField(_)))
    }

    /// The bounds used to clip an overflowing editor's content, in the
    /// editor's local coordinates. No bounds are returned for `Clip::None`
    /// or a fitting, unscrolled editor.
    #[must_use]
    pub fn editable_approximate_paint_clip(&self, id: ElementId) -> Option<Rect> {
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let layout = node.field_layout()?;
        let state = self.text_field_state_live(render);
        editable_content_clip(
            field,
            &layout,
            node.size,
            field.scroll_offset(state.scroll_x, state.scroll_y),
        )
    }

    /// The placeholder (hint) text of a text field, when it has one. Test
    /// automation finds fields by it, the way a user reads an empty field.
    #[must_use]
    pub fn text_field_placeholder(&self, id: ElementId) -> Option<&str> {
        match self.elements.get(id.0)?.widget.kind() {
            WidgetKind::TextField(spec) if !spec.placeholder.is_empty() => Some(&spec.placeholder),
            _ => None,
        }
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

    /// The widget's selection flag for keyboard actions. Pointer and native
    /// selection use the stricter flag on the input traits.
    #[must_use]
    pub fn text_field_keyboard_selection_enabled(&self, id: ElementId) -> bool {
        self.elements.get(id.0).is_some_and(|element| {
            matches!(
                element.widget.kind(),
                WidgetKind::TextField(spec) if spec.enabled && spec.keyboard_selection_enabled
            )
        })
    }

    /// Returns the currently configured selection repair policy for an
    /// editable field and the controller to which it applies.
    #[doc(hidden)]
    pub fn editable_focus_selection_policy(
        &self,
        id: ElementId,
        platform_default: bool,
    ) -> Option<(TextEditingController, bool)> {
        let element = self.elements.get(id.0)?;
        let WidgetKind::TextField(spec) = element.widget.kind() else {
            return None;
        };
        let select_all = !spec.multiline
            && spec.keyboard_selection_enabled
            && spec.select_all_on_focus.unwrap_or(platform_default);
        Some((spec.controller.clone(), select_all))
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
        let current = text_field_display_offset(
            controller,
            controller.selection().extent,
            field.obscure_text.then_some(field.obscuring_character),
        );
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
        let target = text_field_source_offset(
            controller,
            target,
            affinity,
            field.obscure_text.then_some(field.obscuring_character),
        );
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
    /// Flutter's `EditableTextState._showCaretOnScreen`: scrolls the field's
    /// ancestor viewports, innermost first, so the caret or selection box inflated by
    /// `scroll_padding` is visible, as `RenderViewportBase.showInViewport`
    /// does (to its leading edge when scrolled past it, else to its trailing
    /// edge when short of it). Flutter animates for 100 ms; this jumps.
    /// Returns whether any viewport moved.
    pub fn text_field_show_caret_on_screen(&self, id: ElementId) -> bool {
        let Some(element) = self.elements.get(id.0) else {
            return false;
        };
        let WidgetKind::TextField(spec) = element.widget.kind() else {
            return false;
        };
        let Some(selection) = spec.controller.selection_if_set() else {
            return false;
        };
        let mut padding = spec.scroll_padding;
        if let Some(handles) = &spec.selection_handles {
            padding.bottom = padding.bottom.max(handles.caret_reveal_bottom_clearance());
        }
        let Some(target) = self
            .editable_local_selection_extent_rect(id, selection)
            .or_else(|| {
                self.editable_local_caret_rect(
                    id,
                    selection.extent,
                    spec.controller.caret_affinity(),
                )
            })
            .and_then(|rect| padding.inflate_rect(rect))
        else {
            return false;
        };
        // Automatic caret reveal follows the same implicit-scrolling policy
        // and pinned sliver geometry as every other ShowOnScreen request.
        crate::Scrollable::show_on_screen(self, id, crate::ShowOnScreen::new().rect(target)) != 0
    }
    /// Marks this selection's private editor scroll as already settled.
    /// An explicit selection command can change its range without moving
    /// the edit viewport. A later value, range, focus or size change still
    /// follows the normal automatic reveal policy.
    #[doc(hidden)]
    pub fn preserve_editable_selection_scroll(&mut self, id: ElementId) {
        let Some(render) = self.render_id(id) else {
            return;
        };
        let Some(node) = self.renders.get(render.0) else {
            return;
        };
        let (RenderKind::TextField(field), Some(state)) =
            (node.object.kind(), node.text_field_state())
        else {
            return;
        };
        let key = (
            field.controller.revisions().0,
            field.controller.selection(),
            state.focused,
            node.size,
        );
        self.text_field_state_live_mut(render).revealed = Some(key);
    }

    /// Flutter's `EditableTextState._scrollToDocumentBoundary`: brings the
    /// start or end of the text into view without moving the selection.
    pub fn text_field_scroll_to_document_boundary(&mut self, id: ElementId, forward: bool) -> bool {
        let Some(controller) = self.text_controller(id) else {
            return false;
        };
        let offset = if forward { controller.with_value(|value| value.text.len()) } else { 0 };
        self.scroll_text_field_to_position(id, offset, incular_text::TextAffinity::Downstream)
    }

    /// Flutter's EditableText.bringIntoView: make a specific text position
    /// visible without focusing the editor or moving its selection. The
    /// inner edit viewport scrolls first, then its enclosing viewports.
    pub fn text_field_bring_position_into_view(
        &mut self,
        id: ElementId,
        offset: usize,
        affinity: incular_text::TextAffinity,
    ) -> bool {
        if !self.text_controller(id).is_some_and(|controller| {
            controller.with_value(|value| value.text.is_char_boundary(offset))
        }) {
            return false;
        }
        let moved = self.scroll_text_field_to_position(id, offset, affinity);
        let outer_moved = self.editable_local_caret_rect(id, offset, affinity).is_some_and(|rect| {
            crate::Scrollable::show_on_screen(self, id, crate::ShowOnScreen::new().rect(rect)) != 0
        });
        moved || outer_moved
    }

    fn scroll_text_field_to_position(
        &mut self,
        id: ElementId,
        offset: usize,
        affinity: incular_text::TextAffinity,
    ) -> bool {
        let Some(render) = self.render_id(id) else {
            return false;
        };
        let Some(node) = self.renders.get(render.0) else {
            return false;
        };
        let RenderKind::TextField(field) = node.object.kind() else {
            return false;
        };
        if field.fixed_scroll_offset.is_some() {
            return false;
        }
        let Some(layout) = node.field_layout() else {
            return false;
        };
        let empty = field.controller.with_value(|value| value.text.is_empty());
        let (caret, caret_y, caret_height) =
            text_field_caret_geometry(&layout, offset, affinity, empty, field);
        let size = node.size;
        let multiline = field.multiline;
        let available = (size.width - field.content_padding.horizontal()).max(0.);
        let viewport = (size.height - field.content_padding.vertical()).max(0.);
        let horizontal = (!multiline && field.text_direction == TextDirection::Ltr).then(|| {
            (
                field.cursor_width,
                editable_horizontal_max_scroll(field, &layout, available),
            )
        });
        let state = self.text_field_state_live_mut(render);
        let (before_x, before_y) = (state.scroll_x, state.scroll_y);
        if multiline {
            if caret_y < state.scroll_y {
                state.scroll_y = caret_y;
            } else if caret_y + caret_height - state.scroll_y > viewport {
                state.scroll_y = caret_y + caret_height - viewport;
            }
        } else if let Some((cursor_width, max)) = horizontal {
            state.scroll_x = reveal_horizontal_caret(state.scroll_x, caret, cursor_width, available)
                .clamp(0., max);
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
        if field.fixed_scroll_offset.is_some() {
            return false;
        }
        let Some(layout) = node.field_layout() else {
            return false;
        };
        let size = node.size;
        let (target_x, target_y) = if field.multiline {
            let viewport = (size.height - field.content_padding.vertical()).max(0.);
            let max = editable_vertical_max_scroll(&layout, viewport);
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
            let available = (size.width - field.content_padding.horizontal()).max(0.);
            let max = if field.text_direction == TextDirection::Ltr {
                editable_horizontal_max_scroll(field, &layout, available)
            } else {
                let (left, right) =
                    layout
                        .lines()
                        .fold((f32::INFINITY, f32::NEG_INFINITY), |(left, right), line| {
                            (
                                left.min(line.offset()),
                                right.max(line.offset() + line.width()),
                            )
                        });
                if right - left > available {
                    right - available
                } else {
                    0.
                }
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
    /// The editor's maximum scroll extent in content pixels, after layout.
    /// Limited-line editors use the full wrapped document height, not the
    /// height of the visible lines. Single-line editors use the caret tail.
    #[must_use]
    pub fn editable_max_scroll_extent(&self, id: ElementId) -> Option<f32> {
        let node = self.renders.get(self.render_id(id)?.0)?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let state = node.text_field_state()?;
        if node.dirty.contains(DirtyFlags::LAYOUT)
            || field.controller.revisions() != (state.content_revision, state.visual_revision)
        {
            return None;
        }
        let layout = node.field_layout()?;
        let padding = field.content_padding;
        Some(if field.multiline {
            let viewport = (node.size.height - padding.vertical()).max(0.);
            editable_vertical_max_scroll(&layout, viewport)
        } else {
            let viewport = (node.size.width - padding.horizontal()).max(0.);
            editable_horizontal_max_scroll(field, &layout, viewport)
        })
    }

    /// The text field's current scroll offsets in content pixels.
    #[must_use]
    pub fn text_field_scroll_offset(&self, id: ElementId) -> Option<Offset> {
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let state = node.text_field_state()?;
        Some(field.scroll_offset(state.scroll_x, state.scroll_y))
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
        let node = self.renders.get(self.render_id(id)?.0)?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let layout = node.field_layout()?;
        line_boundary(
            &layout,
            &field.controller.text(),
            offset,
            affinity,
            forward,
            continues_at_wrap,
        )
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
        let Some((index, affinity)) = self.text_field_position_at(id, point) else {
            return false;
        };
        let Some(render) = self.render_id(id) else {
            return false;
        };
        let Some(RenderKind::TextField(field)) =
            self.renders.get(render.0).map(|node| node.object.kind())
        else {
            return false;
        };
        let controller = field.controller.clone();
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

    /// The text position under the view point `point` in a field (Flutter's
    /// `RenderEditable.getPositionForPoint`), clamped to the text.
    #[must_use]
    pub fn text_field_position_at(
        &self,
        id: ElementId,
        point: Offset,
    ) -> Option<(usize, incular_text::TextAffinity)> {
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let layout = node.field_layout();
        let state = node.text_field_state()?;
        let scroll = field.scroll_offset(state.scroll_x, state.scroll_y);
        let (multiline, padding) = (field.multiline, field.content_padding);
        let mask = field.obscure_text.then_some(field.obscuring_character);
        let local = self
            .render_world_transform(render)
            .inverse_transform_point(point)?;
        let x = local.x - padding.left + scroll.x;
        let y = local.y - if multiline { padding.top } else { 0. } + scroll.y;
        if !x.is_finite()
            || !y.is_finite()
            || layout.as_ref().is_some_and(|layout| {
                let height = layout.metrics().line_height;
                !height.is_finite() || height <= 0.
            })
        {
            return None;
        }
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
        Some((
            text_field_source_offset(&field.controller, index, affinity, mask),
            affinity,
        ))
    }

    /// Snaps a selection-handle drag to whole lines in the editable's live
    /// local coordinate system, then returns the projected global Y. An
    /// unmeasurable viewport yields no update, not a synthetic offset.
    #[doc(hidden)]
    #[must_use]
    pub fn editable_handle_drag_y(
        &self,
        id: ElementId,
        pointer: Offset,
        previous_global_y: f32,
    ) -> Option<f32> {
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let line_height = node.field_layout()?.metrics().line_height;
        if !line_height.is_finite() || line_height <= 0. {
            return None;
        }
        let transform = self.render_world_transform(render);
        let inverse = transform.inverse()?;
        let pointer = inverse.transform_point(pointer);
        let previous = inverse.transform_point(Offset::new(0., previous_global_y));
        if !pointer.x.is_finite() || !pointer.y.is_finite() || !previous.y.is_finite() {
            return None;
        }
        let lines = ((pointer.y - previous.y) / line_height).trunc();
        let local = Offset::new(0., previous.y + lines * line_height);
        let global_y = transform.transform_point(local).y;
        global_y.is_finite().then_some(global_y)
    }

    /// Replaces a field's selection as a pointer gesture would: the caret
    /// blink restarts and the field repaints.
    pub fn text_field_select(
        &mut self,
        id: ElementId,
        selection: TextSelection,
        affinity: incular_text::TextAffinity,
        now: Instant,
    ) -> bool {
        let Some(render) = self.render_id(id) else {
            return false;
        };
        let Some(RenderKind::TextField(field)) =
            self.renders.get(render.0).map(|node| node.object.kind())
        else {
            return false;
        };
        let controller = field.controller.clone();
        controller.set_selection_with_affinity(selection, affinity);
        controller.reset_caret(now);
        self.mark_render_paint(render);
        true
    }

    /// The context menu of the selection area that owns a selectable label.
    #[must_use]
    pub fn selectable_text_context_menu(
        &self,
        label: ElementId,
    ) -> Option<Rc<crate::context_menu::TextContextMenuState>> {
        let area = self.selection_area_ancestor(label)?;
        match self.elements.get(area.0)?.widget.kind() {
            WidgetKind::SelectionArea { context_menu, .. } => context_menu.clone(),
            _ => None,
        }
    }

    /// Tight local selection boxes from the paragraph's retained layout.
    /// The UTF-8 range covers literal text, excluding synthetic ellipsis.
    #[must_use]
    pub fn paragraph_selection_boxes(
        &self,
        id: ElementId,
        range: TextRange,
    ) -> Option<Vec<incular_text::TextPainterBox>> {
        self.paragraph_selection_boxes_with_style(
            id, range, incular_text::BoxHeightStyle::Tight, incular_text::BoxWidthStyle::Tight,
        )
    }

    /// Selection boxes with caller-chosen line and run bounds.
    #[must_use]
    pub fn paragraph_selection_boxes_with_style(
        &self,
        id: ElementId,
        range: TextRange,
        height_style: incular_text::BoxHeightStyle,
        width_style: incular_text::BoxWidthStyle,
    ) -> Option<Vec<incular_text::TextPainterBox>> {
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let RenderKind::Text { .. } = node.object.kind() else {
            return None;
        };
        Some(node.text_layout()?.boxes_for_selection(range, height_style, width_style))
    }

    /// Local caret origin and full height for a laid-out paragraph. Uses
    /// the retained glyphs and a cached template in the paragraph's style.
    #[must_use]
    pub fn paragraph_caret_metrics(
        &mut self,
        id: ElementId,
        byte: usize,
        affinity: incular_text::TextAffinity,
        caret_width: f32,
    ) -> Option<(Offset, f32)> {
        if self.text_engine.has_pending_application_fonts() {
            return None;
        }
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        if node.dirty.contains(DirtyFlags::LAYOUT) {
            return None;
        }
        let layout = node.text_layout_cloned()?;
        let width = node.size.width;
        let kind = node.object.shared_kind();
        let RenderKind::Text { text, style, text_direction, height_behavior, strut, .. } =
            kind.as_ref()
        else {
            return None;
        };
        let strut = strut.as_deref().filter(|strut| strut.font_size != Some(0.))
            .map(|strut| self.text_engine.resolve_strut(strut, style));
        let template = self.text_engine.layout_with_options(
            " ",
            style,
            incular_text::TextLayoutOptions::new(None, TextAlign::Start)
                .strut(strut)
                .height_behavior(*height_behavior),
        );
        let context = incular_text::ParagraphCaretContext {
            source: text,
            direction: *text_direction,
            content_width: width,
            paint_offset: Offset::ZERO,
            template_baseline: template.metrics.baseline,
            template_height: template.metrics.size.height,
            strut,
        };
        Some((
            layout.caret_offset_in_paragraph(byte, affinity, caret_width, context),
            layout.caret_height_in_paragraph(byte, affinity, context),
        ))
    }

    /// Word boundaries in the visible paragraph's original UTF-8 string,
    /// including space between words and punctuation inside a word.
    #[must_use]
    pub fn paragraph_word_boundary(&self, id: ElementId, byte: usize) -> Option<TextRange> {
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let RenderKind::Text { text, .. } = node.object.kind() else {
            return None;
        };
        node.text_layout()?;
        let (start, end) = incular_text::word_boundary_at(text, byte);
        Some(TextRange::new(start, end))
    }

    /// The byte offset under a view point in a selectable label.
    #[must_use]
    pub fn selectable_text_position_at(&self, id: ElementId, point: Offset) -> Option<usize> {
        self.selectable_text_affinity_position_at(id, point)
            .map(|(byte, _)| byte)
    }

    /// [`Self::selectable_text_position_at`] with the caret affinity of the
    /// hit, which decides which word a boundary offset belongs to.
    #[must_use]
    pub fn selectable_text_affinity_position_at(
        &self,
        id: ElementId,
        point: Offset,
    ) -> Option<(usize, incular_text::TextAffinity)> {
        let render = self.render_id(id)?;
        let layout = self
            .renders
            .get(render.0)
            .and_then(RenderNode::text_layout)?;
        let local = self
            .render_world_transform(render)
            .inverse_transform_point(point)
            .unwrap_or(point);
        let position = layout.closest_text_position(local);
        Some((position.offset, position.affinity))
    }

    /// The context menu state of a field built with a context menu builder.
    #[must_use]
    pub fn text_field_context_menu(
        &self,
        id: ElementId,
    ) -> Option<Rc<crate::context_menu::TextContextMenuState>> {
        self.elements
            .get(id.0)
            .and_then(|element| match element.widget.kind() {
                WidgetKind::TextField(spec) if spec.has_context_menu_builder => {
                    spec.context_menu.clone()
                }
                _ => None,
            })
    }
    /// Whether an editor uses legacy selection controls and its explicit toolbar flags.
    #[doc(hidden)]
    pub fn text_field_toolbar_policy(
        &self,
        id: ElementId,
    ) -> Option<(bool, Option<crate::ToolbarOptions>)> {
        match self.elements.get(id.0)?.widget.kind() {
            WidgetKind::TextField(spec) => Some((spec.legacy_selection_controls, spec.toolbar_options)),
            _ => None,
        }
    }

    /// The shared toolbar overlay, including a currently open suggestions menu.
    #[doc(hidden)]
    #[must_use]
    pub fn text_field_toolbar_state(
        &self,
        id: ElementId,
    ) -> Option<Rc<crate::context_menu::TextContextMenuState>> {
        match self.elements.get(id.0)?.widget.kind() {
            WidgetKind::TextField(spec) => spec.context_menu.clone(),
            _ => None,
        }
    }

    pub(super) fn text_field_spell_check_menu_visible(&self, id: ElementId) -> bool {
        self.elements.get(id.0).is_some_and(|element| {
            let WidgetKind::TextField(spec) = element.widget.kind() else {
                return false;
            };
            spec.context_menu.as_ref().is_some_and(|menu| menu.is_spell_check_showing())
        })
    }

    /// The overlay to which an enabled spell checker can present suggestions.
    #[doc(hidden)]
    #[must_use]
    pub fn text_field_spell_check_toolbar(
        &self,
        id: ElementId,
    ) -> Option<Rc<crate::context_menu::TextContextMenuState>> {
        let element = self.elements.get(id.0)?;
        element.auxiliary_state.as_ref()?.spell_check_session.as_ref()?;
        match element.widget.kind() {
            WidgetKind::TextField(spec) if spec.allows_spell_check_toolbar() => {
                spec.context_menu.clone()
            }
            _ => None,
        }
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
        self.is_selection_label(id)
    }
    /// Whether `id` is a selectable label: a `SelectableText`, or a plain
    /// `Text` whose nearest selection boundary accepts children. Flutter's
    /// `Text` registers its paragraph with `SelectionContainer.maybeOf`, so
    /// text under a `SelectionArea` is selectable while text under
    /// `SelectionContainer.disabled` or outside any area is not.
    #[must_use]
    pub fn is_selection_label(&self, id: ElementId) -> bool {
        match self.elements.get(id.0).map(|element| element.widget.kind()) {
            Some(WidgetKind::SelectableText { .. }) => true,
            Some(WidgetKind::Text { .. }) => {
                let mut current = id;
                while let Some(parent) = self.parent(current) {
                    if let Some(policy) = self.selection_boundary_policy(parent) {
                        return policy.accepts_children();
                    }
                    current = parent;
                }
                false
            }
            _ => false,
        }
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
        // Use shaped line positions and caret stops. A single nominal
        // line height cannot locate a hit between differently sized runs.
        let Some((byte, affinity)) = self.selectable_text_affinity_position_at(id, point) else {
            return false;
        };
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
    /// Extends a static selection from a fixed anchor word to the word under
    /// `point` in `id` (`SelectableRegionState._selectEndTo` with
    /// `TextGranularity.word`). The anchor word stays whole whichever way the
    /// drag goes; the extent snaps to the word start when the drag moves
    /// backwards and to the word end when it moves forwards.
    pub fn selectable_text_select_words_to(
        &mut self,
        anchor: ElementId,
        anchor_word: TextSelection,
        id: ElementId,
        point: Offset,
    ) -> bool {
        let Some(area) = self.selection_area_ancestor(id) else {
            return false;
        };
        if self.selection_area_ancestor(anchor) != Some(area) {
            return false;
        }
        let Some(text) = self.selectable_text_value(id) else {
            return false;
        };
        let Some((byte, _)) = self.selectable_text_affinity_position_at(id, point) else {
            return false;
        };
        let entries = self.selectable_texts_in_area(area);
        let (Some(anchor_index), Some(target_index)) = (
            entries.iter().position(|entry| *entry == anchor),
            entries.iter().position(|entry| *entry == id),
        ) else {
            return false;
        };
        let (word_start, word_end) = incular_text::word_boundary_at(text, byte);
        let (anchor_point, extent) = if (target_index, byte) >= (anchor_index, anchor_word.end()) {
            (anchor_word.start(), word_end)
        } else if (target_index, byte) < (anchor_index, anchor_word.start()) {
            (anchor_word.end(), word_start)
        } else {
            (anchor_word.start(), anchor_word.end())
        };
        self.static_selections.insert(
            area,
            StaticSelection {
                area,
                anchor: StaticSelectionPoint {
                    element: anchor,
                    byte: anchor_point,
                    affinity: incular_text::TextAffinity::Downstream,
                },
                extent: StaticSelectionPoint {
                    element: id,
                    byte: extent,
                    affinity: incular_text::TextAffinity::Downstream,
                },
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
            next_grapheme_boundary(text, current)
        } else {
            previous_grapheme_boundary(text, current)
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
    /// The text of a `SelectableText` and its selection: the area's static
    /// selection when both ends lie in this text, else a caret at its
    /// selection extent within it (at the start when it lies elsewhere).
    /// The flag reports an anchor in another text of the same area, which
    /// [`Self::selectable_text_set_extent`] keeps.
    #[must_use]
    pub fn selectable_text_editing_value(
        &self,
        id: ElementId,
    ) -> Option<(String, TextSelection, bool)> {
        let area = self.selection_area_ancestor(id)?;
        let text = self.selectable_text_value(id)?.to_owned();
        Some(match self.static_selections.get(&area).copied() {
            Some(selection) if selection.extent.element == id => {
                if selection.anchor.element == id {
                    let selection =
                        TextSelection::new(selection.anchor.byte, selection.extent.byte);
                    (text, selection, false)
                } else {
                    (text, TextSelection::collapsed(selection.extent.byte), true)
                }
            }
            _ => (text, TextSelection::collapsed(0), false),
        })
    }
    /// Moves the selection extent to `byte` in a `SelectableText`, keeping
    /// the area's anchor wherever it is.
    pub fn selectable_text_set_extent(&mut self, id: ElementId, byte: usize) -> bool {
        let Some(area) = self.selection_area_ancestor(id) else {
            return false;
        };
        let Some(text) = self.selectable_text_value(id) else {
            return false;
        };
        let extent = StaticSelectionPoint {
            element: id,
            byte: byte.min(text.len()),
            affinity: incular_text::TextAffinity::Downstream,
        };
        let anchor = self
            .static_selections
            .get(&area)
            .map_or(extent, |selection| selection.anchor);
        self.static_selections.insert(
            area,
            StaticSelection {
                area,
                anchor,
                extent,
            },
        );
        self.sync_static_selection(area);
        true
    }
    /// Replaces the area's static selection with `selection` inside one
    /// `SelectableText`.
    pub fn selectable_text_select(&mut self, id: ElementId, selection: TextSelection) -> bool {
        let Some(area) = self.selection_area_ancestor(id) else {
            return false;
        };
        let Some(len) = self.selectable_text_value(id).map(str::len) else {
            return false;
        };
        let point = |byte: usize| StaticSelectionPoint {
            element: id,
            byte: byte.min(len),
            affinity: incular_text::TextAffinity::Downstream,
        };
        self.static_selections.insert(
            area,
            StaticSelection {
                area,
                anchor: point(selection.base),
                extent: point(selection.extent),
            },
        );
        self.sync_static_selection(area);
        true
    }
    /// [`Self::text_field_line_boundary`] for a `SelectableText`.
    #[must_use]
    pub fn selectable_text_line_boundary(
        &self,
        id: ElementId,
        offset: usize,
        forward: bool,
        continues_at_wrap: bool,
    ) -> Option<usize> {
        let layout = self
            .renders
            .get(self.render_id(id)?.0)?
            .text_layout_cloned()?;
        let text = self.selectable_text_value(id)?;
        line_boundary(
            layout.as_ref(),
            text,
            offset,
            incular_text::TextAffinity::Downstream,
            forward,
            continues_at_wrap,
        )
        .map(|(offset, _)| offset)
    }
    /// The offset one line above or below `offset` in a `SelectableText` at
    /// the same caret x, or `None` on its first or last line.
    #[must_use]
    pub fn selectable_text_adjacent_line(
        &self,
        id: ElementId,
        offset: usize,
        down: bool,
    ) -> Option<usize> {
        let layout = self
            .renders
            .get(self.render_id(id)?.0)?
            .text_layout_cloned()?;
        let layout: &dyn TextGeometry = layout.as_ref();
        let line = line_for_byte(layout, offset);
        let target = if down {
            line.checked_add(1)
                .filter(|line| *line < layout.line_count())?
        } else {
            line.checked_sub(1)?
        };
        let x =
            line_caret_x_for_affinity(layout, line, offset, incular_text::TextAffinity::Downstream);
        Some(caret_for_line_position(layout, target, x).0)
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
        let last_len = self.selectable_text_value(last).map_or(0, str::len);
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
    /// Removes the region's selection and all of its paragraph highlights.
    /// A collapsed selection is different: it still preserves an anchor.
    #[doc(hidden)]
    pub fn selectable_text_clear_selection(&mut self, id: ElementId) -> bool {
        let Some(area) = self.selection_area_ancestor(id) else {
            return false;
        };
        if self.static_selections.remove(&area).is_none() {
            return false;
        }
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
            if self.is_selection_label(id) {
                return Some(id);
            }
            id = self.parent(id)?;
        }
    }
    pub(super) fn selection_area_ancestor(&self, mut id: ElementId) -> Option<ElementId> {
        let selectable = id;
        if !self.is_selection_label(id) {
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
    pub(super) fn selectable_text_value(&self, id: ElementId) -> Option<&str> {
        match self.elements.get(id.0)?.widget.kind() {
            WidgetKind::SelectableText { text, .. } | WidgetKind::Text { text, .. } => {
                Some(text.as_str())
            }
            _ => None,
        }
    }
    pub(super) fn selectable_texts_in_area(&self, area: ElementId) -> Vec<ElementId> {
        let Some(policy) = self.selection_boundary_policy(area) else {
            // A standalone SelectableText owns its implicit one-item
            // selection region. It has no explicit boundary policy, but it
            // must still participate in select-all and copy operations.
            let standalone = self.elements.get(area.0).is_some_and(|element| {
                matches!(element.widget.kind(), WidgetKind::SelectableText { .. })
            });
            return if standalone { vec![area] } else { Vec::new() };
        };
        if !policy.accepts_children() {
            return Vec::new();
        }
        let mut entries = Vec::new();
        let mut work = Vec::new();
        if let Some(element) = self.elements.get(area.0) {
            for child in &element.children {
                self.collect_selectable_texts(*child, &mut entries, &mut work);
            }
        }
        entries
    }
    pub(super) fn collect_selectable_texts(
        &self,
        id: ElementId,
        entries: &mut Vec<ElementId>,
        work: &mut Vec<ElementId>,
    ) {
        work.clear();
        work.push(id);
        while let Some(current) = work.pop() {
            let Some(element) = self.elements.get(current.0) else {
                continue;
            };
            // Inside an accepting boundary every `Text` registers, as
            // Flutter's `Text` does with `SelectionContainer.maybeOf`.
            if matches!(
                element.widget.kind(),
                WidgetKind::SelectableText { .. } | WidgetKind::Text { .. }
            ) {
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
    /// Runs the logical cursor action advertised to assistive technology.
    #[doc(hidden)]
    pub fn move_text_accessibility_cursor(
        &self,
        id: ElementId,
        forward: bool,
        by_word: bool,
        extend: bool,
    ) -> bool {
        if !self.text_field_keyboard_selection_enabled(id) {
            return false;
        }
        let Some(element) = self.elements.get(id.0) else { return false };
        let WidgetKind::TextField(spec) = element.widget.kind() else { return false };
        let Some(selection) = spec.controller.selection_if_set() else { return false };
        let target = spec.controller.with_value(|value| {
            let text = value.text.as_str();
            let caret = text.floor_char_boundary(selection.extent);
            let target = |text: &str, caret| {
                if by_word {
                    incular_text::accessibility_word_cursor(text, caret, forward)
                } else if forward {
                    text.get(caret..)?.chars().next().map(|ch| caret + ch.len_utf8())
                } else {
                    text.get(..caret)?.char_indices().next_back().map(|(offset, _)| offset)
                }
            };
            if !spec.obscure_text {
                return target(text, caret);
            }
            // Move on the mask, not the secret. A word action must not
            // reveal the placement of whitespace in a password.
            let mask = spec.obscuring_character;
            let displayed = utf16_units(&text[..caret]) * mask.len_utf8();
            let boundary = with_obscured(mask, utf16_units(text), |masked| target(masked, displayed))?;
            let affinity = if forward {
                incular_text::TextAffinity::Downstream
            } else {
                incular_text::TextAffinity::Upstream
            };
            Some(source_byte_at_utf16(text, boundary / mask.len_utf8(), affinity))
        });
        if let Some(target) = target {
            spec.controller.move_cursor(target, extend);
            true
        } else {
            false
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

    /// Stable mounted client identifier without taking an editing snapshot.
    #[must_use]
    pub fn text_field_client_id(&self, id: ElementId) -> Option<u64> {
        matches!(self.elements.get(id.0)?.widget.kind(), WidgetKind::TextField(_))
            .then_some((u64::from(id.0.generation()) << 32) | u64::from(id.0.index()))
    }

    /// Local size and world transform of the laid-out editable. The
    /// native IME caret rectangle is a separate piece of geometry.
    #[doc(hidden)]
    #[must_use]
    pub fn flutter_editable_size_and_transform(
        &self,
        id: ElementId,
    ) -> Option<(Size, CoreTransform)> {
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        if !matches!(node.object.kind(), RenderKind::TextField(_))
            || node.dirty.contains(DirtyFlags::LAYOUT)
        {
            return None;
        }
        Some((node.size, self.render_world_transform(render)))
    }

    /// Captures the current text-input client state after layout. The runtime
    /// turns this into a platform command; native window handles never enter
    /// the retained tree.
    #[must_use]
    pub fn text_field_input_snapshot(&self, id: ElementId) -> Option<TextFieldInputSnapshot> {
        self.text_field_input_snapshot_with(id, true, EditableCaret::Visual)
    }

    /// [`Self::text_field_input_snapshot`] with an empty `text`: everything a
    /// caller needs to place the caret or configure the input client, without
    /// a copy of the field's whole content.
    #[must_use]
    pub fn text_field_input_geometry(&self, id: ElementId) -> Option<TextFieldInputSnapshot> {
        self.text_field_input_snapshot_with(id, false, EditableCaret::Visual)
    }

    /// Snapshot for the native input bridge. The candidate window starts at
    /// the logical start of a selection, not the active (visual) endpoint.
    /// Pass false for `with_text` when the committed value is unchanged.
    #[doc(hidden)]
    #[must_use]
    pub fn text_field_ime_snapshot(
        &self,
        id: ElementId,
        with_text: bool,
    ) -> Option<TextFieldInputSnapshot> {
        self.text_field_input_snapshot_with(id, with_text, EditableCaret::ImeAnchor)
    }

    /// Whether the field's committed value is exactly this text, selection
    /// and composing range. A host that kept the last snapshot uses this to
    /// see that nothing changed without taking another copy of the text.
    #[must_use]
    pub fn text_field_value_matches(
        &self,
        id: ElementId,
        text: &str,
        selection: TextSelection,
        composing: Option<incular_text::TextRange>,
    ) -> bool {
        self.elements.get(id.0).is_some_and(|element| {
            let WidgetKind::TextField(spec) = element.widget.kind() else {
                return false;
            };
            spec.controller.with_value(|value| {
                value.selection == selection && value.composing == composing && value.text == text
            })
        })
    }

    /// The selection handles of a text field, when it has a handles builder.
    #[must_use]
    pub fn text_field_selection_handles(
        &self,
        id: ElementId,
    ) -> Option<Rc<crate::text_selection_handles::TextSelectionHandlesState>> {
        match self.elements.get(id.0)?.widget.kind() {
            WidgetKind::TextField(spec) => spec.selection_handles.clone(),
            _ => None,
        }
    }

    /// Returns a laid-out editable's selection endpoints in local
    /// coordinates (Flutter `getEndpointsForSelection`). The selection uses
    /// UTF-8 byte offsets. A caret or zero-width range gives one point with
    /// no direction; visible selection boxes give two directional edges.
    #[must_use]
    pub fn editable_selection_endpoints(
        &self,
        id: ElementId,
        selection: TextSelection,
        affinity: incular_text::TextAffinity,
    ) -> Option<Vec<TextSelectionPoint>> {
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let empty = field.controller.with_value(|value| {
            (value.text.is_char_boundary(selection.start())
                && value.text.is_char_boundary(selection.end()))
                .then_some(value.text.is_empty())
        })?;
        let layout = node.field_layout()?;
        let state = node.text_field_state()?;
        let scroll = field.scroll_offset(state.scroll_x, state.scroll_y);
        let padding = field.content_padding;
        let preferred_height = layout.metrics().line_height;
        let top =
            text_field_content_top(field.multiline, node.size.height, preferred_height, padding);
        let local = Offset::new(padding.left - scroll.x, top - scroll.y);
        if !selection.is_collapsed() && field.controller.preedit().is_none() {
            let mask = field.obscure_text.then_some(field.obscuring_character);
            let start = text_field_display_offset(&field.controller, selection.start(), mask);
            let end = text_field_display_offset(&field.controller, selection.end(), mask);
            let width = layout
                .metrics()
                .size
                .width
                .max((node.size.width - padding.horizontal()).max(0.));
            let edge = |first| {
                let (x, bottom, _, direction) = editor_selection_edge(&layout, start, end, first)?;
                Some(TextSelectionPoint::new(
                    local + Offset::new(x.clamp(0., width), bottom),
                    Some(direction),
                ))
            };
            if let (Some(start), Some(end)) = (edge(true), edge(false)) {
                return Some(vec![start, end]);
            }
        }
        let (x, y, _) = text_field_caret_geometry(&layout, selection.extent, affinity, empty, field);
        Some(vec![TextSelectionPoint::new(
            local + Offset::new(x, y + preferred_height),
            None,
        )])
    }

    /// The handles of a field's selection at its current layout (Flutter
    /// `TextSelectionOverlay` endpoints): one collapsed handle on the caret,
    /// or a start and an end handle on a range. `None` until the field is
    /// laid out.
    #[must_use]
    pub fn text_field_selection_handle_geometry(
        &self,
        id: ElementId,
    ) -> Option<crate::TextSelectionHandles> {
        self.text_field_selection_handle_geometry_with_field_direction(
            id,
            incular_config::TargetPlatform::current() == incular_config::TargetPlatform::Ios,
        )
    }

    /// iOS chooses artwork from the field direction; the actual endpoint
    /// directions and anchors still come from the shaped selection.
    #[doc(hidden)]
    #[must_use]
    pub fn text_field_selection_handle_geometry_with_field_direction(
        &self,
        id: ElementId,
        use_field_direction: bool,
    ) -> Option<crate::TextSelectionHandles> {
        use crate::{TextSelectionHandle, TextSelectionHandles};
        use incular_config::TextDirection;

        let snapshot = self.text_field_input_geometry(id)?;
        let selection = snapshot.selection;
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let field_direction = field.text_direction;
        let caret = snapshot.caret_rect;
        let fallback = TextSelectionHandle {
            handle_type: SelectionHandleType::Collapsed,
            anchor: Offset::new(caret.origin.x, caret.origin.y + caret.size.height),
            line_height: caret.size.height,
            direction: field_direction,
        };
        if selection.is_collapsed() {
            return Some(TextSelectionHandles { start: fallback, end: None });
        }

        let layout = node.field_layout()?;
        let state = node.text_field_state()?;
        let scroll = field.scroll_offset(state.scroll_x, state.scroll_y);
        let padding = field.content_padding;
        let top = text_field_content_top(
            field.multiline,
            node.size.height,
            layout.metrics().line_height,
            padding,
        );
        let width = layout.metrics().size.width.max((node.size.width - padding.horizontal()).max(0.));
        let mask = field.obscure_text.then_some(field.obscuring_character);
        let start = text_field_display_offset(&field.controller, selection.start(), mask);
        let end = text_field_display_offset(&field.controller, selection.end(), mask);
        let world = self.render_world_transform(render);
        let has_preedit = field.controller.preedit().is_some();
        let geometry = |first| -> Option<TextSelectionHandle> {
            if has_preedit {
                return None;
            }
            let (x, bottom, height, direction) = editor_selection_edge(&layout, start, end, first)?;
            let anchor = Offset::new(x.clamp(0., width) + padding.left - scroll.x, bottom + top - scroll.y);
            let line = Rect::from_origin_size(anchor - Offset::new(0., height), Size::new(0., height));
            Some(TextSelectionHandle {
                handle_type: SelectionHandleType::Collapsed,
                anchor: world.transform_point(anchor),
                line_height: world.transform_rect_bbox(line).size.height,
                direction,
            })
        };
        let (mut start, mut end) = if let Some(start) = geometry(true) {
            (start, geometry(false).unwrap_or(fallback))
        } else {
            (fallback, fallback)
        };
        let art_direction = |point: TextSelectionHandle| {
            if use_field_direction { field_direction } else { point.direction }
        };
        start.handle_type = match art_direction(start) {
            TextDirection::Ltr => SelectionHandleType::Left,
            TextDirection::Rtl => SelectionHandleType::Right,
        };
        end.handle_type = match art_direction(end) {
            TextDirection::Ltr => SelectionHandleType::Right,
            TextDirection::Rtl => SelectionHandleType::Left,
        };
        Some(TextSelectionHandles { start, end: Some(end) })
    }

    /// The global logical rectangle targeted by the platform text menu.
    /// A collapsed selection has zero width. A selection crossing lines
    /// uses the whole editor width rather than just the selected glyphs.
    #[must_use]
    pub fn editable_selection_menu_rect(&self, id: ElementId) -> Option<Rect> {
        use incular_text::TextAffinity::{Downstream, Upstream};

        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let layout = node.field_layout()?;
        let state = self.text_field_state_live(render);
        let scroll = field.scroll_offset(state.scroll_x, state.scroll_y);
        let preferred_height = layout.metrics().line_height;
        let (selection, empty, edge_clusters) = field.controller.with_value(|value| {
            let selection = value.selection;
            let range = selection.range();
            let clusters = value.text.get(range.as_range()).filter(|s| !s.is_empty()).map(|_| {
                (
                    TextRange::new(range.start, next_grapheme_boundary(&value.text, range.start)),
                    TextRange::new(previous_grapheme_boundary(&value.text, range.end), range.end),
                )
            });
            (selection, value.text.is_empty(), clusters)
        });
        let glyph_height = |range| {
            self.editable_composing_rect(id, range)
                .map_or(preferred_height, |rect| rect.size.height)
        };
        let (start_height, end_height) = edge_clusters.map_or(
            (preferred_height, preferred_height),
            |(start, end)| (glyph_height(start), glyph_height(end)),
        );
        let padding = field.content_padding;
        let content_top = text_field_content_top(
            field.multiline,
            node.size.height,
            preferred_height,
            padding,
        );
        let endpoint = |byte, affinity| {
            let (x, y, line_height) = text_field_caret_geometry(&layout, byte, affinity, empty, field);
            Offset::new(
                x + padding.left - scroll.x,
                y + line_height + content_top - scroll.y,
            )
        };
        let (first, last) = if selection.is_collapsed() {
            let point = endpoint(selection.extent, field.controller.caret_affinity());
            (point, point)
        } else {
            (endpoint(selection.start(), Downstream), endpoint(selection.end(), Upstream))
        };

        let world = self.render_world_transform(render);
        let a = world.transform_point(Offset::ZERO);
        let b = world.transform_point(Offset::new(node.size.width, node.size.height));
        if [a.x, a.y, b.x, b.y].into_iter().any(f32::is_nan) {
            return Some(Rect::default());
        }
        let left = a.x.min(b.x);
        let top = a.y.min(b.y);
        let crosses_lines = last.y - first.y > end_height / 2.;
        let (left, right) = if crosses_lines {
            (left, a.x.max(b.x))
        } else {
            (left + first.x.min(last.x), left + first.x.max(last.x))
        };
        let rect_top = top + first.y - start_height;
        let rect_bottom = top + last.y;
        Some(Rect::from_origin_size(
            Offset::new(left, rect_top),
            Size::maybe_new(right - left, rect_bottom - rect_top)?,
        ))
    }

    /// The one-pixel caret position marker in global coordinates, used for
    /// selection handles and position-to-point interaction. To obtain the
    /// full painted caret, use [Self::editable_local_caret_rect].
    pub fn text_field_offset_rect(
        &self,
        id: ElementId,
        offset: usize,
        affinity: incular_text::TextAffinity,
    ) -> Option<Rect> {
        self.text_field_offset_rect_with(id, offset, affinity, EditableCaret::Visual)
    }

    /// The painted caret rectangle in local coordinates, including its
    /// configured width, height, and the editor's current scroll. The byte
    /// offset and affinity identify a position in the last laid-out text.
    /// Unlike a selection marker, this reports the caret's real thickness.
    #[must_use]
    pub fn editable_local_caret_rect(
        &self,
        id: ElementId,
        offset: usize,
        affinity: incular_text::TextAffinity,
    ) -> Option<Rect> {
        self.text_field_local_offset_rect_with(id, offset, affinity, EditableCaret::ImeAnchor)
            .map(|(_, rect)| rect)
    }

    fn text_field_offset_rect_with(
        &self,
        id: ElementId,
        offset: usize,
        affinity: incular_text::TextAffinity,
        mode: EditableCaret,
    ) -> Option<Rect> {
        let (render, rect) = self.text_field_local_offset_rect_with(id, offset, affinity, mode)?;
        Some(self.render_world_transform(render).transform_rect_bbox(rect))
    }

    fn text_field_local_offset_rect_with(
        &self,
        id: ElementId,
        offset: usize,
        affinity: incular_text::TextAffinity,
        mode: EditableCaret,
    ) -> Option<(RenderObjectId, Rect)> {
        let element = self.elements.get(id.0)?;
        let WidgetKind::TextField(spec) = element.widget.kind() else {
            return None;
        };
        let text_is_empty = spec.controller.with_value(|value| value.text.is_empty());
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let layout = node.field_layout()?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let state = self.text_field_state_live(render);
        let scroll = field.scroll_offset(state.scroll_x, state.scroll_y);
        let (x, y, height) =
            text_field_caret_geometry(&layout, offset, affinity, text_is_empty, field);
        let top = text_field_content_top(
            spec.multiline,
            node.size.height,
            layout.metrics().line_height,
            field.content_padding,
        );
        let local = Offset::new(
            editable_caret_x(field, node.size.width, x) - scroll.x
                + field.content_padding.left,
            y - scroll.y + top,
        );
        let rect = match mode {
            EditableCaret::Visual => Rect::from_origin_size(local, Size::new(1., height.max(1.))),
            EditableCaret::ImeAnchor => editable_cursor_rect(field, local, height),
        };
        Some((render, rect))
    }

    fn editable_local_selection_extent_rect(
        &self,
        id: ElementId,
        selection: TextSelection,
    ) -> Option<Rect> {
        if selection.is_collapsed() {
            return None;
        }
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        // Platform preedit replaces the committed selection in the shaped
        // text, so it no longer owns a box there.
        if field.controller.preedit().is_some() {
            return None;
        }
        let layout = node.field_layout()?;
        let state = self.text_field_state_live(render);
        let scroll = field.scroll_offset(state.scroll_x, state.scroll_y);
        let mask = field.obscure_text.then_some(field.obscuring_character);
        let start = text_field_display_offset(&field.controller, selection.start(), mask);
        let end = text_field_display_offset(&field.controller, selection.end(), mask);
        if start >= end {
            return None;
        }
        let range = TextRange::new(start, end);
        let lines = layout.line_for_offset(start)..=layout.line_for_offset(end - 1);
        let forward = selection.base < selection.extent;
        let box_on_line = |index| selection_box_on_line(layout.get_line(index)?, range, forward);
        let rect = if forward {
            lines.rev().find_map(box_on_line)
        } else {
            lines.into_iter().find_map(box_on_line)
        }?;
        let padding = field.content_padding;
        let top = text_field_content_top(
            field.multiline,
            node.size.height,
            layout.metrics().line_height,
            padding,
        );
        Some(Rect::from_origin_size(
            rect.origin + Offset::new(padding.left - scroll.x, top - scroll.y),
            rect.size,
        ))
    }

    /// Returns the local bounds of the shaped text in a composing range.
    ///
    /// The range indexes the displayed, unmasked UTF-8 text (including
    /// preedit, if any). It includes complete shaped glyphs and zero-width
    /// hard breaks. Invalid or collapsed ranges have no bounds. The result
    /// includes content and scroll offsets without clipping to the viewport.
    #[must_use]
    pub fn editable_composing_rect(&self, id: ElementId, range: TextRange) -> Option<Rect> {
        if range.is_empty() {
            return None;
        }
        let render = self.render_id(id)?;
        let node = self.renders.get(render.0)?;
        let RenderKind::TextField(field) = node.object.kind() else {
            return None;
        };
        let layout = node.field_layout()?;
        let state = node.text_field_state()?;
        let scroll = field.scroll_offset(state.scroll_x, state.scroll_y);
        if node.dirty.contains(DirtyFlags::LAYOUT)
            || field.controller.revisions() != (state.content_revision, state.visual_revision)
        {
            return None;
        }
        let rect = with_text_field_display(&field.controller, "", None, |text| {
            if !range.is_valid_for(text) {
                return None;
            }
            if !field.obscure_text {
                return composing_text_bounds(&layout, range, text);
            }
            let bytes_per_mask = field.obscuring_character.len_utf8();
            let start = utf16_units(&text[..range.start]);
            let end = start + utf16_units(&text[range.as_range()]);
            // The displayed mask has no hard line breaks.
            let range = TextRange::new(start * bytes_per_mask, end * bytes_per_mask);
            composing_text_bounds(&layout, range, "")
        })?;
        let padding = field.content_padding;
        let top = text_field_content_top(
            field.multiline,
            node.size.height,
            layout.metrics().line_height,
            padding,
        );
        Some(Rect::from_origin_size(
            rect.origin + Offset::new(padding.left - scroll.x, top - scroll.y),
            rect.size,
        ))
    }

    fn text_field_input_snapshot_with(
        &self,
        id: ElementId,
        with_text: bool,
        caret: EditableCaret,
    ) -> Option<TextFieldInputSnapshot> {
        let element = self.elements.get(id.0)?;
        let WidgetKind::TextField(spec) = element.widget.kind() else {
            return None;
        };
        let value = spec
            .controller
            .with_value(|value| incular_text::TextEditingValue {
                text: if with_text {
                    value.text.clone()
                } else {
                    String::new()
                },
                selection: value.selection,
                composing: value.composing,
            });
        let bounds = self.element_bounds(id).unwrap_or_default();
        let (offset, affinity) = match caret {
            EditableCaret::Visual => (value.selection.extent, spec.controller.caret_affinity()),
            EditableCaret::ImeAnchor => {
                (value.selection.range().start, incular_text::TextAffinity::Downstream)
            }
        };
        let caret_rect = self
            .text_field_offset_rect_with(id, offset, affinity, caret)
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
                .unwrap_or(if spec.multiline { TextInputTypeHint::Multiline } else { TextInputTypeHint::Text }),
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
    /// Dispatches a validated private input-method message to a retained editor.
    #[doc(hidden)]
    pub fn dispatch_app_private_command(
        &self,
        target: ElementId,
        action: &str,
        data: &serde_json::Map<String, serde_json::Value>,
        mut report: impl FnMut(TextEditingCallbackError),
    ) {
        let Some(element) = self.elements.get(target.0) else {
            return;
        };
        let WidgetKind::TextField(spec) = element.widget.kind() else {
            return;
        };
        if let Some(callback) = &spec.on_app_private_command {
            run_editing_callback(
                TextEditingCallback::AppPrivateCommand,
                || callback(action, data),
                &mut report,
            );
        }
    }

    /// Whether the mounted editor delegates its own pointer gestures.
    #[doc(hidden)]
    #[must_use]
    pub fn text_field_renderer_ignores_pointer(&self, target: ElementId) -> bool {
        self.elements.get(target.0).is_some_and(|element| {
            matches!(
                element.widget.kind(),
                WidgetKind::TextField(spec) if spec.renderer_ignores_pointer
            )
        })
    }

    /// Whether a mounted editor listens to user selection changes.
    #[doc(hidden)]
    #[must_use]
    pub fn observes_text_selection_changes(&self, target: ElementId) -> bool {
        self.elements.get(target.0).is_some_and(|element| {
            matches!(
                element.widget.kind(),
                WidgetKind::TextField(spec) if spec.edit_selection_changed.is_some()
            )
        })
    }

    /// Reports a user selection to the currently mounted editor callback.
    #[doc(hidden)]
    pub fn dispatch_text_selection_changed(
        &self,
        target: ElementId,
        selection: TextSelection,
        cause: incular_text::SelectionChangedCause,
        mut report: impl FnMut(TextEditingCallbackError),
    ) {
        let Some(element) = self.elements.get(target.0) else {
            return;
        };
        let WidgetKind::TextField(spec) = element.widget.kind() else {
            return;
        };
        if let Some(callback) = &spec.edit_selection_changed {
            run_editing_callback(
                TextEditingCallback::SelectionChanged,
                || callback(selection, cause),
                &mut report,
            );
        }
    }

    /// Delivers rich IME content only when the mounted editor accepts its MIME type.
    #[doc(hidden)]
    pub fn dispatch_keyboard_content(
        &self,
        target: ElementId,
        content: incular_platform::KeyboardInsertedContent,
        mut report: impl FnMut(TextEditingCallbackError),
    ) -> bool {
        let Some(element) = self.elements.get(target.0) else {
            return false;
        };
        let WidgetKind::TextField(spec) = element.widget.kind() else {
            return false;
        };
        let Some(configuration) = &spec.content_insertion else {
            return false;
        };
        if !configuration.allowed_mime_types.contains(&content.mime_type) {
            return false;
        }
        run_editing_callback(
            TextEditingCallback::ContentInserted,
            || (configuration.on_content_inserted)(content),
            &mut report,
        );
        true
    }

    /// Completes editing in a field: `on_editing_complete` runs first, then
    /// `on_submit` with the committed text. Focus is owned by the caller.
    pub fn submit_text_field(&self, id: ElementId) -> bool {
        self.submit_text_field_with_errors(id, |error| {
            eprintln!("Editable text callback {:?} failed: {}", error.callback, error.message);
        })
    }

    /// Reports application callback failures without skipping the next
    /// callback or unwinding into the host's input loop.
    #[doc(hidden)]
    pub fn submit_text_field_with_errors(
        &self,
        id: ElementId,
        mut report: impl FnMut(TextEditingCallbackError),
    ) -> bool {
        let Some(element) = self.elements.get(id.0) else {
            return false;
        };
        let WidgetKind::TextField(spec) = element.widget.kind() else {
            return false;
        };
        if let Some(callback) = &spec.on_editing_complete {
            run_editing_callback(TextEditingCallback::EditingComplete, || callback(), &mut report);
        }
        if let Some(callback) = &spec.on_submit {
            run_editing_callback(
                TextEditingCallback::Submitted,
                || callback(spec.controller.text()),
                &mut report,
            );
        }
        true
    }

    /// Whether the field sets `on_editing_complete`, which replaces the default
    /// focus release on a completion action.
    #[must_use]
    pub fn text_field_has_editing_complete(&self, id: ElementId) -> bool {
        self.elements.get(id.0).is_some_and(|element| {
            matches!(
                element.widget.kind(),
                WidgetKind::TextField(spec) if spec.on_editing_complete.is_some()
            )
        })
    }

    /// Whether completing this editor will invoke a submission callback.
    #[doc(hidden)]
    #[must_use]
    pub fn text_field_has_submit(&self, id: ElementId) -> bool {
        self.elements.get(id.0).is_some_and(|element| {
            matches!(
                element.widget.kind(),
                WidgetKind::TextField(spec) if spec.on_submit.is_some()
            )
        })
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

const OBSCURE_MASK: &str = concat!(
    "••••••••••••••••",
    "••••••••••••••••",
    "••••••••••••••••",
    "••••••••••••••••",
);

fn utf16_units(text: &str) -> usize {
    if text.is_ascii() {
        text.len()
    } else {
        text.encode_utf16().count()
    }
}

/// Translates a committed model byte offset to the displayed bullet byte
/// offset. Flutter shows one default bullet per UTF-16 code unit; the
/// portable editor and its shaping engine keep UTF-8 byte positions.
pub(super) fn text_field_display_offset(
    controller: &TextEditingController,
    byte: usize,
    mask: Option<char>,
) -> usize {
    let Some(mask) = mask else {
        return byte;
    };
    let preedit = controller.preedit();
    let units = controller.with_value(|value| {
        let text = value.text.as_str();
        let byte = text.floor_char_boundary(byte.min(text.len()));
        if let Some(preedit) = preedit.as_deref() {
            let selected = value.selection.range();
            if byte > selected.start {
                let prefix = utf16_units(&text[..selected.start]);
                if byte < selected.end {
                    return prefix;
                }
                return prefix + utf16_units(preedit) + utf16_units(&text[selected.end..byte]);
            }
        }
        utf16_units(&text[..byte])
    });
    units * mask.len_utf8()
}

fn source_byte_at_utf16(text: &str, unit: usize, affinity: incular_text::TextAffinity) -> usize {
    if text.is_ascii() {
        return unit.min(text.len());
    }
    let mut prefix = 0;
    for (byte, ch) in text.char_indices() {
        if prefix == unit {
            return byte;
        }
        prefix += ch.len_utf16();
        if unit < prefix {
            // Dart can put a caret inside a surrogate pair, but a Rust
            // String cannot. Resolve that hit to a valid scalar boundary.
            return match affinity {
                incular_text::TextAffinity::Upstream => byte,
                incular_text::TextAffinity::Downstream => byte + ch.len_utf8(),
            };
        }
    }
    text.len()
}

fn text_field_source_offset(
    controller: &TextEditingController,
    byte: usize,
    affinity: incular_text::TextAffinity,
    mask: Option<char>,
) -> usize {
    let preedit = mask.and_then(|_| controller.preedit());
    controller.with_value(|value| {
        let text = value.text.as_str();
        let Some(mask) = mask else {
            return byte.min(text.len());
        };
        let mut unit = byte / mask.len_utf8();
        if let Some(preedit) = preedit.as_deref() {
            let selected = value.selection.range();
            let prefix = utf16_units(&text[..selected.start]);
            if unit > prefix {
                let replacement = utf16_units(preedit);
                if unit < prefix + replacement {
                    return selected.start;
                }
                unit = unit - replacement + utf16_units(&text[selected.start..selected.end]);
            }
        }
        source_byte_at_utf16(text, unit, affinity)
    })
}

fn with_obscured<R>(glyph: char, units: usize, f: impl FnOnce(&str) -> R) -> R {
    if glyph == '•' && units <= OBSCURE_MASK.len() / glyph.len_utf8() {
        return f(&OBSCURE_MASK[..units * glyph.len_utf8()]);
    }
    let mut bytes = [0; 4];
    f(&glyph.encode_utf8(&mut bytes).repeat(units))
}

/// Runs `f` with the text a field displays. The common case (no preedit,
/// no mask) borrows the controller's text or placeholder without copying.
pub(super) fn with_text_field_display<R>(
    controller: &TextEditingController,
    placeholder: &str,
    mask: Option<char>,
    f: impl FnOnce(&str) -> R,
) -> R {
    // Only the mask reaches shaping or paint. Position projection is
    // separate from text replacement, so native editing retains UTF-8
    // offsets even for supplementary characters.
    let preedit = controller.preedit();
    controller.with_value(|value| {
        if let Some(preedit) = preedit {
            let range = value.selection.range();
            let mut text = value.text.clone();
            text.replace_range(range.as_range(), &preedit);
            if let Some(glyph) = mask {
                with_obscured(glyph, utf16_units(&text), f)
            } else {
                f(&text)
            }
        } else if value.text.is_empty() {
            f(placeholder)
        } else if let Some(glyph) = mask {
            with_obscured(glyph, utf16_units(&value.text), f)
        } else {
            f(&value.text)
        }
    })
}

pub(super) fn valid_boundary(text: &str, offset: usize) -> usize {
    text.floor_char_boundary(offset)
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
    let nearest = positions
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            (left.x - x)
                .abs()
                .total_cmp(&(right.x - x).abs())
                .then_with(|| left.x.total_cmp(&right.x))
        });
    let Some((index, position)) = nearest else {
        return (
            caret_for_line_x(line_data, x),
            incular_text::TextAffinity::Downstream,
        );
    };
    // Adjacent clusters can encode the same caret byte at the same screen
    // edge twice. The stops are in visual cluster order, so a pointer on
    // or to the right of the shared edge belongs to the right cluster.
    // Keep distinct logical stops at bidi boundaries separate.
    let position = if x >= position.x {
        positions[index + 1..]
            .iter()
            .take_while(|next| next.x == position.x && next.offset == position.offset)
            .last()
            .unwrap_or(position)
    } else {
        position
    };
    (position.offset, position.affinity)
}
/// Flutter's `_moveToTextBoundary` over `LineBoundary` in `layout` (see
/// [`WidgetTree::text_field_line_boundary`]).
fn line_boundary(
    layout: &dyn TextGeometry,
    text: &str,
    offset: usize,
    affinity: incular_text::TextAffinity,
    forward: bool,
    continues_at_wrap: bool,
) -> Option<(usize, incular_text::TextAffinity)> {
    use incular_text::TextAffinity::{Downstream, Upstream};
    if layout.line_count() == 0 {
        return None;
    }
    let boundary = LineBoundary::new(layout);
    let bytes = text.as_bytes();
    let line_containing = |position: usize| {
        // Flutter's logical range excludes LF. A CRLF caret at the LF is
        // still on the previous logical line even if shaping excludes CR.
        let position = if position > 0 && bytes.get(position - 1..=position) == Some(b"\r\n") {
            position - 1
        } else {
            position
        };
        boundary
            .line_at(position as isize)
            .unwrap_or_else(|| layout.line(layout.line_for_offset(position)))
    };
    let logical_line_end = |line: incular_text::TextLineRef<'_>| {
        let end = line.caret_end();
        end + usize::from(
            bytes
                .get(end..)
                .is_some_and(|suffix| suffix.starts_with(b"\r\n")),
        )
    };
    let mut affinity = affinity;
    if continues_at_wrap {
        let line = line_containing(offset);
        let byte = |at: usize| text.as_bytes().get(at).copied();
        if forward {
            if affinity == Upstream
                && offset == logical_line_end(line)
                && offset != text.len()
                && !matches!(byte(offset), Some(b'\n' | b'\r'))
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
        (logical_line_end(line), Upstream)
    } else {
        (line.start(), Downstream)
    })
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
    let mut index = line_for_byte(layout, byte);
    if affinity == incular_text::TextAffinity::Downstream
        && layout
            .get_line(index)
            .is_some_and(|line| line.end() == byte && line.caret_end() == byte)
        && layout.get_line(index + 1).is_some_and(|next| next.start() == byte)
    {
        // A soft wrap shares one byte offset between its two lines. Do
        // not move a caret across a hard newline that the selection owns.
        index += 1;
    }
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
    let byte = text_field_display_offset(
        &field.controller,
        byte,
        field.obscure_text.then_some(field.obscuring_character),
    );
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

// A selection box spans adjacent selected clusters in the same visual
// direction. Unlike highlight painting, an incomplete cluster owns no box.
fn selection_box_on_line(
    line: incular_text::TextLineRef<'_>,
    range: TextRange,
    last: bool,
) -> Option<Rect> {
    let mut runs = line.clusters().filter(|cluster| {
        let source = cluster.range();
        cluster.left < cluster.right && range.start <= source.start && source.end <= range.end
    });
    let mut edge = runs.next()?;
    for cluster in runs {
        if cluster.rtl == edge.rtl && cluster.left <= edge.right && edge.left <= cluster.right {
            edge.left = edge.left.min(cluster.left);
            edge.right = edge.right.max(cluster.right);
        } else if last {
            edge = cluster;
        } else {
            break;
        }
    }
    Some(Rect::from_origin_size(
        Offset::new(edge.left, line.top()),
        Size::new(edge.right - edge.left, line.shaped().height),
    ))
}

fn composing_text_bounds(layout: &dyn TextGeometry, range: TextRange, text: &str) -> Option<Rect> {
    // Find the first potentially intersecting line without traversing
    // all earlier paragraphs in a large document.
    let mut first = 0;
    let mut limit = layout.line_count();
    while first < limit {
        let mid = first + (limit - first) / 2;
        let line = layout.line(mid);
        // An explicit line break at the caret end can have a zero-width box.
        if line.caret_end() < range.start {
            first = mid + 1;
        } else {
            limit = mid;
        }
    }

    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut top = f32::INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    for index in first..layout.line_count() {
        let line = layout.line(index);
        if line.start() >= range.end {
            break;
        }
        let blank_end = line.caret_end();
        let mut line_left = f32::INFINITY;
        let mut line_right = f32::NEG_INFINITY;
        for cluster in line.clusters() {
            let source = cluster.range();
            // A shaped glyph crossing either endpoint is not selected.
            if cluster.left < cluster.right && range.start <= source.start && source.end <= range.end
            {
                line_left = line_left.min(cluster.left);
                line_right = line_right.max(cluster.right);
            }
        }
        let has_line_break = text
            .get(blank_end..range.end)
            .is_some_and(|ending| ending.starts_with(['\n', '\r']));
        if range.start <= blank_end && has_line_break {
            let x = line_caret_x_for_affinity(
                layout,
                index,
                blank_end,
                incular_text::TextAffinity::Upstream,
            );
            line_left = line_left.min(x);
            line_right = line_right.max(x);
        }
        if line_left <= line_right {
            left = left.min(line_left);
            right = right.max(line_right);
            top = top.min(line.top());
            bottom = bottom.max(line.top() + line.shaped().height);
        }
    }
    (left <= right).then(|| {
        Rect::from_origin_size(Offset::new(left, top), Size::new(right - left, bottom - top))
    })
}

pub(super) fn selection_rects<'a>(
    layout: &'a (dyn TextGeometry + 'a),
    selection: TextRange,
    scroll_x: f32,
    scroll_y: f32,
    top: f32,
) -> impl Iterator<Item = Rect> + 'a {
    layout.lines().enumerate().flat_map(move |(index, line)| {
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
                    Offset::new(left - scroll_x, y),
                    Size::new((right - left).max(1.), layout.metrics().line_height),
                )
            })
    })
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
        valid_boundary(text, first_byte)
    } else {
        0
    };
    let end = if element_index == last_index {
        valid_boundary(text, last_byte)
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
        Some(offset.saturating_add(valid_boundary(text, point.byte)))
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
    // `SelectableRegionState.getSelectedContent` writes each selectable's
    // plain text back to back, with no separator between paragraphs.
    let mut out = String::new();
    for element in entries {
        let Some(range) = static_selection_range(tree, entries, selection, *element) else {
            continue;
        };
        let Some(text) = tree.selectable_text_value(*element) else {
            continue;
        };
        out.push_str(&text[range.start..range.end]);
    }
    out
}

/// The y of a text field's first line inside the field: the top content
/// padding for a multiline field, and centred between the vertical content
/// padding for a single-line one.
pub(super) fn text_field_content_top(
    multiline: bool,
    height: f32,
    line_height: f32,
    padding: incular_config::EdgeInsets,
) -> f32 {
    if multiline {
        padding.top
    } else {
        padding.top + ((height - padding.vertical() - line_height) / 2.).max(0.)
    }
}
