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

Requests between unrelated scopes and manual scopes without a mounted
host are not reparented by this API.

## Focus when parked keyed elements are removed or moved

Pinned `widgets/focus_scope_test.dart:668,743` passed. When an active
branch is removed, the surviving scope and its explicitly autofocused
child receive focus, whether or not the removed scope itself is keyed.
The leaf is keyed in both cases. Incular used to erase the active focus
path while the leaf was merely parked between build and layout. It now
waits for definitive removal, then restores the surviving branch from
scope history.

The authored Flutter and native control move the same keyed raw
editor into a layout builder. Both preserve identity, primary focus,
selection (1–3), and the existing composing range (1–3) through the
move. The two direct removal cases, 41 adjacent scope checks, the editor
control, and four adjacent editing checks passed. No raster or platform
IME integration is inferred from the controller-state assertions.
