# Mounted external focus scope nodes

The pinned Flutter originals `focus_manager_test.dart:1607` and
`focus_scope_test.dart:1085` passed by exact name. A mounted external
scope now has its own retained node. Child focus marks the scope as
focused without giving it primary focus; an explicit
`request_scope_focus()` takes primary focus for the scope rather than
following its previously active descendant. The scope anchor is not
an ordinary Tab stop. The Rust runtime tests use the nested focus
fixture and also request focus on a centered scope measured at exactly
zero width and height.

The initial mount does not depend on layout. The retained-tree scope
lookup resolves explicit ancestor scopes; the implicit Flutter root
scope fallback, focus attachment API and complete unfocus policies are
not covered by these tests.

## Scope request and descendant permissions

Pinned `focus_manager_test.dart:781` verifies that denying requests
for a scope also denies its descendants, without conflating the
independent descendant opt-out. It passes on the pinned SDK and
on Incular's scope node. The mounted test also disables a live scope,
clears a formerly focused child, excludes the subtree from traversal,
rejects an explicit child request, and re-enables the scope without
replaying the rejected request. Changes to the scope or its descendant
permission schedule the idle runtime to process the new state.
The :801 original also asserts scoped focus history, which is not
covered by this batch and leaves that registration partial.

## Retained focused-child history

The paired authored `mounted_focus_scope_history_test.dart` and
`flutter350_focus_scope_nodes.rs` tests exercise mounted controls, not
the standalone focus manager. Focusing a child records it in its nearest
scope. Direct scope focus does not erase the selected child from history.
Moving the focused GlobalKey keeps the same element and primary node,
clears the old scope's child, updates the new scope's child, and sends the
next key only through the new ancestor. Moving the node back while it
is inactive does not resurrect discarded history. An outer scope
remembers its immediate nested scope, not the nested leaf.

Pinned `focus_manager_test.dart:739` also verifies focused-child history
after moving a node within a scope. Its separate manual focus attachment
and parent enumeration are not covered by the mounted Rust fixture, so
it stays partial. The existing :812 widget reparent remains partial
because its root-scope activation and label checks are distinct.

## Requests through a mounted scope

A mounted scope delegates focus to its eligible descendant through the
retained runtime. Supplying a node in another scope is rejected.
`request_focus_in_scope()` follows the last eligible child through
nested scopes, landing on the scope itself if that scope has no eligible
child. The pinned authored probe checks both restoration to the leaf
and fallback to the inner scope when the leaf is disabled. The mounted
Rust tests also verify a request before the first layout. Scoped
requests through the nearest mounted scope complete the original
`focus_scope_test.dart:16` and :30 reactive-label comparisons.

## Inherited eligibility without overwriting child configuration

Pinned `focus_scope_test.dart:2016` disables and re-enables the same
mounted scope and observes the same external child's public
`canRequestFocus` value. The native result is true, false, true,
matching the reference. A second locally-disabled child remains disabled
through both rebuilds. The retained scope owns the permission, and
its mounted descendants compute effective focus eligibility without
rewriting their own flag. Requests denied during the disallowed interval
do not reappear. This check is specific to focus scopes; the runtime
separately enforces actual focus requests and traversal eligibility for
ordinary non-scope Focus ancestors.

## Ordinary Focus descendant flags

The pinned :115, :170, and :210 registrations passed by exact name.
The native mounted two-branch test checks actual routing, focus
history, and traversal membership. An ordinary Focus node remains
focusable when it disallows descendant focus. A blocked child's
explicit request cannot steal primary focus and both branches can
be blocked independently. A traversal-only restriction skips a
descendant on Tab but permits a direct request. An ordinary parent's
own can-request flag does not exclude its children. The independent
ordered traversal queries for :170 and :210 are verified below.

## Unfocus and fallback through scope history

The pinned `focus_manager_test.dart` originals :1081, :1172, :1265,
and :1341 passed by exact name. The mounted Rust fixture reproduces
their two branch scopes, four children, and original focus order. It
compares both unfocus dispositions, repeated refocus and unfocus,
moving to a sibling scope via the parent, preserving an inactive
scope's history, and skipping disabled remembered children. The
default parks on the enclosing scope and clears that scope's history;
the previous-child disposition instead removes only the current child
and restores the most recent eligible one. Disabling the active scope
restores a sibling's child without discarding the disabled scope's
own history. The explicit outer Rust scope supplies the reference
fixture's root focus target; implicit-root setup is independent.

## Explicit focus parents

Pinned `focus_scope_test.dart:438` and `:502` passed by exact name.
The Rust counterparts mount the same two sibling branches, explicitly
attach one to the other's focus node or scope, then insert another widget
ancestor. The original leaf remains primary; its logical ancestors
retain focus, and the inserted ancestor never receives it. The ordinary
node case dispatches a real key and checks the exact recipient order.
The scope case checks every scope's immediate focused child. All 23
targeted mounted-scope tests passed.

An absent or cyclic target safely falls back to widget ancestry in the
runtime. Flutter's manual attachment API and debug diagnostics for
invalid parents are separate from these comparisons.

