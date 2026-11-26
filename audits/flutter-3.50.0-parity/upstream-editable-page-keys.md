# Capped multiline editing and Apple page keys

Pinned `widgets/editable_text_test.dart:9328` tests the same
four-line, right-aligned editor on iOS and macOS, with two visible
lines. Page Up stays at the top, Page Down advances by 80% of the
viewport, another Page Down clamps at the end, and two Page Ups
return to the top. Scrolling does not move the selection.

Before the native fix, the two-line cap truncated the paragraph
itself. Page Down had no content left to reveal and stayed at 0 px
in a 40 px viewport, instead of moving 32 px. The editor now lays
out all the multiline text and caps its natural viewport height,
honoring padding, explicit sizes, and parent constraints separately.
This keeps the last line reachable and lets shrinking text clamp
the previously scrolled viewport.

Both pinned Flutter platform variants and all 36 focused native
checks passed. The single-line registration at :9411 was rerun in
Flutter as a control; it is not yet paired with this fixture.
