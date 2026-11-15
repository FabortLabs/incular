# Suite triage: widgets

Group `suite-triage-widgets`. These tests passed at baseline `e25e45bc` (worktree `base-e25e45bc`) and failed on `fix/flutter-parity` after the parity fix pass. Each one gets a verdict:

- **(a) expectation:** the test encoded non-Flutter behavior. The test is updated with a Flutter 3.41.7 citation and keeps its intent.
- **(b) regression:** the framework is fixed.

No finding ids are reopened, so there is no `.jsonl`.

## utility_coverage: six SafeArea tests (a)

The tests are `safe_area_applies_all_edges`, `safe_area_disabled_edges_fall_to_minimum`, `safe_area_maintain_survives_keyboard_cycle`, `safe_area_minimum_raises_padding`, `safe_area_nested_accumulates` and `safe_area_resolve_with_padding_matches_retained`.

- **Cause:** misc-008 made a non-maintained SafeArea lower to `Builder > Padding > safe-insets scope > child`. This is Flutter's `SafeArea.build`, `Padding` over `MediaQuery.removePadding` (widgets/safe_area.dart:113-136). The tests read `only_child(root)` and so measured the Padding element, not the child.
- **Nested test:** it expected the padding to apply twice. Flutter's `removePadding` hides the consumed sides from the inner SafeArea, so the insets apply once.
- **Change** (`crates/incular-widgets/tests/utility_coverage.rs`):
  - New `safe_area_leaf` helper that walks the single-child chain. `child_height` and the origin checks use it.
  - The nested test is renamed `safe_area_nested_consumes_padding_once`. It now expects origin (10, 20) and size 260x240, with the citation.
- **Harness:** `misc__safe_area_nested_consumption` matches.
- **Residual (not changed, misc-008):**
  - The `maintain_bottom_view_padding` arm still scopes nothing.
  - `safe_area_nested_maintained_accumulates_from_snapshot` still encodes double padding. Flutter's `removePadding` also zeroes `viewPadding`.

## wrap_table_layout: five Table tests (a)

The tests are `table_max_content_columns_and_rows`, `table_ragged_tail_fills_left_to_right`, `table_columns_change_reconfigures_rows`, `table_columns_floor_at_one_and_cells_replace` and `wrap_and_table_hit_and_semantics_follow_layout`.

- **Cause:** layout-005 changed the default column width to `FlexColumnWidth` (rendering/table.dart:377; widgets/table.dart:124), so a bounded table fills its width.
- **Second effect:** Flutter lays each cell out with `BoxConstraints.tightFor(width: widths[x])` (rendering/table.dart:1375), so a narrower cell fills its column. The old expectations (30, 20 and 5 px cells) were content-sized.
- **Change:**
  - The tests opt into `default_column_width(TableColumnWidth::Intrinsic)` to keep their max-content intent.
  - The cell widths are now the column widths: kids[0] is 40, the tail is 40 and the replaced cell is 20. Citations are added.
  - The module doc is updated.
- **Harness:** `layout__table__rows_and_columns` and `layout__table__default_flex_columns` match.

## transform_geometry::rotated_box_keeps_layout_size_and_rotates_geometry (a)

- **Cause:** misc-002. The test asserted that RotatedBox is layout-neutral, which is Transform's behavior.
- **Flutter:** `RenderRotatedBox.performLayout` (rendering/rotated_box.dart:88-99) lays the child out with flipped constraints on odd turns and reports the swapped size. Its paint transform maps the child onto that box.
- **Change:** renamed to `rotated_box_swaps_layout_size_and_rotates_geometry`.
  - The box is 20x40.
  - The child's world bounds are (0, 0, 20, 40).
  - A hit at (10, 30) reaches the child; (35, 5) misses.
  - The semantic bounds are 20x40.
- **Harness:** `misc__rotated_box_layout_size` matches.

## editable_text_direction_repro::explicit_editable_text_direction_resolves_start_for_latin_text (a)

- **Cause:** text-010 removed EditableText's built-in 8 px inset. Flutter's EditableText has no content padding; the padding belongs to InputDecorator. The repro assumed the 8 px inset.
- **Change:** pass `.content_padding(EdgeInsets::all(TEXT_INSET))`, the same as the sibling `editable_text_direction.rs`. The assertions about the direction resolving start are unchanged.

