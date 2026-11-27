# Native key traversal and scroll reveal

Pinned Flutter: `3.50.0` (`ab2912035e160d9cad6306df307576da6128ebdb`).
The four original registrations in `packages/flutter/test/widgets/focus_traversal_test.dart`
at lines 2463, 2553, 2609 and 2733 ran at the pinned revision. Each passed in
both upstream simulator variants (RawKeyEvent and ui.KeyData then RawKeyEvent).
Incular's counterparts send normalized platform key down/up events through a
mounted runtime, not calls to traversal methods or replays of Flutter's raw
platform event translation.

- The four-cell scene checks both complete Tab cycles, including actual Shift
  press/release, and maps each arrow to the expected active control.
- A locally owned autofocus node with `skipTraversal` takes primary focus
  before input. Four Tabs then alternate between the other controls and
  never return to it. Incular previously searched a list of traversal stops
  for the initial autofocus target and could not elect this node.
- Vertical and horizontal lists use the same viewport, fixed cell dimensions,
  leading/trailing controls and eleven items. Every arrow checks both the
  expected primary focus and the exact scroll controller offset in both
  directions, including crossing the viewport boundaries.

Native target: `cargo test -p incular-runtime --test flutter350_focus_key_scroll`.

## Nested scrollables

The registrations at 2858 and 2985 passed in both original key simulator
variants. Mounted lazy lists match the 800×600 viewport, ten line groups,
100-pixel controls, and 32-pixel separators. In each orientation, moving
along the outer axis preserves the chosen cross-axis index for every
intervening group, scrolls the outer controller past the threshold, then
reverses through every group to the sticky starting control.

## Empty focus tree

Both duplicate-named arrow registrations (3253 and 3286) ran in both
variants; the distinct empty-group Tab registration (3272) also ran. The
Incular tests check normalized arrow down/up at the global hardware handler,
once for a container and once for a placeholder without focusable children,
and check that Tab preserves primary focus in a group containing only a label.
