# layout: Static layout widgets

## Summary

Current state (HEAD a19bf487): 34 layout scenarios plus 2 transitions__layout__ scenarios (36 total), 30 match at frame 0 after the run. Confirmed divergences: layout-001 (IntrinsicHeight drops width), layout-002 and layout-004 (overflowing children are not centered), layout-003 (0.625 px baseline text), layout-005 (Table has no column width API; diagnostic), layout-006 (no public BoxConstraints.enforce). The inventory is complete: 112 rows with final status; see the coverage counts under Validation run. The coverage table below predates this pass and is superseded by the rows in coverage/layout.jsonl.

## Coverage

| Flutter item | Method / scenario | Result |
| --- | --- | --- |
| `Flex`, `Row`, `Column`, `MainAxisAlignment`, `CrossAxisAlignment`, `Expanded` / `Flexible` | `layout__row__main_and_cross_alignment`, `layout__flex__expanded_and_spacing`; existing `flex_layout.rs` covers flex fits, factors, spacing, RTL, vertical direction, and baseline layout. | partial; selected configurations match |
| `Stack`, `Positioned`, `IndexedStack` | `layout__stack__positioned`, `layout__indexed_stack__active_child`; existing `layout_stack.rs` checks constructors/builders. | tested selected configurations; match |
| `Wrap` | `layout__wrap__multiple_runs`; existing `layout_wrap.rs` checks constructors and configuration normalization. | tested multiple-run placement; match |
| `Flow` | `layout__flow__child_transforms`; existing `custom_layouts.rs` checks transforms and omitted-child behavior. | tested selected transforms; match |
| `CustomSingleChildLayout`, `CustomMultiChildLayout`, `LayoutId` | `layout__custom_single_child`, `layout__custom_multi_child`; existing `custom_layouts.rs`. | tested selected size, constraints and placement; match |
| `Align`, `Center`, `FractionallySizedBox`, `ConstrainedBox` | `layout__aspect_constraints_and_fraction`; existing `layout_padding_sizing.rs`. | tested selected configuration; match |
| `LimitedBox`, `UnconstrainedBox`, `OverflowBox`, `SizedOverflowBox` | `layout__padding_and_overflow_boxes`, `layout__box_constraints__limited_overflow_intrinsic`. | Limited and unconstrained tested; `OverflowBox` differs at default alignment; `SizedOverflowBox` matches selected configuration. |
| `AspectRatio`, `FittedBox` | `layout__aspect_constraints_and_fraction`, `layout__fitted_box__contain_alignment`; existing `layout_padding_sizing.rs` covers fit modes. | tested selected ratios/fits; match |
| `IntrinsicWidth`, `IntrinsicHeight` | `layout__intrinsic_width__step`, `layout__box_constraints__limited_overflow_intrinsic`; existing `layout_padding_sizing.rs` covers width step measurement. | Width matches; height diverges when its child is a max-sized Row. |
| `Baseline` and baseline `Row` | `layout__baseline__cross_axis`; existing `flex_layout.rs` and `layout_padding_sizing.rs`. | 0.625 px vertical-origin difference for the smaller text; likely text metrics, see layout-003. |
| `Table` | `layout__table__rows_and_columns`; existing `layout_table.rs`. | Match for intrinsic columns after matching the Flutter configuration. Cell vertical alignment options were not covered. |
| `Padding` | `layout__padding_and_overflow_boxes`; existing `layout_padding_sizing.rs`. | Match for symmetric physical padding. Directional and negative padding not frame-tested. |
| `Transform` | `layout__visibility_offstage_and_transform`. | Translation matches; matrix, rotate, skew, origin/alignment and hit-test variants not tested. |
| `LayoutBuilder` | `layout__layout_builder__constraint_size`. | Match for a width derived from incoming max constraints. Other constraint changes not tested. |
| `Offstage`, `Visibility` | `layout__visibility_offstage_and_transform`; existing `visibility.rs` and `visibility_options.rs`. | Match for hidden child geometry/opacity with default size policy and maintained state/animation. Other preservation combinations are unit-tested but not paired. |
| Other in-scope render/layout declarations and configuration variants | Source reading and existing `crates/incular-widgets/tests/layout_*.rs`, `custom_layouts.rs`, and `visibility*.rs`. | not checked: no paired review for every public helper, edge constraint, direction, clipping or intrinsic variant |

