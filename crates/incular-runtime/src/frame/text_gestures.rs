//! Flutter's `TextSelectionGestureDetectorBuilder` secondary-click handling
//! for text fields (`widgets/text_selection.dart`) and the editable context
//! menu it opens (`EditableTextState.showToolbar`,
//! `contextMenuButtonItems`).

use super::*;
use crate::text_shortcuts::{TextIntent, TextShortcutPlatform};
use incular_core::SECONDARY_POINTER_BUTTON;
use incular_text::{TextAffinity, is_layout_whitespace, word_boundary_at};
use incular_widgets::{ContextMenuButtonItem, ContextMenuButtonType, TextSelectionToolbarAnchors};

/// A secondary press that went down on a text field
/// (`onSecondaryTapDown`), waiting for its release.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SecondaryTextPress {
    device: u64,
    pointer: u64,
    field: ElementId,
    position: Offset,
    /// `_lastSecondaryTapWasOnSelection`.
    on_selection: bool,
    /// Whether the field's menu was open before this press dismissed it,
    /// so `toggleToolbar` closes it instead of reopening it.
    menu_was_showing: bool,
}

impl Runtime {
    /// Before transient dismissal runs for a press: whether the menu of the
    /// field under `position` is open.
    pub(super) fn text_menu_open_at(&self, position: Offset) -> bool {
        let menu = match self.tree.text_field_at(position) {
            Some(field) => self.tree.text_field_context_menu(field),
            None => self
                .tree
                .selectable_text_at(position)
                .and_then(|label| self.tree.selectable_text_context_menu(label)),
        };
        menu.is_some_and(|menu| menu.is_showing())
    }

