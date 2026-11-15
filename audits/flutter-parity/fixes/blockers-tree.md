# blockers-tree fixes

Group `blockers-tree`: navigation-001 and misc-011. These records supersede the earlier `navigation`, `navigation-design` (navigation-001) and `misc` (misc-011) records. Machine-readable records are in `blockers-tree.jsonl`.

## navigation-001: fixed
- Cause: `Widget::environment_scope` is a lazily built `LayoutBuilder`. Prevalidation could not see into its builder closure, so route content under the `ModalRoute` scope was validated only when it mounted during layout, after the outlet rebuild applied. The outlet therefore kept the scope off by default.
- Change:
  - `WidgetKind::LayoutBuilder` gains `static_child`. `environment_scope`, `shared_environment_scope` and `environment_boundary` record their fixed child in it.
  - `validate_widget_subtree` descends into `static_child`. Each entry carries its own ambient direction: a `TextDirection` scope provides its value, a boundary hides it, and any other scope passes the parent's direction through.
  - `RouteOutlet::new` sets `modal_route_scopes: true`.
- Tests:
  - `inherited_context_contracts.rs`: `static_scope_children_prevalidate_before_reconciliation`, `static_scope_prevalidation_tracks_ambient_direction`.
  - `route_outlet.rs`: `modal_route_scope_is_on_by_default_and_route_content_still_fails_atomically`. The existing atomicity tests (`outlet_rebuild_failure_consumes_nothing`, `outlet_failed_attempt_causes_no_side_effects`) pass with the scope on.
- Harness: `navigation__` still matches 6 of 12 scenarios, the same as before this change. The same fields diverge in the same scenarios, so there are no new divergences. Only the frame-3 start residual jitters between runs.
- API change: `ModalRoute::of` works by default. To get the old behaviour, call `set_modal_route_scopes(false)`.
- Limitations: only static scopes are descended into. Other builder output still validates when it materializes, as before.

## misc-011: fixed (common case)
- Cause: keyed reuse only searched the old children of the parent being reconciled.
- Change:
  - New `GlobalKey` (`GlobalKey::new`), `Key::Global` and `TreeError::DuplicateGlobalKey`.
  - New module `crates/incular-widgets/src/tree/global_keys.rs`:
    - a registry of live global-keyed elements;
    - a retake step, run when a global-keyed widget mounts: it detaches the element from wherever it is (parked, or still attached to its old parent), re-parents it, rebinds its inherited contexts and updates it;
    - a retire step for the reconcile sweeps: it skips children another parent took over and parks global-keyed descendants of removed subtrees;
    - a per-transaction claim set that detects duplicates.
  - `layout()` unmounts parked elements that nobody retook, because builders materialize during layout.
  - Prevalidation rejects a global key used twice in one static description.
- Tests: `crates/incular-widgets/tests/global_keys.rs`, 6 tests covering:
  - a move between siblings' children in both directions;
  - a move up and down a level;
  - a builder moved across inherited scopes, which keeps its element and reads the new scope;
  - a removed element is unmounted at the end of layout;
  - a duplicate key is rejected atomically;
  - local keys are unchanged.
  - State retention is shown by the element id, the render id and the mount counter.
- API change: additive. Code outside the workspace that matches exhaustively on `Key` or `TreeError` needs a new arm.
- Limitations and remaining work:
  - The move window is one frame. A subtree parked by `update()` keeps its focus nodes, semantics node and listeners until the next `layout()` ends.
  - Duplicates are detected per reconciliation transaction. If two builders produce the same key in one frame, the element moves to the later one instead of the build failing.
  - There is no `currentState` or `currentContext` lookup.
  - Moves out of sliver or two-dimensional dynamic parents remove the parallel child records, but no test covers them.

## Validation
- incular-widgets: deep_tree_lifecycle, element_memory, inherited_context_contracts, navigation_scopes, tree_performance_contracts, tree_error_policy, environment and global_keys pass.
- tree_reconciliation did not compile because of another worker's in-flight `PaintCommand::PushMatrixFilter` change. It was not rerun.
- incular-runtime `runtime route_outlet`: 88 pass. incular-navigation: 83 pass. `crates/incular/tests/heroes.rs`: 6 pass.
- `crates/incular/tests/flutter_directionality.rs`: 14 pass and 1 fails. `editable_text_directionality_case_checks_physical_start_alignment` fails on RTL glyph placement in paint. The mount succeeds, so it is outside prevalidation, and the cause is most likely the text worker's in-flight edits.
- `cargo clippy -p incular-widgets --all-targets` stops on another worker's file (`render_object/layers.rs`, large_enum_variant) before it reports on my code.