## public_api::forms_restoration_undo_environment_and_routes_are_deployable (a)

- **Cause:** misc-004 changed what `FormController::save` does.
  - It no longer validates. It returns the cached validity, and the last recorded error was "required".
  - Flutter's `FormState.save` is void and saves unconditionally (widgets/form.dart:314-319).
  - Validate-then-save is now `submit`.
- **Change:** `assert!(form.submit())`, with the citation.

## migrated_tests::autocomplete_tests::builder_preserves_autocomplete_defaults_and_accepts_widgets (b)

- **Cause:** a parity fix made a zero-alpha `Opacity` drop its child's semantics, which is Flutter's `RenderOpacity.visitChildrenForSemantics` (rendering/proxy_box.dart:956-960). The controls autocomplete, OTP field and number field overlay an invisible `Opacity(0)` editor on a custom visual, so the editor's node and value disappeared. That is a regression for these controls.
- **Fix:** `.always_include_semantics(true)` on that Opacity, in:
  - `crates/incular-controls/src/autocomplete.rs`;
  - `crates/incular-controls/src/otp_field.rs`;
  - `crates/incular-controls/src/number_field.rs` (same pattern, fixed preventively).
- **Tests:**
  - `migrated_tests` 67/67;
  - `w7_form_text`, `w7_compound_controls` and `controls_test`: all pass.

## navigation_platform_scopes::animated_modal_barrier_options (b)

- **Cause:** not switchers-005.
  - semantics-007 made a `GestureDetector` with `on_tap` expose Activate. The barrier's dismiss detector used `on_tap`, so a labeled barrier with `barrier_semantics_dismissible(false)` gained Activate, bypassing the semantic-dismiss gate.
  - Flutter's `_ModalBarrierGestureDetector` uses `_AnyTapGestureRecognizer.onAnyTapUp` (widgets/modal_barrier.dart:392-396, 438). That recognizer is not a `TapGestureRecognizer`, so it adds no semantic tap.
- **Fix:** in `crates/incular-widgets/src/navigation/barrier.rs`, the detector dismisses through `on_tap_up` instead of `on_tap`, with a citation. Pointer dismissal is unchanged.
- **Tests:**
  - `navigation_platform_scopes` 18/18;
  - `navigation_scopes` 8/8;
  - `completed_module_integration`: passes;
  - `incular-navigation --test navigation` 83/83.
- **Harness:** `semantics__gesture_tap` still matches.
- **Note:** Incular keeps the barrier label when semantic dismissal is off. Flutter excludes the barrier semantics entirely in that case (modal_barrier.dart:241, 258-266). This was an existing documented divergence and is not changed here.

## raw_tooltip::descriptor_retains_semantics_and_composited_overlay_hooks (a)

- **Cause:** semantics-006 moved the tooltip message from `description` (hint) to `state.tooltip`. Flutter's RawTooltip wraps its child in `Semantics(tooltip: semanticsTooltip)` (widgets/raw_tooltip.dart:850-853).
- **Change:** the test reads `node.state.tooltip`.
- **Harness:** `semantics__value_hint_tooltip` matches.

## transient_surfaces::transient_portal_reports_anchor_and_popup_but_not_barrier_geometry (a)

- **Cause:** navigation-009 made OverlayPortal pass the parent constraints to its anchor. Flutter's `_RenderLayoutSurrogateProxyBox` is a `RenderProxyBox`. Under the test's tight 100x80 root, the anchor became 100x80.
- **Change:** lay out under loose 100x80 constraints, so the content-sized anchor geometry the test measures is kept.
- **Harness:** `navigation__overlay_portal__*` 3/3 match.

## Validation

- **Every target, rerun by name:**
  - `utility_coverage`: 37/37. The two split_view failures seen at the start were fixed by another worker meanwhile.
  - `wrap_table_layout` 17/17, `transform_geometry` 11/11 and `editable_text_direction_repro` 1/1.
  - `navigation_platform_scopes` 18/18, `raw_tooltip` 6/6 and `transient_surfaces` 11/11.
  - `incular` `public_api` 6/6 and `incular-controls` `migrated_tests` 67/67.
- **Clippy:** `cargo clippy -p incular-controls -p incular-widgets --all-targets -- -D warnings` is clean.
- **Framework edits outside test files:** `barrier.rs`, controls `autocomplete.rs`, `otp_field.rs` and `number_field.rs`.