    /// Feeds secondary-button transitions to text fields. Returns whether the
    /// event changed anything.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn text_secondary_pointer(
        &mut self,
        phase: PointerPhase,
        button: Option<u32>,
        device: u64,
        pointer: u64,
        position: Offset,
        hit: &PointerHitTest,
        menu_was_showing: bool,
    ) -> bool {
        match phase {
            PointerPhase::Down if button == Some(SECONDARY_POINTER_BUTTON) => {
                let Some(field) = self.tree.text_field_for_hit(hit) else {
                    self.secondary_text_press = None;
                    return self.tree.selectable_text_for_hit(hit).is_some_and(|label| {
                        self.selectable_text_right_click(label, position, menu_was_showing)
                    });
                };
                let selection = self
                    .tree
                    .text_controller(field)
                    .map(|controller| controller.selection());
                let on_selection = self
                    .tree
                    .text_field_position_at(field, position)
                    .zip(selection)
                    .is_some_and(|((offset, _), selection)| {
                        selection.start() <= offset && selection.end() >= offset
                    });
                self.secondary_text_press = Some(SecondaryTextPress {
                    device,
                    pointer,
                    field,
                    position,
                    on_selection,
                    menu_was_showing,
                });
                false
            }
            PointerPhase::Up if button == Some(SECONDARY_POINTER_BUTTON) => {
                let Some(press) = self.secondary_text_press.take() else {
                    return false;
                };
                if press.device != device
                    || press.pointer != pointer
                    || self.tree.text_field_for_hit(hit) != Some(press.field)
                {
                    return false;
                }
                self.text_field_secondary_tap(press)
            }
            PointerPhase::Cancel => {
                self.secondary_text_press = None;
                false
            }
            _ => false,
        }
    }

    /// `TextSelectionGestureDetectorBuilder.onSecondaryTap`.
    fn text_field_secondary_tap(&mut self, press: SecondaryTextPress) -> bool {
        let field = press.field;
        let Some(snapshot) = self.tree.text_field_input_snapshot(field) else {
            return false;
        };
        // `delegate.selectionEnabled`.
        if !snapshot.traits.enable_interactive_selection
            || !self.text_field_pointer_selection_enabled(field)
        {
            return false;
        }
        let focused = self.focused == Some(field);
        let now = Instant::now();
        let show = match self.text_shortcut_platform {
            TextShortcutPlatform::Ios | TextShortcutPlatform::MacOs => {
                if !press.on_selection || !focused {
                    self.text_field_select_word_at(field, press.position, now);
                }
                // `hideToolbar(); showToolbar();`
                true
            }
            TextShortcutPlatform::Android
            | TextShortcutPlatform::Linux
            | TextShortcutPlatform::Windows => {
                if !focused {
                    self.tree
                        .text_field_set_caret(field, press.position, false, now);
                }
                // `toggleToolbar()`.
                !press.menu_was_showing
            }
        };
        self.set_focus(Some(field));
        if let Some(menu) = self.tree.text_field_context_menu(field) {
            if show {
                let items = self.text_field_context_menu_items(field);
                menu.show(TextSelectionToolbarAnchors::at(press.position), items);
            } else {
                menu.hide();
            }
        }
        self.frame_requested = true;
        true
    }

    /// `SelectableRegionState._handleRightClickDown`.
    fn selectable_text_right_click(
        &mut self,
        label: ElementId,
        position: Offset,
        toolbar_visible: bool,
    ) -> bool {
        let previous = self.last_selectable_secondary.replace(position);
        self.set_focus(Some(label));
        let menu = self.tree.selectable_text_context_menu(label);
        let hide = |runtime: &mut Self| {
            if let Some(menu) = &menu {
                menu.hide();
            }
            runtime.frame_requested = true;
            true
        };
        let on_selection = || {
            let (Some((_, selection, _)), Some(offset)) = (
                self.tree.selectable_text_editing_value(label),
                self.tree.selectable_text_position_at(label, position),
            ) else {
                return false;
            };
            !selection.is_collapsed() && selection.start() <= offset && offset <= selection.end()
        };
        match self.text_shortcut_platform {
            TextShortcutPlatform::Android | TextShortcutPlatform::Windows => {
                if !on_selection() {
                    self.tree
                        .selectable_text_set_selection(label, position, false);
                }
            }
            TextShortcutPlatform::Ios => {
                self.selectable_text_select_word_at(label, position);
            }
            TextShortcutPlatform::MacOs => {
                if previous == Some(position) && toolbar_visible {
                    return hide(self);
                }
                self.selectable_text_select_word_at(label, position);
            }
            TextShortcutPlatform::Linux => {
                if toolbar_visible {
                    return hide(self);
                }
                if !on_selection() {
                    self.tree
                        .selectable_text_set_selection(label, position, false);
                }
            }
        }
        if let Some(menu) = menu {
            let items = self.selectable_text_context_menu_items(label);
            menu.show(TextSelectionToolbarAnchors::at(position), items);
        }
        self.frame_requested = true;
        true
    }

    fn selectable_text_select_word_at(&mut self, label: ElementId, position: Offset) {
        let (Some((text, _, _)), Some(offset)) = (
            self.tree.selectable_text_editing_value(label),
            self.tree.selectable_text_position_at(label, position),
        ) else {
            return;
        };
        let (start, end) = word_boundary_at(&text, offset);
        self.tree
            .selectable_text_select(label, TextSelection::new(start, end));
    }

    /// `SelectableRegion.getSelectableButtonItems` on desktop: copy while
    /// something is selected, select all while there is content.
    fn selectable_text_context_menu_items(
        &mut self,
        label: ElementId,
    ) -> Vec<ContextMenuButtonItem> {
        let can_copy = self
            .tree
            .selectable_text_selected_text(label)
            .is_some_and(|text| !text.is_empty());
        let can_select_all = self
            .tree
            .selectable_text_editing_value(label)
            .is_some_and(|(text, _, _)| !text.is_empty());
        let dispatcher = self.dispatcher();
        let mut items = Vec::with_capacity(2);
        if can_copy {
            let dispatcher = dispatcher.clone();
            items.push(ContextMenuButtonItem::new(
                ContextMenuButtonType::Copy,
                move || {
                    dispatcher.dispatch(move |runtime: &mut Runtime| {
                        if let Some(text) = runtime.tree.selectable_text_selected_text(label)
                            && !text.is_empty()
                        {
                            runtime.clipboard.set_text(text);
                        }
                        runtime.frame_requested = true;
                    });
                },
            ));
        }
        if can_select_all {
            items.push(ContextMenuButtonItem::new(
                ContextMenuButtonType::SelectAll,
                move || {
                    dispatcher.dispatch(move |runtime: &mut Runtime| {
                        runtime.tree.selectable_text_select_all(label);
                        runtime.frame_requested = true;
                    });
                },
            ));
        }
        items
    }

    /// `RenderEditable.selectWord` at a view point: Flutter's
    /// `getWordAtOffset` for the target platform.
    pub(super) fn text_field_select_word_at(
        &mut self,
        field: ElementId,
        point: Offset,
        now: Instant,
    ) -> bool {
        let Some((offset, affinity)) = self.tree.text_field_position_at(field, point) else {
            return false;
        };
        let Some(controller) = self.tree.text_controller(field) else {
            return false;
        };
        let text = controller.text();
        let read_only = !self.tree.text_field_is_editable(field);
        let (selection, affinity) = word_at_offset(
            &text,
            offset,
            affinity,
            self.text_field_obscured(field),
            read_only,
            self.text_shortcut_platform,
        );
        self.tree.text_field_select(field, selection, affinity, now)
    }

    /// `EditableTextState.contextMenuButtonItems` with Flutter's
    /// `TextSelectionHandleControls` gates (`cutEnabled`, `copyEnabled`,
    /// `pasteEnabled`, `selectAllEnabled`) for the target platform.
    fn text_field_context_menu_items(&mut self, field: ElementId) -> Vec<ContextMenuButtonItem> {
        let Some(snapshot) = self.tree.text_field_input_snapshot(field) else {
            return Vec::new();
        };
        let read_only = !self.tree.text_field_is_editable(field);
        let obscured = snapshot.obscure_text;
        let collapsed = snapshot.selection.is_collapsed();
        let text_len = snapshot.text.len();
        let cut = !read_only && !obscured && !collapsed;
        let copy = !obscured && !collapsed;
        // `ClipboardStatus.pasteable`: the clipboard holds text.
        let paste = !read_only
            && self
                .clipboard
                .get_text()
                .is_some_and(|text| !text.is_empty());
        let select_all = snapshot.traits.enable_interactive_selection
            && !(read_only && obscured)
            && match self.text_shortcut_platform {
                TextShortcutPlatform::MacOs => false,
                TextShortcutPlatform::Ios => text_len > 0 && collapsed,
                TextShortcutPlatform::Android
                | TextShortcutPlatform::Linux
                | TextShortcutPlatform::Windows => {
                    text_len > 0
                        && !(snapshot.selection.start() == 0
                            && snapshot.selection.end() == text_len)
                }
            };
        let dispatcher = self.dispatcher();
        let item = |button_type, intent: TextIntent| {
            let dispatcher = dispatcher.clone();
            ContextMenuButtonItem::new(button_type, move || {
                dispatcher.dispatch(move |runtime: &mut Runtime| {
                    runtime.run_text_menu_intent(field, intent);
                });
            })
        };
        let mut items = Vec::with_capacity(4);
        if cut {
            items.push(item(ContextMenuButtonType::Cut, TextIntent::Cut));
        }
        if copy {
            items.push(item(ContextMenuButtonType::Copy, TextIntent::Copy));
        }
        if paste {
            items.push(item(ContextMenuButtonType::Paste, TextIntent::Paste));
        }
        if select_all {
            items.push(item(
                ContextMenuButtonType::SelectAll,
                TextIntent::SelectAll,
            ));
        }
        items
    }

    /// Runs a context menu button's intent on its field, which may have
    /// been rebuilt or removed since the menu opened.
    fn run_text_menu_intent(&mut self, field: ElementId, intent: TextIntent) {
        let Some(controller) = self.tree.text_controller(field) else {
            return;
        };
        let editable = self.tree.text_field_is_editable(field);
        if self.run_text_intent(field, &controller, intent, editable) {
            controller.reset_caret(Instant::now());
        }
        self.frame_requested = true;
    }

    /// Closes the focused field's context menu, as Flutter's keyboard edits
    /// do through `hideToolbar`.
    pub(super) fn hide_text_field_menu(&mut self, field: ElementId) {
        if let Some(menu) = self.tree.text_field_context_menu(field)
            && menu.hide()
        {
            self.frame_requested = true;
        }
    }
}