## Findings

### layout-001 — IntrinsicHeight removes horizontal constraints

- Severity: P1
- Category: layout
- Flutter: `rendering/proxy_box.dart:783-827` (`RenderIntrinsicHeight`) tightens only the child's height to its max intrinsic height while retaining the incoming width constraints; `widgets/basic.dart:3846` exposes the widget.
- Incular: `crates/incular-widgets/src/layout/basic/sizing_constraints.rs:472-490` implements `IntrinsicHeight` by wrapping its child in `UnconstrainedBox`, which loosens both axes.
- Evidence: `layout__box_constraints__limited_overflow_intrinsic`, frame 0. Both start from the same 800×600 root and use an IntrinsicHeight containing a two-child Row with 40 px and 40 px children. Flutter's probed row is `[0,140,800,190]` (800 px wide); Incular's is `[360,140,440,190]` (80 px wide). Rerunning `harness/run-frames.sh layout__` reproduces the difference.
- Impact: an IntrinsicHeight around a default max-sized Row can shrink the row to content width in Incular, changing layout of adjacent or distributed children.
- Likely cause: Incular aliases intrinsic-height sizing to an unconstrained wrapper that removes width constraints too.
- Fix direction: implement intrinsic-height sizing by tightening only the child's intrinsic height and preserving the incoming width constraints.
- Confidence: high

### layout-002 — OverflowBox lacks Flutter's centered child alignment

- Severity: P2
- Category: layout
- Flutter: `widgets/basic.dart:3385-3416` provides `OverflowBox.alignment`, defaulting to `Alignment.center`; `rendering/shifted_box.dart:635-646` passes alignment to `RenderConstrainedOverflowBox`.
- Incular: `crates/incular-widgets/src/layout/basic/sizing_constraints.rs:264-315` exposes only min/max width and height; `crates/incular-widgets/src/tree/layout/behavior/core.rs:153-176` places the child at `Offset::ZERO`.
- Evidence: `layout__box_constraints__limited_overflow_intrinsic`, frame 0. The parent is 100×80 and the constrained child is 150×110. Flutter centers it at `[325,45,475,155]`; Incular starts it at the parent origin `[350,60,500,170]`. The frame-zero parent constraints and child extents match.
- Impact: children larger than their OverflowBox are shifted 25 px right and 15 px down in this case, changing alignment of intentionally overflowing content.
- Likely cause: the Incular descriptor and layout path have no alignment field and use a zero child offset.
- Fix direction: add an alignment value defaulting to center and place the constrained child by the alignment delta between parent and child sizes.
- Confidence: high

### layout-003 — Baseline-aligned small text is shifted by 0.625 px

- Severity: P3
- Category: layout
- Flutter: `rendering/flex.dart:978-996` aligns children using their measured baseline offsets; `rendering/paragraph.dart:876-886` obtains a Text baseline from `TextPainter`.
- Incular: `crates/incular-widgets/src/tree/layout/behavior/core.rs:65-69` supplies the text layout baseline, and `crates/incular-layout/src/algorithms.rs` performs baseline alignment.
- Evidence: `layout__baseline__cross_axis`, frame 0. Text widths and heights agree for both children. The 12 px child's top is 11.625 px in Flutter and 11 px in Incular; the 24 px child's top agrees at zero. The difference is below one pixel and limited to the smaller child's baseline placement.
- Impact: a small vertical shift can occur for fractional font metrics in baseline-aligned rows.
- Likely cause: the layout algorithms use child baselines, but the text engines report a 0.625 px difference in the smaller font's baseline. This is likely overlapping text-metrics scope rather than a flex placement error.
- Fix direction: compare and reconcile the Inter baseline metric between Flutter and Incular; consolidate with any text-package finding for the same metric.
- Confidence: medium

