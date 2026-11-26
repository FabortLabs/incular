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
