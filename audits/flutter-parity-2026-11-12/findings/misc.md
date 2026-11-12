# misc: Other neutral widgets

## Summary

The filtered eleven-scenario run found five divergences: Expansible expansion height, RotatedBox measured size, MediaQuery aspect invalidation, Form saving an invalid field, and SafeArea inset consumption. Spacer, InheritedModel, the text-backed FormField lifecycle, FutureBuilder keyed movement, StreamBuilder events, and Directionality matched. Source review also found gaps in RawAutocomplete, generic FormField lifecycle, subclassable InheritedWidget notifications, and nested KeyedSubtree keys. Other miscellaneous APIs remain unchecked.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| Expansible body-height animation | `incular_widgets::Expansible` | source comparison; filtered frame trace | `misc__expansible_height_animation` | divergent: 120×32 vs 120×112 from frame 3; 13 frames, max 80 px |
| RotatedBox odd-turn measured size | `incular_widgets::RotatedBox` | source comparison; filtered frame trace | `misc__rotated_box_layout_size` | divergent: Flutter 40×80, Incular 80×40 at frame 0; 2 frames, max 40 px |
| Spacer flex allocation (`widgets/spacer.dart:41-59`) | `incular_widgets::Spacer` (`crates/incular-widgets/src/layout/flex.rs:370-420`) | source comparison; filtered frame trace | `misc__spacer_flex_shares` | match |
| InheritedModel aspect notifications | `incular_widgets::InheritedModel` | source comparison; filtered frame trace | `misc__inherited_model_aspects` | match |
| Builder context callback | `widgets/basic.dart:8329-8396`; `crates/incular-widgets/src/layout/basic/sizing_constraints.rs:395-425` | source comparison; exercised by the inherited-data scenarios | `misc__inherited_model_aspects`, `misc__media_query_size_aspect` | context lookup works; MediaQuery invalidation diverges below |
| MediaQuery size-only dependency | `incular_widgets::MediaQuery` and `BuildContext::depend_on` | source comparison; filtered frame trace | `misc__media_query_size_aspect` | divergent: builder runs Flutter 1, Incular 2 after DPR-only change |
| FormState validation followed by save | `incular_widgets::{Form, FormField}`; `incular_controls::Form` | source comparison; filtered frame trace | `misc__form_save_invalid_field` | divergent: invalid value still saved by Flutter; Incular skips callback |
| Text-backed FormField edit/validate/save/reset lifecycle | `incular_widgets::FormField` | source comparison; filtered frame trace | `misc__form_field_lifecycle` | match |
| Generic FormField retained value and validation lifecycle | `incular_widgets::GenericFormField` and `FormFieldState<T>` | source comparison; no lifecycle twin available | none | missing integration between widget state and validation |
| FutureBuilder completion under keyed reorder | `incular_widgets::FutureBuilder` and `KeyedSubtree` | source comparison; filtered frame trace | `misc__future_builder_keyed_reorder` | match |
| StreamBuilder waiting/data/error/done snapshots | `incular_widgets::StreamBuilder` | source comparison; filtered frame trace | `misc__stream_builder_events` | match |
| SafeArea padding consumption for nested children | `incular_widgets::SafeArea` | source comparison; filtered frame trace | `misc__safe_area_nested_consumption` | divergent: nested padding is applied twice and remains visible to descendants |
| Directionality changing Row's start edge | `incular_widgets::Directionality` | source comparison; filtered frame trace | `misc__directionality_row_order` | match |
| KeyedSubtree wrapping a child with its own key | `incular_widgets::KeyedSubtree` | source comparison; nested-key state not observable in this harness | none | outer key replaces child's key |
| RawAutocomplete dynamic options and overlay | `incular_widgets::RawAutocomplete` | source audit; no equivalent scenario can be expressed | none | missing interactive API |
| InheritedWidget custom notification contract | typed environment scopes and `InheritedModel` are partial counterparts | source comparison | none | missing public subclass/updateShouldNotify contract |
| Remaining miscellaneous APIs | various | not checked | none | audit incomplete |

## Findings

### misc-001 — Expansible adds and removes its body without animation

- Severity: P1
- Category: behavior
- Flutter: `widgets/expansible.dart:193-197` describes the animated body height; `:230` and `:235` set the defaults to 200 ms and Curves.ease; `:356-374` constructs the controller and curved height factor; `:410-449` drives expansion and clips/height-aligns the body.
- Incular: `crates/incular-widgets/src/utilities.rs:13-36` exposes only a boolean expanded state, and `:39-48` immediately builds either the header or the header/body Column.
- Evidence: filtered frame scenario `misc__expansible_height_animation`. The collapsed geometry matches; at frame 3 (48 ms), Flutter remains 120×32 while Incular is already 120×112. The size diverges for 13 of 17 frames, with an 80 px maximum difference.
- Impact: expanding or collapsing the body jumps to its final height instead of animating over the configured duration and curve; the API also has no expansion animation value for builders.
- Likely cause: expansion is represented as an immediate boolean composition.
- Fix direction: retain expansion state and animate body height with a configurable lifecycle and builder animation value.
- Confidence: high

