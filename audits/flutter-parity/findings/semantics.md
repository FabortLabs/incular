# semantics: Semantic tree and accessibility annotations

Flutter reference: `C:/Manual Installs/flutter/packages/flutter/lib/src` (Flutter 3.41.7).
Revalidated at HEAD `a19bf487`. Earlier findings were recorded at `a425aa2e`; `tree/semantics.rs` changed since (incremental semantic pass, retained child lists, sibling blockers), and all three earlier findings still reproduce.

## Summary

Nineteen paired frame scenarios now inspect semantic nodes through Flutter's test `SemanticsOwner` and Incular's retained `SemanticsTree`. Nine match: explicit button metadata and state, default `ExcludeSemantics`, `BlockSemantics`, indexed child metadata, action dispatch, stale-action rejection, Offstage and Visibility exclusion (including `maintainSemantics`), roles and flags (header, link, image, toggled, hidden), and the increase/decrease action flags with callback dispatch.

Nine diverge. Three earlier findings reproduce (`semantics-001` merge, `semantics-002` enabled default, `semantics-003` scroll edge actions). Four are new scenario-backed findings: a label-only container node is dropped (`semantics-004`), `onTap` implies a Button role (`semantics-005`), hint and tooltip share one field (`semantics-006`), and a `GestureDetector` tap is not exposed to assistive technology (`semantics-007`). `Opacity` with alpha 0 keeps child semantics in Incular; that is already recorded as `implicit-004` and the scenario `semantics__opacity_zero` is evidence for it, not a new finding.

Source-only missing features are recorded as `semantics-008` to `semantics-014` and `semantics-016`: long press, increased and decreased values, sort keys, show-on-screen, container and explicit child nodes, custom-painter semantics, and text-span semantics labels.

## Coverage

Rows are in `audits/flutter-parity/coverage/semantics.jsonl` (24 rows; one is unchecked with a reason).

| Flutter behavior | Incular counterpart | Method / scenario | Result |
| --- | --- | --- | --- |
| Explicit button label, enabled, selected, checked, tap | `Semantics` builder | `semantics__properties_state` | Match |
| Omitted `enabled` | `SemanticState::enabled` | `semantics__enabled_default` | Diverges (semantics-002) |
| Default subtree exclusion | `ExcludeSemantics` | `semantics__exclude` | Match |
| Descendant merge | `MergeSemantics` | `semantics__merge` | Diverges (semantics-001) |
| Blocking earlier siblings | `BlockSemantics` | `semantics__block_previous` | Match |
| Explicit child index and scroll child count | `IndexedSemantics`, `ListView` | `semantics__indexed`, `semantics__scrollable` | Match for index and count |
| Scroll position, extent and edge actions | `ListView` | `semantics__scrollable` | Diverges on actions (semantics-003) |
| Tap dispatch and stale id | `Semantics::on_tap`, runtime dispatch | `semantics__action_dispatch`, `semantics__stale_action` | Match |
| Label-only container node | `Semantics` without role | `semantics__container_label_only` | Diverges (semantics-004) |
| Role implied by `onTap` | `Semantics::on_tap` | `semantics__container_label_only` | Diverges (semantics-005) |
| Value, hint and tooltip | `Semantics::hint`, `tooltip`, `value` | `semantics__value_hint_tooltip` | Value matches; hint and tooltip diverge (semantics-006) |
| Increase and decrease actions | `Semantics::on_increase`, `on_decrease` | `semantics__increase_decrease` | Flags and dispatch match; value fields missing (semantics-009) |
| Header, link, image, toggled, hidden flags | `Semantics` builder roles and state | `semantics__roles_flags` | Match |
| Opacity 0 removes child semantics | `Opacity` | `semantics__opacity_zero` | Diverges; evidence for implicit-004 |
| Offstage and Visibility exclusion | `Offstage`, `Visibility` | `semantics__offstage_visibility` | Match, including `maintainSemantics` |
| Live region flag | `Semantics::live_region` | `semantics__live_region` | Match |
| setText on a text field | `EditableText` text-field action | `semantics__set_text` | Diverges: Flutter advertises setText only when focused (semantics-015) |
| Horizontal scroll actions | `ListView` on horizontal axis | `semantics__horizontal_scroll` | Diverges on actions, as semantics-003 (position and extent match) |
| `GestureDetector` onTap as semantic action | `GestureDetector` | `semantics__gesture_tap` | Diverges (semantics-007) |
| Long press action | none | source | Missing (semantics-008) |
| `increasedValue` / `decreasedValue` | none | source | Missing (semantics-009) |
| `sortKey` / `OrdinalSortKey` | none | source | Missing (semantics-010) |
| `showOnScreen` | none | source | Missing (semantics-011) |
| `container`, `explicitChildNodes` | none | source | Missing (semantics-012) |
| `CustomPainter` semantics builder | `CustomPaint` | source | Missing (semantics-013) |
| `TextSpan.semanticsLabel` | span model | source | Missing (semantics-014) |
| `SemanticsDebugger`, debug dumps | n/a | source | Out of scope (debug tooling) |
| `PlaceholderSpanIndexSemanticsTag` | WidgetSpan | none | Missing: WidgetSpan has no child widget (semantics-016) |

