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

The pinned widget affinity and ending-with-selection registrations
also ran. Their exact multi-line pixel coordinates, secondary
background caret and the Flutter force-press selection notification
are not fully paired.

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
caret, not the remaining exact wrapped-line pixel or selection-cause checks,
and adds no upstream count.
