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

## Apple document keys and ordered host selectors

Pinned `widgets/editable_text_test.dart:15172,15247,15316,15395,15494,15582`
passed across all 11 named expansions. The mounted native comparison
uses the four-line, right-aligned 400 px editor and the pinned
FlutterTest font. Actual Ctrl chords on iOS and macOS policies move
to offset 19 with upstream affinity, back to 0 downstream, one
character forward and back, and down to the next line at 20 and
back to 0.

Ctrl+T leaves an initial caret and a selected range alone, swaps
the same Latin characters and end-of-document punctuation, and
moves a family emoji as one grapheme without splitting it. The
UTF-16 caret positions match the pinned 8 and 10 positions. The
macOS host sends selector packets that move across the newline in
order, reaching 9, 5, and finally 0. All four focused native tests
passed, along with 19 adjacent editing shortcut checks.

## Character-movement shortcut and action overrides

Pinned **12307** and **13053** passed. A typed shortcut maps five real
Linux key sequences (Left and Control+X/C/V/A) to one forward character,
while the unbound Right arrow still uses the default. All six land at
offset one without changing document contents.

A separate mounted override handles the built-in Right arrow's
`ExtendSelectionByCharacterIntent` exactly once and leaves the caret
at offset zero. The kind of movement and whether it collapses the
selection are carried by the intent; ordinary uncustomized controls
keep the same platform editing logic. Twelve focused native tests
passed, including default navigation and logical RTL movement.
