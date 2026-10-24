//! Execution of Flutter text-editing intents against the focused text field.
//!
//! [`crate::text_shortcuts`] decides which intent a key chord means on the
//! target platform; this module is Flutter 3.41's `EditableTextState` action
//! table (`widgets/editable_text.dart`): `_DeleteTextAction`,
//! `_UpdateTextSelectionAction`, `_UpdateTextSelectionVerticallyAction`,
//! `_TransposeCharactersAction` and the clipboard actions.

use super::*;
use crate::text_shortcuts::{TextIntent, resolve};
use incular_text::{TextAffinity, TextBoundaryKind};

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

impl Runtime {
    /// Runs the text-editing intent that `event` is bound to on the focused
    /// field. Returns whether anything happened, in which case the caller
    /// resets the caret blink and requests a frame.
    pub(super) fn run_text_field_key(
        &mut self,
        field: ElementId,
        controller: &TextEditingController,
        event: &KeyboardEvent,
    ) -> bool {
        let editable = self.tree.text_field_is_editable(field);
        // Enter is not a text shortcut (Flutter maps it to
        // `DoNothingAndStopPropagationTextIntent` and relies on the IME's
        // newline/submit action); desktop key events have no IME action, so
        // the key itself performs the field's input action.
        if event.code == Code::Enter {
            if !editable {
                return false;
            }
            if self.tree.is_multiline_text_field(field) {
                controller.insert("\n");
            } else {
                self.tree.submit_text_field(field);
            }
            return true;
        }
        let Some(intent) = resolve(self.text_shortcut_platform, event.code, event.modifiers) else {
            return false;
        };
        self.run_text_intent(field, controller, intent, editable)
    }

    fn run_text_intent(
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
                self.clipboard.set_text(controller.selected_text());
                true
            }
            Cut => self.cut_text_field_selection(field, controller),
            Paste => {
                if !editable {
                    return false;
                }
                if let Some(text) = self.clipboard.get_text() {
                    controller.insert(&text);
                }
                true
            }
            SelectAll => {
                controller.select_all();
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
                controller.set_value(
                    TextEditingValue::new(transposed)
                        .with_selection(TextSelection::collapsed(caret)),
                );
                true
            }
            DeleteCharacter { forward } => {
                if !editable {
                    return false;
                }
                self.count_delete(forward);
                if selection.is_collapsed() {
                    let target = by_character(selection.base, forward);
                    self.delete_to(controller, target);
                } else {
                    controller.replace_selection("");
                }
                true
            }
            DeleteToNextWordBoundary { forward } => {
                if !editable {
                    return false;
                }
                self.count_delete(forward);
                if selection.is_collapsed() {
                    let target = word.move_beyond(text, selection.base, forward);
                    self.delete_to(controller, target);
                } else {
                    controller.replace_selection("");
                }
                true
            }
            DeleteToLineBreak { forward } => {
                if !editable {
                    return false;
                }
                self.count_delete(forward);
                if selection.is_collapsed() {
                    let (target, _) = self.line_boundary(
                        field,
                        controller,
                        selection.base,
                        forward,
                        false,
                        obscured,
                    );
                    self.delete_to(controller, target);
                } else {
                    controller.replace_selection("");
                }
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
    ) -> bool {
        use TextIntent::{
            Copy, ExpandToDocumentBoundary, ExpandToLineBreak, ExtendByCharacter,
            ExtendToDocumentBoundary, ExtendToLineBreak, ExtendToNextParagraphBoundary,
            ExtendToNextParagraphBoundaryOrCaretLocation, ExtendToNextWordBoundary,
            ExtendToNextWordBoundaryOrCaretLocation, ExtendVerticallyToAdjacentLine,
            ExtendVerticallyToAdjacentPage, SelectAll,
        };
        let Some(intent) = resolve(self.text_shortcut_platform, event.code, event.modifiers) else {
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

    /// Deletes between the collapsed caret and `target`.
    fn delete_to(&self, controller: &TextEditingController, target: usize) {
        controller.move_cursor(target, true);
        controller.replace_selection("");
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

    #[allow(clippy::too_many_arguments)]
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
            (controller.text().len(), TextAffinity::Upstream)
        } else {
            (0, TextAffinity::Downstream)
        }
    }
}