/// `RenderEditable.getWordAtOffset`.
pub(super) fn word_at_offset(
    text: &str,
    offset: usize,
    affinity: TextAffinity,
    obscured: bool,
    read_only: bool,
    platform: TextShortcutPlatform,
) -> (TextSelection, TextAffinity) {
    if offset >= text.len() {
        return (TextSelection::collapsed(text.len()), TextAffinity::Upstream);
    }
    if obscured {
        return (TextSelection::new(0, text.len()), affinity);
    }
    let (start, end) = word_boundary_at(text, offset);
    let effective = match affinity {
        TextAffinity::Upstream => text[..offset]
            .char_indices()
            .next_back()
            .map_or(0, |(index, _)| index),
        TextAffinity::Downstream => offset,
    };
    let whitespace_at = |index: usize| {
        text[index..]
            .chars()
            .next()
            .is_some_and(is_layout_whitespace)
    };
    if effective > 0 && whitespace_at(effective) {
        let previous = previous_word(text, start);
        match platform {
            TextShortcutPlatform::Ios => {
                return match previous {
                    None => match next_word(text, start) {
                        None => (TextSelection::collapsed(offset), affinity),
                        Some((_, next_end)) => (TextSelection::new(offset, next_end), affinity),
                    },
                    Some((previous_start, _)) => {
                        (TextSelection::new(previous_start, offset), affinity)
                    }
                };
            }
            TextShortcutPlatform::Android if read_only => {
                return match previous {
                    None => {
                        let next = text[offset..]
                            .chars()
                            .next()
                            .map_or(offset, |c| offset + c.len_utf8());
                        (TextSelection::new(offset, next), affinity)
                    }
                    Some((previous_start, _)) => {
                        (TextSelection::new(previous_start, offset), affinity)
                    }
                };
            }
            _ => {}
        }
    }
    (TextSelection::new(start, end), affinity)
}

