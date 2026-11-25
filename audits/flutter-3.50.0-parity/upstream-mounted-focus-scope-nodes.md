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

This change does not infer the nearest scope from a build context or
implement Flutter's complete per-scope focused-child history.

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
