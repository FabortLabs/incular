//! Execution of Flutter text-editing intents against the focused text field.
//!
//! [`crate::text_shortcuts`] decides which intent a key chord means on the
//! target platform; this module is Flutter 3.41's `EditableTextState` action
//! table (`widgets/editable_text.dart`): `_DeleteTextAction`,
//! `_UpdateTextSelectionAction`, `_UpdateTextSelectionVerticallyAction`,
//! `_TransposeCharactersAction` and the clipboard actions.

use super::*;
use crate::text_shortcuts::{TextIntent, resolve};
use incular_core::Modifiers;
use incular_platform::TransferFormat;
use incular_text::{SelectionChangedCause, TextAffinity, TextBoundaryKind};

/// The selection operation that attempted to access the system clipboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorClipboardOperation {
    Copy,
    Cut,
    Paste,
}

impl EditorClipboardOperation {
    #[must_use]
    pub const fn context(self) -> &'static str {
        match self {
            Self::Copy => "while copying selection to clipboard",
            Self::Cut => "while cutting selection to clipboard",
            Self::Paste => "while pasting text from clipboard",
        }
    }
}

/// A clipboard backend failure during an editor command or shortcut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorClipboardError {
    pub field: ElementId,
    pub operation: EditorClipboardOperation,
    pub error: ClipboardError,
}

/// `TextSelection.expandTo`.
fn expand_to(selection: TextSelection, position: usize, extent_at_index: bool) -> TextSelection {
    let (start, end) = (selection.start(), selection.end());
    if position >= start && position <= end {
        return selection;
    }
    let normalized = selection.base <= selection.extent;
    if position <= start {
        if extent_at_index {
            return TextSelection::new(end, position);
        }
        return if normalized {
            TextSelection::new(position, selection.extent)
        } else {
            TextSelection::new(selection.base, position)
        };
    }
    if extent_at_index {
        return TextSelection::new(start, position);
    }
    if normalized {
        TextSelection::new(selection.base, position)
    } else {
        TextSelection::new(position, selection.extent)
    }
}

/// The start of the code point before `offset` (`_CodePointBoundary`, which
/// obscured fields use instead of grapheme clusters).
fn code_point_before(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

/// The end of the code point at `offset`.
fn code_point_after(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    text[offset..]
        .chars()
        .next()
        .map_or(text.len(), |c| offset + c.len_utf8())
}

/// The editor deletes whole graphemes, except in obscured fields where the
/// atomic unit is a Unicode scalar. A word or line command may supply a
/// farther endpoint, but the near edge still covers the entire atomic unit.
fn deletion_range(
    text: &str,
    selection: TextSelection,
    obscured: bool,
    forward: bool,
    target: Option<usize>,
) -> TextRange {
    if !selection.is_collapsed() {
        if obscured {
            return selection.range();
        }
        let start = TextBoundaryKind::Character
            .leading_at(text, selection.start() as isize)
            .unwrap_or(text.len());
        let end = TextBoundaryKind::Character
            .trailing_at(text, selection.end() as isize - 1)
            .unwrap_or(text.len());
        return TextRange::new(start, end);
    }

    let caret = selection.extent;
    let (leading, trailing) = if obscured {
        if forward {
            (caret, code_point_after(text, caret))
        } else {
            (code_point_before(text, caret), caret)
        }
    } else {
        // A forward edit includes the unit at the caret; a backward edit
        // includes the unit at the previous byte, if any. TextBoundaryKind
        // maps an interior UTF-8 byte to its scalar before finding the unit.
        let position = if forward {
            caret as isize
        } else {
            caret as isize - 1
        };
        let (leading, trailing) = TextBoundaryKind::Character.at(text, position);
        (leading.unwrap_or(0), trailing.unwrap_or(text.len()))
    };
    match target {
        None => TextRange::new(leading, trailing),
        Some(target) if forward => TextRange::new(leading, target).normalized(),
        Some(target) => TextRange::new(target, trailing).normalized(),
    }
}

/// Whether a handled shortcut uses the common overlay cleanup or manages
/// the toolbar and handles separately as part of its own action.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ShortcutOverlays {
    HideAll,
    ManagedByIntent,
}

