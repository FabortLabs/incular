# text-3 fixes

These records supersede the earlier text-003, text-005 and text-013 records in `rich-text.md` and `text-editing.md`, and the misc-005 record in `forms.md`.

## text-003: fixed (WidgetSpan)

- **Confirmed** before the fix: `text__widget_span_layout` gave 220x19 (Incular) against 220x40 (Flutter).
- **Cause:** a `WidgetSpan` was shaped as a literal U+FFFC. No inline box was reserved, and the tree had no slot for an inline child widget.
- **Engine (`incular-text`):**
  - New file `engine/inline.rs`. Each placeholder becomes a Parley `InlineBox`, aligned as SkParagraph's `Run::updateMetrics` does:
    - baseline, above-baseline and below-baseline from the box alone;
    - top and bottom against the font (text-top and text-bottom);
    - middle centred on `(ascent - descent) / 2`, from the font metrics after shaping, then the lines are broken again.
  - U+FFFC is shaped as a zero-width U+200B at the same byte offsets.
  - `TextLayout::placeholders` reports each placed box.
  - The caret walk steps over boxes, so a caret after a box sits at the box's right edge.
  - Boxes follow `snap_line_boxes` and are dropped by `clipped_layout`.
  - New API: `TextEngine::layout_rich_with_options` and `TextSpan::placeholders`. `RichText::layout` now reserves the boxes.
- **Widgets (`incular-widgets`):**
  - New `RichParagraph::new(rich).child(id, widget)`.
  - Each child is laid out with `BoxConstraints(maxWidth)`, as `RenderParagraph` does. Its size and baseline fill its box, and the child is placed, painted and hit-tested there.
  - A plain `RichText` reserves the sizes stated by its childless spans.
- **Tests:**
  - `incular-text/tests/inline_placeholders.rs`: 3 placeholder tests.
  - `incular-widgets/tests/rich_text_widget_span.rs`: 2 tests.
- **Harness:** `text__widget_span_layout` **matches**. The diagnostic Incular twin now uses `RichParagraph` with the same 40x40 blue `ColoredBox` child that the Flutter twin uses, so the probe colour compares too.
- **Limitations:**
  - Intrinsic size queries ignore the boxes.
  - A child whose box is clipped or ellipsized stays at the paragraph origin and is still painted. Flutter skips it.
  - The text scaler does not scale children.
  - The ideographic baseline is not modelled.

## text-005: fixed (unset initial selection)

- **Confirmed** before the fix: before focus, Flutter reported a selection of -1 and Incular reported 3.
- **Cause:** `with_text` stored a collapsed caret at the text end. In Flutter, `TextEditingController(text:)` and the `text` setter leave `collapsed(offset: -1)`.
- **Change in `editing.rs`:**
  - A new `selection_unset` flag. `new()`, `with_text()` and `set_text()` set it.
  - Any selection, edit, value change or restoration clears it. Going from unset to set notifies listeners even when the offsets are unchanged.
  - New `selection_if_set()` and `collapse_unset_selection()`. `selection()` is unchanged, so existing callers keep working.
- **Change in `tree/text.rs`:** `set_focused` collapses an unset selection at the end when a field gains focus, as `EditableText._handleFocusChanged` does.
- **Test:** `incular-text/tests/editing.rs::initial_text_leaves_the_selection_unset_until_one_is_applied`.
- **Harness:** the Incular twins now read `selection_if_set`, which gives -1 while unset. These scenarios now match:
  - `text__editable_initial_selection_focus`
  - `word_arrow_selection`
  - `home_end_shift_select`
  - `backspace_delete`
  - `multiline_vertical_document`
- **Limitation:** `from_value(TextEditingValue::new(..))` still starts with a selection set.

## text-013: fixed (strutStyle)

- **Change in `incular-text`:**
  - New `TextLayoutOptions::strut`, part of the cache key.
  - New `TextEngine::resolve_strut`. It takes the strut font's ascent and descent and scales them by `height`, with proportional or even leading.
  - `apply_strut` runs on the unrounded line boxes before `snap_line_boxes`. A strut widens each box; `force_strut_height` replaces it.
- **Change in `incular-widgets`:** new `Text::strut_style` and `RichParagraph::strut_style`. The text scaler applies to the strut size, and an unset size is 14.
- **Tests:**
  - `inline_placeholders.rs::a_forced_strut_sets_every_line_height`
  - `rich_text_widget_span.rs::a_forced_strut_fixes_the_text_line_height`
