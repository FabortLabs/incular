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

## Positioned sibling traversal

The authored pinned probe places "below", "right", and "left" in
declaration order. Flutter traverses both upper siblings before the
lower one, using LTR or RTL within the row, even when the upper boxes
overlap. The Rust baseline instead returned declaration order.

Semantic sibling lists now use parent-relative layout geometry before
applying ordinal sort-key runs. Bounds changes mark the affected
ancestor lists. The retained test moves one node in the compositor,
preserves its ID and layout count, and observes the updated reading
order. The corresponding Flutter transform keeps the same ID as well.
All 27 targeted Rust semantics tests and three authored Flutter cases
passed. These checks do not certify every rotated, degenerate, or
portal-grafted arrangement, nor the native screen-reader traversal.

## Accessibility focus blocking

Pinned Flutter distinguishes a blocked node from a blocked subtree.
An explicit None on a child does not override its parent's subtree
block. The retained Rust scope publishes the effective blockade
without removing either node or child's other actions. The direct
semantic Focus action is refused for blocked nodes, but actual
keyboard focus and keyboard dispatch stay independent.

- `widgets/semantics_test.dart:1167` matches the retained parent's
  unblocked-to-subtree update; both parent and child receive the flag.
- `:875` and `:940` match node/subtree propagation with both explicit
  and default child settings. They are partial because Rust does not
  expose the originals' custom semantic action descriptors.
- `:1149` matches clearing explicit focused metadata on a blocked
  node. Rust uses its effective block type and false boolean in
  place of Flutter's separate flag and nullable focused value;
  its independent action control uses Activate, not a custom action.

The authored Flutter control keeps an actual FocusNode focused and
delivers keys across all three modes. The mounted Rust control likewise
retains the node IDs, focus and activation callbacks. Merging retains
the strongest block. All 33 focused Rust tests and four named Flutter
originals passed. No native assistive-technology run is claimed.
