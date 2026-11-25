# Semantic updates at the adapter boundary

Four pinned Flutter cases have equivalent retained comparisons in
`flutter350_semantic_native_updates`:

- `semantics_update_test.dart:29`: the full AccessKit snapshot has one host
  and one merged leaf, with no independently published child. After the inner
  label changes, AccessKit and the mobile projection emit only the same
  retained merged node, with the exact newline-separated label.
- `semantics_update_test.dart:308`: hiding the overlay removes its item from
  native traversal without reassigning the anchor; the native tree receives
  no item update or surviving child reference. The Rust control also rejects
  native or queued actions for the removed item.
- `semantics_test.dart:453`: a role-only change from the generic/untyped
  container to Tab advances the node's semantic revision and publishes just
  that native node with the updated role. Rust revisions are the counterpart
  to Flutter's pending semantic dirty flag.
- `semantics_test.dart:466`: after a full publication leaves no pending
  update, replacing one named custom action dirties only that node, keeps
  its native identity, and rejects the old action in both adapters.

Three other named `semantics_update_test.dart` cases passed only on Flutter:
attributed speech spans (:95), engine traversal matrices for nested portals
(:194), and unresolved explicit traversal-parent identifiers (:271).
These do not count as paired. This slice validates update data and the real
AccessKit consumer, not native assistive-technology speech.