impl Runtime {
    /// Explicitly selects all text in a mounted editable field, like
    /// EditableTextState.selectAll. This is distinct from enabling a
    /// keyboard shortcut or showing the selection command in a toolbar.
    /// A read-only obscured field does not select or acquire focus.
    #[must_use]
    pub fn select_all_text(&mut self, field: ElementId, cause: SelectionChangedCause) -> bool {
        let Some(snapshot) = self.tree.text_field_input_geometry(field) else {
            return false;
        };
        if snapshot.read_only && snapshot.obscure_text {
            return false;
        }
        let Some(controller) = self.tree.text_controller(field) else {
            return false;
        };
        let previous = self.observed_editor_selection(field);
        controller.select_all();
        if self.focused != Some(field) {
            self.set_focus(Some(field));
        }
        self.report_editor_selection_delta(field, previous, cause);
        controller.reset_caret(self.input_now());
        if cause == SelectionChangedCause::Toolbar
            && matches!(
                self.text_shortcut_platform,
                TextShortcutPlatform::Linux
                    | TextShortcutPlatform::MacOs
                    | TextShortcutPlatform::Windows
            )
        {
            self.hide_text_field_overlays(field);
        }
        if cause == SelectionChangedCause::Toolbar
            && matches!(
                self.text_shortcut_platform,
                TextShortcutPlatform::Android
                    | TextShortcutPlatform::Linux
                    | TextShortcutPlatform::Windows
            )
        {
            self.caret_reveal_key = None;
        } else {
            self.preserve_selection_scroll(field, &controller);
        }
        self.frame_requested = true;
        true
    }

    fn preserve_selection_scroll(&mut self, field: ElementId, controller: &TextEditingController) {
        // The window's enclosing viewport and the field's own text viewport
        // have separate reveal keys. A deferred pass must not turn this
        // command into an ordinary change that reveals the selected end.
        if self.focused == Some(field) {
            self.caret_reveal_key = Some((field, controller.revisions().0, controller.selection()));
        }
        self.tree.preserve_editable_selection_scroll(field);
    }

    /// Copies a mounted editor's selection without invoking its keyboard
    /// shortcuts. Obscured text never reaches the clipboard.
    #[must_use]
    pub fn copy_text_selection(&mut self, field: ElementId, cause: SelectionChangedCause) -> bool {
        let Some(controller) = self.tree.text_controller(field) else {
            return false;
        };
        if !self.text_field_copy_allowed(field, &controller) {
            return false;
        }
        self.write_editor_clipboard(field, EditorClipboardOperation::Copy, controller.selected_text());
        if cause == SelectionChangedCause::Toolbar {
            self.tree.text_field_bring_position_into_view(
                field,
                controller.selection().extent,
                controller.caret_affinity(),
            );
            if let Some(toolbar) = self.tree.text_field_toolbar_state(field) {
                toolbar.hide();
            }
            if self.text_shortcut_platform == TextShortcutPlatform::Android {
                // Android and Fuchsia share this platform policy. Collapse
                // to the logical trailing edge, not the active endpoint of
                // a backward selection.
                let before = self.observed_editor_selection(field);
                let mut value = controller.value();
                value.selection = TextSelection::collapsed(value.selection.end());
                value.composing = None;
                controller.set_value(value);
                self.report_editor_selection_delta(field, before, cause);
                controller.reset_caret(self.input_now());
                self.hide_text_field_handles(field);
            }
            self.preserve_selection_scroll(field, &controller);
            self.frame_requested = true;
        }
        true
    }

    /// Cuts a mounted editor's selection directly. Read-only and obscured
    /// fields, and collapsed selections, leave text and clipboard unchanged.
    #[must_use]
    pub fn cut_text_selection(&mut self, field: ElementId, cause: SelectionChangedCause) -> bool {
        self.edit_selection_with_clipboard(field, cause, Self::cut_text_field_selection)
    }

