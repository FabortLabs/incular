# Keyboard floating caret

The pinned services coordinates regression :295 pairs. An active
text-input client accepts integer and decimal coordinates in the
same wire object; an update without a matching start is harmless.
Start and end normalize their offsets to zero as Flutter does.
Nonfinite/native-unrepresentable coordinates and stale attachments
do not move a different field.

The mounted editor uses the shaped caret's local geometry and current
controller identity. During the gesture it paints an inflated rounded
caret while leaving selection unchanged. Dragging beyond the edge and
reversing direction releases at the edge instead of requiring a long
drag back. On End the caret eases to the text position over 125 ms.
A collapsed selection moves only after settling; a selection already
extended by the IME is not overridden. The authored pinned mounted
sequence and native padding/timing/identity checks verify those cases.

The pinned ending-with-selection widget registration is paired
through the retained force-press, forward/reverse range and hard-break
geometry checks below. The wrapped-line and background-caret cases have
their own checks.

Four focused native floating-caret tests and 13 lifecycle regressions passed.

## Editor unmount

Pinned `widgets/editable_text_test.dart:13306`, “EditableText does not leak
animation controllers,” passes. The native regression mounts an autofocus
editor with animated caret opacity, verifies its blink deadline and visible
floating-caret reset, and unmounts it before the 125 ms reset completes. No
caret remains, no new frame or animation deadline is requested, and the old
controller's value is untouched even after the reset would have elapsed.
Incular checks actual frame demand rather than counting Flutter's internal
ticker objects. All five focused native floating-caret tests passed.

## Background caret during drag

When configured, Flutter paints the regular caret in
`backgroundCursorColor` at the text position closest to the floating caret.
`RenderEditable` suppresses it when the squared distance between their
centers is less than `15 × 15` logical pixels. It is not drawn during the
reset animation, and the ordinary caret color returns when the reset finishes.

Incular previously never drew that text anchor. The new
`EditableText::background_cursor_color` builder is optional, matching the
raw renderer's absence of a second caret when no color is supplied. It uses
the current shaped geometry and regular caret radius, preserves paint order,
and does not change text layout or the selection. The authored
`editor_floating_caret_background_test.dart` passed against the pinned SDK;
the two focused Rust targets passed eight tests. This covers the background
caret, not the separate selection-cause checks, and adds no upstream count.

## Live caret configuration

Pinned `rendering/editable_test.dart:261`, “Can change cursor color, radius,
visibility,” passes. The retained editor also starts hidden, then paints an
exact red `(40, 0, 1, 10)` rectangle with pinned Ahem. The mounted blue
rounded caret is `(40, 0, 4, 10)` with radius 3 and becomes
`(80, 0, 4, 20)` at 2× text scaling without growing in width or radius.
Hiding it again removes the paint command. Focus identity, selection, and
upstream affinity are unchanged through all updates. Five native tests passed.

## Soft-wrap affinity and floating caret prototype

Pinned `widgets/editable_text_test.dart:13347`, “Floating cursor affinity,”
passes. The initially unpaired Incular renderer put both affinities on the
first line, clamped the trailing-space edge too soon, and painted a caret
four pixels too tall on Linux. With the pinned Ahem font, Downstream now
paints `(0.5, 15, 3, 12)` at the start of the next line, while Upstream
paints `(378.5, 1, 3, 12)` at the trailing edge of the previous line.
The selection stays unchanged. The normal caret code switches lines only for
a shared soft-wrap edge, preserving the hard-break selection reveal behavior.

The raw floating prototype follows Flutter's platform grouping: top 2 and
height minus 4 on Android/Fuchsia/Linux/Windows; top 0 and height plus 2 on
iOS/macOS. Only then are the half-pixel horizontal and one-pixel vertical
margins applied. The force-line viewport boundary is independent of shorter
glyph runs. The pinned authored overshoot probe measured x coordinates
180.5, 180.5, 155.5 and 55.5 through reversal into a 180-pixel viewport.
The four focused Rust targets passed 17 tests; the native Apple path was
not run in this Linux environment.

## Fixed viewport offset

Pinned `rendering/editable_test.dart:2035`, “Floating cursor position is
independent of viewport offset,” passes. The retained regression starts
with the cursor hidden and then uses the same floating prototype as the
native input route. At a fixed drag point of `(50, 50)`, the Linux display
list contains the exact rounded rectangle `(49.5, 51, 2, 8)` at radius 1
before and after the text viewport scrolls by 200 pixels. The ordinary
text caret moves by exactly 200 pixels, so the scroll assertion cannot
pass vacuously. Both focused Rust targets passed 13 tests.

## Ideographic caret position and scale

Pinned `rendering/editable_test.dart:385`, “Cursor with ideographic script,”
passes. The native test reads the bounded `FlutterTest` font array directly
from the pinned engine source; it does not assume a system CJK fallback
has the same geometry. The same Chinese text and logical selection (UTF-16
offset 4, native UTF-8 byte 12) paint the exact 40-pixel horizontal position
and 10-pixel height. At 2× scale those are 80 and 20, while the requested
four-pixel caret width and radius three stay fixed. The same retained field
has no caret before showing or after hiding it again. Nine native tests passed.

## IME selection notification

Pinned `widgets/editable_text_test.dart:13422`, “Floating cursor ending with
selection,” passed. Incular now reports a `ForcePress` selection callback
when a dragged caret settles, including a gesture that ends at the same
logical offset. While the gesture is active, a forwarded platform range
change also reports `ForcePress`. Ending that gesture does not erase the
extended range or send a second notification. An ordinary platform
selection change reports `Keyboard`; direct controller changes remain
silent. Callback panics go through the same typed editor error reporter as
the other editor callbacks.

This implements the platform and floating routes, not all host key,
gesture or accessibility causes. Three focused Rust targets passed 25
tests. The full pinned rendering and range sequence was subsequently
checked below.

## Ending with selection at a hard break

The retained Ahem fixture now reproduces the original `:13422` sequence:
start at x=0.5, drag to x=50.5, and settle to offset 4 with a force-press
notification. A reverse drag starts at x=56.5 and crosses back to x=0.5;
the forward drag does the opposite. In either direction, a platform
range 0→4 remains intact after the animation and does not report a
duplicate callback. The hard-break case starts at `(0.5, 15, 3, 12)`
at byte 27, then moves to `(800.5, 1, 3, 12)` on the first line.

The original checks the old offset immediately after End. The pinned
authored hard-break probe also advances through the 125 ms reset:
it selects the previous line's end, offset 26 with upstream affinity.
The native case verifies both points in time and the final force-press
notification. Nine focused native floating-caret tests passed.
