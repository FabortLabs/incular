# Navigation initial focus

## Push, pop, and nested traversal

The pinned `widgets/focus_traversal_test.dart` originals at lines 435, 559,
and 1534 passed. The native widget-order and ordered test cases first
reproduced the bug: the initial page focused its autofocus control,
but after an actual pointer tap pushed a new page, the new autofocus control
did not receive focus. The outgoing page's staged autofocus was the first
global candidate, and rejecting it after lookup discarded all candidates.
The outlet now selects within the newly active route before choosing a
candidate. Tests use real pointer input, show that the outgoing control
loses focus and the incoming control receives it, then pop by pointer and
check the previous control and its containing scope regain focus. The
nested navigator test also verifies each step forward and backward,
including wrap-around outside the navigator. All three focused native
cases passed after the fix.

## Editor nested inside a focus boundary

Pinned `widgets/routes_test.dart:517` passed. A real Incular EditableText
inside a Focus wrapper receives primary focus on the initial route and
on a pushed route. Typed input reaches only the currently focused editor,
and returning restores the original editor. The native test checks both
Flutter's Linux single-line select-all default and Android's retained
cursor behavior on return. Another test verifies that a page which sets
`request_focus(false)` cannot steal focus with its own autofocus control.
All five focused navigation tests passed.
