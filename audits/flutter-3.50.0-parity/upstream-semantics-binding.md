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

## Ordinal sort keys and role updates

- `semantics_test.dart:553`: all 16 differently named comparisons
  match, with unnamed keys first and names overriding numeric order.
- `:528`: all eight numeric comparisons for equal names match. The
  separate assertion for comparing to a different sort-key subclass is
  not a Rust runtime operation for this concrete, typed API.
- `:453`: changing a generic node to Tab marks the same node dirty
  without a geometry update. Rust tracks this with revision deltas and
  has no separate Flutter "none" role, so this is partial.

An authored probe also verifies UTF-16 group-name ordering and that
positive and negative zero are ties. The mounted Rust test previously
reordered the signed-zero siblings; using the ordinal comparison now
retains their traversal order. All 33 focused Rust semantics tests
passed. Groups still keep their relative traversal positions; the
public key comparison does not globally reorder named groups.

## Touching traversal grid

Pinned `traversal_order_test.dart:11` and the retained WidgetTree
agree on the nine labels and their row-major traversal. The Rust
fixture checks that each 50×50 cell really touches its neighbors
before reading the published semantic child list. This is partial:
the TestWidgetsApp shell's semantic grouping and raw numeric IDs
are not ported to the generational Rust tree.
