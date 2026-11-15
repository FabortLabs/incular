# layout group fixes

Status: code changes are in. Passing focused tests: `incular-widgets` `layout_padding_sizing` (30/30), `layout_table` (7/7), and `incular-config` `constraints` (6/6). Harness scenarios were not rerun.

## layout-001 (fixed, P1): IntrinsicHeight removed width constraints
- Cause: IntrinsicHeight aliased UnconstrainedBox, which loosened both axes.
- Change: IntrinsicHeight is now `ProxyLayout { IntrinsicHeight }`. The child keeps the incoming width and is tightened to its measured natural height, unless the incoming height is already tight (Flutter `RenderIntrinsicHeight._childConstraints`).
- Tests: `intrinsic_height_keeps_the_incoming_width`, `unconstrained_and_intrinsic_aliases_measure_content`.
- Harness: `layout__box_constraints__limited_overflow_intrinsic` not rerun.

## layout-002 (fixed, P2): OverflowBox lacked centered alignment
- Cause: no alignment field; the child was placed at the origin.
- Change: `OverflowBox` gained `alignment` (default center, builder `.alignment()`). The Overflow arm places the child with `aligned_child_offset` (Flutter `alignChild`).
- API: default changes overflowing children from top-left to centered. Use `.alignment(Alignment::TOP_LEFT)` to keep the old placement.
- Tests: `overflow_box_centers_an_overflowing_child`; the existing `constrained_limited_overflow_boxes` expectation changed to centered (50, 60).
- Residual: box size is still `constrain(child)`, not Flutter's `fit: max` (`constraints.biggest`). Not in the findings.

## layout-003 (blocked, P3): baseline shift of 0.625 px
- Cause (likely): Inter baseline metric in the text engine, outside this group's files.
- Work: none. Needs the `incular-text` owner, and should be consolidated with the text finding for the same metric.

**Text residual pass: fixed.** Same text-engine fix as text-011 (`snap_line_boxes` in `crates/incular-text/src/engine.rs`). `layout__baseline__cross_axis` now matches, and incular-layout is unchanged.

## layout-004 (fixed, P2): UnconstrainedBox ignored alignment
- Change: `UnconstrainedBox` gained `alignment` (default center). The Unconstrained arm aligns with the child's measured size.
- Note: `IntrinsicWidth` now passes `TOP_LEFT` explicitly, because it is a proxy in Flutter.
- Tests: `unconstrained_box_centers_overflow_in_a_tight_slot`, `unconstrained_and_intrinsic_aliases_measure_content`.
- Harness: `layout__unconstrained_box__constrained_axis` not rerun.

## layout-005 (fixed, P1): Table had no column width or vertical alignment API
- Change: `TableColumnWidth` (Fixed, Fraction, Intrinsic, Flex; default `Flex(1.0)`) and `TableCellVerticalAlignment` (Top, Middle, Bottom; default Top). `table_column_widths` follows Flutter's `_computeColumnWidths` for the cases it covers. The tree lays out every cell at its tight column width in a second pass.
- API: `Table::column_width`, `default_column_width`, `default_vertical_alignment`. A bounded Table now fills its width by default. Migrate with `.default_column_width(TableColumnWidth::Intrinsic)`.
- Tests: `layout_table.rs` `default_flex_columns_fill_the_bounded_width`, `fixed_column_leaves_the_rest_to_flex_columns`, `intrinsic_columns_size_to_content`, `middle_vertical_alignment_centers_short_cells_in_their_row`. Not run.
- Limitations: Fill, Baseline and IntrinsicHeight alignments are not implemented; no column deficit shrinking; no minIntrinsicWidth; no per-cell alignment.
- Harness: `layout__table__default_flex_columns` not rerun.

## layout-006 (fixed, P3): no public BoxConstraints.enforce
- Change: `Constraints::enforce(self, other)` clamps each bound into `other` (Flutter semantics).
- Test: `incular-config/tests/constraints.rs::enforce_clamps_each_bound_into_the_other_constraints`. Not run.

