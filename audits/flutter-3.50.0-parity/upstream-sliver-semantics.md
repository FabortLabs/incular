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

A sliver viewport with tagged children now projects an outer boundary
and a scrolling pane, keeping the viewport's outer ID and the usual
child IDs. The use of the renderer's reserved anchor IDs changes
neither leaf child IDs nor paint/hit routing.

## Viewport tag scope

Pinned line 73 and its matching Incular target exercise a main-axis
group with a non-semantic spacer and a text sliver. The implicit scroller
does not tag itself, but its immediate semantic child carries the shared
renderer tag. That stays true across an idle frame without ID or
revision churn. A more specific Flutter/Incular probe checks that
`RenderViewport.twoPane` crosses a layout-only padding wrapper and stops
at the first semantic boundary, while an application tag declared on
that boundary goes only to its own child. A separately constructed
same-name tag is not the framework tag.

The pinned line 682 and 735 cases also passed. The retained topology
comparison is described under nested container labels. The original pinned-header tag
transition (`pinned_header_sliver_test.dart:250`) and nested-pane
transform (`scrollable_semantics_test.dart:814`) passed on the reference;
pinned transition and nested reveal results are described below. The empty-viewport transition is covered by the focused pinned
Flutter probe `sliver_semantic_viewport_panes_test.dart` and retained
`flutter350_semantics_tags`: attaching a semantic child splits the
viewport, keeps the outer ID, and only the inner pane accepts
ScrollToOffset. Removing all children retires the inner ID, restoring
the single outer scroller. Reattaching a child allocates a new inner
ID so old native scroll requests cannot affect it. The 38 focused Rust
checks include action routing, native updates, and transformed-cache
regressions under the new layout.

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

## Transformed descendants at the viewport's semantics cache

Pinned Flutter lines 2808 and 2857 passed. The native
`flutter350_sliver_semantics_restart` fixture verifies the actual graph:
a singular fitted sliver removes its nested scroll node but leaves the
800×600 outer viewport untouched. Restoring a positive height creates
the child again, and returning to zero removes it.

The rotated-label case uses the reference Ahem geometry. Labels 1 and 2
are visible; 4 and 5 are cache-only and hidden. Label 3 is outside the
cache and absent, not just hidden. Translating the subtree 30 px brings
label 3 into the cache as a hidden node, then removes it again while
labels 4 and 5 keep their IDs. The original's two-pane shell still
differs, so that registration is partial.

Incular had checked the cache at direct sliver children but not their
transformed descendants. The shared window now follows the subtree.
It prunes projected nodes rather than cutting off traversal at a
roleless wrapper. A separate Flutter/Rust comparison with opposite
500 px translations and an intermediate padding box verifies that a
descendant restored to (2, 2) is still exposed. Existing ensured-sliver
and off-cache regressions also pass.

## Reading order through nested axis groups

Pinned lines 1519, 1606 and 1661 and the retained `sliver_semantics`
fixtures cover ordinary and keyed traversal. Unkeyed nested groups read
labels 1–5 in spatial order. The keyed cross-axis group reads 2, 3, 1,
with the keys and text on the same nodes rather than extra unlabeled
wrappers. Nested keyed groups read [5, 4, 3], then 2 and 1 without
flattening the explicit child-node boundaries. Updating the keys changes
the order without replacing the existing labeled nodes.

Incular's named `OrdinalSortKey` represents the key group in the native
fixtures; Dart subclass sort-key identity and the two-pane wrapper
are still open. The unkeyed behavioral comparison is paired.

## Validation result and precedence

Pinned Flutter lines 2538 and 2583 passed. Incular now exposes
`SemanticValidationResult` with `None`, `Valid`, and `Invalid`
in the retained graph. The native sliver suite verifies all nine
outer/inner combinations: invalid outranks valid, and valid outranks
an unspecified result. The merged result is on the single labeled
node. Rebuilding a labeled annotation changes its validation state
without replacing that node's ID.

AccessKit projection reports invalid for either the typed invalid
result or the legacy invalid flag; explicit validity and unspecified
both omit AccessKit's invalid marker but remain distinct in the
portable state. The web mapping uses the same effective invalid bit;
a browser/wasm build was not run here. Flutter's direct render
configuration query (2538) remains partial, while the labeled
precedence case is paired.

## Blocking semantic user actions

All six pinned Flutter originals at lines 2298, 2341, 2385, 2421, 2453,
and 2494 passed. The new `Semantics::block_user_actions` applies both
to box annotations and to sliver annotations, whose descendants are
physically retained next to the annotation anchor. The native runtime
tests confirm that:

