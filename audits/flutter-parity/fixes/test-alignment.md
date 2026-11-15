# test-alignment fixes

Group `test-alignment`. Follow-up to text-006 (programmatic `set_text`/`set_value` no longer run
edit transforms, input formatters or `on_changed`, matching Flutter 3.41.7). No jsonl: these are
test alignments, not findings.

## Verdicts

| Test | Verdict | Justification |
| --- | --- | --- |
| `incular-material/tests/length_formatting.rs` (6 tests) | expectation updated | Each test drove formatting through `set_text`/`set_value`, the old non-Flutter path. Flutter's formatters run only in `_formatAndSetValue` from `userUpdateTextEditingValue` (editable_text.dart:4516, 4960), never on `controller.text =`. Rewritten to type through `replace_selection` / `apply_delta` (the same user-edit helpers `editable_text_interaction.rs` uses), keeping the coverage: mount, rebuild and callback replacement, unmount release, formatter history and panic recovery, nested `on_changed` edit, grapheme safety, composition deferral. Each test also asserts a programmatic update is stored as given. `restored_over_long_value_is_formatted_on_mount` renamed to `..._not_reformatted_on_mount` because Flutter does not format on mount. |
| `incular-text/tests/editing_copies.rs::a_selection_change_still_passes_through_edit_transforms` | expectation updated (renamed `a_selection_change_skips_edit_transforms_but_an_edit_runs_them`) | Flutter runs formatters only when text changes or a composition commits (`_formatAndSetValue`: `if (textChanged \|\| textCommitted)`, editable_text.dart:4528). A tap or programmatic `selection =` is selection-only and skips them. Incular's `update_ranges` matches this. The test now asserts the transform is not called on a selection change and is called once for a user edit. |
| `incular-controls/tests/w7_compound_controls.rs::autocomplete_semantic_set_text_reaches_editor` | expectation updated (renamed `autocomplete_user_edit_reaches_query_callback`) | The test called `controller.set_text`, a programmatic update, and expected the query callback. The callback is the editor's user-edit `on_changed`, so the test now uses `replace_selection` and asserts a programmatic update does not fire it. Semantic SetText itself is covered at runtime level (next row). `incular-controls` cannot depend on `incular-runtime` (not in `specs/architecture.json`), so the semantic path cannot be tested from this crate. |
| Semantic SetText dispatch (not a listed test) | code fixed | `Runtime::dispatch_semantic_action` (crates/incular-runtime/src/frame.rs, `SemanticAction::SetText`) called `controller.set_text`. Flutter's `RenderEditable._handleSetText` calls `userUpdateTextEditingValue` (rendering/editable.dart:1409), so accessibility replacement is a user edit: formatters and `on_changed` run. Now it calls `apply_delta` with a full-range replacement and a collapsed caret at the end, as Flutter does. New test: `crates/incular-runtime/tests/semantic_set_text_user_edit.rs`. |
| Transpose characters (not listed; same bug class) | code fixed | `crates/incular-runtime/src/frame/text_actions.rs` (transpose) called `controller.set_value`. Flutter's `TransposeCharactersTextIntent` is a user edit, so the new value now goes through `apply_delta`. Outside this group's file list (small change, reported). Not covered by a test in this pass. |
| `incular/tests/heroes.rs::heroes_do_not_fly_without_a_route_animation_or_on_pop` | code fixed + test justified | See the heroes section below. |

## Heroes investigation

- Baseline `base-e25e45bc`: the committed test (no reverse expectation, FROM shown immediately on pop) passes. The working-tree version (navigation-005 edit, uncommitted) fails at the final assertion (line 391 before my change).
- Cause 1 (code, Flutter parity): `HeroFlights::end()` read the raw route status. Flutter drives a pop through `ReverseAnimation(manifest.animation)` (heroes.dart:694, the status feeds `endFlight` at 589-590). Without the reversal, the revealed hero kept its placeholder after a pop. Fixed in `crates/incular-widgets/src/hero.rs` (`end()` reverses the status for `HeroFlightDirection::Pop`).
- Cause 2 (test/harness): the pending pop flight starts only on the third route-animation tick (`start_ready`: `waited > 2`), and the harness, which drives the reverse directly without `RouteOutlet`, ticked twice. The flight never started, so the revealed hero stayed hidden. `RouteOutlet` ticks every frame, so production is unaffected. Fix: the test now has one more tick, with a comment citing `start_ready`.
- Rejected: starting pop flights immediately when the destination is laid out. It starts before the reverse begins (`hero_controller.rs` notes the reverse starts on the next tick), so the flight sees `Completed` and ends early. Reverted.
- The navigation-design fixes file does not exist. Hero code was not edited by any other worker at the time of this change.
- Harness limitation (not changed): `navigator.routes()` drops a popped route immediately; only `RouteOutlet` keeps it mounted through its reverse. The test's source hero therefore has no location at pop time. This is why the flight reads the source location from the previous frame. Not fixed here.

## Test results (final state)

- `cargo test -p incular-material --test length_formatting`: 6 passed.
- `cargo test -p incular-text --test editing_copies`: 7 passed. (The brief named `incular-widgets`; the file lives in `incular-text`.)
- `cargo test -p incular-controls --test w7_compound_controls`: 5 passed. (The brief said `w7_compound_controls` without a crate; found in `incular-controls`.)
- `cargo test -p incular-runtime --test semantic_set_text_user_edit`: 1 passed (new).
- `cargo test -p incular --test heroes`: 6 passed.
- `cargo clippy -p incular-runtime --all-targets -- -D warnings`: clean. `cargo clippy -p incular-widgets -p incular-navigation -p incular-material -p incular-text -p incular-controls --all-targets -- -D warnings`: clean.

## Shared-file and out-of-ownership edits

- `crates/incular-runtime/src/frame.rs`: semantic SetText path; removed an unused `TextEditingValue` import.
- `crates/incular-runtime/src/frame/text_actions.rs`: transpose uses `apply_delta`.
- `crates/incular-widgets/src/hero.rs`: pop-flight keep-placeholder status (`end()`).
- New file: `crates/incular-runtime/tests/semantic_set_text_user_edit.rs`.
- Heroes test (`crates/incular/tests/heroes.rs`): one extra tick added; the navigation worker's uncommitted expectations are otherwise kept.

## Limitations and risks

- Transpose has no test. Its behavior change is covered only by reasoning from Flutter's intent.
- Other workers' in-progress edits repeatedly broke the shared build (`incular-accessibility`, `incular-rendering`, `incular-widgets`). Results above are from green builds.
- The popped-route harness gap (above) remains a known difference between the bare `Harness` and `RouteOutlet`.
