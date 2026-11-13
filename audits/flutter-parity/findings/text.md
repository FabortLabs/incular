# text: Text and editing

Revalidated at HEAD `a19bf487`. The frame suite `harness/run-frames.sh text__` ran 23 text scenarios; 5 match. The report also includes `transitions__text__default_style` from another package, which is excluded from these counts. The findings are in `findings/text.jsonl` (15 entries, all `pending-review`).

## Summary

- Earlier findings text-001 to text-008 still reproduce or are still supported by source at HEAD.
- New matches after fixes: EditableText word movement (ctrl+arrow, shift extension), Home/End with shift, Backspace and Delete, read-only typing, expands sizing, Text softWrap false, and the linear 1.5 scaled text (diagnostic).
- New divergences, now findings: formatters never applied (text-009, P1), extra line height in maxLines and minLines sizing (text-010), height behavior drift of 0.06 px (text-011), Text lacks textScaler (text-012) and strutStyle (text-013), SelectionArea long press and selection callback (text-014, P1), onEditingComplete missing (text-015).
- Open, not a finding: undo. Incular restores and redoes; Flutter does not restore within the same timeline, even after a 600 ms pump past the 500 ms UndoHistory throttle. The cause is unresolved, so no match or divergence is claimed.
- Caret blink timing and textAlign remain unverified (see Harness requests).

## Coverage (summary; per-row statuses are in `coverage/text.jsonl`)

| Area | Method | Scenarios | Result |
| --- | --- | --- | --- |
| Text maxLines and ellipsis | scenario | `text__text_max_lines_ellipsis` | match |
| Text softWrap false | scenario | `text__text_soft_wrap_false` | match |
| Text textScaler | diagnostic scenario | `text__text_scaler_linear` | layout match; API missing (text-012) |
| Text textHeightBehavior | scenario | `text__text_height_behavior_first_ascent` | 0.06 px drift (text-011) |
| Text strutStyle | source | none | missing (text-013) |
| Text textAlign (all modes) | not frame-checked | none | no public line-box or glyph-start accessor on either side; pixel request |
| RichText styled runs and WidgetSpan | scenario | `text__rich_span_style_metrics`, `text__widget_span_layout` | divergent (text-001, text-003) |
| DefaultTextStyle maxLines | diagnostic | `text__default_text_style_max_lines` | non-equivalent, missing API (text-004) |
| EditableText focus and initial selection | scenario | `text__editable_initial_selection_focus` | divergent before focus (text-005) |
| EditableText programmatic update | scenario | `text__editable_programmatic_controller_update` | divergent (text-006) |
| EditableText long press | scenario | `text__editable_long_press_word_selection` | divergent (text-007) |
| EditableText word movement, shift extension | scenario | `text__editable_word_arrow_selection` | match after focus |
| EditableText Home, End, shift select | scenario | `text__editable_home_end_shift_select` | match after focus |
| EditableText Backspace, Delete | scenario | `text__editable_backspace_delete` | match after focus |
| EditableText undo and redo | scenario | `text__editable_undo_redo` | open question (see Summary) |
| EditableText read-only | scenario | `text__editable_read_only_typing` | match |
| EditableText maxLines, minLines sizing | scenario | `text__editable_max_lines_sizing`, `text__editable_min_lines_sizing` | divergent, +16 px (text-010) |
| EditableText expands | scenario | `text__editable_expands_sizing` | match |
| EditableText onChanged, onSubmitted, onEditingComplete | scenario | `text__editable_enter_callback_order` | onEditingComplete missing (text-015); onChanged matches |
| Input formatters (LengthLimiting, digitsOnly, both orders) | scenario | `text__editable_formatter_length_then_digits`, `text__editable_formatter_digits_then_length` | divergent (text-009) |
| Caret paint during range selection | source | none | divergent (text-008) |
| Caret blink | not frame-checked | none | see Harness requests |
| obscureText | not frame-checked | none | pixel request |
| SelectionArea long press and selection callback | scenario | `text__selection_area_long_press_drag` | divergent (text-014) |
| SelectableRegion protocol (about 40 declarations) | reading or missing per row | none | see coverage rows |