fn only_whitespace(text: &str, start: usize, end: usize) -> bool {
    text[start..end].chars().all(is_layout_whitespace)
}

/// `RenderEditable._getNextWord`.
fn next_word(text: &str, mut offset: usize) -> Option<(usize, usize)> {
    loop {
        let (start, end) = word_boundary_at(text, offset);
        if start == end {
            return None;
        }
        if !only_whitespace(text, start, end) {
            return Some((start, end));
        }
        offset = end;
    }
}

/// `RenderEditable._getPreviousWord`.
fn previous_word(text: &str, offset: usize) -> Option<(usize, usize)> {
    let mut offset = offset as isize;
    while offset >= 0 {
        let (start, end) = word_boundary_at(text, offset as usize);
        if start == end {
            return None;
        }
        if !only_whitespace(text, start, end) {
            return Some((start, end));
        }
        offset = start as isize - 1;
        while offset > 0 && !text.is_char_boundary(offset as usize) {
            offset -= 1;
        }
    }
    None
}

/// A run of consecutive primary taps on one field and the drag that follows
/// the latest press.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TextTapRun {
    field: ElementId,
    down_position: Offset,
    last_up: Option<Instant>,
    raw_count: u32,
    effective_count: u32,
    shift: bool,
}

impl Runtime {
    pub(super) fn track_keyboard_modifiers(&mut self, event: &KeyboardEvent) {
        let mut modifiers = event.modifiers;
        if matches!(event.code, Code::ShiftLeft | Code::ShiftRight) {
            modifiers.set(incular_core::Modifiers::SHIFT, event.state.is_down());
        }
        self.keyboard_modifiers = modifiers;
    }

