# EditableText can opt into directional focus through a shortcut

Flutter 3.50.0, `ab2912035e160d9cad6306df307576da6128ebdb`:
`packages/flutter/test/widgets/focus_traversal_test.dart:3112`.
Both original transit variants passed: RawKeyEvent and ui.KeyData followed
by RawKeyEvent.

The native comparison mounts four 100px editors in the same grid, starts
with the upper-left editor focused, and applies Right, Down, Left and Up
using both native message modes. The explicitly configured typed intent
with `ignore_text_fields(false)` traverses the four editors. Rebuilding
with `true` retains the mounted focus and does not traverse. This matches
the original focus behavior. The binding is installed through Incular's
typed shortcut map on `FocusableActionDetector`.

The runtime uses the same retained directional history and reveal behavior
for this intent as its default navigation. A locally handled application
action still takes precedence. An arrow without a custom binding continues
to move the caret in an editor rather than traversing away. Those two extra
native regressions passed in addition to the two transit fixtures.

All 17 native cases passed across editable directional shortcuts, focus
key routing, and shortcut registrar tests. The native fixture does not
try to reproduce Flutter's service channel or expose the deprecated Dart
transit mode API.
