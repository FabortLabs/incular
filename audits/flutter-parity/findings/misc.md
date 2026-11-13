# misc: Other neutral widgets

Revision: `a19bf487`. Scope: the 294 `widgets/` rows in `coverage/misc.jsonl`. Findings: `findings/misc.jsonl` (misc-001 to misc-014).

## Summary

Frame parity: 9 of 14 misc scenarios match. Five divergences reproduce at `a19bf487`: Expansible body animation (misc-001), RotatedBox odd-turn size (misc-002), MediaQuery size-aspect rebuilds (misc-003), Form.save with an invalid field (misc-004) and nested SafeArea (misc-008). Placeholder fallback sizing matches through a public-widget composition. Source review added misc-005 to misc-014: RawAutocomplete interaction, the InheritedWidget contract, generic FormField state, KeyedSubtree keys, CustomPaint fill sizing, GlobalKey reparenting, restorable property types, the unexported Placeholder, and missing State lifecycle hooks.

## Coverage

Row statuses in `coverage/misc.jsonl` (294 rows): 11 scenario, 7 scenario+reading, 6 partial-scenario, 37 reading, 61 partial-reading, 41 missing, 131 out-of-scope, 0 unchecked.

| Area | Rows | Statuses | Notes |
| --- | --- | --- | --- |
| Async builders (`async.dart`) | 5 | scenario / scenario+reading / partial-scenario | Future and stream scenarios match; replacement and cancel unchecked |
| Forms and autocomplete (`form.dart`, `autocomplete.dart`) | 10 | scenario, partial-scenario, partial-reading, missing | misc-004, 005, 007 |
| Inherited data and environment (`inherited_*`, `media_query.dart`, `localizations.dart`, `orientation_builder.dart`, `lookup_boundary.dart`, `shared_app_data.dart`, `sensitive_content.dart`) | 24 | scenario, partial-scenario, reading, missing | misc-003, misc-006 |
| Framework element and widget base classes (`framework.dart`) | 35 | out-of-scope, missing, partial-reading, reading | Element hierarchy is internal (ARCHITECTURE principle 8); GlobalKey and State hooks missing (misc-011, misc-014) |
| Basic layout (`basic.dart`) | 13 | scenario, scenario+reading, partial-reading, out-of-scope | Builder, ColoredBox, KeyedSubtree, LimitedBox, RotatedBox, Spacer-related rows |
| Safe area, orientation, placeholder, spacer, directionality | 8 | scenario, partial-scenario | misc-008, misc-013 |
| Animation-related (`repeating_animation_builder.dart`, `implicit_animations.dart` tweens, `ticker_provider.dart`) | 13 | scenario, partial-reading, missing | Repeat builder matches; tween rows owned by implicit; tickers by anim_core |
| Keys, restoration (`restoration*.dart`, `page_storage.dart`, `undo_history.dart`) | 31 | reading, partial-reading, missing | misc-012; page storage reassigned to navigation |
| Overlays and menus (`raw_menu_anchor.dart`, `raw_tooltip.dart`, `tap_region.dart`, `modal_barrier.dart`, `routes.dart`) | 24 | reading, partial-reading, missing | Routes and modal barrier reassigned to navigation |
| Radio, toggle, magnifier, icon, banner, misc widgets | 40 | reading, partial-reading, out-of-scope | Material-only items out of scope |
| Platform menu, platform view, view, app, binding | 37 | partial-reading, out-of-scope, missing | Embedder plumbing out of scope; WidgetsApp partial |
| Windowing, debug, inspector, service extensions | 60 | out-of-scope | Native windowing, debug tooling and DevTools protocol |
| Widget state (`widget_state.dart`) | 10 | partial-reading, missing | Rules live only in incular-material (see Harness requests) |

Out-of-scope rows give a reason in `coverage`. Rows owned by other packages carry `reassign`. Reading-based rows cite the Incular `path:line` read and the Flutter line compared.

## Scenarios

| Scenario | Result | Notes |
| --- | --- | --- |
| misc__expansible_height_animation | diverges, 17 frames | misc-001: 120x32 vs 120x112 at 48 ms |
| misc__rotated_box_layout_size | diverges, 2 frames | misc-002: 40x80 vs 80x40 at frame 0 |
| misc__spacer_flex_shares | match, 2 frames | |
| misc__inherited_model_aspects | match, 4 frames | |
| misc__media_query_size_aspect | diverges, 3 frames | misc-003: builder runs 1 vs 2 at 32 ms |
| misc__form_save_invalid_field | diverges, 4 frames | misc-004: saved count 1 vs 0 at 48 ms |
| misc__form_field_lifecycle | match, 7 frames | |
| misc__future_builder_keyed_reorder | match, 4 frames | |
| misc__stream_builder_events | match, 5 frames | |
| misc__safe_area_nested_consumption | diverges, 3 frames | misc-008: 100x70 vs 100x90 at 32 ms |
| misc__directionality_row_order | match, 3 frames | |
| misc__repeating_builder_reverse_pause | match, 23 frames | Reverse mode across pause and resume |
| misc__orientation_builder_constraints | match, 5 frames | Constraint-based orientation at two sizes |
| misc__placeholder_fallback_size | match, 2 frames | LimitedBox(400x400) with probes on the filled box; Placeholder itself not exported (misc-013) |

