# Runtime primary focus notification and node replacement

Pinned Flutter `widgets/focus_manager_test.dart:2265` observes primary
focus while an unkeyed two-child column changes to contain only the second
child. Flutter clears the removed node, does not focus the surviving
sibling, and notifies exactly once. The original passes on the pinned SDK.
The Rust runtime test uses the same unkeyed column, text and RTL fixture.
It reproduced a real drift: Incular reused the first retained element
and silently granted focus to the node now on that element.

The runtime now remembers the external node that owned primary focus.
Replacing or removing that node no longer grants focus merely because
the element ID survives. An explicit pending request remains independent
of the old ownership. The new window-wide observer runs after widget
focus and text input state update. The second focused test matches the
listener counts and deferred, last-request-wins behavior of original
:2045, but not the external nested FocusScopeNode attachment API used
to set it up, so that registration remains partial.

## Nested observer settling

Pinned `:2045` now has the full nested fixture: an external mounted
scope, a parent focus node, and a second focus node below that parent.
The runtime observer sees one notification for programmatic autofocus,
one for each settled explicit change, and one when unfocus parks
primary focus on the scope. It sees no intermediate requests.

Pinned `:1906` passed with a top scope and two child scopes. The
native test checks each callback and sampled focus state at five
boundaries: initial focus, child-to-scope unfocus, scope-to-child
focus, moving between the sibling scopes, and a pair of requests
whose final state is unchanged. A shared ancestor is not notified
just because its active branch changes. All 29 selected native
observer, scope and key-routing checks passed.
