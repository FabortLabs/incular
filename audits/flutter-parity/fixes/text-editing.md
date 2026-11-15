# text-editing fixes

## text-006: fixed
- Outcome: fixed (confirmed by source; scenario twin expected to match).
- Cause: edit transforms and on_changed hung off the controller commit path, so set_text/set_value ran formatters and user callbacks.
- Change: editing.rs routes only editing operations through transforms; set_value/set_text are stored as given. observe_user_edits added. reconciliation.rs subscribes on_changed to user edits and drops mount-time normalization.
- Tests: incular-text/tests/editing.rs (two tests); widgets editable_text_interaction.rs (formatter order test).
- Harness: text__editable_programmatic_controller_update not rerun (shared build blocked).
- Limitation: undo/redo now bypass on_changed and transforms, as Flutter's UndoHistory does.

## text-009: fixed
- Outcome: fixed.
- Cause: TextInputFormatter existed but EditableText had no input formatter builder.
- Change: EditableText::input_formatter appends formatters; compose_edit_transform folds them in order on user edits (descriptors.rs).
- Tests: editable_text_interaction.rs::input_formatters_apply_in_order_to_user_edits_only.
- Harness: text__editable_formatter_length_then_digits / digits_then_length not rerun.

## text-015: fixed (behavior change to confirm)
- Outcome: fixed.
- Cause: no onEditingComplete; Enter called submit directly.
- Change: on_editing_complete builder; submit_text_field runs it before on_submit; frame.rs finish_text_editing releases focus when no completion callback is set; desktop Enter routes through the Newline action.
- Tests: editable_text_interaction.rs (order, presence); existing runtime text_input_gating.rs Enter test expected to still pass.
- Behavior change: a single-line desktop Enter now unfocuses by default, as Flutter's default does. Please confirm.

## text-008: fixed
- Caret paint in content.rs now requires a collapsed selection. Source-only; no test (not visible to the frame recorder).

## text-005: deferred
- Flutter's unset selection (-1) cannot be represented: TextSelection uses usize fields that are public. Needs a public type decision.

**Text residual pass: still deferred, representation decided.** The plan keeps `TextSelection` unchanged and adds an unset flag to the controller state in `editing.rs`. `with_text` starts unset, and a new `selection_if_set() -> Option<TextSelection>` returns None until any selection or edit is applied. `selection()` keeps its current value for existing callers. EditableText focus applies Flutter's collapsed-at-end selection while the flag is unset, and the twins read `selection_if_set`. The work was deferred because `editing.rs` is outside this group's files.

## text-010: blocked
- Checked: Material text fields and controls text_input.rs depend on the built-in inset (they pass size and rely on intrinsic height). Not changed here; the inset should move to those wrappers first.
- The 16 px is EditableText's built-in 8 px vertical inset, not an extra line. Removing it changes Material text-field visuals and scroll viewports, so it needs a layout-contract decision.

**Text residual pass: fixed.** The inset moved out of the editor. The new `EditableText::content_padding(EdgeInsets)` defaults to zero, as in Flutter, and drives the text width, height, caret, selection, viewport, pointer mapping and IME rect; the 32 px floor is gone from the editor. The controls and Material `TextField` set an 8 px padding and wrap the editor in a 32 px minimum-height `ConstrainedBox`, so their visuals are unchanged; the Material search bar sets the padding too. `selection_rects` also lost a stray +8 px that shifted static selection highlights. `text__editable_max_lines_sizing` and `text__editable_min_lines_sizing` now match. Tests: `editable_text.rs::the_bare_editor_is_its_lines_tall_and_content_padding_adds`; the geometry tests in `editable_text_{alignment,bidi,direction,paragraph_layout}.rs` now pass an explicit 8 px padding; `text_field_scroll_keys.rs` uses a 96 px page. The controls and Material text-field tests pass.

## text-016: blocked
- Not confirmed. Needs a harness run once the build is green.

**Text residual pass: fixed.** The cause was confirmed by a harness run and has two parts. First, the runtime restarted the blink with the wall clock (`Instant::now()` in `set_focus`, `reset_caret` calls and the tap time) instead of the input event's time; `frame.rs` and `text_gestures.rs` now use `input_now()`. Second, the Flutter twin read `renderEditable.showCursor`, which starts true before focus (editable_text.dart initState) although no caret is painted. The twin now reads `cursorCurrentlyVisible`. `text__editable_caret_blink_1200ms` matches: on at focus, off at 528 ms and on at 1024 ms on both sides.
