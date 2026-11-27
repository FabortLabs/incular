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

## Typed lookup and optional callbacks

Four exact registrations passed. Looking up a typed action returns
the registered action state across repeated builds without invoking
it. Optional lookup gives none for an absent mapping; required lookup
returns a typed Rust error instead of a Dart assertion.

The callback factory forwards the original typed command to the
registered action and returns no callback for a missing or disabled
action. The native fixture also disables the same action after
obtaining its callback, then verifies that the callback rechecks
its enabled state. All 21 focused native runtime tests passed.

## Focusable action detector

Five exact registrations passed. The two highlight cases verify that
a focused, hovered detector invokes its shortcut while enabled;
disabling clears the visible hover and focus highlights and blocks
the shortcut; re-enabling under the stationary pointer restores only
hover; and leaving while disabled prevents stale hover on re-enable.
The native retained reconciliation previously copied hover silently
even when its derived visible highlight changed.

A specified text cursor overrides the surrounding forbidden cursor;
the default defers back to the surrounding region. The detector can
be skipped in traversal while remaining explicitly focusable. Its
focus semantics default on, can be removed independently, and leave
the descendant labels intact. All 53 focused Rust runtime, focus,
keyboard, and semantics tests passed.

## Direct callback adapters

Both pinned Action subclasses originals passed. A CallbackAction
passes its invocation's typed intent unchanged and returns the
callback's result. Its optional binding into a Rust command registry
still uses an explicit command key. VoidCallbackAction invokes the
callback carried on the supplied intent and returns no value. All 11
focused native callback and dispatcher tests passed.

## Keyboard dispatch and key consumption

The pinned base-action conversion original passed. Successfully
invoked actions now distinguish a handled native key from a key that
should bypass the remaining framework handlers without consuming
the native event. The latter is not the same as declining the action.

An authored pinned probe confirmed that a shortcut reads the action
from the focused element's ancestor chain but uses the dispatch
policy at the focused caller. Direct programmatic invocation still
uses the action owner's dispatcher. Native cases check both paths,
including the returned consumed status and the untouched outer
fallback handler. The passive shortcut scope supplies bindings only
and does not take focus. Only the one original conversion registration
adds paired credit. All 32 focused native tests and the renderer-neutral
facade check passed.

## Detector descendant permissions and disabled action shadowing

Five more pinned originals passed. A disabled inner mapping stops
action lookup without invoking either action or dispatcher. Rust
reports the found but ignored mapping separately from an absent
mapping. A detector without highlight callbacks still routes its
enabled shortcut, and disabling it stops the invocation.

The detector forwards its traversal skip and independent descendant
focus/traversal permissions to the retained focus hierarchy. Revoking
focus clears a focused child and rejects a later request. Revoking
traversal still permits direct focus. A pinned authored comparison
distinguished two edge cases: with no eligible next node, an empty
root keeps its current focus; the TestWidgetsApp route uses ParentScope
edge behavior and moves focus from the child to the route scope.
The native runtime now preserves that distinction. All 23 focused
native tests and both authored pinned cases passed.

## Directional focus while disabled

The pinned directional-navigation original passed. In traditional
navigation a disabled detector cannot take focus or show a focus
highlight. In directional navigation it remains a valid focus target
and shows its traditional-mode keyboard highlight, but its disabled
shortcuts still do not invoke actions. The native detector now reads
the scoped navigation mode separately from its enabled state, and
tracks that media field independently of unrelated traits. The native
regression failed before the fix; all 58 focused tests passed after it.

The follow-up keeps the detector's focus element available immediately,
before layout, and responds to navigation updates without remounting it.
A pinned authored probe also confirmed that a disabled directional
detector removes both local maps: Enter no longer activates the local
action, and explicit invocation falls through to the outer action.
The native regression checks the same behavior. All 62 targeted native
tests passed, including the earlier nested dispatch and autofocus tests.
