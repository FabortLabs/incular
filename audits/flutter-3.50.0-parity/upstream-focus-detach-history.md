# Flutter focus fallback after detachment

Pinned `widgets/focus_manager_test.dart:1777` passed. Replacing the
autofocused node with a non-autofocus node in the same widget slot
now leaves neither child active and focuses the retained scope
itself. Before the fix, Incular left the scope unfocused.

Detachment now resolves the old hosted scope chain instead of the
replacement's ancestry. A real pending focus request wins. With no
request, the nearest still-live scope restores its most recent
eligible child, or takes primary focus itself. A retired node is
not restored, and a used autofocus request on a surviving child
does not replay.

The pinned authored probes and Rust tests cover an earlier history
entry, a fresh request on the replacement before it attaches, and
retention of a nonfocused surviving autofocus child. That survivor
test formerly expected no native focus; Flutter confirms that the
scope itself takes focus while the survivor stays inactive. A
separate native control replaces a pending autofocus before first
layout and verifies neither the dead node nor the reused slot gets it.

Twenty-nine focused Rust checks passed, covering mounted scopes,
editable focus, mount autofocus and retained focus history. The three
authored Flutter controls do not add upstream registrations.

## Programmatic scope autofocus

Pinned `widgets/focus_manager_test.dart:660` passed. The native
mounted scope now accepts an explicit autofocus candidate and
evaluates it after attachment and explicit focus requests. In the
matching test, the first request selects the second child of an
ordinary focus parent, and a later autofocus request does not
replace the explicitly focused child or change scope history.

The authored Flutter control verifies a target moved out of the
enqueuing scope is discarded and the first valid candidate wins.
Native controls also cover queue isolation between two runtimes
sharing one UI thread. The native API requires a mounted scope;
an unmounted scope does not enqueue a request. All 26 selected
Rust tests passed. The authored case does not add an upstream
registration.

## Scope skip traversal

Pinned `widgets/focus_manager_test.dart:846` passed. A new mounted
snapshot separates raw traversal descendants from actual Tab stops.
A scope anchor is eligible in the raw parent query by default;
setting its skip flag omits only that anchor. Both leaf nodes stay
in the scope snapshot, the active leaf keeps primary focus, and the
scope keeps the same focused child. Incular still does not make
the anchor an ordinary Tab stop.

A separate pinned authored control verifies child-first depth-first
order and the different behavior of `descendantsAreTraversable`:
that flag removes the descendants from traversal without changing
the active leaf or blocking explicit focus. All 34 selected native
focus checks passed. Nested Tab policy is not inferred from the
raw snapshot, and this authored case adds no upstream registration.

## Scope history and keyed reparenting

Pinned `widgets/focus_manager_test.dart:1825` passed. Incular dropped the
queued scoped autofocus when a scheduled build moved the same GlobalKey
element into a different scope. The mounted request now settles before
the scheduled build, so focus follows the retained element into the new
scope. A target moved synchronously before the focus boundary is still
rejected by its old scope; a target awaiting attachment is evaluated
after layout without letting later requests jump the queue.

Pinned `:1874` also passed. The native case changes a keyed leaf's
ordinary parent while its scope is inactive. The scope's focused child
remains the same, the other scope stays focused, and a later request
for the original scope restores that leaf. All 40 selected native
checks passed, including autofocus mount, direct requests, actual
reparenting, detach fallback, and per-runtime queue isolation.