    fn is_desktop_text_platform(&self) -> bool {
        matches!(
            self.text_shortcut_platform,
            TextShortcutPlatform::MacOs
                | TextShortcutPlatform::Linux
                | TextShortcutPlatform::Windows
        )
    }

    /// `_TextSelectionGestureDetectorState._getEffectiveConsecutiveTapCount`.
    fn effective_tap_count(&self, raw: u32) -> u32 {
        match self.text_shortcut_platform {
            TextShortcutPlatform::Android | TextShortcutPlatform::Linux => {
                if raw <= 3 {
                    raw
                } else if raw.is_multiple_of(3) {
                    3
                } else {
                    raw % 3
                }
            }
            TextShortcutPlatform::Ios | TextShortcutPlatform::MacOs => raw.min(3),
            TextShortcutPlatform::Windows => {
                if raw < 2 {
                    raw
                } else {
                    2 + raw % 2
                }
            }
        }
    }

    /// `onTapDown`, `onDoubleTapDown` and `onTripleTapDown` of Flutter's
    /// `TextSelectionGestureDetectorBuilder` for a primary press.
    pub(super) fn text_field_primary_down(
        &mut self,
        field: ElementId,
        position: Offset,
        was_focused: bool,
    ) {
        use incular_widgets::internal::gesture_constants::{DOUBLE_TAP_SLOP, DOUBLE_TAP_TIMEOUT};
        let now = Instant::now();
        let consecutive = self.text_taps.is_some_and(|run| {
            run.field == field
                && run
                    .last_up
                    .is_some_and(|up| now.saturating_duration_since(up) <= DOUBLE_TAP_TIMEOUT)
                && (position - run.down_position)
                    .x
                    .hypot((position - run.down_position).y)
                    <= DOUBLE_TAP_SLOP
        });
        let raw_count = if consecutive {
            self.text_taps.map_or(1, |run| run.raw_count + 1)
        } else {
            1
        };
        let effective_count = self.effective_tap_count(raw_count);
        let shift = self.keyboard_modifiers.shift();
        self.text_taps = Some(TextTapRun {
            field,
            down_position: position,
            last_up: None,
            raw_count,
            effective_count,
            shift,
        });
        self.hide_text_field_menu(field);
        match effective_count {
            2 => {
                self.text_field_select_word_at(field, position, now);
            }
            3 => self.text_field_select_boundaries_in_range(field, position, position, now),
            _ => {
                if self.is_desktop_text_platform() && shift {
                    self.text_field_shift_tap(field, position, was_focused, now);
                } else {
                    self.tree.text_field_set_caret(field, position, false, now);
                }
            }
        }
    }

    /// `_extendSelection` (Linux, Windows) or `_expandSelection` (macOS).
    fn text_field_shift_tap(
        &mut self,
        field: ElementId,
        position: Offset,
        was_focused: bool,
        now: Instant,
    ) {
        let (Some((offset, affinity)), Some(controller)) = (
            self.tree.text_field_position_at(field, position),
            self.tree.text_controller(field),
        ) else {
            return;
        };
        let current = controller.selection();
        let next = if self.text_shortcut_platform == TextShortcutPlatform::MacOs {
            let from = if was_focused {
                current
            } else {
                TextSelection::collapsed(0)
            };
            let base_is_closer = (offset as isize - from.base as isize).abs()
                < (offset as isize - from.extent as isize).abs();
            TextSelection::new(
                if base_is_closer {
                    from.extent
                } else {
                    from.base
                },
                offset,
            )
        } else {
            TextSelection::new(current.base, offset)
        };
        self.tree.text_field_select(field, next, affinity, now);
    }

