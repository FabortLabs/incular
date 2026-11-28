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

## Handle-free presentation and updated builder

Pinned `widgets/text_selection_test.dart:1740` and
`widgets/editable_text_test.dart:16005` passed. A configured native
lens opens in a mounted 200 × 50 editor with no handle overlay;
its builder receives the gesture (10, 10) and the editor's actual
bounds. Opening it does not request focus. When a retained rebuild
changes the builder's output key, the overlay replaces that child
without remounting the editor, and explicit hide removes it.

A native control also verifies that iOS and Android long presses
open the lens at the 500 ms threshold, update it at the moved
pointer, and dismiss it on release. The Linux policy does not
open a touch long-press lens. All five magnifier and 19 selection
handle tests passed. The long-press control adds no registration row.

## Magnifier diagnostics and geometry limits

Pinned `widgets/magnifier_test.dart:302,313` passed. The four loop
expansions cover upper-left, unchanged, right, and bottom-right
placement; the native test preserves the rectangle dimensions and
asserts all four exact positions. The empty geometry display string
matches Flutter's position, line, caret, and field order and number
format. The six focused native magnifier tests passed.

## Magnifier controller lifecycle and ordering

Pinned `widgets/magnifier_test.dart:123,151,205,240` all passed.
A mounted controller without an animation clears its overlay entry on
hide. With a real animation controller, the lens reports shown while
entering or completed, but not while reversing or dismissed. Its
mounted child stays available during reverse and is removed after
completion. The native clock advances past the terminal tick, as
Flutter's pump-and-settle does.

The controller can mount a lens below a preexisting higher overlay
child without remounting that child. The native paint stream verifies
that the lens's backdrop capture precedes the higher child's paint.
Replacing a showing lens never starts a reverse animation, and a
canceled old exit cannot remove the new lens. All four focused
mounted native tests passed.

## Translated focal-point pixels

The pinned authored Flutter probe renders an 8 × 8 scene into an image
and checks four exact RGBA samples before and after inserting a
translated lens. At (3, 3) and (3, 4) the lens changes the blue
background to the black focal region; the outside samples do not
change. The Incular wgpu test uses the production device-space
backdrop matrix and the embedded linear resample shader. Actual
offscreen readback through surfaceless GL verifies the same two focal
pixels. That test isolates the GPU resample pass rather than the complete
native view. The original magnifier golden (:45), including its shadow,
opacity, and full native clip composition, remains open.
