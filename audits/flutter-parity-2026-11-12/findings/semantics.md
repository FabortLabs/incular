# semantics: Semantic tree and accessibility annotations

Flutter reference: `C:/Manual Installs/flutter/packages/flutter/lib/src` (Flutter 3.41.7).

## Summary

Nine paired frame scenarios inspect semantic nodes directly through Flutter's test `SemanticsOwner` and Incular's retained `SemanticsTree`. Explicit button metadata, default `ExcludeSemantics`, `BlockSemantics`, indexed child metadata, action dispatch, and stale-action rejection match. Three behaviors differ: `MergeSemantics` removes Incular descendants without producing a merged node; an interactive annotation with no `enabled` value is stored as disabled; and scroll nodes keep both directional actions enabled at both scroll boundaries. The AccessKit projection emits the default-disabled button as disabled despite its advertised activation action.

The semantic snapshots are sampled into `h.value` after mount; frame 0 is a zero-valued baseline, and frame 1 (16 ms) contains the initial semantic tree. The scroll scenario additionally samples the same tree at the maximum offset on frame 3 (48 ms). All nine scenarios have matching probe geometry, transform, size, opacity, and fill after the fixtures are aligned. No shared recorder changes were needed.

## Coverage

| Flutter behavior | Incular counterpart | Method / scenario | Result |
| --- | --- | --- | --- |
| Explicit button label, enabled, selected, checked, and tap-action metadata | `Semantics` label/role/state/action | `semantics__properties_state` | Match: one `Ready` Button node with all four true states and Activate/tap advertised. |
| An omitted `Semantics.enabled` value | Default `SemanticState.enabled` and AccessKit mapping | `semantics__enabled_default` | Divergent: Flutter leaves the enabled flag unspecified; Incular stores false and projects the interactive node as disabled. Both advertise tap. |
| Default subtree exclusion | `ExcludeSemantics::new` | `semantics__exclude` | Match: `Visible` is present once and `Hidden` is absent. |
| Descendant label aggregation and exposed-node count | `MergeSemantics` | `semantics__merge` | Divergent: Flutter exposes one node labelled `North\nSouth`; Incular retains no semantic nodes for the merged subtree. |
| Blocking earlier painted semantic siblings | `BlockSemantics` | `semantics__block_previous` | Match: `Behind` is removed and the dialog node remains. |
| Explicit child index | `IndexedSemantics` | `semantics__indexed` | Match: index 7 is attached to the indexed semantic node. |
| Automatic list scroll node, position, extent, child count, and directional actions | Flutter `ListView` / Incular `ListView` | `semantics__scrollable` | Position, maximum extent, one scroll node, and child count 12 match. Flutter removes the exhausted direction at each edge; Incular advertises both directions at both edges. |
| Semantic activation callback dispatch | `Semantics.onTap` / `Semantics::on_tap` | `semantics__action_dispatch` | Match: dispatching the tap action calls the callback once on each side. |
| Action to a removed semantic node | `SemanticsOwner.performAction` / `Runtime::dispatch_semantic_action` | `semantics__stale_action` | Match: removing the node then dispatching its old ID leaves the callback count at zero; Incular rejects the stale ID. |

The merge scenario counts Flutter nodes as the platform-facing tree does: it stops descending below `mergeAllDescendantsIntoThisNode`. Flutter's `SemanticsNode._addToUpdate` serializes a merge boundary without hit-test children, while `getSemanticsData` aggregates descendant labels. On Incular, the count uses `SemanticsTree::iter()` across every retained node; its zero is not a test-finder miss or a filter that hides the children.

The scroll scenario reads the scroll node after layout and after jumping to its maximum offset. Flutter's axis-down mapping makes `scrollUp` the forward action and `scrollDown` the backward action. Incular's `ScrollForward`/`ScrollBackward` actions map to those directions, but the retained tree currently advertises both regardless of the controller's offset.

## Findings

### semantics-001 — MergeSemantics drops descendant semantics

- Severity: P1
- Category: semantics
- Flutter: `widgets/basic.dart:8113-8126` documents one merged semantic node and newline-separated labels; `rendering/proxy_box.dart:4379-4389` sets the merge boundary. `semantics/semantics.dart:3748-3753, 3809-3884` builds merged node data, `_concatAttributedString` joins labels with a newline at `:6884-6905`, and `_addToUpdate` emits no child IDs for a merging node at `:4008-4045`.
- Incular: `crates/incular-widgets/src/semantics.rs:430-452` converts `MergeSemantics` to `widget.merge_semantics()`. In `crates/incular-widgets/src/tree/semantics.rs:235-244`, a merge-descendants widget returns no semantic children. The merge wrapper has no role of its own; only role-bearing elements are collected into nodes at `:757-793`.
- Evidence: `semantics__merge`; initial probe geometry and fill match. At frame 1 (16 ms), Flutter exposes one node with label `North\nSouth` (11 characters); Incular's retained tree has zero nodes and no aggregate label. The two child semantics carry `North` and `South`, each with Button role, so the fixture has meaningful descendant semantics on both sides.
- Impact: putting a checkbox, label, and related control under Incular `MergeSemantics` can remove the whole group from the accessibility tree, leaving no spoken label or semantic control.
- Likely cause: Incular treats “merge descendants” as “do not traverse descendants” but does not aggregate their labels, state, role, or actions onto a node for the wrapper.
- Fix direction: build one semantic node from the merged descendants, matching Flutter's label joining and its merge rules for roles, state, and actions.
- Confidence: high.