    /// Pastes plain text into a mounted editor at its current valid selection.
    /// A missing or failed clipboard read leaves the editor untouched.
    #[must_use]
    pub fn paste_text_selection(&mut self, field: ElementId, cause: SelectionChangedCause) -> bool {
        self.edit_selection_with_clipboard(field, cause, |runtime, field, controller| {
            runtime.paste_from_editor_clipboard(field, controller)
        })
    }

    fn paste_from_editor_clipboard(
        &mut self,
        field: ElementId,
        controller: &TextEditingController,
    ) -> bool {
        if !self.tree.text_field_is_editable(field) || controller.selection_if_set().is_none() {
            return false;
        }
        let Some(text) = self.read_editor_clipboard(field) else {
            return false;
        };
        controller.insert(&text);
        true
    }

    fn edit_selection_with_clipboard(
        &mut self,
        field: ElementId,
        cause: SelectionChangedCause,
        edit: impl FnOnce(&mut Self, ElementId, &TextEditingController) -> bool,
    ) -> bool {
        let Some(controller) = self.tree.text_controller(field) else {
            return false;
        };
        let previous = self.observed_editor_selection(field);
        let _notifications = previous
            .is_some()
            .then(|| controller.batch_user_edit_notifications());
        if !edit(self, field, &controller) {
            return false;
        }
        self.report_editor_selection_delta(field, previous, cause);
        controller.reset_caret(self.input_now());
        if cause == SelectionChangedCause::Toolbar {
            self.hide_text_field_overlays(field);
            self.caret_reveal_key = None;
        } else {
            self.preserve_selection_scroll(field, &controller);
        }
        self.frame_requested = true;
        true
    }

    pub(super) fn write_editor_clipboard(
        &mut self,
        field: ElementId,
        operation: EditorClipboardOperation,
        text: String,
    ) {
        let write = self.clipboard.write(DataTransfer::plain_text(text)).and_then(|report| {
            if report.written.contains(&TransferFormat::PlainText) {
                Ok(())
            } else {
                Err(ClipboardError::UnsupportedFormat(TransferFormat::PlainText))
            }
        });
        if let Err(error) = write {
            self.report_editor_clipboard_error(EditorClipboardError { field, operation, error });
        }
    }

    fn read_editor_clipboard(&mut self, field: ElementId) -> Option<std::sync::Arc<str>> {
        let read = self.clipboard.read(TransferReadRequest::format(TransferFormat::PlainText))
            .and_then(|transfer| transfer.plain_text_value().map_err(ClipboardError::from));
        match read {
            Ok(text) => text,
            Err(ClipboardError::Unavailable | ClipboardError::UnsupportedFormat(TransferFormat::PlainText)) => None,
            Err(error) => {
                self.report_editor_clipboard_error(EditorClipboardError {
                    field,
                    operation: EditorClipboardOperation::Paste,
                    error,
                });
                None
            }
        }
    }

    /// Applies a user replacement to a mounted, editable field using the
    /// current text, like Flutter's `ReplaceTextIntent`. Rust ranges use
    /// UTF-8 bytes. An identical replacement still focuses an unfocused
    /// field and requests caret reveal, but does not re-run its formatters.
    #[must_use]
    pub fn replace_text(
        &mut self,
        field: ElementId,
        range: TextRange,
        replacement: &str,
        cause: SelectionChangedCause,
    ) -> bool {
        if !self.tree.text_field_is_editable(field) {
            return false;
        }
        let Some(controller) = self.tree.text_controller(field) else {
            return false;
        };
        let before = (
            controller.selection_if_set(),
            controller.caret_affinity(),
            controller.selection_is_directional(),
        );
        let Ok(was_changed) = controller.replace_range_as_user_edit(range, replacement) else {
            return false;
        };
        let after = (
            controller.selection_if_set(),
            controller.caret_affinity(),
            controller.selection_is_directional(),
        );
        if self.focused != Some(field)
            && (!was_changed || (before != after && cause != SelectionChangedCause::Keyboard))
        {
            // Flutter requestKeyboard marks this as internal focus; the
            // desktop select-all-on-external-focus default must not win.
            self.set_focus_with_external_selection(Some(field), false);
        }
        if self.focused == Some(field) && cause != SelectionChangedCause::Drag {
            controller.reset_caret(self.input_now());
            self.caret_reveal_key = None;
        }
        self.frame_requested = true;
        true
    }