## Pixel scene proposals

No paint-only layout scene is needed for these frame geometry differences. Clipping and overflow paint behavior remain unverified by pixels.

## Open questions

- The current `inventory.md` has many rendering-level declarations assigned to `layout`; review which are observable widget behavior and reconcile them one by one.
- Paired coverage remains open for every main/cross-axis alignment variant, flex baseline without text, RTL/vertical direction, Stack fit/clip modes, Wrap alignments, intrinsic edge constraints, Table vertical alignment, Transform matrices and hit testing, and all `Visibility` preservation options.
- A true clip/shadow/overflow paint comparison requires the orchestrator's pixel batch.

### layout-004 — UnconstrainedBox ignores alignment; overflowing children stay at top-left

- Severity: P2
- Category: layout
- Flutter: `widgets/basic.dart:3117-3180` (UnconstrainedBox with `alignment` and `constrainedAxis` through ConstraintsTransformBox); `widgets/basic.dart:2949-2969` (`widthUnconstrained`, `heightUnconstrained`).
- Incular: `crates/incular-widgets/src/tree/layout/behavior/core.rs:179-204` returns `ChildOffsets::One(Offset::ZERO)`; `crates/incular-widgets/src/layout/basic/sizing_constraints.rs:322-360` has no alignment.
- Evidence: `layout__unconstrained_box__constrained_axis`, frame 0. Sizes match on all three variants, including overflowing child heights and widths. Placement differs: Flutter `ub_h` rect `[250,-30,550,230]` vs Incular `[250,0,550,260]`; `ub_none` `[200,370,600,630]` vs `[250,400,650,660]`.
- Impact: overflowing UnconstrainedBox children are placed at the top-left, not centered.
- Cause: the render kind has no alignment and always uses a zero offset.
- Fix direction: add alignment (default center) and place the child with `alignment.within` on the box size, allowing negative offsets.
- Confidence: high. Limitation: the smaller-child case is by reading only. The Flutter twin uses `clipBehavior: Clip.hardEdge` to avoid Flutter's overflow error; the Incular twin has no clip, which does not change geometry.

### layout-005 — Table has no column width or vertical alignment API

- Severity: P1
- Category: layout (missing feature)
- Flutter: `widgets/table.dart:104-124` (default `FlexColumnWidth`); `rendering/table.dart:43-361` (column width specs and `TableCellVerticalAlignment`).
- Incular: `crates/incular-widgets/src/layout/table.rs:10-45`; `crates/incular-layout/src/algorithms.rs:800-841` (max-content columns only).
- Evidence: `layout__table__default_flex_columns`, a non-equivalent diagnostic. Flutter's default table is 800 px wide; Incular's (the only available configuration) is 100 px. The intrinsic configuration matches in `layout__table__rows_and_columns`.
- Impact: a default Flutter Table fills the available width; Incular cannot express fixed, flex, fraction, min or max column widths, and has no table-level cell vertical alignment.
- Cause: Incular Table is intrinsic-only.
- Fix direction: add column width specs with a flex default and top vertical alignment default, following Flutter's distribution rules.
- Confidence: high. Limitation: the flex default is evidenced by a diagnostic only; table borders are not checked.

### layout-006 — No public BoxConstraints.enforce helper

- Severity: P3
- Category: api (missing feature)
- Flutter: `BoxConstraints.enforce`, used by ConstrainedBox.
- Incular: `crates/incular-config/src/constraints.rs:151-206` (no public enforce); `crates/incular-layout/src/algorithms.rs:1019` (private `enforce_constraints`).
- Evidence: `layout__box_constraints__helper_values`. Flutter returns 100 for `enforce_min_w`; the Incular twin has no value (null). Rendering is not affected because ConstrainedBox uses the private helper.
- Impact: API-shape only; no runtime layout failure is claimed.
- Confidence: high.

