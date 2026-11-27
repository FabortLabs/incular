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

The pinned ending-with-selection widget registration remains partial:
its exact sequence and the Flutter force-press selection notification
are not fully paired. The separate wrapped-line affinity registration
and the background caret have their own checks below.

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
