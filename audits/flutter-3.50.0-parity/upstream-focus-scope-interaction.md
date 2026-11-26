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
