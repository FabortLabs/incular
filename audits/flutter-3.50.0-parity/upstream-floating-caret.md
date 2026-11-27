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
