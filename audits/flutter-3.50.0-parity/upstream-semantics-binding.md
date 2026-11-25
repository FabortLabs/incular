# Flutter 3.50.0 semantics binding and owner

Both pinned originals passed, and the retained Runtime tests match:

- `semantics_binding_test.dart:135`: for an 800×600 physical view at
  DPR 2, a 100×50 target translated by (40,20) reports that exact
  rectangle in logical view coordinates. Incular receives a real view
  metrics event and uses the retained semantic node's bounds.
- `semantics_owner_test.dart:12`: removing the labeled subtree removes
  its semantic node. A subsequent ShowOnScreen referencing that old
  node does not panic or handle a different target.

These cover a valid single-view lookup and stale framework action
dispatch. They do not exercise OS accessibility adapters, cross-view
lookups, or invalid view IDs.

Rust target: `flutter350_upstream_semantics_binding`.