## Findings

Full details (evidence, cause, fix direction) are in `findings/text.jsonl`.

- text-001 (P1, layout): RichText drops styles after the first run. Scenario `text__rich_span_style_metrics`: 220x36 vs 220x17.
- text-002 (P2, api-numeric): Editing offsets are UTF-8 bytes. Scenario `text__editing_selection_offsets`: 3 vs 5 for emoji plus x.
- text-003 (P2, layout): WidgetSpan child is not laid out or painted. Scenario `text__widget_span_layout`: 220x40 vs 220x19.
- text-004 (P2, missing-api): No inherited DefaultTextStyle maxLines (diagnostic only).
- text-005 (P2, edge-case): Controller selection before focus: Flutter -1, Incular valid caret.
- text-006 (P0, wrong-final-state): Transforms and onChanged run on programmatic updates. Scenario `text__editable_programmatic_controller_update`: `a1b2` vs `12`, 0 vs 1 callback.
- text-007 (P1, missing-feature): EditableText long press does not select a word. Scenario `text__editable_long_press_word_selection`: [0,5] vs collapsed at 1.
- text-008 (P2, edge-case, source): Caret painted during range selection.
- text-009 (P1, missing-feature): EditableText never applies input formatters. Scenarios `text__editable_formatter_length_then_digits` (1 vs 8) and `text__editable_formatter_digits_then_length` (3 vs 8).
- text-010 (P2, layout): maxLines and minLines heights are one line (16 px) taller. Scenarios `text__editable_max_lines_sizing` (51 vs 67) and `text__editable_min_lines_sizing` (34 vs 50). Cause not isolated.
- text-011 (P3, api-numeric): textHeightBehavior with height 2 measures 19.0 vs 18.94. Scenario `text__text_height_behavior_first_ascent`.
- text-012 (P2, missing-api): Text has no textScaler. Diagnostic `text__text_scaler_linear` matches layout at a scaled size, so only the API is missing.
- text-013 (P2, missing-api, source): Text has no strutStyle; StrutStyle exists only as a type.
- text-014 (P1, missing-feature): SelectionArea does not select on long press and has no selection callback. Scenario `text__selection_area_long_press_drag`: Flutter 5 vs Incular 0; Flutter onSelectionChanged fires, Incular has no callback. Same root as text-007, for SelectionArea.
- text-015 (P2, missing-api): EditableText has no onEditingComplete. Scenario `text__editable_enter_callback_order`; the Enter-key submit path also differs (Flutter's test key does not submit; the IME action does).

Deduplication: support-003 (undo manager integration) and support-004 (spell check) are separate from the findings above. The undo open question is not a support-003 duplicate because it concerns the in-widget UndoHistory replay, not the platform undo manager.

## Harness requests

- Caret blink (text-008 and blink timing): compare visibility per frame with `h.value`. Flutter reads `EditableTextState.renderEditable.showCursor.value`. Incular's `caret_visible(now)` and `caret_opacity(now, animates)` (`crates/incular-text/src/editing.rs:670-680`) take a `now` that is the wall clock in the painter; a value closure cannot read the harness frame clock, so the Incular side needs the fake clock exposed to value closures. Until then the blink phase is not compared.
- textAlign: no public line-box or glyph-start accessor exists on either side. Pixel scene spec: one `Text` at width 200 under ltr, Inter 14 black, with three lines `ab`, `abcdefgh`, `abcdefghijkl` and textAlign center, end, justify, start; capture at 1x; compare line starts.
- obscureText: pixel scene: an EditableText with obscureText true and text `secret` at 200x48, unfocused, to compare the dot glyph run width.
- Mouse drag selection: the recorder has no mouse-button down and up. Both sides send touch via `h.down`. A `mouseDown`/`mouseUp` method on both sides is needed to test the desktop drag path.
- Undo timing: confirm the Flutter UndoHistory push timing against the harness timeline before comparing undo.

## Open questions

- Undo: Flutter does not restore in the frame timeline (see Summary). Verify whether the Flutter UndoHistory push occurs with `tester.pump` at this timeline or needs an explicit undo-stack flush.
- Tap caret placement: in `text__editable_word_arrow_selection`, a tap at x=100 (beyond the end of `hello big world`) places Flutter's caret at 15 and Incular's at 14. This is not yet a finding; check the hit test for taps past the last glyph.
- Enter key: Flutter's test key does not submit a single-line field; Incular's does. Decide the intended desktop behavior before a fix.
- Untested areas: plain Text textAlign (all modes), textScaler on the system scale, strutStyle line metrics, multiline caret up and down, paragraph and document boundaries, IME delta protocol, ContextMenuController and toolbar layout, magnifier.

## Validation run

- `harness/run-frames.sh text__` (FLUTTER set to `C:/Manual Installs/flutter/bin/flutter`): exit 0. Report `harness/out/frames/report-text__.md`: 23 text scenarios, 5 match; the report's total of 24 includes `transitions__text__default_style` from another package.
- `flutter analyze test/frames/text_test.dart`: no issues.
- `cargo check -q -p incular-parity-harness --tests`: passes. Two unrelated warnings from `tests/frames/host.rs` (another worker's file).
- `rustfmt --edition 2024 harness/incular/tests/frames/text.rs`: applied.
- `pwsh -NoProfile -File audits/flutter-parity/inventory_tool/render-inventory.ps1`: `text: total 169, unchecked 17`.
- JSON validation: `findings/text.jsonl` (15 lines) and `coverage/text.jsonl` (169 lines) parse.

## Twin corrections made in this pass

- Flutter keyboard twins lacked `DefaultTextEditingShortcuts`; added around the EditableText field, as the editing scenario does.
- Flutter SelectionArea needs Localizations, Overlay, and a Material localization delegate; added in the twin.
- Incular Column defaulted to center cross alignment; the twin now sets start alignment to match Flutter.
- Incular undo twin pumps 600 ms before ctrl+z to match Flutter's UndoHistory throttle.
- Input: `h.down` sends a touch pointer on both sides (`harness/flutter/test/frames/recorder.dart:139-141`, `harness/incular/src/lib.rs:283`). The earlier note that the harness used a mouse pointer was wrong; there is no pointer-kind mismatch.

## Final pass (caret blink, multiline, undo, unchecked rows)

Harness: `harness/run-frames.sh text__` (report `harness/out/frames/report-text__.md`). Dart `flutter analyze` on `harness/flutter/test/frames/text_test.dart`: no issues. `cargo check -q -p incular-parity-harness --tests`: exit 0. `inventory_tool/check-findings.ps1`: prints nothing.

- **Caret blink (text__editable_caret_blink_1200ms):** Flutter `showCursor` and Incular `caret_visible(clock.get())` both blink at a 500 ms half-period. Incular is hidden for the first ~16 ms after focus and its edges trail Flutter by one frame (text-016, pending-review). Text-008 evidence is unchanged.
- **Multiline vertical and document (text__editable_multiline_vertical_document):** ArrowDown, ArrowDown, ArrowUp, Ctrl+End and Ctrl+Home match Flutter from frame 2 (offsets 9, 1, 5, 12, 5, 12, 0 on both sides). The only divergence is the pre-focus selection (text-018, pending-review). VerticalCaretMovementRun and DocumentBoundary are now partial-scenario. ParagraphBoundary is reading only.
- **Undo (text__editable_undo_no_programmatic_selection):** Divergence persists without a programmatic selection and with a 600 ms settle, so the throttle and selection-set hypotheses are ruled out. Flutter does not restore; Incular does (text-017, pending-review, cause unknown).
- **Unchecked rows:** All 17 now have a final status with path:line evidence in `coverage/text.jsonl`: 7 missing, 5 reading, 2 partial-scenario, 3 out-of-scope (TextInput, TextInputClient, TextInputType as platform IME plumbing).
