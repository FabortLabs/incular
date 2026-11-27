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
