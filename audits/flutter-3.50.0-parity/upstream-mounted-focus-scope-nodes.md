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