## Ordered traversal queries

The pinned `focus_manager_test.dart:170` and `:210` originals
passed by exact name. The mounted native counterpart asserts
every ordered traversal list before and after disabling each branch,
then the whole scope. A parent that cannot request focus drops
out of its own parent's list without removing its child from its
local traversal list or moving existing primary focus.

The pinned authored `explicit_focus_scope_traversal_test.dart`
probe additionally verifies that the explicit scope owns traversal
membership and direct focus eligibility, not the visual scope.
Disabling traversal alone still permits direct requests. Clearing the
override moves focus history to the visual scope without changing the
mounted leaf's identity or bounds. Native coverage adds simultaneous
valid reparenting followed by a true cycle, which falls back safely
without disabling focus. All 43 targeted mounted-focus, key-routing
and traversal tests passed. The authored probe and cycle checks do not
increase upstream coverage.

## Raw mounted descendants

Pinned `focus_scope_test.dart:1151` passed. The mounted fixture
locates two different enclosing focus nodes through keyed elements.
It checks the whole seven-node postorder and separately verifies
the two-node sub-branch. The raw enumeration still includes nodes
that cannot receive focus, and removing a branch unlinks every
held handle. The runtime updates the links only when topology
changes, keeping existing siblings in their attachment order.
All 32 focused scope and primary-observer tests passed. The
Flutter build owner's synthetic root scope is a separate contract.

## Moving active subtrees between scopes

Pinned `widgets/focus_manager_test.dart:886,932` passed. In the native
mounted fixture, reparenting the active leaf clears its old scope history
and appends the same node to the new parent without changing its primary
focus or element. Moving a whole scope beneath a former sibling's leaf
updates every ancestor of the active leaf, and both complete postorder
lists match. The native fixture provides explicit equivalents of the
Flutter test binding's root, traversal group and view nodes; it verifies
the topology behavior, not implicit Flutter binding initialization.
All 34 targeted focus and lifecycle tests passed.

## Inherited permissions on a logical focus branch

The authored pinned `explicit_focus_ancestor_permissions_test.dart`
checks ordinary Focus parents, not just scopes. The public node's
effective focus request and traversal settings follow the assigned
parent. A traversal-only restriction does not prevent a direct
request. A denied request does not reappear when the permission
is restored. Clearing the override restores both inherited values
from the physical parent's focus node. This regression reproduced
the old native getter error before the fix. The mounted ancestor
walk no longer allocates a separate list to check scope permissions.
All 45 targeted focus tests passed. This probe adds no original
registration credit.

### Focus widget gates and recorded history

The authored pinned `authored_focus_descendant_gate_test.dart` checks a
configured Focus widget, rather than an externally mutated node. Disabling
its descendants restores the previous eligible control in the same scope.
When the leaf is explicitly assigned to a different focus parent, the
disabled visual wrapper no longer blocks traversal or direct focus.
The native baseline reported the right public properties but incorrectly
excluded the leaf from actual traversal. The permission now lives only
on the focus node, and disabling it settles focus using scope history.
Explicit exclusion and hidden-content regressions remain green. All 50
focused tests passed. This authored probe adds no registration credit.

### Scope and descendant traversal permissions

Three exact Flutter registrations passed in `widgets/focus_scope_test.dart`
(1382, 1508, 1602), with corresponding mounted Rust runtime assertions in
`flutter350_focus_traversal_permissions`. The nested scope/parent/leaf
fixture checks all five ordered descendant lists from Flutter, including
the case where excluding the inner scope does not exclude its focus
children. Marking a parent as non-traversable reports no traversal
descendants on that parent, removes only its descendants from the scope's
list, and still permits an explicitly requested focus on the leaf. The
mounted permission update reaches a reactive builder observing focus even
when its node is unfocused and its displayed focus state does not change.

Previously the scope widget had no authored controls for skipping its own
node or limiting descendants' focus and traversal separately; only the
supplied node exposed the flags. The same settings now reach owned or
provided scope nodes during retained updates. The external-node constructor
keeps the node's settings authoritative. The focused native controls also
verify owned scope identity remains unchanged and distinguish a denied
request from simply omitting a descendant in traversal.

### Context lookup with explicit focus parents

The authored pinned `explicit_focus_scope_lookup_test.dart` mounts a left
and right scope, with a focus boundary mounted under the left scope but
attached to the right one. The boundary still owns `Focus.of`, but
`FocusScope.of` inside the branch resolves to the right scope. A left
sibling outside the branch still resolves to the left scope. A nested
scope attached to the right is its own nearest scope. Updating the focus
parent while retaining the widget and node moves the branch lookup back
to the left scope.

The native baseline returned the left scope for the entire widget
subtree, even though the branch node itself already knew it was attached
to the right. `FocusScope::maybe_of` now finds the context's nearest
widget focus boundary and then follows its mounted focus parents. Its
optional API still returns `None` without a scope, rather than returning
a framework-wide root scope as Dart's non-optional API does. This
authored comparison adds no original coverage credit.
