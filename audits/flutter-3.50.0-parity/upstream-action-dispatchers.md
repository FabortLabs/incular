# Action dispatcher and nearest scoped actions

Five pinned Flutter originals passed. The default dispatcher receives
the intent, invokes an enabled action once, and returns its result.
The native command registry exposes the same direct operation, plus an
inherited ActionScope which does not need to claim focus. Programmatic
lookups from a mounted descendant return handled for the known command
and none for an absent mapping without calling any handler.

The original titled “invoke throws when no action is found” actually
calls the optional `Actions.maybeInvoke` path and asserts null, just
like the preceding original. Both native comparisons cover the
executed null path; this is not evidence of a throwing invocation
API. All 13 focused runtime and keyboard routing tests passed.

## Custom and inherited dispatcher selection

Four exact originals passed. The native code finds the action first,
then looks for a dispatcher from the owning action scope outward.
A closer dispatcher with no matching action does not intercept an
outer action. The independent dispatcher query does return the
closest explicitly registered instance.

An authored pinned probe confirms an action in an inner mapping with
no dispatcher uses the outer dispatcher. A disabled inner mapping
shadows the outer action and invokes no dispatcher. The native fixture
checks intent, action, handler order and dispatcher identity.
All 25 focused runtime tests and the renderer-neutral facade check
passed. This batch verifies programmatic action dispatch; separate
shortcut-bound action paths are not counted here.
