# layout: Static layout widgets

## Summary

Seventeen frame scenarios cover the main retained geometry paths. Fifteen match. Two frame-zero differences reproduce: Incular's `IntrinsicHeight` drops horizontal constraints, making a default `Row` shrink to its children instead of expanding to the available width; and Incular's `OverflowBox` places an oversized child at the parent's top-left instead of Flutter's centered default. A baseline-alignment scenario also differs by 0.625 px in the smaller text's vertical origin while width and height agree; this appears to come from text baseline metrics and should be reconciled with the text package finding. Existing Incular unit tests cover many flex, constraints, custom-layout, and visibility cases, but they do not establish parity for every Flutter layout option.

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

## Harness requests

None.

## Open questions

- The current `inventory.md` has many rendering-level declarations assigned to `layout`; review which are observable widget behavior and reconcile them one by one.
- Paired coverage remains open for every main/cross-axis alignment variant, flex baseline without text, RTL/vertical direction, Stack fit/clip modes, Wrap alignments, intrinsic edge constraints, Table vertical alignment, Transform matrices and hit testing, and all `Visibility` preservation options.
- A true clip/shadow/overflow paint comparison requires the orchestrator's pixel batch.