## Findings

Full records are in `findings/misc.jsonl`.

### misc-001 — Expansible adds and removes its body without animation

- Severity: P1. Category: behavior (visible-motion).
- Flutter: `widgets/expansible.dart:193-197,230,235,356-374,410-449`. Incular: `crates/incular-widgets/src/utilities.rs:13-48`.
- Evidence: scenario `misc__expansible_height_animation`. Frame 3 (48 ms): Flutter 120x32, Incular 120x112; diverges in 13 of 17 frames, max 80 px. Reproduced at `a19bf487`.
- Impact: expanding or collapsing jumps to the final height instead of animating over the configured duration and curve.
- Cause: expansion is an immediate boolean composition with no retained animation.
- Fix direction: retain expansion state and animate body height with configurable duration, curve and a builder animation value.
- Confidence: high. Limitations: only expand-from-collapsed traced.
- Status: accepted.

### misc-002 — RotatedBox retains unrotated layout dimensions for odd turns

- Severity: P1. Category: layout.
- Flutter: `widgets/basic.dart:2230-2244`; `rendering/rotated_box.dart:83-96`. Incular: `crates/incular-widgets/src/layout/basic/transforms.rs:7-34`.
- Evidence: scenario `misc__rotated_box_layout_size`. Frame 0: Flutter 40x80, Incular 80x40; both frames diverge, max 40 px.
- Impact: parents lay out siblings with the unrotated width and height.
- Cause: RotatedBox is a paint transform, not a layout operation that flips constraints.
- Fix direction: flip constraints and report swapped dimensions for odd quarter turns before the paint transform.
- Confidence: high. Limitations: quarter turn 1 only.
- Status: accepted.

### misc-003 — MediaQuery readers rebuild for unrelated environment changes

- Severity: P2. Category: behavior (edge-case).
- Flutter: `widgets/media_query.dart:1141-1143,1496,2094-2104`. Incular: `crates/incular-widgets/src/environment.rs:17-58`; `crates/incular-widgets/src/tree/context.rs:77-82`.
- Evidence: scenario `misc__media_query_size_aspect`. DPR-only change at 32 ms: size-only Builder runs Flutter 1, Incular 2.
- Impact: descendants reading one media property rebuild on unrelated changes.
- Cause: dependencies track the whole RuntimeEnvironment value.
- Fix direction: expose aspect-specific reads and track only the fields each dependent uses.
- Confidence: high. Limitations: size aspect only.
- Status: accepted.

### misc-004 — Form.save skips callbacks when a field is invalid

- Severity: P0. Category: behavior (wrong-final-state).
- Flutter: `widgets/form.dart:314-319,336-348,643-645`. Incular: `crates/incular-widgets/src/forms.rs:201-218`.
- Evidence: scenario `misc__form_save_invalid_field`. Validate then save on an empty field: saved count Flutter 1, Incular 0 at 48 ms.
- Impact: code that saves current values despite validation errors cannot reproduce FormState.save.
- Cause: Form::save validates first and returns early.
- Fix direction: keep save independent from validation; leave validate-then-save to the submission flow.
- Confidence: high. Limitations: one invalid-field case.
- Status: accepted.

### misc-005 — RawAutocomplete has no interactive options behavior

- Severity: P1. Category: missing-api (missing-feature).
- Flutter: `widgets/autocomplete.dart:170-195,421-492,574-634,678-699`. Incular: `crates/incular-widgets/src/forms.rs:525-574`.
- Evidence: source only. Static options and a display callback lower to the child or a zero-height box; no text controller, focus, option view, overlay or highlight.
- Impact: typing cannot produce selectable suggestions.
- Cause: RawAutocomplete is a configuration descriptor, not a stateful input widget.
- Fix direction: controller-driven option updates, focus behavior, keyboard selection and an overlay view builder.
- Confidence: high. Limitations: no frame pair can express the interaction on the current surface.
- Status: pending-review.

### misc-006 — InheritedWidget has no public custom notification counterpart