    /// Runs the text-editing intent that `event` is bound to on the focused
    /// field. A handled shortcut returns its overlay policy; the caller
    /// resets the caret blink and requests a frame.
    pub(super) fn run_text_field_key(
        &mut self,
        field: ElementId,
        controller: &TextEditingController,
        event: &KeyboardEvent,
        effective_modifiers: Modifiers,
    ) -> Option<ShortcutOverlays> {
        let editable = self.tree.text_field_is_editable(field);
        // Enter is not a text shortcut (Flutter maps it to
        // `DoNothingAndStopPropagationTextIntent` and relies on the IME's
        // newline/submit action); desktop key events have no IME action, so
        // the key itself performs the field's input action.
        if event.code == Code::Enter {
            if !editable {
                return None;
            }
            if matches!(
                self.text_shortcut_platform,
                TextShortcutPlatform::Android | TextShortcutPlatform::Ios
            ) {
                return Some(ShortcutOverlays::HideAll);
            }
            // Desktop raw keys have no second text-input action. Insert
            // here, not in the native multiline newline notification.
            if self.tree.is_multiline_text_field(field) {
                controller.insert("\n");
            } else {
                self.handle_text_input_action(TextInputAction::Newline);
            }
            return Some(ShortcutOverlays::HideAll);
        }
        let intent = resolve(self.text_shortcut_platform, event.code, effective_modifiers)?;
        self.run_text_shortcut_intent(field, controller, intent, editable)
    }

    /// Selection actions from keyboard shortcuts (including platform
    /// selectors) stay separate from explicit editor and toolbar commands.
    pub(super) fn run_text_shortcut_intent(
        &mut self,
        field: ElementId,
        controller: &TextEditingController,
        intent: TextIntent,
        editable: bool,
    ) -> Option<ShortcutOverlays> {
        if matches!(
            intent,
            TextIntent::Copy | TextIntent::Cut | TextIntent::Paste
        ) && (!self.tree.text_field_keyboard_selection_enabled(field)
            || controller.selection_if_set().is_none())
        {
            return None;
        }
        let overlays = match intent {
            TextIntent::DeleteCharacter { .. }
            | TextIntent::DeleteToNextWordBoundary { .. }
            | TextIntent::DeleteToLineBreak { .. } => ShortcutOverlays::ManagedByIntent,
            _ => ShortcutOverlays::HideAll,
        };
        self.run_text_intent(field, controller, intent, editable)
            .then_some(overlays)
    }