### misc-002 — RotatedBox retains unrotated layout dimensions for odd turns

- Severity: P1
- Category: layout
- Flutter: `widgets/basic.dart:2230-2244` creates and updates `RenderRotatedBox`; `rendering/rotated_box.dart:83-96` flips constraints and swaps width and height for odd quarter turns.
- Incular: `crates/incular-widgets/src/layout/basic/transforms.rs:7-12` documents that allocated dimensions never swap, and `:30-34` lowers RotatedBox to a paint Transform.
- Evidence: filtered frame scenario `misc__rotated_box_layout_size`. At frame 0 the outer size is Flutter [40, 80] and Incular [80, 40]; the size diverges in both frames, with a 40 px maximum difference.
- Impact: parents lay out siblings and constraints using the unrotated width and height, so layout diverges even when the painted child is rotated.
- Likely cause: RotatedBox is implemented as a paint transform instead of a pre-layout render object.
- Fix direction: measure with flipped constraints and report swapped dimensions for odd quarter turns before applying the paint transform.
- Confidence: high

### misc-003 — MediaQuery readers rebuild for unrelated environment changes

- Severity: P2
- Category: behavior
- Flutter: `widgets/media_query.dart:1141-1143` makes MediaQuery an InheritedModel; `:1496` subscribes `sizeOf` to the size aspect; `:2094-2104` only notifies that dependent when size changes, even though devicePixelRatio changed.
- Incular: `crates/incular-widgets/src/environment.rs:17-58` publishes the complete RuntimeEnvironment as one typed scope; `crates/incular-widgets/src/tree/context.rs:77-82` tracks a dependency on the whole value.
- Evidence: filtered frame scenario `misc__media_query_size_aspect` changes only device pixel ratio while the stable Builder reads viewport size. At 32 ms Flutter's builder count stays 1 and Incular's rises to 2.
- Impact: descendants that read one media property rebuild when unrelated environment properties change.
- Likely cause: MediaQuery dependencies are tracked at whole-snapshot granularity rather than by property aspect.
- Fix direction: expose aspect-specific MediaQuery reads and track only the fields each dependent uses.
- Confidence: high

### misc-004 — Form.save skips callbacks when a field is invalid

- Severity: P0
- Category: behavior
- Flutter: `widgets/form.dart:314-319` calls each FormFieldState.save without validation; validation is a separate `FormState.validate` operation at `:336-348`, and field save invokes `onSaved` at `:643-645`.
- Incular: `crates/incular-widgets/src/forms.rs:201-218` validates inside `Form::save` and returns before invoking callbacks when any field is invalid.
- Evidence: filtered frame scenario `misc__form_save_invalid_field` calls validate, then save on an empty field. Validation is false on both sides; at frame 3 (48 ms) Flutter's saved callback count is 1 and Incular's is 0.
- Impact: code that deliberately saves current values despite validation errors cannot reproduce Flutter FormState.save behavior.
- Likely cause: Incular combines validation and saving in one operation.
- Fix direction: keep `save` independent from validation and leave validate-then-save coordination to submission flows.
- Confidence: high

### misc-005 — RawAutocomplete has no interactive options behavior

- Severity: P1
- Category: missing-api
- Flutter: `widgets/autocomplete.dart:170-195` requires option and view builders and controller/focus wiring; `:421-492` updates suggestions from text and focus; `:574-634` builds the overlay; `:678-699` mounts it.
- Incular: `crates/incular-widgets/src/forms.rs:525-574` stores a static options vector and display callback, then lowers the widget to the supplied child or a zero-height box.
- Evidence: direct source comparison. The Incular type has no text controller, focus listener, option builder, overlay builder, highlighted selection, or selection callback. No equivalent frame pair can exercise Flutter's interactive feature with this counterpart.
- Impact: typing cannot produce selectable suggestions through Incular's RawAutocomplete.
- Likely cause: RawAutocomplete is currently a data/configuration descriptor rather than a stateful input widget.
- Fix direction: implement controller-driven option updates, focus behavior, keyboard selection, and an overlay view builder.
- Confidence: high

### misc-006 — InheritedWidget has no public custom notification counterpart

