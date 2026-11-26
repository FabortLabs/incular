# Focus scopes during simultaneous page pops

Pinned Flutter 3.50.0 `widgets/routes_test.dart:1921,1964` both
pair. The native route outlet mounts a retained focus scope for each
normal Page route. On a committed activation, a page with no focused
child/autofocus can focus its own scope instead of leaving focus
vacant; it does not override a live unrelated focus target or an
invalidated explicit saved record.

The native test pushes three actual routes, verifies focus belongs to
only the active page's scope, and pops two using one `pop_until`
call. The surviving first scope regains focus both with inert page
contents and when the second page had moved primary focus from its
scope to a real keyboard-focusable child. Popped scopes no longer
own focus. The new test reproduced a missing scope before the fix.

Regression controls distinguish a page's scope from a content
control and check that the scope's one presentation follow-up does
not turn into a navigation retry loop. The recovery control uses
a third focus target to distinguish live scope history from stale
captured focus after a rejected rebuild.

The scoped outlet target passes 91 tests, including the new two-case
test. Its 11 pre-existing failures reproduce with the same names
before and after this change; they include focus lost during an
abandoned build and retry scheduling. This does not claim parity
for those transactions or for implicit scopes on popup presentations.

## Preserving committed focus while a route is still speculative

The 11 baseline outlet failures above are now resolved. A retained
outgoing page keeps its current focus while the next page is only a
speculative build. A rejected frame or a builder that pushes another
route cannot clear that focus. On commit, a normal incoming Page
releases the exact captured outgoing focus, not an unrelated request.
A transparent popup and a focus-neutral reorder still do not steal
focus. Reentrant navigation from a focus listener waits until the
new route actually mounts.

An EditableText regression starts an unsupported route during the
next page's build. The committed editor continues to receive real
text input after the rejected frame, stops receiving it once the
new page commits, and receives it again on return to the same retained
editor. The combined ownership fixture explicitly removes a rejected
pageless route instead of assuming a compatible page update removes
it; it still verifies detached outlets do not schedule work.

All 103 tests in the focused route-outlet filter passed. The additional
recovery checks are native regression coverage, not new upstream
registrations.

## Explicit route and dialog focus

Pinned :2051 and :2776 now pair. `ModalRoute.request_focus()` reports the
resolved default true on a page and false on an explicit opt-out.
An opt-in dialog takes primary focus in its own scope, stops editor
keyboard delivery, and returns focus to the same editor when popped.
An opt-out dialog leaves that editor focused and its own scope
unfocused. The native route uses `Route::dialog().request_focus(..)`
rather than Flutter's `showGeneralDialog` function.

An authored Flutter control and native test also verify that an
opaque page can decline focus while its prior retained page still
owns the focus. The native default for an unspecified popup stays
focus-neutral; the new behavior is opt-in for dialogs. The
focused route target passed all 106 tests.

### Stacked dialogs, one of which declines focus

Pinned Flutter was probed with an explicitly neutral dialog above an
editor, then a focus-taking dialog above both. The second dialog owns
primary focus; on pop the first dialog stays open and unfocused, and
the editor regains primary focus. Incular's initial regression failed
when the top dialog tried to take focus: the outgoing active route
wasn't the actual editor owner. The outlet now captures actual focus
ownership along with the target, releases that exact target only when
a focus-taking route commits, and can restore it when the neutral route
returns. The native regression sends actual text before, during, and
after the focused dialog, with select-all-on-focus explicitly disabled
so the assertions isolate delivery rather than selection behavior.
The authored Flutter case adds no upstream registration credit.
