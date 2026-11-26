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
