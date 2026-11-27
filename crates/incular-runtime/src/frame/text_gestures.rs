//! Flutter's `TextSelectionGestureDetectorBuilder` secondary-click handling
//! for text fields (`widgets/text_selection.dart`) and the editable context
//! menu it opens (`EditableTextState.showToolbar`,
//! `contextMenuButtonItems`).

use super::*;
use crate::text_shortcuts::{TextIntent, TextShortcutPlatform};
use incular_core::SECONDARY_POINTER_BUTTON;
use incular_text::{TextAffinity, is_layout_whitespace, word_boundary_at};
use incular_widgets::internal::gesture_constants::{LONG_PRESS_TIMEOUT, TOUCH_SLOP};
use incular_widgets::{
    ContextMenuButtonItem, ContextMenuButtonType, SelectionHandleType, TextSelectionToolbarAnchors,
};

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
            Some(field) => self.tree.text_field_toolbar_state(field),
            None => self
                .tree
                .selectable_text_at(position)
                .and_then(|label| self.tree.selectable_text_context_menu(label)),
        };
        menu.is_some_and(|menu| menu.is_showing())
    }

    /// Feeds secondary-button transitions to text fields. Returns whether the
    /// event changed anything.
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the pointer event fields Flutter's gesture builder receives"
    )]
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
        let Some(snapshot) = self.tree.text_field_input_geometry(field) else {
            return false;
        };
        // `delegate.selectionEnabled`.
        if !snapshot.traits.enable_interactive_selection
            || !self.text_field_pointer_selection_enabled(field)
        {
            return false;
        }
        let focused = self.focused == Some(field);
        let now = self.input_now();
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
        // Like Flutter requestKeyboard, this is internal focus: it must
        // not replace the selection made by the right-click with select-all.
        self.set_focus_with_external_selection(Some(field), false);
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

    /// `SelectableRegionState._selectWordAt`: selects the word under `position`
    /// and returns the selected range, which anchors a word-granular drag.
    fn selectable_text_select_word_at(
        &mut self,
        label: ElementId,
        position: Offset,
    ) -> Option<TextSelection> {
        let (Some((text, _, _)), Some((offset, affinity))) = (
            self.tree.selectable_text_editing_value(label),
            self.tree
                .selectable_text_affinity_position_at(label, position),
        ) else {
            return None;
        };
        // An upstream hit on a word boundary belongs to the word before it,
        // so a press on the space after "alpha" selects "alpha".
        let probe = match affinity {
            TextAffinity::Upstream => text[..offset]
                .char_indices()
                .next_back()
                .map_or(0, |(index, _)| index),
            TextAffinity::Downstream => offset,
        };
        let (start, end) = word_boundary_at(&text, probe);
        let selection = TextSelection::new(start, end);
        self.tree.selectable_text_select(label, selection);
        Some(selection)
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
        let read_only = !self.tree.text_field_is_editable(field);
        let obscured = self.text_field_obscured(field);
        let platform = self.text_shortcut_platform;
        let (selection, affinity) = controller.with_value(|value| {
            word_at_offset(&value.text, offset, affinity, obscured, read_only, platform)
        });
        self.tree.text_field_select(field, selection, affinity, now)
    }

    /// `EditableTextState.contextMenuButtonItems` before attaching handlers.
    fn text_field_context_menu_button_types(
        &mut self,
        field: ElementId,
    ) -> [Option<ContextMenuButtonType>; 4] {
        use ContextMenuButtonType::{Copy, Cut, Paste, SelectAll};
        use incular_widgets::ToolbarOptions;

        let Some(snapshot) = self.tree.text_field_input_geometry(field) else {
            return [None; 4];
        };
        let Some(controller) = self.tree.text_controller(field) else {
            return [None; 4];
        };
        let read_only = snapshot.read_only || !snapshot.enabled;
        let obscured = snapshot.obscure_text;
        let can_select_all = snapshot.traits.enable_interactive_selection && !(read_only && obscured);
        let (legacy_controls, options) =
            self.tree.text_field_toolbar_policy(field).unwrap_or_default();
        let (cut, copy, paste, select_all) =
            if legacy_controls {
                let options =
                    options.unwrap_or_else(|| ToolbarOptions::for_field(read_only, obscured));
                (
                    options.cut && !read_only && !obscured,
                    options.copy && !obscured,
                    options.paste && !read_only,
                    options.select_all && can_select_all,
                )
            } else {
                let collapsed = snapshot.selection.is_collapsed();
                let text_len = controller.with_value(|value| value.text.len());
                let select_all = match self.text_shortcut_platform {
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
                let options = options
                    .filter(|options| *options != ToolbarOptions::EMPTY)
                    .unwrap_or_else(|| ToolbarOptions::for_field(false, false));
                (
                    options.cut && !read_only && !obscured && !collapsed,
                    options.copy && !obscured && !collapsed,
                    options.paste && !read_only && self.clipboard.has_text(),
                    options.select_all && can_select_all && select_all,
                )
            };
        [(Cut, cut), (Copy, copy), (Paste, paste), (SelectAll, select_all)]
            .map(|(kind, enabled)| enabled.then_some(kind))
    }

    fn text_field_context_menu_items(&mut self, field: ElementId) -> Vec<ContextMenuButtonItem> {
        let dispatcher = self.dispatcher();
        self.text_field_context_menu_button_types(field)
            .into_iter()
            .flatten()
            .filter_map(|button_type| {
                let intent = match button_type {
                    ContextMenuButtonType::Cut => TextIntent::Cut,
                    ContextMenuButtonType::Copy => TextIntent::Copy,
                    ContextMenuButtonType::Paste => TextIntent::Paste,
                    ContextMenuButtonType::SelectAll => TextIntent::SelectAll,
                    _ => return None,
                };
                let dispatcher = dispatcher.clone();
                Some(ContextMenuButtonItem::new(button_type, move || {
                    dispatcher.dispatch(move |runtime: &mut Runtime| {
                        runtime.run_text_menu_intent(field, intent);
                    });
                }))
            })
            .collect()
    }

    /// Queries the focused text editor's default system menu and builds its
    /// platform command. This does not open an in-widget toolbar or move focus.
    pub fn show_focused_system_context_menu(
        &mut self,
        menus: &mut incular_platform::SystemContextMenus,
        controller: &incular_platform::SystemContextMenuController,
    ) -> Result<Option<incular_platform::SystemContextMenuCommand>, incular_platform::SystemContextMenuError>
    {
        use incular_platform::{SystemContextMenuError, SystemContextMenuItem};
        let connection = self
            .text_input_client
            .filter(|connection| Some(connection.target) == self.focused)
            .ok_or(SystemContextMenuError::NoTextInput)?;
        let rect = self
            .tree
            .editable_selection_menu_rect(connection.target)
            .ok_or(SystemContextMenuError::InvalidTarget)?;
        let items = self
            .text_field_context_menu_button_types(connection.target)
            .into_iter()
            .flatten()
            .filter_map(|kind| {
                Some(match kind {
                    ContextMenuButtonType::Cut => SystemContextMenuItem::Cut,
                    ContextMenuButtonType::Copy => SystemContextMenuItem::Copy,
                    ContextMenuButtonType::Paste => SystemContextMenuItem::Paste,
                    ContextMenuButtonType::SelectAll => SystemContextMenuItem::SelectAll,
                    _ => return None,
                })
            })
            .collect();
        menus.show_with_items(controller, Some(connection.client), rect, items)
    }

    /// Runs a context menu button's intent on its field, which may have
    /// been rebuilt or removed since the menu opened.
    fn run_text_menu_intent(&mut self, field: ElementId, intent: TextIntent) {
        if matches!(intent, TextIntent::SelectAll) {
            let _ = self.select_all_text(field, incular_text::SelectionChangedCause::Toolbar);
            return;
        }
        let Some(controller) = self.tree.text_controller(field) else {
            return;
        };
        let editable = self.tree.text_field_is_editable(field);
        if self.run_text_intent(field, &controller, intent, editable) {
            controller.reset_caret(self.input_now());
        }
        self.frame_requested = true;
    }

    /// Presents the configured spell-check suggestion toolbar for the
    /// caret in a focused editor. This does not impose Material or Cupertino
    /// styling. A retained button cannot apply after the value changes.
    #[must_use]
    pub fn show_spell_check_suggestions_toolbar(&mut self, field: ElementId) -> bool {
        if self.focused != Some(field) || cfg!(target_arch = "wasm32") {
            return false;
        }
        let Some(menu) = self.tree.text_field_spell_check_toolbar(field) else {
            return false;
        };
        let Some(controller) = self.tree.text_controller(field) else {
            return false;
        };
        let Some(selection) = controller.selection_if_set() else {
            return false;
        };
        let Some(span) = self.tree.text_field_suggestion_span_at_cursor_index(field, selection.extent)
        else {
            return false;
        };
        let range = TextRange::new(span.range.start, span.range.end);
        if range.is_empty() || !controller.with_value(|value| range.is_valid_for(&value.text)) {
            return false;
        }
        let Some(snapshot) = self.tree.text_field_input_geometry(field) else {
            return false;
        };
        let content_revision = controller.revisions().0;
        let buttons = span.suggestions.iter().map(|suggestion| {
            let suggestion = suggestion.clone();
            let controller = controller.clone();
            ContextMenuButtonItem::custom(suggestion.clone(), move || {
                if controller.revisions().0 == content_revision
                    && controller.selection_if_set() == Some(selection)
                {
                    let _ = controller.replace_range_as_user_edit(range, &suggestion);
                }
            })
        }).collect();
        let anchors = self.text_field_menu_anchors(field, snapshot.caret_rect.origin);
        self.hide_text_field_handles(field);
        self.frame_requested = true;
        menu.show_spell_check(anchors, buttons, span)
    }

    /// Closes the field's touch toolbar and selection handles, as Flutter's
    /// keyboard edits do through `hideToolbar`, and a mouse press does.
    pub(super) fn hide_text_field_overlays(&mut self, field: ElementId) {
        self.hide_text_field_handles(field);
        if let Some(menu) = self.tree.text_field_toolbar_state(field)
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
    ios_misspelling_tap: bool,
    menu_was_showing: bool,
    /// Whether the field had focus before this press, so its tap asks for
    /// the keyboard again rather than attaching it.
    was_focused: bool,
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
        kind: PointerDeviceKind,
        menu_was_showing: bool,
    ) {
        use incular_widgets::internal::gesture_constants::{DOUBLE_TAP_SLOP, DOUBLE_TAP_TIMEOUT};
        let now = self.input_now();
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
        let touch = matches!(
            kind,
            PointerDeviceKind::Touch | PointerDeviceKind::Stylus | PointerDeviceKind::InvertedStylus
        );
        let ios_misspelling_tap = self.text_shortcut_platform == TextShortcutPlatform::Ios
            && matches!(kind, PointerDeviceKind::Touch | PointerDeviceKind::Unknown)
            && effective_count == 1
            && !shift
            && self.tree.text_field_spell_check_toolbar(field).is_some()
            && self.tree.text_field_position_at(field, position).is_some_and(|(offset, _)| {
                self.tree.text_field_suggestion_span_at_cursor_index(field, offset).is_some()
            });
        self.text_taps = Some(TextTapRun {
            field,
            down_position: position,
            last_up: None,
            raw_count,
            effective_count,
            shift,
            ios_misspelling_tap,
            menu_was_showing,
            was_focused,
        });
        self.hide_text_field_overlays(field);
        // The iOS selection gesture is confirmed on tap-up; a touch
        // drag or long press must not select a misspelling on the press.
        if ios_misspelling_tap {
            return;
        }
        match effective_count {
            2 => {
                self.text_field_select_word_at(field, position, now);
                // `onDoubleTapDown`: a touch or stylus double tap shows the
                // toolbar for the new word; a mouse double click does not
                // (`shouldShowSelectionToolbar`).
                if touch {
                    self.show_text_field_touch_menu(field, position);
                    self.show_text_field_handles(field);
                }
            }
            3 => self.text_field_select_boundaries_in_range(field, position, position, now),
            _ => {
                if self.is_desktop_text_platform() && shift {
                    self.text_field_shift_tap(field, position, was_focused, now);
                } else {
                    self.tree.text_field_set_caret(field, position, false, now);
                    // Flutter `_shouldShowSelectionHandles`: an Android touch tap that
                    // places the caret in a focused field shows the caret handle.
                    if touch
                        && was_focused
                        && self.text_shortcut_platform == TextShortcutPlatform::Android
                    {
                        self.show_text_field_handles(field);
                    }
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
    pub(super) fn text_field_primary_drag(
        &mut self,
        field: ElementId,
        position: Offset,
        kind: PointerDeviceKind,
    ) {
        if !self.text_field_touch_drag_selects(field, kind) {
            return;
        }
        let now = self.input_now();
        let Some(run) = self.text_taps.filter(|run| run.field == field) else {
            self.tree.text_field_set_caret(field, position, true, now);
            return;
        };
        if run.shift
            && (self.is_desktop_text_platform()
                || self.text_shortcut_platform == TextShortcutPlatform::Android)
        {
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
                // Android tracks a finger with a collapsed caret. A precise
                // pointer or stylus instead extends a text selection.
                let extend = self.text_shortcut_platform != TextShortcutPlatform::Android
                    || !matches!(kind, PointerDeviceKind::Touch | PointerDeviceKind::Unknown);
                self.tree.text_field_set_caret(field, position, extend, now);
            }
        }
    }

    pub(super) fn text_field_primary_up(
        &mut self,
        position: Offset,
        kind: PointerDeviceKind,
        was_long_press: bool,
    ) {
        let now = self.input_now();
        let Some(run) = &mut self.text_taps else {
            return;
        };
        run.last_up = Some(now);
        let run = *run;
        if self.focused != Some(run.field) {
            return;
        }
        // `TextSelectionGestureDetectorBuilder.onSingleTapUp` re-opens the
        // keyboard if this editor was already focused.
        if run.was_focused {
            self.text_input_show_requested = true;
        }
        let delta = position - run.down_position;
        let single_tap = run.effective_count == 1
            && !run.shift
            && !was_long_press
            && delta.x.hypot(delta.y) <= TOUCH_SLOP;
        if single_tap && run.ios_misspelling_tap {
            let Some(controller) = self.tree.text_controller(run.field) else {
                return;
            };
            let previous = (controller.selection_if_set(), controller.caret_affinity());
            self.text_field_select_word_at(run.field, run.down_position, now);
            let current = (controller.selection_if_set(), controller.caret_affinity());
            if previous != current {
                let _ = self.show_spell_check_suggestions_toolbar(run.field);
            } else if !run.menu_was_showing {
                self.show_text_field_touch_menu(run.field, position);
            }
            return;
        }
        if single_tap
            && self.text_shortcut_platform == TextShortcutPlatform::Android
            && !self.show_spell_check_suggestions_toolbar(run.field)
            && matches!(
                kind,
                PointerDeviceKind::Touch | PointerDeviceKind::Stylus | PointerDeviceKind::InvertedStylus
            )
            // A field with the spell overlay keeps the handles visible when
            // the tapped word is spelled correctly, including first focus.
            && (run.was_focused || self.tree.text_field_spell_check_toolbar(run.field).is_some())
        {
            self.show_text_field_handles(run.field);
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
        let obscured = self.text_field_obscured(field);
        let read_only = !self.tree.text_field_is_editable(field);
        let platform = self.text_shortcut_platform;
        let (selection, affinity) = controller.with_value(|value| {
            let text = value.text.as_str();
            let (from_word, affinity) = word_at_offset(
                text,
                from_offset,
                from_affinity,
                obscured,
                read_only,
                platform,
            );
            let (to_word, _) =
                word_at_offset(text, to_offset, to_affinity, obscured, read_only, platform);
            let selection = if from_word.start() < to_word.end() {
                TextSelection::new(from_word.base, to_word.extent)
            } else {
                TextSelection::new(from_word.extent, to_word.base)
            };
            (selection, affinity)
        });
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
        if !self.tree.is_multiline_text_field(field) {
            let len = controller.with_value(|value| value.text.len());
            self.tree.text_field_select(
                field,
                TextSelection::new(0, len),
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
        let runtime: &Self = self;
        let selection = controller.with_value(|value| {
            let text = value.text.as_str();
            let range = |offset: usize| -> (usize, usize) {
                if lines {
                    let start = runtime
                        .tree
                        .text_field_line_boundary(
                            field,
                            offset,
                            TextAffinity::Downstream,
                            false,
                            false,
                        )
                        .map_or(0, |(offset, _)| offset);
                    let end = runtime
                        .tree
                        .text_field_line_boundary(
                            field,
                            offset,
                            TextAffinity::Downstream,
                            true,
                            false,
                        )
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
                        .leading_at(text, leading_from)
                        .unwrap_or(0);
                    let end = incular_text::TextBoundaryKind::Paragraph
                        .trailing_at(text, offset as isize)
                        .unwrap_or(text.len());
                    (start, end)
                }
            };
            let (from_start, from_end) = range(from_offset);
            let (to_start, to_end) = range(to_offset);
            if from_start < to_end {
                TextSelection::new(from_start, to_end)
            } else {
                TextSelection::new(from_end, to_start)
            }
        });
        self.tree
            .text_field_select(field, selection, TextAffinity::Downstream, now);
    }
}

/// What a held press on text may turn into a long press on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LongPressTarget {
    /// An `EditableText` (`TextSelectionGestureDetector.onSingleLongTapStart`).
    Field(ElementId),
    /// Selectable text in a selection area (`SelectableRegion` long press).
    Label(ElementId),
}

/// A primary press that may become a long press after `LONG_PRESS_TIMEOUT`
/// while it stays inside the touch slop.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TextLongPress {
    pub(crate) target: LongPressTarget,
    pub(crate) down_position: Offset,
    pub(crate) down_time: Instant,
    /// `_shouldShowSelectionToolbar`: a touch or stylus press shows the
    /// toolbar when the long press ends.
    pub(crate) touch: bool,
    /// The field was not focused when the press went down
    /// (`_longPressStartedWithoutFocus`).
    pub(crate) without_focus: bool,
    pub(crate) started: bool,
    /// The word a selectable-text long press selected; the drag extends it.
    pub(crate) anchor_word: Option<TextSelection>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PendingMobileTextTouch {
    pub(super) field: ElementId,
    pub(super) position: Offset,
    pub(super) menu_was_showing: bool,
}

impl PendingMobileTextTouch {
    pub(super) fn within_slop(self, position: Offset) -> bool {
        let delta = position - self.position;
        delta.x.hypot(delta.y) <= TOUCH_SLOP
    }
}

impl Runtime {
    pub(super) fn cancel_pending_mobile_text_touch(&mut self) {
        if self.pending_mobile_text_touch.take().is_some() {
            self.captured_text_field = None;
            self.text_long_press = None;
        }
    }

    /// Records a primary press on a text target as a possible long press.
    pub(super) fn text_long_press_begin(
        &mut self,
        target: LongPressTarget,
        position: Offset,
        touch: bool,
        without_focus: bool,
    ) {
        self.text_long_press = Some(TextLongPress {
            target,
            down_position: position,
            down_time: self.input_now(),
            touch,
            without_focus,
            started: false,
            anchor_word: None,
        });
    }

    /// The frame-clock deadline of a pending text long press
    /// (`kLongPressTimeout`). Returns whether one is still pending, so the
    /// scheduler keeps running until it fires or is cancelled.
    pub(super) fn pump_text_long_press(&mut self, now: Instant) -> bool {
        let Some(press) = self.text_long_press else {
            return false;
        };
        if press.started {
            return false;
        }
        if now.saturating_duration_since(press.down_time) < LONG_PRESS_TIMEOUT {
            return true;
        }
        let mut press = press;
        press.started = true;
        match press.target {
            LongPressTarget::Field(field) => {
                if self.captured_text_field != Some(field) {
                    self.text_long_press = None;
                    return false;
                }
                if self
                    .pending_mobile_text_touch
                    .is_some_and(|pending| pending.field == field)
                {
                    if !self.tree.can_request_focus(field) {
                        self.cancel_pending_mobile_text_touch();
                        return false;
                    }
                    self.pending_mobile_text_touch = None;
                    self.text_taps = None;
                    self.set_focus(Some(field));
                }
                let ios_like = matches!(
                    self.text_shortcut_platform,
                    TextShortcutPlatform::Ios | TextShortcutPlatform::MacOs
                );
                let read_only = !self.tree.text_field_is_editable(field);
                if ios_like && !press.without_focus && !read_only {
                    // iOS and macOS place the caret; a drag then extends it.
                    self.tree
                        .text_field_set_caret(field, press.down_position, false, now);
                } else {
                    // Android, Fuchsia, Linux and Windows select the word.
                    self.text_field_select_word_at(field, press.down_position, now);
                }
            }
            LongPressTarget::Label(label) => {
                if self.captured_selectable_text != Some(label) {
                    self.text_long_press = None;
                    return false;
                }
                press.anchor_word = self.selectable_text_select_word_at(label, press.down_position);
                self.set_focus(Some(label));
            }
        }
        self.text_long_press = Some(press);
        self.frame_requested = true;
        false
    }

    /// Cancels a pending long press once the pointer leaves the touch slop
    /// before the deadline; the press then behaves as a drag.
    pub(super) fn text_long_press_moved(&mut self, position: Offset) {
        if let Some(press) = self.text_long_press
            && !press.started
        {
            let delta = position - press.down_position;
            if delta.x.hypot(delta.y) > TOUCH_SLOP {
                self.text_long_press = None;
            }
        }
    }

    /// Handles a move of a started long press. Returns whether it owns the
    /// move, in which case the ordinary drag selection must not run.
    pub(super) fn text_long_press_move(
        &mut self,
        position: Offset,
        selectable_target: Option<ElementId>,
    ) -> bool {
        let Some(press) = self.text_long_press else {
            return false;
        };
        if !press.started {
            return false;
        }
        let now = self.input_now();
        match press.target {
            LongPressTarget::Field(field) => {
                let ios_like = matches!(
                    self.text_shortcut_platform,
                    TextShortcutPlatform::Ios | TextShortcutPlatform::MacOs
                );
                let read_only = !self.tree.text_field_is_editable(field);
                if ios_like && !press.without_focus && !read_only {
                    self.tree.text_field_set_caret(field, position, true, now);
                } else {
                    self.text_field_select_words_in_range(
                        field,
                        press.down_position,
                        position,
                        now,
                    );
                }
            }
            LongPressTarget::Label(label) => {
                if let (Some(anchor), Some(target)) = (press.anchor_word, selectable_target)
                    && self
                        .tree
                        .selectable_text_select_words_to(label, anchor, target, position)
                {
                    self.set_focus(Some(target));
                }
            }
        }
        self.frame_requested = true;
        true
    }

    /// Shows `field`'s selection handles at its selection, once its layout
    /// is known. The per-frame sync keeps them on the endpoints.
    fn show_text_field_handles(&mut self, field: ElementId) {
        if let Some(state) = self.tree.text_field_selection_handles(field)
            && let Some(handles) = self.tree.text_field_selection_handle_geometry(field)
        {
            state.show(handles);
            self.frame_requested = true;
        }
    }

    /// Closes `field`'s selection handles.
    pub(super) fn hide_text_field_handles(&mut self, field: ElementId) {
        if let Some(state) = self.tree.text_field_selection_handles(field)
            && state.hide()
        {
            self.frame_requested = true;
        }
    }

    /// Keeps the focused field's visible handles on its selection after
    /// layout. A field that stops being editable closes them.
    pub(super) fn sync_text_selection_handles(&mut self) {
        let Some(field) = self.focused.filter(|field| self.tree.is_text_field(*field)) else {
            return;
        };
        let Some(state) = self.tree.text_field_selection_handles(field) else {
            return;
        };
        if !state.is_showing() {
            return;
        }
        let changed = if !self.tree.text_field_is_editable(field) {
            state.hide()
        } else {
            match self.tree.text_field_selection_handle_geometry(field) {
                Some(handles) => state.update(handles),
                None => state.hide(),
            }
        };
        if changed {
            self.frame_requested = true;
        }
    }

    /// Whether a drag of `kind` on `field` extends its selection. Flutter
    /// `TextSelectionGestureDetectorBuilder`: mouse drags always select; on iOS
    /// touch and stylus drags never do; on Android a touch or stylus drag
    /// selects only when the field already had focus when the press went down.
    fn text_field_touch_drag_selects(&self, field: ElementId, kind: PointerDeviceKind) -> bool {
        let touch = matches!(
            kind,
            PointerDeviceKind::Touch
                | PointerDeviceKind::Unknown
                | PointerDeviceKind::Stylus
                | PointerDeviceKind::InvertedStylus
        );
        if !touch {
            return true;
        }
        match self.text_shortcut_platform {
            TextShortcutPlatform::Ios => false,
            TextShortcutPlatform::Android => self
                .text_taps
                .is_some_and(|run| run.field == field && run.was_focused),
            _ => true,
        }
    }

    /// Shows the touch toolbar of `field` for the press at `position`, if the
    /// field has a context menu builder.
    fn show_text_field_touch_menu(&mut self, field: ElementId, position: Offset) {
        if let Some(menu) = self.tree.text_field_context_menu(field) {
            let items = self.text_field_context_menu_items(field);
            let anchors = self.text_field_menu_anchors(field, position);
            menu.show(anchors, items);
        }
    }

    /// `EditableText.contextMenuAnchors`: at the press's x, above the top of
    /// the selection's line, or below its bottom.
    fn text_field_menu_anchors(
        &self,
        field: ElementId,
        position: Offset,
    ) -> TextSelectionToolbarAnchors {
        self.tree.text_field_input_geometry(field).map_or(
            TextSelectionToolbarAnchors::at(position),
            |snapshot| {
                let caret = snapshot.caret_rect;
                TextSelectionToolbarAnchors {
                    primary_anchor: Offset::new(position.x, caret.origin.y),
                    secondary_anchor: Some(Offset::new(
                        position.x,
                        caret.origin.y + caret.size.height,
                    )),
                }
            },
        )
    }

    /// `onSingleLongTapEnd` and `_handleTouchLongPressEnd`: a started long
    /// press shows the context menu, for touch and stylus on text fields.
    pub(super) fn text_long_press_end(&mut self, position: Offset) {
        let Some(press) = self.text_long_press.take() else {
            return;
        };
        if !press.started {
            return;
        }
        match press.target {
            LongPressTarget::Field(field) => {
                if !press.touch {
                    return;
                }
                self.show_text_field_touch_menu(field, position);
                self.show_text_field_handles(field);
            }
            LongPressTarget::Label(label) => {
                if let Some(menu) = self.tree.selectable_text_context_menu(label) {
                    let items = self.selectable_text_context_menu_items(label);
                    menu.show(TextSelectionToolbarAnchors::at(position), items);
                }
            }
        }
        self.frame_requested = true;
    }
}

/// A press on one of the focused field's visible selection handles, from
/// touch down to lift. Flutter `_SelectionHandleOverlay`'s pan gesture feeds
/// the `TextSelectionOverlay` drag handlers, and the knob's tap is
/// `TextSelectionHandleControls` calling `onSelectionHandleTapped`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TextHandleDrag {
    field: ElementId,
    /// The pressed handle. `Collapsed` and `Left` are the start handle, and
    /// `Right` is the end handle.
    handle: SelectionHandleType,
    down_position: Offset,
    /// The press landed on the handle's drawn knob, where its tap lives.
    on_knob: bool,
    line_height: f32,
    /// The contact's global y, snapped to whole lines
    /// (`_startHandleDragPosition` / `_endHandleDragPosition`).
    drag_y: f32,
    /// Distance from `drag_y` to the centre of the handle's line
    /// (`_startHandleDragTarget` / `_endHandleDragTarget`).
    target: f32,
    /// The selection when the press began (`_dragStartSelection`, used on
    /// Apple platforms).
    start_selection: TextSelection,
    /// Whether the press moved past touch slop, so it is a drag and not a tap.
    started: bool,
}

/// `_getHandleDy`: the handle's y after a drag from `handle_dy` to `drag_dy`.
/// The handle jumps whole lines, once the contact has moved a full line.
fn handle_dy(drag_dy: f32, handle_dy: f32, line_height: f32) -> f32 {
    if line_height <= 0.0 {
        return handle_dy;
    }
    let distance = drag_dy - handle_dy;
    let direction = if distance < 0.0 { -1.0 } else { 1.0 };
    let lines = direction * (distance.abs() / line_height).floor();
    handle_dy + lines * line_height
}

impl Runtime {
    /// The focused field and the handle a touch or stylus press at `position`
    /// lands on, if any. Pointer routing asks this before dismissing
    /// transients: a handle belongs to its field, as Flutter's tap region
    /// groups a text field with its handles and toolbar.
    pub(super) fn text_handle_under(
        &self,
        position: Offset,
        kind: PointerDeviceKind,
    ) -> Option<(ElementId, SelectionHandleType)> {
        let touch = matches!(
            kind,
            PointerDeviceKind::Touch
                | PointerDeviceKind::Stylus
                | PointerDeviceKind::InvertedStylus
        );
        if !touch {
            return None;
        }
        let field = self
            .focused
            .filter(|field| self.tree.is_text_field(*field))?;
        let handles = self.tree.text_field_selection_handles(field)?.handles()?;
        Some((field, handles.hit_test(position)?))
    }

    /// Starts a handle drag when a touch or stylus press lands on a visible
    /// handle of the focused field. Flutter's handle hit area wins over the
    /// field, so the press neither moves focus nor starts a long press.
    pub(super) fn text_handle_press(
        &mut self,
        position: Offset,
        kind: PointerDeviceKind,
    ) -> Option<ElementId> {
        let (field, handle) = self.text_handle_under(position, kind)?;
        let handles = self.tree.text_field_selection_handles(field)?.handles()?;
        let geometry = match handle {
            SelectionHandleType::Right => handles.end?,
            _ => handles.start,
        };
        // `_handleSelectionEndHandleDragStart`: aim at the centre of the line
        // the handle points to, not at the handle itself.
        let centre = geometry.anchor.y - geometry.line_height / 2.0;
        let start_selection = self
            .tree
            .text_controller(field)
            .map_or(TextSelection::collapsed(0), |controller| {
                controller.selection()
            });
        self.text_handle_drag = Some(TextHandleDrag {
            field,
            handle,
            down_position: position,
            on_knob: geometry.rect().contains(position),
            line_height: geometry.line_height,
            drag_y: position.y,
            target: centre - position.y,
            start_selection,
            started: false,
        });
        Some(field)
    }

    /// Advances a handle drag on pointer move. Movement inside touch slop is
    /// not yet a drag, so a tap on the handle can still be recognised.
    pub(super) fn text_handle_drag_move(&mut self, position: Offset) {
        let Some(mut drag) = self.text_handle_drag.take() else {
            return;
        };
        if !drag.started {
            let delta = position - drag.down_position;
            if delta.x.hypot(delta.y) <= TOUCH_SLOP {
                self.text_handle_drag = Some(drag);
                return;
            }
            drag.started = true;
            // `_handleSelectionStartHandleDragStart` hides the toolbar while
            // the handle moves; `_handleAnyDragEnd` shows it again.
            if let Some(menu) = self.tree.text_field_context_menu(drag.field) {
                menu.hide();
            }
        }
        let now = self.input_now();
        drag.drag_y = handle_dy(position.y, drag.drag_y, drag.line_height);
        let target = Offset::new(position.x, drag.drag_y + drag.target);
        self.text_handle_select(&drag, target, now);
        self.text_handle_drag = Some(drag);
        self.frame_requested = true;
    }

    /// The selection update of `_handleSelectionEndHandleDragUpdate` and
    /// `_handleSelectionStartHandleDragUpdate`. Apple platforms anchor on
    /// the drag-start selection and never invert it; the others keep the
    /// handles from crossing or collapsing.
    fn text_handle_select(&mut self, drag: &TextHandleDrag, target: Offset, now: Instant) {
        let Some((offset, affinity)) = self.tree.text_field_position_at(drag.field, target) else {
            return;
        };
        let Some(controller) = self.tree.text_controller(drag.field) else {
            return;
        };
        let current = controller.selection();
        let next = if matches!(
            self.text_shortcut_platform,
            TextShortcutPlatform::Ios | TextShortcutPlatform::MacOs
        ) {
            let start = drag.start_selection;
            if start.is_collapsed() {
                TextSelection::collapsed(offset)
            } else {
                let normalized = start.extent >= start.base;
                match (drag.handle, normalized) {
                    (SelectionHandleType::Right, true) => TextSelection::new(start.base, offset),
                    (SelectionHandleType::Right, false) => TextSelection::new(start.extent, offset),
                    (_, true) => TextSelection::new(start.extent, offset),
                    (_, false) => TextSelection::new(start.base, offset),
                }
            }
        } else if current.is_collapsed() {
            TextSelection::collapsed(offset)
        } else {
            let next = match drag.handle {
                SelectionHandleType::Right => TextSelection::new(current.base, offset),
                _ => TextSelection::new(offset, current.extent),
            };
            // Flutter: do not allow order swapping.
            if next.base >= next.extent {
                return;
            }
            next
        };
        self.tree.text_field_select(drag.field, next, affinity, now);
    }

    /// Ends a handle press. A tap on a collapsed handle toggles the touch
    /// toolbar (`toggleToolbar`); a drag shows it again when the selection is
    /// not collapsed (`_handleAnyDragEnd`).
    pub(super) fn text_handle_drag_end(&mut self, position: Offset) {
        let Some(drag) = self.text_handle_drag.take() else {
            return;
        };
        self.frame_requested = true;
        if !drag.started {
            let delta = position - drag.down_position;
            if drag.handle == SelectionHandleType::Collapsed
                && drag.on_knob
                && delta.x.hypot(delta.y) <= TOUCH_SLOP
            {
                self.text_field_toggle_toolbar(drag.field, drag.down_position);
            }
            return;
        }
        let collapsed = self
            .tree
            .text_controller(drag.field)
            .is_none_or(|controller| controller.selection().is_collapsed());
        if !collapsed {
            self.show_text_field_touch_menu(drag.field, position);
        }
    }

    /// `EditableTextState.toggleToolbar` with the default `hideHandles`:
    /// closes the toolbar and handles when the toolbar shows, else shows it.
    fn text_field_toggle_toolbar(&mut self, field: ElementId, position: Offset) {
        let showing = self
            .tree
            .text_field_toolbar_state(field)
            .is_some_and(|menu| menu.is_showing());
        if showing {
            self.hide_text_field_overlays(field);
        } else {
            self.show_text_field_touch_menu(field, position);
        }
    }
}