## Candidates checked and not confirmed

- UnconstrainedBox "keeps cross-axis max constraint" (suspected from `algorithms.rs:450-479`): not confirmed. Child sizes match Flutter in `layout__unconstrained_box__constrained_axis`; the only divergence is placement, recorded as layout-004.
- IntrinsicWidth stepHeight rounding: not confirmed. `layout__intrinsic_width__steps_and_flex_child` matches at frame 0 (100x45 box).
- Flex child intrinsic width inside IntrinsicWidth: not confirmed. The same scenario matches (flex child intrinsic width 60).

## Recorder and twin notes

- `layout__visibility__maintain_flags`, `probe.vis_size.opacity`: Flutter reports 1.0 and Incular reports 0.0. Flutter's `_RenderVisibility` hides a `maintainSize` child by skipping paint, not by an opacity object, so the recorder does not see it. Geometry matches. This is a recorder limitation, not a layout divergence (no finding).
- The frame suite records Flutter's `RenderOffstage`/`RenderAnimatedOpacity` opacity only; a harness request is to record `_RenderVisibility.visible == false` as zero opacity on the Flutter side.
- `layout__box_constraints__helper_values` has no Incular `enforce` value; see layout-006.

## Harness requests

- Recorder: read `_RenderVisibility.visible` (widgets/visibility.dart:591-626) as hidden opacity so `maintainSize` visibility compares.
- None others.

## Validation run

- `FLUTTER="C:/Manual Installs/flutter/bin/flutter" harness/run-frames.sh layout__`: 30 of 36 scenarios match. Divergences: layout-001, -002, -003, -004 (ub_h, ub_none, ub_v), layout-005 (diagnostic table), layout-006 (enforce value), and the visibility opacity recorder limitation. Of the 17 new scenarios added in this pass, 13 match at frame 0; the non-matching ones are layout__unconstrained_box__constrained_axis, layout__visibility__maintain_flags (recorder only), layout__box_constraints__helper_values (missing value) and layout__table__default_flex_columns (diagnostic).
- `flutter analyze test/frames/layout_test.dart`: no issues.
- `cargo check -q -p incular-parity-harness --tests`: passes.
- `rendering/` and `widgets/` coverage rows: 112 rows, statuses: scenario 37, scenario+reading 21, partial-scenario 11, reading 10, partial-reading 4, missing 7, out-of-scope 18, unchecked 4 (OverflowBar, OverflowBarAlignment, SizeChangedLayoutNotification, SizeChangedLayoutNotifier; reassigned to misc).

### layout-007 — FractionallySizedBox drops the parent minimum on an axis without a factor

- Severity: P2. Category: behavior (layout). Added by the orchestrator from the coverage sweep; status accepted.
- Flutter: `rendering/shifted_box.dart:712-722 (RenderFractionallySizedOverflowBox._getInnerConstraints)`. Incular: `crates/incular-widgets/src/tree/widget/geometry.rs:150-166 (fractional_constraints)`.
- Evidence (scenario+source, layout__fractionally_sized_box__null_factor_tight_parent): Under a tight 200x100 parent, FractionallySizedBox(heightFactor: 0.5) with a 50x20 SizedBox child: Flutter passes width min=max=200 (parent constraints kept for the null widthFactor), so the child is 200x50; Incular passes width 0..200, so the child keeps its 50 px width (50x50).
- Impact: A FractionallySizedBox that sets only one factor lets its child shrink on the other axis where Flutter forces the parent's minimum, so children inside tight slots (Expanded, SizedBox, grid tiles) render narrower or shorter than in Flutter.
- Cause: fractional_constraints uses 0 as the minimum when a factor is absent instead of the parent's minimum.
- Fix direction: For an axis without a factor, keep both the parent's minimum and maximum; for an axis with a factor, use the factored size as both.
- Confidence: high. Limitations: Found by the orchestrator from a sweep candidate; one tight-parent case was compared. The negative-factor clamp (setter clamps to 0 where Flutter asserts) was not compared.