    pub(super) fn run_text_intent(
        &mut self,
        field: ElementId,
        controller: &TextEditingController,
        intent: TextIntent,
        editable: bool,
    ) -> bool {
        use TextIntent::{
            Copy, Cut, DeleteCharacter, DeleteToLineBreak, DeleteToNextWordBoundary, DoNothing,
            ExpandToDocumentBoundary, ExpandToLineBreak, ExtendByCharacter,
            ExtendToDocumentBoundary, ExtendToLineBreak, ExtendToNextParagraphBoundary,
            ExtendToNextParagraphBoundaryOrCaretLocation, ExtendToNextWordBoundary,
            ExtendToNextWordBoundaryOrCaretLocation, ExtendVerticallyToAdjacentLine,
            ExtendVerticallyToAdjacentPage, Paste, Redo, ScrollPage, ScrollToDocumentBoundary,
            SelectAll, TransposeCharacters, Undo,
        };
        let value = controller.value();
        let text = value.text.as_str();
        let selection = value.selection;
        let obscured = self.text_field_obscured(field);
        // `_nextWordBoundary` and `_linebreak` treat obscured text as a single
        // word and line; characters step by code point.
        let word = if obscured {
            TextBoundaryKind::Document
        } else {
            TextBoundaryKind::Word
        };
        let by_character = |offset: usize, forward: bool| {
            if !obscured {
                TextBoundaryKind::Character.move_beyond(text, offset, forward)
            } else if forward {
                code_point_after(text, offset)
            } else {
                code_point_before(text, offset)
            }
        };
        match intent {
            Copy => {
                if !self.text_field_copy_allowed(field, controller) {
                    return false;
                }
                self.write_editor_clipboard(field, EditorClipboardOperation::Copy, controller.selected_text());
                true
            }
            Cut => {
                if !self.cut_text_field_selection(field, controller) {
                    return false;
                }
                self.preserve_selection_scroll(field, controller);
                true
            }
            Paste => {
                if !editable || controller.selection_if_set().is_none() {
                    return false;
                }
                if self.paste_from_editor_clipboard(field, controller) {
                    self.preserve_selection_scroll(field, controller);
                }
                true
            }
            SelectAll => {
                if !self.tree.text_field_keyboard_selection_enabled(field) {
                    return false;
                }
                controller.select_all();
                self.preserve_selection_scroll(field, controller);
                true
            }
            Undo | Redo => {
                let Some(history) = self.ensure_text_history(field) else {
                    return false;
                };
                if matches!(intent, Redo) {
                    history.redo()
                } else {
                    history.undo()
                }
            }
            DoNothing => false,
            // The scroll intents move the field's viewport without touching
            // the selection.
            ScrollToDocumentBoundary { forward } => self
                .tree
                .text_field_scroll_to_document_boundary(field, forward),
            ScrollPage { forward } => self.tree.text_field_scroll_page(field, forward),
            TransposeCharacters => {
                if !editable
                    || !selection.is_collapsed()
                    || selection.base == 0
                    || TextBoundaryKind::Character.move_beyond(text, 0, true) >= text.len()
                {
                    return false;
                }
                let base = selection.base;
                let (first_start, split, second_end) = if base >= text.len() {
                    let split = TextBoundaryKind::Character.move_beyond(text, base, false);
                    (
                        TextBoundaryKind::Character.move_beyond(text, split, false),
                        split,
                        text.len(),
                    )
                } else {
                    (
                        TextBoundaryKind::Character.move_beyond(text, base, false),
                        base,
                        TextBoundaryKind::Character.move_beyond(text, base, true),
                    )
                };
                let mut transposed = String::with_capacity(text.len());
                transposed.push_str(&text[..first_start]);
                transposed.push_str(&text[split..second_end]);
                transposed.push_str(&text[first_start..split]);
                transposed.push_str(&text[second_end..]);
                let caret = first_start + (second_end - first_start);
                // Transpose is a user edit (Flutter's TransposeCharactersTextIntent
                // goes through userUpdateTextEditingValue), so it runs formatters.
                controller.apply_delta(&incular_text::TextEditingDelta {
                    range: TextRange::new(0, text.len()),
                    replacement: transposed,
                    selection: Some(TextSelection::collapsed(caret)),
                    composing: None,
                });
                true
            }
            DeleteCharacter { forward }
            | DeleteToNextWordBoundary { forward }
            | DeleteToLineBreak { forward } => {
                if !editable || controller.selection_if_set().is_none() {
                    return false;
                }
                self.count_delete(forward);
                let target = if selection.is_collapsed() {
                    match intent {
                        DeleteToNextWordBoundary { .. } => {
                            Some(word.move_beyond(text, selection.base, forward))
                        }
                        DeleteToLineBreak { .. } => Some(
                            self.line_boundary(
                                field,
                                controller,
                                selection.base,
                                forward,
                                false,
                                obscured,
                            )
                            .0,
                        ),
                        _ => None,
                    }
                } else {
                    None
                };
                let range = deletion_range(text, selection, obscured, forward, target);
                if range.start != range.end {
                    // _DeleteTextAction hides only the toolbar, and only
                    // when the proposed replacement actually removes text.
                    // Selection geometry updates the handles independently.
                    self.hide_text_field_toolbar_only(field);
                }
                // One replacement leaves empty ranges alone and does not
                // publish a temporary expanded selection to observers.
                let _ = controller.replace_range_as_user_edit(range, "");
                true
            }
            ExtendByCharacter { forward, collapse } => {
                if !selection.is_collapsed() && collapse {
                    controller.set_selection(TextSelection::collapsed(if forward {
                        selection.end()
                    } else {
                        selection.start()
                    }));
                } else if obscured {
                    controller.move_cursor(by_character(selection.extent, forward), !collapse);
                } else if forward {
                    // Flutter's `DirectionalCaretMovementIntent`s move in the
                    // logical order of the string, whatever its writing
                    // direction: Right is forward, even in right-to-left text.
                    controller.move_right(!collapse);
                } else {
                    controller.move_left(!collapse);
                }
                true
            }
            ExtendToNextWordBoundary { forward, collapse } => {
                let target = word.move_beyond(text, selection.extent, forward);
                controller.move_cursor(target, !collapse);
                true
            }
            ExtendToNextParagraphBoundary { forward, collapse } => {
                let target =
                    TextBoundaryKind::Paragraph.move_beyond(text, selection.extent, forward);
                controller.move_cursor(target, !collapse);
                true
            }
            ExtendToDocumentBoundary { forward, collapse } => {
                let target =
                    TextBoundaryKind::Document.move_beyond(text, selection.extent, forward);
                controller.move_cursor(target, !collapse);
                true
            }
            ExtendToNextWordBoundaryOrCaretLocation { forward } => {
                let target = word.move_beyond(text, selection.extent, forward);
                Self::extend_or_collapse_at_reversal(controller, selection, target);
                true
            }
            ExtendToNextParagraphBoundaryOrCaretLocation { forward } => {
                let target =
                    TextBoundaryKind::Paragraph.move_beyond(text, selection.extent, forward);
                Self::extend_or_collapse_at_reversal(controller, selection, target);
                true
            }
            ExtendToLineBreak {
                forward,
                collapse,
                continues_at_wrap,
            } => {
                let (target, affinity) = self.line_boundary(
                    field,
                    controller,
                    selection.extent,
                    forward,
                    continues_at_wrap,
                    obscured,
                );
                controller.move_cursor_with_affinity(target, affinity, !collapse);
                true
            }
            ExpandToLineBreak { forward } => {
                let from = Self::expand_origin(selection, forward);
                let affinity = if from == selection.base {
                    TextAffinity::Downstream
                } else {
                    controller.caret_affinity()
                };
                let (target, affinity) = self
                    .line_boundary_at(field, controller, from, affinity, forward, false, obscured);
                let expanded = expand_to(selection, target, selection.is_collapsed());
                if expanded != selection {
                    controller.set_selection_with_affinity(expanded, affinity);
                }
                true
            }
            ExpandToDocumentBoundary { forward } => {
                // `_moveToTextBoundary` over the document: its trailing and
                // leading boundaries are the text's ends.
                let target = if forward { text.len() } else { 0 };
                let affinity = if forward {
                    TextAffinity::Upstream
                } else {
                    TextAffinity::Downstream
                };
                let expanded = expand_to(selection, target, true);
                if expanded != selection {
                    controller.set_selection_with_affinity(expanded, affinity);
                }
                true
            }
            ExtendVerticallyToAdjacentLine { forward, collapse } => {
                if !self
                    .tree
                    .text_field_move_vertical(field, forward, !collapse)
                {
                    // No adjacent line (a single-line field, or the first or
                    // last line): the caret goes to the document edge.
                    controller.move_cursor(if forward { text.len() } else { 0 }, !collapse);
                }
                true
            }
            ExtendVerticallyToAdjacentPage { forward, collapse } => {
                let lines = self.tree.text_field_page_lines(field);
                if !self
                    .tree
                    .text_field_move_lines(field, forward, lines, !collapse)
                {
                    controller.move_cursor(if forward { text.len() } else { 0 }, !collapse);
                }
                true
            }
        }
    }