### semantics-002 — Unspecified enabled state is projected as disabled

- Severity: P1
- Category: default-value
- Flutter: `semantics/semantics.dart:1732-1738` makes `SemanticsProperties.enabled` nullable; `rendering/object.dart:4805-4806` writes `config.isEnabled` only when that property is non-null. The scenario supplies a Button role and `onTap` but omits `enabled`, so the Flutter node has `Tristate.none` for enabled and still advertises tap.
- Incular: `crates/incular-semantics/src/lib.rs:130-137` stores `enabled` as a plain `bool`, whose default is false. `crates/incular-widgets/src/semantics.rs:46-62` initializes that default; `.button(true)` sets the role without enabling it at `:261-267`, while `.on_tap` adds Activate at `:310-314`. The AccessKit adapter sets disabled for every interactive role whose state is false at `crates/incular-accessibility/src/lib.rs:805-806`.
- Evidence: `semantics__enabled_default`; both trees contain a Button labelled `Action` and both advertise Activate/tap. At frame 1 (16 ms), `value.semantics.enabled_state` is `2` in Flutter (`Tristate.none`) and `0` in Incular (`false`).
- Impact: an application that follows Flutter's default by omitting `enabled` can expose an actionable Incular button to AccessKit as disabled.
- Likely cause: the retained semantic state cannot distinguish “not specified” from “explicitly disabled,” and the native adapter interprets the default false as a user-disabled control.
- Fix direction: preserve the unspecified state and set AccessKit disabled only for an explicit false; keep explicit true/false states distinct.
- Confidence: high.

### semantics-003 — Scroll actions remain advertised at exhausted edges

- Severity: P2
- Category: behavior
- Flutter: `widgets/scroll_position.dart:755-771` maps the logical forward/backward directions to axis-specific actions and includes each action only while the current offset is not already at that edge.
- Incular: `crates/incular-widgets/src/tree/semantics.rs:675-702` creates both `ScrollForward` and `ScrollBackward` actions for a `Scroll` or `SliverViewport` without checking the controller's current offset against its extents.
- Evidence: `semantics__scrollable`; the scroll node count, position, maximum extent (600 px), and child count (12) agree. At frame 1 (16 ms), Flutter has `scrollUp=1, scrollDown=0`, while Incular has both actions. At frame 3 (48 ms), after both controllers reach 600 px, Flutter has `scrollUp=0, scrollDown=1`, while Incular still advertises both.
- Impact: assistive technology offers a scroll direction that cannot move the list at either boundary.
- Likely cause: Incular publishes a fixed action vector for a scroll node rather than deriving actions from the current offset and scroll extent.
- Fix direction: update the retained action set when the offset or extents change, using the axis-aware forward/backward mapping from Flutter.
- Confidence: high.

## Pixel scene proposals

None. Semantic node content and state are not visible in static pixels.

## Harness requests

None for the checked behaviors. The tests query Flutter's semantics owner and Incular's retained semantics tree through existing scenario APIs and record numeric summaries with `h.value`.

## Open questions

- `SliverSemantics` and `SemanticsDebugger` were not exercised against Flutter in this pass.
- The full `Semantics` property surface remains unchecked: sort/traversal keys, custom actions, dismiss/expand/collapse, route scopes, text selection and editing actions, live regions, accessibility focus, and role-specific properties.
- Text-span and custom-painter semantic annotations, the `hasImplicitScrolling` flag, semantic indices in lazy/partially materialized lists, and general semantic update/reparent/disposal lifecycles were not compared. The list scenario covers scroll position/extents, child count, and edge actions; the retained node model has no direct `hasImplicitScrolling` field.
- This pass checks retained-tree values and the AccessKit disabled projection from source; it does not run VoiceOver, TalkBack, or other native assistive technology.

## Validation

- `flutter test test/frames/semantics_test.dart`: all nine Flutter scenarios passed.
- `INCULAR_PARITY=1 PARITY_SCENARIO=semantics__ cargo test -p incular-parity-harness --test frames`: all nine Incular scenarios ran; six matched and the three divergences above were reported.
- No pixel harness or broad workspace test suite was run.
