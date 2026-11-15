# semantics fixes

Scenario result after the fixes: `harness/run-frames.sh semantics__` gives 17 of 19 matching. The two remaining divergences are `enabled_default` (raw bool value only; the projection is fixed) and `opacity_zero` (implicit-004, not in this group).

## semantics-001 (fixed): MergeSemantics drops descendants
- Cause: merge was "do not traverse" with no aggregate node.
- Change: a merging element always emits a node; its label joins its own and descendant labels with newlines (`tree/semantics.rs`).
- Tests: `semantics_flutter_parity.rs::merge_semantics_emits_one_node_with_newline_joined_label`. Harness `semantics__merge`: match.
- Limits: descendant actions, flags and values are not merged.

## semantics-002 (fixed in projection; representation residual): unspecified enabled read as disabled
- Cause: the default `enabled` was false.
- Change: `SemanticState` defaults to `enabled = true`; only an explicit `false` disables.
- Tests: `unspecified_enabled_is_not_disabled_and_explicit_false_is`, `default_semantic_state_is_enabled`, accessibility `unspecified_enabled_state_is_not_projected_as_disabled`.
- Harness `semantics__enabled_default`: still diverges in the raw twin value (bool 1 vs Flutter tristate 2).
- API: default changed. Code relying on default-disabled must call `enabled(false)`.

## semantics-003 (fixed): scroll actions at exhausted edges
- Cause: actions were fixed, and scroll offset changes never re-collected the node.
- Change: `scroll_edge_actions` gates forward/backward on offset; `update_semantics` re-marks scroll nodes when the scroll offset stamp changes.
- Harness `semantics__scrollable`, `semantics__horizontal_scroll`: match.

## semantics-004 (fixed): label-only Semantics dropped
- Change: `Semantics::explicit()` returns a GenericContainer descriptor for any metadata; role-less wrappers overlay built-in roles without replacing them.
- Tests: `label_only_semantics_creates_a_generic_container_node`, `semantics_wrapper_without_metadata_creates_no_node`. Harness `semantics__container_label_only`: match.

## semantics-005 (fixed): onTap implies Button
- Change: `on_tap`, `on_increase`, `on_decrease`, and the scroll callbacks no longer set a role.
- Test: `on_tap_does_not_imply_button_role`. Harness `semantics__container_label_only`: match.
- API: apps relying on the implied Button must call `button(true)`.

## semantics-006 (fixed): hint and tooltip share one field
- Change: `tooltip()` writes `state.tooltip`; `hint()` keeps the description.
- Test: `tooltip_does_not_overwrite_hint`. Harness `semantics__value_hint_tooltip`: match.
- Limit: hint and description still share one field.

## semantics-007 (fixed): GestureDetector tap not exposed
- Change: a Gesture element with `on_tap` or `on_long_press` emits a generic node with Activate/LongPress; dispatch falls back to the detector's callback (`tree/retained.rs`, one expression).
- Test: `gesture_detector_tap_is_exposed_and_dispatches_its_callback`. Harness `semantics__gesture_tap`: match.
- Risk: a detector under a role-bearing wrapper may add a nested node; not checked beyond the scenario.

## semantics-008 (fixed in portable model; AccessKit gap): no long-press action
- Change: `SemanticActionKind::LongPress`, `Semantics::on_long_press`, the callback slot in `tree/specs.rs`, the dispatch arm in `incular-runtime/src/frame.rs`, and gesture long press.
- Test: `on_long_press_is_a_semantic_action`. No scenario (source-only).
- API: new enum variants; exhaustive downstream matches need an arm.
- Projection: AccessKit does not advertise it (`map_action` returns None). The mobile projection does.

## semantics-009 (partial): increased and decreased values
- Change: `SemanticState::increased_value` and `decreased_value`, with builders.
- Test: `increased_and_decreased_values_are_retained`. Harness `semantics__increase_decrease`: match (the twin compares action flags only).
- Limit: AccessKit has no property for these values; the mobile projection carries them.

## semantics-010 (deferred): sort key
- Reason: a sort needs a parent's child list re-sorted when a child's key changes, which requires a parent re-mark in `tree/reconciliation.rs` (not owned here).

## semantics-011 (deferred): showOnScreen
- Reason: needs a scroll reveal API in `incular-scroll`, a new action kind, and runtime routing. No scenario.

## semantics-012 (deferred): container and explicitChildNodes
- Reason: Incular has no implicit merge into an ancestor, so these flags would be no-ops until that rule is decided.

## semantics-013 (deferred): CustomPaint region semantics
- Reason: needs a public painter API (regions, bounds, labels) and a harness scene. The Semantics wrapper workaround stays.

## semantics-014 (fixed): TextSpan semanticsLabel
- Change: `TextSpan::semantics_label`, `semantics_text()` (`incular-text/src/spans.rs`, additive); `RichText` conversion applies the combined label (`tree/descriptors.rs`).
- Test: `span_semantics_labels_combine_into_the_spoken_text` in `incular-text/tests/spans.rs`. No scenario.

## semantics-015 (fixed): SetText on an unfocused field
- Change: SetText is advertised only when the field is enabled, not read-only, and focused.
- Tests: `editable_text.rs::semantic_text_selection_and_actions` updated to assert absence while unfocused and presence once focused. Harness `semantics__set_text`: match after the twin change.
- Limit: the focused state is not twinned.