    /// Runs the key's text-editing intent on a focused `SelectableText`.
    /// Flutter's `SelectableText` is a read-only `EditableText`, so it shares
    /// the platform's `DefaultTextEditingShortcuts`; the editing intents
    /// (delete, cut, paste, undo, transpose) are disabled and stay unhandled.
    pub(super) fn run_selectable_text_key(
        &mut self,
        label: ElementId,
        event: &KeyboardEvent,
        effective_modifiers: Modifiers,
    ) -> bool {
        use TextIntent::{
            Copy, ExpandToDocumentBoundary, ExpandToLineBreak, ExtendByCharacter,
            ExtendToDocumentBoundary, ExtendToLineBreak, ExtendToNextParagraphBoundary,
            ExtendToNextParagraphBoundaryOrCaretLocation, ExtendToNextWordBoundary,
            ExtendToNextWordBoundaryOrCaretLocation, ExtendVerticallyToAdjacentLine,
            ExtendVerticallyToAdjacentPage, SelectAll,
        };
        let Some(intent) = resolve(self.text_shortcut_platform, event.code, effective_modifiers) else {
            return false;
        };
        let Some((text, selection, foreign_anchor)) =
            self.tree.selectable_text_editing_value(label)
        else {
            return false;
        };
        let text = text.as_str();
        let moved = |target: usize, extend: bool| {
            if extend {
                TextSelection::new(selection.base, target)
            } else {
                TextSelection::collapsed(target)
            }
        };
        // `_extendSelectionOrCollapseAtReversal`.
        let or_collapse_at_reversal = |target: usize| {
            let base = selection.base as i64;
            if (base - selection.extent as i64) * (base - target as i64) < 0 {
                TextSelection::collapsed(selection.base)
            } else {
                TextSelection::new(selection.base, target)
            }
        };
        let next = match intent {
            Copy => {
                return match self.tree.selectable_text_selected_text(label) {
                    Some(selected) if !selected.is_empty() => {
                        self.clipboard.set_text(selected);
                        true
                    }
                    _ => false,
                };
            }
            SelectAll => return self.tree.selectable_text_select_all(label),
            ExtendByCharacter { forward, collapse } => {
                if !selection.is_collapsed() && collapse {
                    TextSelection::collapsed(if forward {
                        selection.end()
                    } else {
                        selection.start()
                    })
                } else {
                    let target =
                        TextBoundaryKind::Character.move_beyond(text, selection.extent, forward);
                    moved(target, !collapse)
                }
            }
            ExtendToNextWordBoundary { forward, collapse } => moved(
                TextBoundaryKind::Word.move_beyond(text, selection.extent, forward),
                !collapse,
            ),
            ExtendToNextWordBoundaryOrCaretLocation { forward } => or_collapse_at_reversal(
                TextBoundaryKind::Word.move_beyond(text, selection.extent, forward),
            ),
            ExtendToNextParagraphBoundary { forward, collapse } => moved(
                TextBoundaryKind::Paragraph.move_beyond(text, selection.extent, forward),
                !collapse,
            ),
            ExtendToNextParagraphBoundaryOrCaretLocation { forward } => or_collapse_at_reversal(
                TextBoundaryKind::Paragraph.move_beyond(text, selection.extent, forward),
            ),
            ExtendToDocumentBoundary { forward, collapse } => {
                moved(if forward { text.len() } else { 0 }, !collapse)
            }
            ExpandToDocumentBoundary { forward } => {
                expand_to(selection, if forward { text.len() } else { 0 }, true)
            }
            ExtendToLineBreak {
                forward,
                collapse,
                continues_at_wrap,
            } => {
                let target = self
                    .tree
                    .selectable_text_line_boundary(
                        label,
                        selection.extent,
                        forward,
                        continues_at_wrap,
                    )
                    .unwrap_or(if forward { text.len() } else { 0 });
                moved(target, !collapse)
            }
            ExpandToLineBreak { forward } => {
                let from = Self::expand_origin(selection, forward);
                let target = self
                    .tree
                    .selectable_text_line_boundary(label, from, forward, false)
                    .unwrap_or(if forward { text.len() } else { 0 });
                expand_to(selection, target, selection.is_collapsed())
            }
            ExtendVerticallyToAdjacentLine { forward, collapse } => {
                // No adjacent line: the caret goes to the document edge.
                let target = self
                    .tree
                    .selectable_text_adjacent_line(label, selection.extent, forward)
                    .unwrap_or(if forward { text.len() } else { 0 });
                moved(target, !collapse)
            }
            ExtendVerticallyToAdjacentPage { forward, collapse } => {
                // The text is laid out whole, so a page reaches its edge.
                moved(if forward { text.len() } else { 0 }, !collapse)
            }
            _ => return false,
        };
        if next == selection {
            return true;
        }
        if foreign_anchor && next.base == selection.base && !next.is_collapsed() {
            // Extending a selection that started in another text of the
            // area keeps its anchor there.
            return self.tree.selectable_text_set_extent(label, next.extent);
        }
        self.tree.selectable_text_select(label, next)
    }

