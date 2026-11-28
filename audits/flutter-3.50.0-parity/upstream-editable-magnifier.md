# EditableText selection magnifier

## Default and scaled handle drag

Pinned `widgets/editable_text_test.dart:17058,17088` both passed.
The native default editor does not build a magnifier, and configured
editors can use a neutral overlay builder without Material or Cupertino.

The mounted native comparison uses the pinned FlutterTest font, right
alignment, five text lines and a retained 0.5 transform. It taps the
editor and drags a real touch selection handle. The custom lens stays
on its line after the first 3.5 px, moves exactly 7 px after the second,
stays there midway back, returns to the first line, and disappears on
release. Pointer cancellation and focus loss also dismiss it.

The builder receives transformed caret, line and field bounds. Handles
and magnifier share one retained overlay when possible, so a builder can
choose whether handles are painted before or after the backdrop lens.
Enabling, disabling, and re-enabling the builder does not remount a
field with configured handles. The three native magnifier tests and
19 handle regression tests passed. No design-system styling or physical
touch-device screenshot is claimed.