- Severity: P2. Category: missing-api (missing-feature).
- Flutter: `widgets/framework.dart:1853-1874`. Incular: `crates/incular-widgets/src/tree/widget/constructors.rs:735-749`; `crates/incular-widgets/src/inherited_data.rs:173-253`.
- Evidence: source only. No public InheritedWidget or custom updateShouldNotify; typed scopes and InheritedModel cover common reads.
- Impact: callers cannot define a custom inherited widget with its own update predicate.
- Cause: inherited data is exposed through typed scopes and fixed abstractions.
- Fix direction: a public inherited-widget abstraction with a caller-defined update predicate and dependent lookup.
- Confidence: high. Limitations: the aspect case is covered by InheritedModel; the gap is the non-aspect contract.
- Status: pending-review.

### misc-007 — GenericFormField is not connected to retained field state

- Severity: P0. Category: missing-api (missing-feature).
- Flutter: `widgets/form.dart:482-597,602-699`. Incular: `crates/incular-widgets/src/forms.rs:452-523,652-688`.
- Evidence: source only. GenericFormField calls its builder with the initial value; FormFieldState<T> (forms.rs:654) has no validator or save callback and is not used by that conversion.
- Impact: typed FormField<T> cannot retain, validate, save and reset through the rendered field.
- Cause: generic form state and the builder widget are separate descriptors.
- Fix direction: connect GenericFormField to retained FormFieldState with validator, save, reset and autovalidation callbacks.
- Confidence: high. Limitations: severity P0 carried over from earlier review; may be P1 under the brief's definition.
- Status: pending-review.

### misc-008 — SafeArea does not consume the insets seen by descendants

- Severity: P1. Category: layout.
- Flutter: `widgets/safe_area.dart:113-135`. Incular: `crates/incular-widgets/src/safe_area.rs:203-211`; `crates/incular-widgets/src/tree/layout/behavior/containers.rs:372-399`.
- Evidence: scenario `misc__safe_area_nested_consumption`. 20 px top inset: nested SafeAreas measure 100x70 (Flutter) vs 100x90 (Incular) at 32 ms; child top padding 0 vs 20.
- Impact: nested SafeAreas double-pad protected edges.
- Cause: SafeArea adjusts geometry but does not publish the reduced environment.
- Fix direction: child environment with consumed safe insets removed, keeping view padding and view insets.
- Confidence: high. Limitations: top edge only; bottom and maintainBottomViewPadding untested.
- Status: accepted.

### misc-009 — KeyedSubtree replaces the child key instead of adding an outer key

- Severity: P2. Category: behavior (edge-case).
- Flutter: `widgets/basic.dart:8260-8292`. Incular: `crates/incular-widgets/src/layout/basic/utility.rs:8-33`.
- Evidence: source only. `child.set_key(Some(value.key))` overwrites the child's own key. `misc__future_builder_keyed_reorder` matches only without a child key.
- Impact: wrapping an already-keyed widget loses its key and can change element reuse.
- Cause: one key slot; KeyedSubtree overwrites it.
- Fix direction: keep a separate wrapper key.
- Confidence: high. Limitations: no nested-key scenario.
- Status: pending-review.

### misc-010 — CustomPaint cannot request a fill-to-constraints size

- Severity: P2. Category: behavior (missing-feature).
- Flutter: `rendering/custom_paint.dart:580` (`constraints.constrain(preferredSize)`); `widgets/placeholder.dart:93` (`Size.infinite`). Incular: `crates/incular-core/src/geometry.rs:24` (Size::new asserts finite); `crates/incular-widgets/src/layout/basic/painting.rs:74-85`.
- Evidence: source, plus the harness build panicked when `f32::INFINITY` was passed to `CustomPaint::new`.
- Impact: copying Flutter's infinite-size fill pattern panics on construction.
- Cause: Size is finite-only; Flutter's unbounded meaning has no representation.
- Fix direction: a fill-to-constraints option on CustomPaint instead of an infinite Size.
- Confidence: high. Limitations: not a frame comparison; may be intentional API design.
- Status: pending-review.

### misc-011 — GlobalKey reparenting (state-preserving move across parents) is missing

- Severity: P2. Category: behavior (missing-feature).
- Flutter: `widgets/framework.dart:159-244`. Incular: `crates/incular-widgets/src/tree/reconciliation.rs:1074-1090,1128-1189`.
- Evidence: source. `rg` finds no GlobalKey or GlobalObjectKey type in `crates/` (only doc mentions in `forms.rs:72` and `tree.rs:521`). Keyed reuse uses a transaction-local index from one parent's old children, so a widget moved to another parent is remounted.
- Impact: a subtree moved between parents loses element state (animations, focus, controllers).
- Cause: key matching is scoped to sibling lists; no tree-wide registry.
- Fix direction: a tree-wide key registry to detach a keyed element from an old parent and attach it to a new one in the same frame.
- Confidence: medium. Limitations: a state-preservation twin needs a stateful probe; architecture principle 8 allows divergence.
- Status: pending-review.