    fn count_delete(&mut self, forward: bool) {
        if forward {
            self.editing_diagnostics.delete_commands += 1;
        } else {
            self.editing_diagnostics.backspace_commands += 1;
        }
    }

    /// `collapseAtReversal`: extends the selection to `target`, unless that
    /// would swing the extent across the base, which collapses at the base.
    fn extend_or_collapse_at_reversal(
        controller: &TextEditingController,
        selection: TextSelection,
        target: usize,
    ) {
        let base = selection.base as i64;
        if (base - selection.extent as i64) * (base - target as i64) < 0 {
            controller.set_selection(TextSelection::collapsed(selection.base));
        } else {
            controller.move_cursor(target, true);
        }
    }

    /// `shouldTargetBase`: expanding forward over a backward selection (or
    /// backward over a forward one) moves the base instead of the extent.
    fn expand_origin(selection: TextSelection, forward: bool) -> usize {
        let target_base = if forward {
            selection.base > selection.extent
        } else {
            selection.base < selection.extent
        };
        if target_base {
            selection.base
        } else {
            selection.extent
        }
    }

    /// `_moveToTextBoundary` over `_linebreak`: the line edge nearest `offset`
    /// at the caret's own affinity. Obscured text and fields without a layout
    /// use the document boundary.
    fn line_boundary(
        &self,
        field: ElementId,
        controller: &TextEditingController,
        offset: usize,
        forward: bool,
        continues_at_wrap: bool,
        obscured: bool,
    ) -> (usize, TextAffinity) {
        self.line_boundary_at(
            field,
            controller,
            offset,
            controller.caret_affinity(),
            forward,
            continues_at_wrap,
            obscured,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the line-boundary query needs the caret offset, affinity, direction, wrap flag and obscured state"
    )]
    fn line_boundary_at(
        &self,
        field: ElementId,
        controller: &TextEditingController,
        offset: usize,
        affinity: TextAffinity,
        forward: bool,
        continues_at_wrap: bool,
        obscured: bool,
    ) -> (usize, TextAffinity) {
        if !obscured
            && let Some(boundary) = self.tree.text_field_line_boundary(
                field,
                offset,
                affinity,
                forward,
                continues_at_wrap,
            )
        {
            return boundary;
        }
        if forward {
            (
                controller.with_value(|value| value.text.len()),
                TextAffinity::Upstream,
            )
        } else {
            (0, TextAffinity::Downstream)
        }
    }
}