- **Harness:** new paired twins `text__strut_style_min_height`, in `harness/flutter/test/frames/text_test.dart` and `harness/incular/tests/frames/text.rs`. They use `'one\ntwo'` in Inter 14 with an Inter 24 strut. The scenario **matches**.
- **Limitation:** `StrutStyle.leading` is not modelled. The type has no such field, and Flutter's default is 0.

## misc-005: fixed (RawAutocomplete)

- **Change in `forms.rs`:** RawAutocomplete now has retained state, like `_RawAutocompleteState`.
  - `options_builder` runs when the text changes.
  - The options view is shown in an `OverlayPortal` under the field, at the field's width and inside a `TextFieldTapRegion`. It shows while the field has focus and there are options.
  - Keys:
    - Up and Down move the highlight, clamped to the options.
    - PageUp and PageDown move it by 4.
    - Ctrl or Cmd with Up or Down jumps to the first or last option.
    - An arrow key re-opens a hidden view.
    - Escape hides the view, or passes through when it is already hidden.
  - Submitting the field (`onFieldSubmitted`) selects the highlighted option.
  - Selecting sets the display string with a collapsed caret, calls `on_selected` and closes the view.
  - New `AutocompleteHighlightedOption::of(context)`.
  - New builders: `field_view_builder`, `text_editing_controller`, `focus_node`. The default field view is an `EditableText` under `Focus(node)`.
- **Tests:** `incular-widgets/tests/raw_autocomplete.rs`, 3 tests.
- **Limitations:**
  - The view only opens downward.
  - Selecting the same option again is not deduplicated.
  - There is no semantics announcement and no `initialValue`.

## Triage: `tree_layout_painting`

Both tests pass in the baseline checkout. Neither failure was a regression; both expectations encoded the old structure or behavior.

- **`default_text_style_is_resolved_for_descendant_text`:** after text-004, `DefaultTextStyle` lowers to two nested environment scopes. The test now walks down to the `Text` node. The style assertions are unchanged.
- **`table_uses_max_content_cell_sizes_and_row_major_offsets`:** after layout-005, a table defaults to Flutter's `FlexColumnWidth` (table.dart `defaultColumnWidth`). The test now sets `default_column_width(TableColumnWidth::Intrinsic)` to keep its max-content intent, with a citation in a comment.
- All 41 tests pass.

## Validation

- **Scenarios:**

  | Prefix | Before | After |
  | --- | --- | --- |
  | `text__` | 17/27 | 24/28 (one new scenario) |
  | `layout__` | 35/37 | 35/37 |
  | `implicit__` | 19/19 | 19/19 |

  The 4 remaining text divergences were present before this pass:
  - `editing_selection_offsets` (UTF-16 against UTF-8 lengths);
  - `undo_redo`;
  - `undo_no_programmatic_selection`;
  - `enter_callback_order`.

  No scenario regressed.
- **Tests:** all of these pass.
  - incular-text: `inline_placeholders`, `editing`, `editing_copies`, `engine`, `rich_runs`, `height_behavior`.
  - incular-widgets: `rich_text_widget_span`, `raw_autocomplete`, `tree_layout_painting`, `forms`, `selection_area_lifetime`, `text_height_behavior`, `focus_keyboard`, `selectable_text_pointer`, `editable_text_alignment`, `editable_text_bidi`.
  - incular-runtime: the text tests, `caret_on_screen`, `focus_mount`, `semantic_set_text_user_edit`.
  - incular-controls: `w7_compound_controls`.
  - incular-material: `length_formatting`.
- **Not run:** the `editable_text*` tests that use `tests/common`. Another worker's `PaintCommand::PushMatrixFilter` change made `tests/common/mod.rs` fail to compile at the time.
- **Clippy:**
  - incular-text: clean.
  - incular-widgets library: clean in my files. The only error is in `render_object/layers.rs:110`, which belongs to another worker. That error stopped the `--all-targets` lint before it reached the test targets.

## Out-of-ownership edits

- **incular-widgets plumbing:** `tree/specs.rs` and `tree.rs` (fields `inline` and `strut`), `tree/widget/structure.rs`, `tree/widget/constructors.rs`, `tree/rendering/lowering/visual.rs`, `render_object/update.rs`, `devtools_props.rs`, and the `lib.rs` exports.
- **Twins:** `harness/incular/tests/frames/text.rs` has selection reads through `selection_if_set`, the widget-span twin on `RichParagraph`, the new strut twin, and an unused import removed. `harness/flutter/test/frames/text_test.dart` has the new strut twin only.
