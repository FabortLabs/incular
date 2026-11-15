# Misc fixes

## misc-003: MediaQuery readers rebuild for unrelated environment changes

- Outcome: fixed.
- Cause: every RuntimeEnvironment scope exposed one whole-value key; a size reader depended on it and rebuilt on any field change (DPR-only change rebuilt the size reader).
- Change: `crates/incular-widgets/src/tree/context.rs` installs seven per-aspect wrappers (size, device pixel ratio, text scale, platform brightness, padding, view padding, view insets) in each RuntimeEnvironment scope, and updates only changed aspects. New `BuildContext::media_query_*` readers return `Option`. `reconciliation.rs` calls install on scope mount and update on value change.
- Tests: `crates/incular-widgets/tests/environment.rs::media_query_aspect_readers_rebuild_only_for_their_aspect`.
- Harness: `misc__media_query_size_aspect` now matches (builder runs 1 vs 1; was 1 vs 2). The Incular twin in `harness/incular/tests/frames/misc.rs` now uses `media_query_size()`, the diagnostic-twin exception; reported.
- Limitations: only the seven aspects above; locales, accessibility flags and display features still use the whole-value key.

## misc-006: InheritedWidget custom update predicate

- Outcome: already-fixed.
- Cause: none. `InheritedModelData::update_should_notify` is the public predicate, and `InheritedModel::inherit_from(context, None)` is the aspect-less dependent, equivalent to Flutter's InheritedWidget.
- Tests: existing `inherited_model_rebuilds_dependents_by_aspect`.
- Limitations: naming only; there is no type called InheritedWidget.

## misc-011: GlobalKey reparenting

- Outcome: deferred.
- Cause: keyed reuse only searches the old children of the parent being reconciled.
- Design sketch: tree-wide GlobalKey registry; on a registry hit under another parent, detach from the old parent's children and render links and attach to the new one; the old parent's unmount sweep skips elements claimed this transaction; rebind inherited subtree and render layers; keep focus, semantics and dependencies.
- Remaining work: multi-parent transaction ordering. Not small.

## misc-012: Restorable property types

- Outcome: fixed (partial by design).
- Cause: `Restorable<T>` already accepts any serde type, but enums and non-i64 numbers were untested and undocumented.
- Change: documentation on `Restorable` in `crates/incular-runtime/src/restoration.rs`.
- Tests: `crates/incular-runtime/tests/restoration.rs::enum_and_numeric_values_restore_across_relaunch`.
- Deferred: RestorableDateTime (no time type in the architecture allow-list) and RestorableListenable/ChangeNotifier (no serialization contract on the Listenable trait).

## Validation

- `cargo test -p incular-widgets --test environment --test inherited_data --test inherited_context_contracts`: 16 passed.
- `cargo test -p incular-runtime --test restoration`: 22 passed.
- `cargo clippy -p incular-widgets` and `-p incular-runtime` with `--all-targets -D warnings`: no warnings in touched files.
- `cargo check -p incular-parity-harness --tests`: passes.