### misc-012 — Restorable property types are missing for num, DateTime, enum, Listenable and ChangeNotifier

- Severity: P2. Category: behavior (missing-feature).
- Flutter: `widgets/restoration_properties.dart:182-615,394,441`. Incular: `crates/incular-runtime/src/restoration.rs:1097-1117`.
- Evidence: source. Aliases cover Int, Double, Bool and String plus nullable variants over `Restorable<T>`; `rg` finds no RestorableNum, RestorableDateTime, RestorableEnum, RestorableListenable or RestorableChangeNotifier.
- Impact: enum, date and change-notifier state must be converted by hand to String or i64 to restore.
- Cause: only primitive aliases were added.
- Fix direction: typed wrappers for enums, date-times and num, and a listenable-backed property, over the existing generic.
- Confidence: high. Limitations: restart-time behavior; no round-trip scenario.
- Status: pending-review.

### misc-013 — Placeholder widget is not publicly exported

- Severity: P2. Category: behavior (missing-feature).
- Flutter: `widgets/placeholder.dart:50-95`. Incular: `crates/incular-widgets/src/layout/basic/painting.rs:99-192`; `crates/incular-widgets/src/layout/basic.rs:35`; `crates/incular-widgets/src/lib.rs:76,254-260`.
- Evidence: `layout` is a private module (`lib.rs:76`), and the public re-export list (`lib.rs:254-260`) omits Placeholder. Scenario `misc__placeholder_fallback_size` matches Flutter's LimitedBox(400x400)+fill composition.
- Impact: application code cannot use Placeholder.
- Cause: the implementation exists but is not exported.
- Fix direction: export Placeholder and its builder from incular-widgets and the facade.
- Confidence: high. Limitations: painter strokes and colors not compared.
- Status: pending-review.

### misc-014 — No public stateful-widget lifecycle hooks

- Severity: P2. Category: behavior (missing-feature).
- Flutter: `widgets/framework.dart:916` (State), `:771` (StatefulWidget). Incular: `crates/incular-widgets/src/tree/widget/constructors.rs:795-860`; `crates/incular-widgets/src/tree/widget/mod.rs:64`.
- Evidence: source. `rg` finds none of initState, didChangeDependencies, didUpdateWidget, reassemble, deactivate, activate or dispose in the widget tree crates; the activate/deactivate hits are in incular-android and incular-accessibility. The only teardown is Rust `Drop` on Widget, not scoped to an element mount.
- Impact: no setup on mount, no reaction to parent updates before rebuild, and no release when an element leaves the tree.
- Cause: stateful widgets are closures over revision cells, not element-scoped State objects.
- Fix direction: decide whether lifecycle hooks belong to the Rust API; if so, mount, update and unmount hooks on the stateful builder, with unmount tied to element removal.
- Confidence: medium. Limitations: architecture principle 8 says Flutter is vocabulary, so this may be an accepted divergence; no frame twin without a State equivalent.
- Status: pending-review.

## Pixel scene proposals

None.

## Harness requests

- None outstanding for the frame recorder; transparent-fill and non-finite handling are in place.
- WidgetState resolution (widget_state.dart rows, misc-level coverage) has no neutral counterpart: `WidgetStateProperty` resolution (`crates/incular-material/src/theme/state.rs:226,263,290`) lives only in `incular-material`, which the harness does not depend on and which is out of scope. Recorded as present-but-not-neutral; no frame scenario written. Needs a neutral location in incular-widgets to be framed.

## Open questions

- Whether GlobalKey (misc-011) and State lifecycle hooks (misc-014) are intended gaps or accepted Rust divergences.
- ObjectKey identity semantics: `KeyObject` is a `ValueKey` alias with value equality (`crates/incular-core/src/key.rs:170`), not identity.
- DeviceOrientationBuilder has no MediaQuery-sourced orientation in Incular.
- Not frame-tested: Expansible collapse, InheritedModel nesting, FutureBuilder future replacement, StreamBuilder cancellation on dispose, autovalidate ordering, SafeArea bottom edge.

## Validation

- `harness/run-frames.sh misc__` at `a19bf487`: 9 of 14 match (table above).
- `cargo check -q -p incular-parity-harness --tests`: passes.
- `flutter analyze test/frames/misc_test.dart`: no issues.
- `pwsh -NoProfile -File audits/flutter-parity/inventory_tool/check-findings.ps1`: prints nothing for misc.
- `findings/misc.jsonl` has 14 lines and parses as JSON.
