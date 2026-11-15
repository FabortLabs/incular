# Fixes: text-selection

Both findings reproduced before the fix (scenario and source). Both now match their harness scenario.

## text-007: EditableText long press does not select a word (P1)

- **Outcome:** fixed.
- **Cause:** the text-field pointer path had no long-press recognizer, so a held press never reached Flutter's `onSingleLongTapStart`.
- **Change:** `crates/incular-runtime/src/frame/text_gestures.rs` adds a long-press state machine. The 500 ms deadline is on the frame clock. On Android, Fuchsia, Linux and Windows the word is selected; on iOS and macOS the caret is placed when the field is focused. Moves beyond the touch slop cancel it, and a started press extends by word. A touch or stylus press shows the menu on release. `crates/incular-runtime/src/frame.rs` adds the `text_long_press` field, the frame hook (its liveness keeps frames running until the deadline), and the Down, Move, Up and Cancel branches.
- **Tests:** `crates/incular-runtime/tests/text_long_press.rs`: `text_field_long_press_selects_the_word_on_windows`, `text_field_long_press_waits_for_the_long_press_deadline`, `text_field_long_press_on_macos_places_the_caret_when_focused`, `text_field_long_press_cancels_when_the_pointer_leaves_the_slop`, `text_field_long_press_drag_extends_by_word`.
- **Harness:** `text__editable_long_press_word_selection`: match (4 frames).
- **Limitations:** the drag caret path on iOS and macOS is not separately tested.

## text-014: SelectionArea long press and selection callback (P1)

- **Outcome:** fixed.
- **Cause:** selectable text had no long-press path, and the controller had no selected-content callback. Separately, labels were joined with a newline, while Flutter concatenates them with no separator (`selectable_region.dart:2826`, `RenderParagraph.getSelectedContent` in `rendering/paragraph.dart:1596`).
- **Change:**
  - Touch and stylus presses on selectable text start a long press (`_kLongPressSelectionDevices`). The word is selected, and a move extends by word through the anchor word.
  - `crates/incular-widgets/src/selection.rs` adds `set_selection_changed_listener` and `clear_selection_changed_listener`. The callback fires when the selected text changes.
  - `crates/incular-material/src/inputs/text_fields.rs` adds `SelectionArea::on_selection_changed`.
  - Outside my ownership, `crates/incular-widgets/src/tree/text.rs` gets three small additions: `selectable_text_affinity_position_at`, `selectable_text_select_words_to`, and the no-separator join in `static_selection_text`.
  - A press on the space after a word now selects that word, using the upstream hit affinity.
- **Tests:** in `crates/incular-runtime/tests/text_long_press.rs`: `selection_area_touch_long_press_selects_the_word_and_reports_it`, `selection_area_mouse_press_does_not_long_press`. Expectation changes, newline to no separator, in `crates/incular-widgets/tests/selection_area_lifetime.rs` (four tests), `crates/incular-widgets/tests/tree_editing_semantics_gestures.rs`, and `crates/incular-runtime/tests/runtime.rs` (`selectable_text_pointer_drag_shift_extension_and_copy_are_read_only`).
- **Harness:** `text__selection_area_long_press_drag`: match (5 frames).
- **Harness twin changes (`harness/incular/tests/frames/text.rs`, `selection_area_long_press_drag`):**
  - Labels are `Widget::selectable_text_styled`, not plain `Text`. Plain `Text` is not a selectable child in Incular, so the twin did not represent the Flutter scenario, where `Text` is selectable.
  - The twin records `selection_changes` through the new callback, as the Flutter twin does.
  - `TextAlign` is imported for the new labels.
- **Limitations:** plain `Text` inside a SelectionArea is not selectable in Incular. That is a separate gap and is not fixed here. Mouse drag selection is not harnessed.

## Validation

- `cargo check -p incular-runtime -p incular-material --tests`, and `cargo check -p incular-parity-harness --tests`: clean.
- `cargo clippy -p incular-runtime -p incular-widgets -p incular-material --all-targets --no-deps -- -D warnings`: clean.
- Passing: `text_long_press` (7), `text_field_click_gestures` (3), `text_context_menu` (6), `selectable_text_shortcuts` (6), `selection_area_lifetime` (7), `selectable_text_pointer` (6), `tree_editing_semantics_gestures`, `stack_layout`.
- Pre-existing or outside text selection, not investigated: `runtime.rs` `semantic_callbacks_make_custom_controls_actionable` and `runtime_routes_identified_contacts_to_retained_scale_regions`; `tree_editing_semantics_gestures.rs` `gesture_region_combines_identified_contacts_for_scale_updates`.