## layout-007 (fixed, P2): FractionallySizedBox dropped the parent minimum
- Change: `fractional_constraints` keeps the parent's min and max on axes without a factor (Flutter `_getInnerConstraints`).
- Test: `fractionally_sized_box_keeps_parent_minimum_on_unfactored_axis`.
- Harness: `layout__fractionally_sized_box__null_factor_tight_parent` not rerun.

## misc-001 (fixed, P1): Expansible had no body animation
- Change: the body is in an `AnimatedSize` with 200 ms and `Curve::Ease` defaults, `Clip::HardEdge`; `duration()` and `curve()` builders added.
- Limitation: a collapsed body is replaced by an empty box, so body state is not kept. No unit test. Harness not rerun.

## misc-002 (fixed, P1): RotatedBox kept unrotated dimensions
- Change: `RotatedBox` wraps its Transform in `ProxyLayout { QuarterTurns }`. Odd turns use flipped constraints, a swapped size, and a centered child (Flutter `rotated_box.dart:83-96`).
- Test: `rotated_box_swaps_dimensions_on_odd_quarter_turns`. Not run against the harness.

## misc-008 (fixed, P1): nested SafeArea double-padded
- Change: the consumed sides are scoped to descendants with `safe_insets_scope` (Flutter `MediaQuery.removePadding`); padding comes from `context.safe_insets()`.
- Limitations: `maintain_bottom_view_padding` still uses the retained render arm, and descendants do not see reduced insets there. View padding is not scoped; Flutter also reduces viewPadding. The finding said to keep view padding unchanged, but Flutter's source reduces it, so that part is a residual.
- Harness: `misc__safe_area_nested_consumption` not rerun.

## misc-009 (fixed, P2): KeyedSubtree overwrote the child key
- Change: a child that already has a key is wrapped in a transparent builder that carries the outer key. Unkeyed children keep the direct-key path.
- Test: none added.

## misc-013 (fixed, P2): Placeholder not exported
- Change: `Placeholder` added to the `incular-widgets` and facade `layout` re-exports. The facade also re-exports the new table types.
- Harness: `misc__placeholder_fallback_size` already matched.

## Edits outside owned files (for coordinator review)
- `crates/incular-widgets/src/tree.rs`: `ChildLayoutMode` re-export; RenderKind Overflow/Unconstrained/Table fields; new `ProxyLayout` variant.
- `crates/incular-widgets/src/tree/specs.rs`: `ChildLayoutMode`; WidgetKind fields and `ProxyLayout`.
- `crates/incular-widgets/src/tree/widget/structure.rs`: Debug, PartialEq, WidgetType entries for the new kinds and fields.
- `crates/incular-widgets/src/tree/rendering/lowering/layout.rs`, `tree/layout/behavior/mod.rs`, `tree/painting/behavior/mod.rs`: lowering and dispatch arms.
- `crates/incular-widgets/src/tree/context.rs`: `safe_insets_scope`.
- `crates/incular-widgets/src/tree/widget/mod.rs`: re-export `aligned_child_offset`.
- `crates/incular-widgets/src/scrolling/layout.rs` and `scrolling/reorderable.rs`: `ProxyLayout` child arms; Overflow arm now uses `..`.
- `crates/incular-layout/src/lib.rs`: exports for the table types and `table_column_widths`.
- `crates/incular/src/lib.rs`: facade exports for `Placeholder`, `TableColumnWidth`, `TableCellVerticalAlignment`.

## Not verified
- Harness scenarios `harness/run-frames.sh layout__` and the misc scenarios named in the brief.
- `cargo check -p incular-parity-harness --tests` (needed if public API used by harness changed) and `cargo clippy -p incular-widgets -p incular-layout -p incular-config --all-targets -- -D warnings`.
- Verified passing: `cargo test -p incular-widgets --test layout_padding_sizing --test layout_table` and `cargo test -p incular-config --test constraints`.
