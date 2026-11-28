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

Nested overlay world bounds and point routing (:194) are covered below.
The engine traversal matrix wire format is not paired. Attributed speech spans (:95) are
tracked separately. The unresolved traversal parent (:271) is now
paired below. This slice validates update data and the real
AccessKit consumer, not native assistive-technology speech.

## Mixed-direction merged labels

The authored `merged_semantic_label_directions_test.dart` reads Flutter's
merged semantics data, not the outer node's unmerged label. A stable LTR
node publishes `Welcome\n\u202Bمرحبا\u202C` for its RTL child. Changing only
the child's direction to LTR removes the marks without changing the node ID.
With an empty but LTR outer label, the opposing first contribution is still
wrapped, unlike the standalone label builder.

Incular's retained test previously got plain newline concatenation. It now
matches all three strings using its ambient Directionality and publishes
only the same changed leaf to AccessKit and mobile. An additional control
changes only the explicit semantic direction and verifies the native property
overrides surrounding direction. The focused binding, custom action, forest,
and incremental adapter controls pass. This authored probe does not change
the upstream count and does not assert native speech or mixed-direction hints.

## Unchanged semantics are not re-published

Pinned `rendering/simple_semantics_test.dart:13` passed. The native
`flutter350_semantic_native_updates` fixture instruments a mounted
painter's semantics callback, then explicitly asks a replacement
delegate to rebuild the same labelled, keyed region. That callback
runs, but the retained revision does not change and neither AccessKit
nor the mobile projection emits an update. Changing the label to
`bye` then emits exactly one node without replacing the semantic
identity. All seven focused native semantic-update tests passed.

## Named traversal parents and missing nodes

Pinned `semantics_update_test.dart:271` passed. Incular now supports
unique named semantic traversal parents and referring children. The
mounted original puts a named child next to an ordinary 10×10 box
without declaring its parent. The framework and both native projections
receive no accessible children; point dispatch cannot address a
discarded child. Ordinary pointer and layout structure is unchanged.

The additional native test places the target away from the rendered
child. The named node appears beneath the target in traversal, but
the actual hit entry stays on its original physical branch. Removing
the target immediately removes the previously published child and its
hit entry, even though the physical widget still exists. Clearing the
last named link restores ordinary ancestry. A cycle or duplicate
target name cannot create orphaned native roots. Ordinary trees do
not run the named traversal pass.

## Point actions in nested overlays

The pinned :194 matrix original passed, as did the authored
`nested_overlay_semantic_points_test.dart`. Flutter routes taps to the
three controls at view positions (0,0), (10,11), and (110,211).

Incular's native projections already reported the right world bounds,
but its point-action lookup clipped both popup branches to the small
anchor rectangles. A portal's semantic group can now traverse its
out-of-anchor children without changing clipping for an ordinary
semantic parent. A round trip through the AccessKit consumer resolves
the same controls, and the Runtime dispatches their real callbacks.
Moving the middle anchor keeps all three semantic IDs, moves only the
middle control's exported geometry, and leaves the nested content
activatable at its fixed view position.

This remains partial for :194: Incular exports world bounds rather
than per-node Flutter engine traversal matrices. The authored point
probe does not add an upstream registration.