- An explicit group can keep both child labels and withhold their
  tap actions. Requests to the blocked IDs are rejected, while a
  physical tap on the same box still works. Unblocking restores the
  callbacks without replacing either semantic node.
- Two child tap annotations remain distinct even if one is blocked.
  Only the unblocked node can invoke its handler. The explicit/action
  contract matches; Incular's portable reveal action may appear on
  surrounding unblocked nodes.
- When a blocked tap and an unblocked long press have compatible
  annotations, their labels merge in traversal order on the sliver
  node. The blocked tap is not advertised or dispatched. The live
  long-press handler can be replaced without a native tree update, and
  unblocking restores tap dispatch on the same merged node.
- Action conflicts are assessed before filtering blocked handlers. Two
  declared taps still prevent merging even if one cannot execute.

The ten focused action tests and fifteen runtime action/keyboard focus
regressions passed. The merged cases are partial matches because
Incular exposes its portable `ShowOnScreen` action on the node,
whereas Flutter uses a separate fallback rather than an action bit.

## Typed text action payloads

The pinned all-actions case (line 784) passed. The focused Flutter
probe `sliver_semantic_text_payloads_test.dart` and Incular's
`flutter350_sliver_semantics_actions` exercise `SetText` and
`SetSelection` on a plain annotation, without a text editor. The
handler receives multiline Unicode or an empty string verbatim and
preserves reversed and forward base/extent pairs. Replacing the
handlers uses the same semantic node without publishing an update.
Removing them rejects requests to the previously advertised actions;
a blocked callback or an unbound payload kind is not advertised.

All 27 targeted Rust tests passed, including the existing formatter,
on-change, and disabled-field behavior. The all-actions case remains
partial: the clipboard and expansion cases are described below;
cursor and focus callbacks are covered below. Flutter's two-pane
tags and host-specific event routing remain open.

### Clipboard and expansion callbacks

The pinned `sliver_semantic_clipboard_expansion_test.dart` probe and
the retained/mobile-projection fixture verify that Copy, Cut, Paste,
Expand, and Collapse invoke only their own registered callbacks.
Replacing handlers retains the native ID without a tree revision;
blocking or removing the actions withdraws their advertisement.
Flutter's low-level `SemanticsOwner.performAction` still invokes a
blocked handler when given an exact ID directly. Incular instead
rejects the stale request at both runtime and adapter validation.

The AccessKit fixture also verifies that switching from Expand to
Collapse preserves the native ID, withdraws Expand, and dispatches
only Collapse. Clipboard actions are verified through the portable
mobile projection, not host clipboard or assistive-technology
integration. The 29 focused Rust checks passed.

### Character cursor callbacks

The pinned `sliver_semantic_cursor_payloads_test.dart` probe and the
matching retained/mobile projection test exercise forward and backward
movement with each value of the selection-extension flag. Both
directions invoke their own current callbacks. Replacement emits no
native update; removal withdraws the two action bits and rejects stale
requests. The 30 targeted Rust tests passed. Host assistive-technology
routing and movement by word are not covered by this slice; neither
word callback is publicly configured by Flutter's SliverSemantics.

### Accessibility focus lifecycle

The pinned `sliver_semantic_focus_callbacks_test.dart` and retained
mobile projection tests distinguish requests for input focus from
assistive technology's gain/loss notifications. The plain annotation
runs the current callback without becoming keyboard-focusable or
moving existing input focus. A user-action block withdraws Focus but
retains both lifecycle notifications. Replacing handlers does not
change the native tree; removing them rejects previously advertised
requests and does not publish a bare unbound action. The existing
keyboard-focus suite still passes (31 focused Rust checks in total).
This tests portable requests, not actual screen-reader event delivery.

### Nested viewport pane transforms

The pinned `scrollable_semantics_test.dart:814` original and
`nested_shrink_wrapped_viewport_panes_stay_aligned_during_semantic_reveals`
both compare the inner and outer pane's world rectangles through a
series of accessibility reveals inside a shrink-wrapped list. Incular
had failed to mark rows clipped by the outer box scroller as hidden,
and sent reveal requests to the zero-range inner list. The outer clip
now propagates to the nested semantic nodes, and a zero-range
viewport passes implicit reveal to an eligible scrolling ancestor.
The retained check exercises moving to the end, leaving an already
visible row stationary, moving back to the start, and moving forward
again; the requested row is not hidden after any reveal. All 25
focused runtime regressions passed.

### PinnedHeaderSliver pane migration

