# Focus switches, removal, and scope permissions

Three exact `widgets/focus_scope_test.dart` registrations passed on the
pinned Flutter checkout and were backed by live Incular tests:

- **85:** autofocus sets the first visible label, tapping it again is
  idempotent, and taps on the other and back again update both labels
  without leaving both nodes focused.
- **360:** although its name says "moves focus to next widget", the
  original asserts the remaining sibling is **not** focused. Removing
  the primary child preserves the sibling's identity and unfocused label.
- **1399:** toggling permissions on a leaf blocks its own requests,
  toggling permission on an ordinary Focus parent does not block its
  child, and disabling either of two nested FocusScopes blocks every
  descendant until the scope is re-enabled.

Native: `flutter350_focus_scope_labels` and
`flutter_focus_policy_upstream_parity` in `incular-runtime` (14 tests
total). No production changes were needed.

## Preferred mounted focus scope

Exact `widgets/focus_scope_test.dart:265` passed. Selecting a descendant
of an active scope restores its remembered leaf, or gives primary focus
to that scope itself when it has no focused-child history. Later
requests restore the last selected leaf, not its original autofocus
target. The native fixture checks the first-scope flags after each
switch. A pinned authored and native control changes an inactive scope's
preferred branch without interrupting focus elsewhere.

The keyed and unkeyed removal cases are separately tracked; their
surviving child currently loses focus in the native fixture. Requests
between unrelated scopes and manual scopes without a mounted host are
not reparented by this API.