- Severity: P2
- Category: missing-api
- Flutter: `widgets/framework.dart:1853-1874` defines an extensible ProxyWidget contract with `createElement` and custom `updateShouldNotify`.
- Incular: `crates/incular-widgets/src/tree/widget/constructors.rs:735-749` offers a typed environment scope; `crates/incular-widgets/src/inherited_data.rs:222-253` offers the aspect-aware InheritedModel contract.
- Evidence: source/API comparison found no public Incular InheritedWidget class or custom `updateShouldNotify` hook. Typed scopes and InheritedModel cover common inherited-value patterns, but they do not expose the same subclassable notification contract. No direct frame twin was possible.
- Impact: callers cannot define a custom inherited widget that owns its own update predicate and dependent notification policy.
- Likely cause: inherited data is exposed through typed scopes and fixed framework abstractions.
- Fix direction: consider a public inherited-widget abstraction with a caller-defined update predicate and dependent lookup.
- Confidence: high

### misc-007 — GenericFormField is not connected to retained field state

- Severity: P0
- Category: missing-api
- Flutter: `widgets/form.dart:482-597` creates a stateful field widget; `:602-699` retains its value, validation state, user-interaction state, reset, and save behavior.
- Incular: `crates/incular-widgets/src/forms.rs:452-523` stores initial/configuration values and converts the field by calling its builder with only the initial value. `FormFieldState<T>` at `:652-688` is a separate value/error container with no validator or save callback and is not used by that conversion. The controller-backed `FormField` at `:284-398` is a distinct string-editing adapter.
- Evidence: source comparison. The filtered scenario `misc__form_field_lifecycle` validates that the text-backed adapter's change, validation, save, and reset path matches; it cannot exercise generic typed FormField state because the generic widget does not retain or expose it.
- Impact: callers cannot use GenericFormField as Flutter FormField<T> to retain arbitrary typed values, validate them, and save/reset through the rendered field lifecycle.
- Likely cause: generic form state and the builder widget are implemented as separate descriptors.
- Fix direction: connect GenericFormField to retained FormFieldState with validator, save, reset, and autovalidation lifecycle callbacks.
- Confidence: high

### misc-008 — SafeArea does not consume the insets seen by descendants

- Severity: P1
- Category: layout
- Flutter: `widgets/safe_area.dart:113-135` reads MediaQuery padding and wraps the child in `MediaQuery.removePadding` for the sides it protects.
- Incular: `crates/incular-widgets/src/safe_area.rs:203-211` lowers SafeArea to a render kind; `crates/incular-widgets/src/tree/layout/behavior/containers.rs:372-399` applies padding during layout without changing the descendant environment.
- Evidence: filtered frame scenario `misc__safe_area_nested_consumption`. With a 20 px top inset, nested SafeAreas measure Flutter 100×70 and Incular 100×90; Flutter's child reads top padding 0 while Incular's reads 20. The differences occur in the frame after metrics change.
- Impact: nested SafeAreas double-pad protected edges, and descendants keep reacting to insets already consumed by an ancestor.
- Likely cause: SafeArea only adjusts geometry and does not publish the reduced MediaQuery/environment to its child.
- Fix direction: install a child environment with the consumed safe insets removed while preserving view padding and view insets.
- Confidence: high

### misc-009 — KeyedSubtree replaces the child key instead of adding an outer key

- Severity: P2
- Category: behavior
- Flutter: `widgets/basic.dart:8260-8292` gives KeyedSubtree its own key and returns the child unchanged; `KeyedSubtree.wrap` at `:8268-8269` derives the wrapper key from the child's existing key or index.
- Incular: `crates/incular-widgets/src/layout/basic/utility.rs:8-33` assigns the requested key directly to the child with `child.set_key(Some(value.key))`.
- Evidence: source comparison. `misc__future_builder_keyed_reorder` matches when the wrapped FutureBuilder has no key of its own. Nested child-key preservation was not tested because the frame recorder's probe itself uses a KeyedSubtree key.
- Impact: wrapping an already-keyed widget loses that widget's key and can change which element state is reused during reconciliation.
- Likely cause: the retained widget descriptor has one key slot and KeyedSubtree lowers by overwriting it.
- Fix direction: retain a separate wrapper element/key so the child's own key remains intact.
- Confidence: high

## Pixel scene proposals

None.

## Harness requests

None.

## Open questions

Other Expansible lifecycle details and remaining miscellaneous APIs need independent review. RawAutocomplete, the generic FormField lifecycle, and the InheritedWidget notification contract have no equivalent pair with the current Incular surface. InheritedModel nesting/shadowing, other MediaQuery aspects, FutureBuilder future replacement, StreamBuilder stream replacement/disposal, SafeArea maintain-bottom-view-padding options, Form autovalidation/restoration, nested KeyedSubtree child keys, and other Expansible lifecycle paths were not exercised. The filtered run covered only the scenarios listed above.