    /// `onDragSelectionUpdate` for the press that started at the run's
    /// latest down.
    pub(super) fn text_field_primary_drag(&mut self, field: ElementId, position: Offset) {
        let now = Instant::now();
        let Some(run) = self.text_taps.filter(|run| run.field == field) else {
            self.tree.text_field_set_caret(field, position, true, now);
            return;
        };
        if run.shift && self.is_desktop_text_platform() {
            // Shift-drag keeps the selection's base and moves its extent.
            self.tree.text_field_set_caret(field, position, true, now);
            return;
        }
        match run.effective_count {
            2 => self.text_field_select_words_in_range(field, run.down_position, position, now),
            3 => {
                self.text_field_select_boundaries_in_range(field, run.down_position, position, now)
            }
            _ => {
                self.tree.text_field_set_caret(field, position, true, now);
            }
        }
    }

    pub(super) fn text_field_primary_up(&mut self) {
        if let Some(run) = &mut self.text_taps {
            run.last_up = Some(Instant::now());
        }
    }

    /// `RenderEditable.selectWordsInRange`.
    fn text_field_select_words_in_range(
        &mut self,
        field: ElementId,
        from: Offset,
        to: Offset,
        now: Instant,
    ) {
        let (Some(controller), Some((from_offset, from_affinity)), Some((to_offset, to_affinity))) = (
            self.tree.text_controller(field),
            self.tree.text_field_position_at(field, from),
            self.tree.text_field_position_at(field, to),
        ) else {
            return;
        };
        let text = controller.text();
        let obscured = self.text_field_obscured(field);
        let read_only = !self.tree.text_field_is_editable(field);
        let platform = self.text_shortcut_platform;
        let (from_word, affinity) = word_at_offset(
            &text,
            from_offset,
            from_affinity,
            obscured,
            read_only,
            platform,
        );
        let (to_word, _) =
            word_at_offset(&text, to_offset, to_affinity, obscured, read_only, platform);
        let selection = if from_word.start() < to_word.end() {
            TextSelection::new(from_word.base, to_word.extent)
        } else {
            TextSelection::new(from_word.extent, to_word.base)
        };
        self.tree.text_field_select(field, selection, affinity, now);
    }

    /// `selectAll` for single-line fields, else `_selectParagraphsInRange`
    /// (or `_selectLinesInRange` on Linux) between two view points.
    fn text_field_select_boundaries_in_range(
        &mut self,
        field: ElementId,
        from: Offset,
        to: Offset,
        now: Instant,
    ) {
        let Some(controller) = self.tree.text_controller(field) else {
            return;
        };
        let text = controller.text();
        if !self.tree.is_multiline_text_field(field) {
            self.tree.text_field_select(
                field,
                TextSelection::new(0, text.len()),
                TextAffinity::Downstream,
                now,
            );
            return;
        }
        let (Some((from_offset, _)), Some((to_offset, _))) = (
            self.tree.text_field_position_at(field, from),
            self.tree.text_field_position_at(field, to),
        ) else {
            return;
        };
        let lines = self.text_shortcut_platform == TextShortcutPlatform::Linux;
        let range = |runtime: &Self, offset: usize| -> (usize, usize) {
            if lines {
                let start = runtime
                    .tree
                    .text_field_line_boundary(field, offset, TextAffinity::Downstream, false, false)
                    .map_or(0, |(offset, _)| offset);
                let end = runtime
                    .tree
                    .text_field_line_boundary(field, offset, TextAffinity::Downstream, true, false)
                    .map_or(text.len(), |(offset, _)| offset);
                (start, end)
            } else {
                // `_moveToTextBoundary` over `ParagraphBoundary`.
                let leading_from = if offset == text.len() {
                    offset as isize - 1
                } else {
                    offset as isize
                };
                let start = incular_text::TextBoundaryKind::Paragraph
                    .leading_at(&text, leading_from)
                    .unwrap_or(0);
                let end = incular_text::TextBoundaryKind::Paragraph
                    .trailing_at(&text, offset as isize)
                    .unwrap_or(text.len());
                (start, end)
            }
        };
        let (from_start, from_end) = range(self, from_offset);
        let (to_start, to_end) = range(self, to_offset);
        let selection = if from_start < to_end {
            TextSelection::new(from_start, to_end)
        } else {
            TextSelection::new(from_end, to_start)
        };
        self.tree
            .text_field_select(field, selection, TextAffinity::Downstream, now);
    }
}
