# Sliver semantics annotations

The ten pinned Flutter originals at `widgets/sliver_semantics_widget_test.dart:
18,73,114,210,232,251,298,342,386,437` passed.

- **Reading direction (210, 232, 251, 298).** The real retained node
  inherits the ambient RTL/LTR direction. An explicit direction overrides
  it in either direction, as the original assertions show despite the
  test names. The two inherited cases are paired; the tests for explicit
  direction also assert Flutter's complete two-pane tree.
- **Reparenting (114).** Two annotations on one sliver previously collided
  on the same generated child ID, rejecting layout. The native test inserts
  and removes a nested container and verifies exact edges, stale node
  removal, and a stable outer node. The native fixture does not model
  Flutter's GlobalKey-based render-object move or two-pane graph.
- **Annotation content (342, 386, 437).** The native graph includes label,
  hint, and current value on one node. Independent hints merge in order
  with one newline for both sliver and box annotations. The field and
  merging checks pass; the renderer-specific outer topology is partial.
- **Disable/restart (18).** The host projection removes the label when
  disabled, schedules a frame when re-enabled, and publishes the fresh
  node with the same direction beneath an implicit-scroll ancestor.
  The native host stays idle once the update is presented.

Incular currently projects one scroll semantic node per viewport; it
does not split Flutter's scrolling and non-scrolling panes or install
the internal `RenderViewport.twoPane` tag. The direct viewport tag
constraint at line 73 remains reference-only. The use of the renderer's
reserved anchor IDs changes neither leaf child IDs nor paint/hit routing.

## Merging a hint and a value without merging competing values

Pinned Flutter lines 482, 535, 584 and 635 and the retained
`sliver_semantics` tests compare annotations on both sliver wrappers
and boxes in a lazy list. Two current values remain distinct siblings
in reading order. A hint and one current value merge onto the same
node, both with LTR reading direction. These are exact annotation
checks within the subtree; the original assertions also include the
Flutter-only two-pane ancestor described above.

## Action replacement, descriptions, and exclusion

All seven pinned Flutter registrations at lines 1170, 1210, 1351, 1393,
1423, 1474 and 1715 passed. The native
`flutter350_sliver_semantics_actions` tests dispatch through Runtime, not
the descriptor closures alone:

- Replacing tap's callback runs the newer function on the same node ID.
  It does not change the semantic revision or republish a native update.
  Adding or removing long press publishes one projection; the removed
  action cannot invoke the stale handler.
- Tap/long-press hints annotate the appropriate standard action without
  exposing an additional invokable custom action. Two named custom
  actions have distinct IDs and invoke only their own callbacks.
- Tooltip, current value, increased value, and decreased value stay
  separate. The increase/decrease actions remain present.
- `SliverSemantics::exclude_semantics(true)` keeps the parent annotation
  and removes descendant semantics only. The box still paints at its
  original 10 px height and receives real pointer input. Toggling the
  option restores the child beneath the same parent node.
- Each of the nine currently supported callback kinds is added, invoked,
  removed, and checked for stale dispatch. Independent copy/cut/paste,
  accessibility focus gain/loss, and set-selection handlers on an
  arbitrary sliver remain open; editor-specific capabilities are separate.

The original complete-tree checks also include Flutter's two-pane graph.
Its default reveal uses a callback without advertising an extra action,
whereas Incular's portable projection exposes `ShowOnScreen` directly.
The tested callback and annotation behavior matches; these broader
dispositions remain partial.

## Heading annotation on a sliver's text

Pinned lines 2672, 2702 and 2773 passed. A sliver configured as a heading
previously created an unlabeled heading node while its text appeared on
a plain child. The `sliver_semantics` tests now verify the actual six
labeled heading nodes (levels 1–6) and a non-heading sibling, including
the semantic role. The heading consumes compatible text, not independent
actions: a descendant button remains a separate actionable node.

The same tests reject signed values −1, 0, 7, 8 and 9 in debug builds.
The `flutter350_semantics_heading_geometry` target passed all nine
parent/descendant level combinations in a merged retained graph.
The Dart tests also examine the descriptor directly and call its
render configuration's `absorb` method. Those direct APIs are partial;
the rendered heading-node case is paired.
