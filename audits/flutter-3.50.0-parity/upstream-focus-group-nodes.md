# Focus traversal group's retained node

Both exact Flutter 3.50.0 `widgets/focus_traversal_test.dart:3471,4147`
passed. The group's creation callback receives the actual retained
parent of its descendants, not a detached observer handle. The node is
not itself traversable or focusable. Changing the group's policy
preserves its focus node, does not call the creation hook again, and
does not interfere with the child's explicit focus requests.

An explicit focus parent moves the group and its descendants out of
their enclosing visual scope. The native fixture supplies a retained
outer scope as the root counterpart, since Runtime's implicit root
does not expose a public FocusScopeNode. The child is still absent
from the enclosing scope's descendants and actually focuses under
the explicit parent.

A plain FocusScope inherits the containing group policy; only a
deliberate Incular scope policy overrides it. The adjacent ordered
and lexical navigation tests caught this while introducing the group
node, and passed after the fix. Thirty-five focused tests passed:
group, permissions, ordering, scope, keyboard, and ordinary traversal.
