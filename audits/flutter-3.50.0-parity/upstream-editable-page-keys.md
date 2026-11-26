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

## Single-line scrolling

The right-aligned :9411 case now also pairs on iOS and macOS.
Both Page Up and Page Down go to the maximum position, Home returns
to the start, and all three leave the collapsed selection alone.
A narrower native control exercised a real overflow instead of
only the full-width case. The old Page Up offset stopped three
pixels short because it ignored the default two-pixel caret and
one-pixel gap.

For fitting right-aligned text, the pinned authored probe also
verified the four-pixel caret ended at x=79 in an 80 px viewport.
Incular previously painted that caret at x=80 through x=84.
Single-line LTR layout now reserves the same caret margin used by
clipping and scrolling; the retained RTL path stays separate.

Both Flutter variants, two authored Flutter probes, and all 38
focused native checks passed.