The pinned upstream case at `pinned_header_sliver_test.dart:250`
and the authored Flutter/Rust pane fixtures compare the header
before, at, and beyond its pin boundary. At offsets 99 and 100 the
header stays in the scrolling pane. At 101 and 130 it is pinned at
y=0, carries the framework exclusion tag, and becomes a child of
the outer pane. Scrolling back to 100 restores its original pane.
The native header, body, and both pane IDs survive every transition,
including a viewport resize while pinned. Scroll actions and range
stay on the inner pane. All 19 focused Rust cases passed.

The upstream case checks a separately tagged render wrapper above
the text node. The portable graph attaches the tag to the first
projected node instead of generating that extra wrapper, so the
case is recorded as partial.

### Scroll collection size and position

The pinned `sliver_semantic_scroll_collection_test.dart` and retained
tests verify that the scrolling pane exposes the full collection size
and first visible indexed child, while the outer boundary exposes
neither. In a 50-item, 50-pixel list, a cached hidden row retains its
node ID as the reported index changes 0 → 1 → 2 → 1 → 0. Pinned
headers do not contribute an index. The same child and collection
metadata reaches the mobile projection without changing its native
scrolling-pane ID.

The GridView comparison reports 31 tiles rather than 11 physical
rows. Scrolling 300 pixels advances its first visible item from 0
to 3, not 1. Scoped ID zero no longer turns headers and other
unindexed sliver boxes into index 4,294,967,295. The 23 focused
Rust regressions passed. These are authored tests, not new upstream
registration sites. Operating-system scroll-event delivery is not
verified.

### Per-sliver semantic paint clip

The pinned Dart overlap probe and retained fixture run on both axes
with reverse on and off. A 20-pixel pinned header leaves one pixel of
row 0 exposed at offset 49. At exactly 50 and at 55, the row is
still in the semantic cache but is hidden behind the pinned header.
The first visible index is 0, then 1; the row's semantic ID persists.
The clip uses the following sliver's own overlap and remaining paint
extent in viewport coordinates, rather than the single viewport-wide
rectangle. The scroll index uses the pre-sort child order: a reversed
viewport can have a different reading order without reporting its
last visible item as its first. All 21 focused Rust tests passed.

### Resizing and persistent header policies

The pinned upstream `sliver_resizing_header_test.dart:401`, paired
authored header probe, and retained tests use a 300-pixel header
after a 100-pixel spacer. The resizing renderer stays in the
scrolling pane at offsets 99 and 100, then moves out at 101 while
keeping the same node ID and 800×300 rectangle. Scrolling back
restores it. A pinned persistent header is excluded even at
offset zero. Incular checks both its fixed and delegate-based
persistent header forms. The 18 focused Rust cases passed.
The upstream resizing case is partial because it additionally
asserts Flutter's separately tagged render wrapper.

## Declared flags and actual input focus

The pinned all-flags registration at line 934 passed. The authored
focus probe and paired retained check distinguish an unspecified flag
from unconditionally declared false. `focused(false)` and
`focusable(true)` report a focusable but unfocused semantic node.
`focused(true)` reports focused; removing the declaration or setting
only `focusable(false)` removes focusability. The node ID remains
stable, and none of these annotation changes creates a keyboard
target or makes an unbound Focus action executable. The projected
values follow through the portable mobile adapter. All 37 targeted
Rust checks passed. The upstream combined role and route flags are
not yet completely covered.

### Independent role and route annotations

The pinned authored flag fixture and retained/runtime projection
exercise ten independently settable flags on one semantic node.
Button, slider, link, keyboard key, text field, header, image, route
scope, route name, and exclusive group state survive alongside the
single primary role used by platform projections. Rebuilds withdraw
the flags without replacing the node. Expanded false remains explicit,
and an indeterminate check is distinct from the toggled-only state.
Merged annotations union their independent flags. Route scope requires
explicit child nodes in debug, matching the reference precondition.
All 50 selected Rust regressions passed. Platform role dispatch and
route announcements still have separate verification work.

### Nested container labels

The originally unported tag cases at lines 682 and 735 now have
native fixtures for both nested SliverSemantics and box annotations
in a lazy SliverList. The scrolling pane has one tagged parent
with two untagged, labeled leaf children in reading order. A plain
sliver boundary absorbs its compatible label instead of creating an
unlabeled wrapper and a redundant text node. Explicit child nodes
remain separate. The pinned authored probe and native regression
also verify text replacement, release as an explicit child, and
reabsorption with the same parent ID and stale leaf removal.

All 51 focused Rust tests passed. The upstream cases remain partial
because Incular still exposes its portable default ShowOnScreen
action and legacy numeric scroll value in that graph.