Actions reviewed by reading but not scenario-checked: `setText` and `setSelection` on text fields (`tree/semantics.rs` gates `SetText` on enabled and not read-only, which matches Flutter's rule), `scrollUp`/`scrollDown` on horizontal axes, and `showOnScreen`.

## Scenarios

Match: `semantics__properties_state`, `semantics__exclude`, `semantics__block_previous`, `semantics__indexed`, `semantics__action_dispatch`, `semantics__stale_action`, `semantics__offstage_visibility`, `semantics__increase_decrease`, `semantics__roles_flags`, `semantics__live_region`.

Diverge: `semantics__enabled_default` (002), `semantics__merge` (001), `semantics__scrollable` (003), `semantics__horizontal_scroll` (003, horizontal axis), `semantics__container_label_only` (004 and 005), `semantics__value_hint_tooltip` (006), `semantics__opacity_zero` (implicit-004), `semantics__gesture_tap` (007), `semantics__set_text` (015).

Scenario notes:
- `semantics__container_label_only` records a labelled node with onTap (node count, tap action, Button flag) and a second labelled container with no action (node count). Only the label-only count and the Button flag diverge.
- `semantics__offstage_visibility` uses `maintainSize`, `maintainState` and `maintainAnimation` with `maintainSemantics` on the third child, because Flutter asserts `maintainSize || !maintainSemantics`. Incular's twin sets the same four flags, so both sides lay out the same row.
- `semantics__increase_decrease` sets `increasedValue` and `decreasedValue` on the Flutter side, because Flutter's debug assertion requires them with a non-empty value. The Incular twin has no such fields, which is the gap in semantics-009; the scenario compares only the action flags and callback counts.
- `semantics__value_hint_tooltip` records lengths. Incular's `state.tooltip` is always empty, and its description holds the tooltip (length 6, Flutter hint length 14).
- `semantics__gesture_tap` records the count of tap-action nodes and the callback count after dispatch. Flutter has one node and one call; Incular has no node, so no dispatch happens.

All twins use the same geometry, transform and fill. Frame 0 is the zero baseline; values are sampled after mount and recorded at frame 1.

## Findings

### semantics-001 — MergeSemantics drops descendant semantics

- Severity: P1. Category: semantics. Impact class: wrong-final-state. Status: accepted (revalidated).
- Flutter: `widgets/basic.dart:8113-8126` documents one merged node and newline-joined labels; `rendering/proxy_box.dart:4379-4389` sets the merge boundary; `semantics/semantics.dart:3748-3884` builds merged node data and `:4008-4045` emits no child IDs for a merging node.
- Incular: `crates/incular-widgets/src/semantics.rs:446-453` converts `MergeSemantics` to `merge_semantics()`; `crates/incular-widgets/src/tree/semantics.rs:257-265` returns no children for a merge_descendants widget; role-bearing elements are emitted at `:818-832`.
- Evidence: `semantics__merge`, frame 1 (16 ms). Flutter: one node, label `North\nSouth` (11 characters). Incular: zero nodes.
- Impact: a group under `MergeSemantics` can disappear from the accessibility tree, with no spoken label or actionable control.
- Cause: merge is implemented as "do not traverse", with no aggregate node.
- Fix direction: build one node for the merging wrapper from its descendants, following Flutter's newline label join and its merge rules for role, state and actions.
- Confidence: high. Limitations: only label count and length were sampled.

### semantics-002 — Unspecified enabled state is projected as disabled

- Severity: P1. Category: default-value. Impact class: wrong-final-state. Status: accepted (revalidated).
- Flutter: `semantics/semantics.dart:1732-1738` makes `enabled` nullable; `rendering/object.dart:4805-4806` writes it only when set.
- Incular: `crates/incular-semantics/src/lib.rs:131-133` stores `enabled` as `bool`; `crates/incular-widgets/src/semantics.rs:48-66` initialises it to false; `:262-266` and `:310-315` set the Button role and Activate without enabling; `crates/incular-accessibility/src/lib.rs:805-807` sets disabled whenever an interactive role has `enabled == false`.
- Evidence: `semantics__enabled_default`, frame 1. Flutter `enabled_state` is 2 (Tristate.none); Incular is 0. Both advertise tap.
- Impact: an app that leaves `enabled` unspecified, as Flutter allows, exposes an actionable button to AccessKit as disabled.
- Cause: unspecified and explicitly disabled are the same state.
- Fix direction: represent the unspecified state separately (for example `Option<bool>`), and set disabled only for an explicit false.
- Confidence: high.

### semantics-003 — Scroll actions remain advertised at exhausted edges

- Severity: P2. Category: behavior. Impact class: edge-case. Status: accepted (revalidated).
- Flutter: `widgets/scroll_position.dart:755-771` maps the logical directions to axis actions and includes each only when the offset is not at that edge.
- Incular: `crates/incular-widgets/src/tree/semantics.rs:730-757` creates both `ScrollForward` and `ScrollBackward` for a scroll or sliver viewport regardless of offset.
- Evidence: `semantics__scrollable`. Position 0, maximum extent 600 and child count 12 match. Frame 1: Flutter scroll_up 1, scroll_down 0; Incular both. Frame 3 (at maximum): Flutter scroll_up 0, scroll_down 1; Incular scroll_up still 1.
- Impact: assistive technology offers a direction that cannot move the list at that edge.
- Cause: the action list is fixed rather than derived from offset and extent.
- Fix direction: recompute the advertised actions when offset or extent changes, with Flutter's axis-aware mapping.
- Confidence: high. Limitations: vertical axis only.

### semantics-004 — Label-only Semantics without a role or action is dropped

- Severity: P1. Category: semantics. Impact class: missing-feature. Status: pending-review.
- Flutter: `widgets/basic.dart:7942` (Semantics); `rendering/proxy_box.dart:4309-4339` (RenderSemanticsAnnotations); `semantics/semantics.dart:5116-5186` (configuration, including `container`).
- Incular: `crates/incular-widgets/src/semantics.rs:363-364` (`explicit()` returns None when there is no role) and `:381-402` (label and description only reach a node through the explicit path); `crates/incular-widgets/src/tree/semantics.rs:655-656` (no role for unknown kinds) and `:818-832` (a node is emitted only with a role).
- Evidence: `semantics__container_label_only`, frame 1. Flutter has one node labelled `Label only`; Incular has zero.
- Impact: labelled containers that carry no control role (a labelled region, a summary) are absent from the accessibility tree.
- Cause: node creation requires a role, and the label fallback runs only inside roled nodes.
- Fix direction: emit a generic container node when a Semantics wrapper has a label, value, description, flag or action.
- Confidence: high. Limitations: Flutter's non-container merge into an ancestor was not compared (see semantics-012).

### semantics-005 — onTap and onIncrease imply Button and Slider roles

- Severity: P2. Category: edge-case. Impact class: edge-case. Status: pending-review.
- Flutter: `semantics/semantics.dart:2309` (`onTap` is a separate property from `button`).
- Incular: `crates/incular-widgets/src/semantics.rs:310-313` (`on_tap` calls `role.get_or_insert(Button)`); `:317-321` (`on_increase` calls `get_or_insert(Slider)`).
- Evidence: `semantics__container_label_only`, frame 1. Flutter `tap_is_button` 0; Incular 1. The tap node count and tap action matched.
- Impact: screen readers announce a control type the app did not declare (for example "button" for a plain tappable region), and the role changes if the app later sets one.
- Cause: setting a callback changes the role as a side effect.
- Fix direction: attach the action without touching the role; the app specifies the role.
- Confidence: high. Limitations: the increase-to-slider role was not scenario-checked.

### semantics-006 — Hint and tooltip share one description field; state.tooltip is never set

- Severity: P2. Category: semantics. Impact class: wrong-final-state. Status: pending-review.
- Flutter: `semantics/semantics.dart:2193` (hint), `:2219` (tooltip).
- Incular: `crates/incular-widgets/src/semantics.rs:119-131` (`hint` and `tooltip` both write `description`); `crates/incular-accessibility/src/lib.rs:836-838` (native tooltip reads `state.tooltip`, which the builder never sets).
- Evidence: `semantics__value_hint_tooltip`, frame 1. Hint length: Flutter 14, Incular 6 (the tooltip string `Volume` replaced the hint). Tooltip length: Flutter 6, Incular 0. Value length matched (3).
- Impact: a hint is lost when a tooltip is also set, and the tooltip reaches the platform as a description rather than as a tooltip.
- Cause: three builders write one field.
- Fix direction: keep hint, tooltip and description as separate fields, and have the builder write `state.tooltip`.
- Confidence: high. Limitations: lengths compared, not strings.

### semantics-007 — GestureDetector onTap is not exposed as a semantic tap action

- Severity: P1. Category: missing-feature. Impact class: missing-feature. Status: pending-review.
- Flutter: `widgets/gesture_detector.dart:1575-1600` (`_updateSemanticsForRenderObject` attaches the tap action); `semantics/semantics.dart:2309` (`onTap`).
- Incular: `crates/incular-widgets/src/gestures.rs:32-60` (GestureDetector stores callbacks only; no semantic contribution); `crates/incular-widgets/src/tree/semantics.rs:655-690` (roles come from widget kinds; no gesture arm).
- Evidence: `semantics__gesture_tap`. Flutter: one node with the tap action; dispatching it calls `onTap` once (frame 2). Incular: no node with an Activate action, so nothing is dispatched.
- Impact: any control built only from a GestureDetector is invisible to assistive technology and cannot be activated through it.
- Cause: pointer callbacks are not mirrored into the semantic action set.
- Fix direction: contribute tap and long-press actions from `GestureDetector` and `RawGestureDetector`, creating a generic node when needed, as Flutter's `RenderSemanticsGestureHandler` does.
- Confidence: high. Limitations: tap only; drag and scroll gestures were not compared.

### semantics-008 — No long-press semantic action

- Severity: P2. Category: missing-feature. Impact class: missing-feature. Status: pending-review.
- Flutter: `semantics/semantics.dart:2325` (`onLongPress`); `widgets/gesture_detector.dart:1575-1600`.
- Incular: `crates/incular-semantics/src/lib.rs:46-55` (`SemanticActionKind` has no long press); `crates/incular-widgets/src/gestures.rs:107-112` (`on_long_press` has no semantic callback).
- Evidence: source only. rg for `LongPress` and `long_press` across the semantics, widgets and accessibility crates finds only the gesture callbacks.
- Impact: long-press-only actions (such as an item context menu) have no accessibility action.
- Cause: the portable action enum omits `SemanticsAction.longPress`, and the gesture layer does not feed it.
- Fix direction: add a LongPress kind and a `Semantics::on_long_press`, and map `GestureDetector` long press as in semantics-007.
- Confidence: high. Limitations: no scenario is possible without an API.

### semantics-009 — increasedValue and decreasedValue are not supported

- Severity: P2. Category: missing-feature. Impact class: missing-feature. Status: pending-review.
- Flutter: `semantics/semantics.dart:2125` (`increasedValue`); a debug assertion requires these when an adjust action is available with a non-empty value.
- Incular: `crates/incular-widgets/src/semantics.rs:317-330` (`on_increase` and `on_decrease` set only callbacks); `crates/incular-semantics/src/lib.rs:131-190` (one `value` field; no next or previous value).
- Evidence: source. `semantics__increase_decrease` matched on action flags and dispatch counts, so the action path works; the value fields are the gap.
- Impact: a stepper or slider cannot announce what the next increment or decrement will produce.
- Cause: the node model has no fields for these values.
- Fix direction: add optional increased and decreased values to the semantic state, set them from the builders, and project them to the native adapter.
- Confidence: high.

### semantics-010 — No semantic sort key or ordinal traversal order

- Severity: P2. Category: missing-feature. Impact class: missing-feature. Status: pending-review.
- Flutter: `semantics/semantics.dart:2269` (`sortKey`); `:6924` (`SemanticsSortKey`); `:7000` (`OrdinalSortKey`).
- Incular: `crates/incular-semantics/src/lib.rs:131-190` (no order field); `crates/incular-widgets/src/tree/semantics.rs:257-290` (children follow widget order).
- Evidence: source. rg for `sort_key`, `sortKey`, `ordinal` and `OrdinalSortKey` finds nothing in the semantics, widgets or accessibility crates.
- Impact: screen-reader order follows the widget tree, and an app cannot correct a difference between visual and reading order.
- Cause: no ordering key exists, and siblings are not sorted.
- Fix direction: add an optional ordinal key to the semantic properties, and sort siblings by it (stable, widget order as tiebreaker) as Flutter does.
- Confidence: medium. Limitations: sorted output was not compared against Flutter.

### semantics-011 — showOnScreen action is not available

- Severity: P2. Category: missing-feature. Impact class: missing-feature. Status: pending-review.
- Flutter: `semantics/semantics.dart:2758` and `:2800` (the node's `showOnScreen` handler, which scrollables set so assistive technology can reveal an off-screen node).
- Incular: `crates/incular-semantics/src/lib.rs:46-55` (no show-on-screen kind); the scroll views only handle ScrollForward and ScrollBackward (page steps).
- Evidence: source.
- Impact: assistive technology cannot ask a scroll view to reveal an off-screen item before focusing it.
- Cause: the action is absent from the portable set.
- Fix direction: add a ShowOnScreen action routed to the scroll controller to reveal the target's bounds.
- Confidence: medium. Limitations: no scenario exercises reveal.

### semantics-012 — container and explicitChildNodes have no Incular equivalent

- Severity: P2. Category: missing-feature. Impact class: missing-feature. Status: pending-review.
- Flutter: `widgets/basic.dart:7942` (`container`); `semantics/semantics.dart:5129` and `:5186` (`explicitChildNodes`).
- Incular: `crates/incular-widgets/src/semantics.rs:12-37` (no container or explicit-child flag); `:381-402` (node creation is role-driven, see semantics-004).
- Evidence: source. Flutter's default non-container annotation merges into the nearest ancestor node; Incular has no equivalent rule.
- Impact: apps cannot choose whether a labelled wrapper is its own node, so grouping differs from Flutter's default merge.
- Cause: no container concept.
- Fix direction: add `container` and `explicit_child_nodes`, after semantics-004 is fixed.
- Confidence: medium. Limitations: Flutter's merge outcome for a non-container label was not run.

### semantics-013 — CustomPaint has no per-region semantics

- Severity: P2. Category: missing-feature. Impact class: missing-feature. Status: pending-review.
- Flutter: `rendering/custom_paint.dart:307` (`CustomPainterSemantics`; `CustomPainter.semanticsBuilder`).
- Incular: `crates/incular-widgets/src/painting_effects/custom_paint.rs` (no semantic output; rg finds none).
- Evidence: source. The workaround is wrapping the whole painted area in `Semantics`.
- Impact: meaningful painted parts (chart bars, custom controls) are unreachable unless the app wraps each one in its own widget.
- Cause: painters have no semantic output in the Incular API.
- Fix direction: an optional semantic region builder on the custom paint descriptor, producing child nodes with bounds and labels.
- Confidence: medium. Limitations: no custom-paint semantics scene exists in the harness.

### semantics-014 — TextSpan has no semanticsLabel

- Severity: P2. Category: missing-feature. Impact class: missing-feature. Status: pending-review.
- Flutter: `painting/inline_span.dart:58` (`InlineSpanSemanticsInformation`), `:129` (`combineSemanticsInfo`); `TextSpan.semanticsLabel`.
- Incular: `crates/incular-text/src/spans.rs` (no semantics field, no combine step; rg for `semanticsLabel` finds only unrelated image and menu labels).
- Evidence: source.
- Impact: a span whose glyphs read badly (symbols, abbreviations) cannot be given a spoken replacement inside rich text.
- Cause: the span model carries no semantics information.
- Fix direction: an optional semantics label on spans, combined into the paragraph node label as Flutter does.
- Confidence: medium. Limitations: the combined label shape was not compared.

### Opacity semantics (existing: implicit-004)

Not a new finding. `semantics__opacity_zero` records that Flutter's `RenderOpacity` drops child semantics at alpha 0 (`rendering/proxy_box.dart:956-960`) and Incular keeps them (Incular faded node count 1, Flutter 0). `tree/semantics.rs:935-941` (`semantics_hidden`) does not consider opacity. This is evidence for `implicit-004`.

### semantics-015 — SetText is advertised on an unfocused text field

- Severity: P3. Category: behavior. Impact class: edge-case. Status: pending-review.
- Flutter: `rendering/editable.dart:1390-1391` sets `onSetText` only when `hasFocus && !readOnly`.
- Incular: `crates/incular-widgets/src/tree/semantics.rs:700-704` adds SetText when the field is enabled and not read-only; `:960-962` keeps the action for a TextField.
- Evidence: `semantics__set_text`, frame 1. Flutter `set_text_nodes` 0 (no setText action on the unfocused field, so the twin's dispatch is skipped and the text stays empty). Incular `set_text_nodes` 1, and dispatch sets the text to `hello` (length 5 at frame 2; Flutter 0).
- Impact: assistive technology can write into a field the user has not focused. Flutter does not allow that.
- Cause: the action gate checks enabled and read-only but not focus.
- Fix direction: advertise SetText only when the field is focused and editable.
- Confidence: high. Limitations: only the unfocused state was compared. The focused state (where Flutter does advertise setText) was not run, because the twin would need a focus API that was not located.

### semantics-016 — Inline WidgetSpan has no child widget

- Severity: P2. Category: missing-feature. Impact class: missing-feature. Status: pending-review.
- Flutter: `rendering/paragraph.dart:55` (`PlaceholderSpanIndexSemanticsTag`); `painting/inline_span.dart` (WidgetSpan child).
- Incular: `crates/incular-text/src/spans.rs:13-17` (`WidgetSpan` holds an id, size and alignment only).
- Evidence: source. rg for `PlaceholderSpan` and placeholder semantics found no counterpart. No frame scenario exists, because no widget can be placed inline in a span.
- Impact: a widget placed inline in rich text (an icon or badge) cannot contribute its own accessibility node in reading order.
- Cause: the span reserves a placeholder by id and size, with no child widget and no semantic index.
- Fix direction: give WidgetSpan an optional child that is laid out at the placeholder and contributes semantics at its index in the paragraph.
- Confidence: medium. Limitations: an app may place a widget at the placeholder id outside the span. That path was not traced to confirm it has no semantics.

## Pixel scene proposals

None. Semantic structure is not visible in pixels.

## Harness requests

None for the checked behaviors. Possible future work, which needs no shared-file change:
- A paired scenario for `GestureDetector` long press once semantics-008 is addressed.
- A custom-paint semantic region scene once the API exists.
- A focused-field twin for semantics-015 once a focus request is available to both sides.

## Open questions

- Source-only items semantics-008 to -014 and -016 have no Incular API, so no paired scenario is possible; they stay source-only until the API exists.
- Focus-driven semantics (focused flag, focus action, reveal on focus) were not scenario-checked.
- `readOnly`, `obscured` and custom actions were not scenario-checked.
- `SliverSemantics` grouping was not compared.
- Native assistive technology (VoiceOver, TalkBack, Narrator) was not run; AccessKit projection is checked by source only.

## Validation

- `harness/run-frames.sh semantics__` at HEAD `a19bf487` on the updated recorder: 19 scenarios, 10 match, 9 diverge. Diverging: `enabled_default` (002), `merge` (001), `scrollable` (003), `horizontal_scroll` (003), `container_label_only` (004, 005), `opacity_zero` (implicit-004), `value_hint_tooltip` (006), `gesture_tap` (007), `set_text` (015).
- `flutter analyze test/frames/semantics_test.dart`: no issues. The 16 earlier infos were removed by replacing the deprecated `pipelineOwner` access with a tree walk to the owner that holds the root semantics node (`semanticsOwnerOf` in the twin). Behavior is unchanged: the same owner is chosen and the earlier 16 scenarios produce the same values.
- `cargo check -q -p incular-parity-harness --tests`: passes (warnings in another worker's `host.rs` only).
- `rustfmt --edition 2024` applied to `harness/incular/tests/frames/semantics.rs`.
- JSONL lines for `coverage/semantics.jsonl` and `findings/semantics.jsonl` parse.
- No framework code, shared harness files or commits changed.

## Resume status

Done at resume: the recorder rerun; analyzer infos fixed; twins added for live region, horizontal scroll and setText; semantics-015 (setText focus gate) and semantics-016 (WidgetSpan child) recorded; semantics-003 evidence extended to the horizontal axis; the last unchecked row (PlaceholderSpan) now has a missing status with its finding.

Not done, with reasons: semantics-008 to -014 have no Incular API, so no twin can express them; focus was not twinned because no focus request was located for both sides in time.
