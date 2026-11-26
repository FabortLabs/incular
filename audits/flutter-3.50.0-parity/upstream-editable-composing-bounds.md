# Editable composing-range bounds

Pinned `rendering/editable_test.dart:917,937` passed. A retained editor
now exposes the local bounding rectangle of the text in a composing
range. Invalid or collapsed ranges and a range selecting only the
left-to-right mark have no bounds. With the same 22 Ahem characters,
200-pixel wrapped editor and three runs (1, 10, and 1 pixel font sizes),
the native rectangle spans more than 190 pixels. The widget's spell
style supplies the two small runs without replacing its text or layout.

The authored pinned probe checks that a scroll of 20 translates the
first line and the line break to -20. The newline contributes a
zero-width box at x=30, not an extra painted pixel. It also checks
that selecting only part of a combined Inter glyph produces no box.
Native controls also exercise paragraph-local offsets, selected
preedit text that differs from the committed value, content padding,
and the UTF-16 width of a masked supplementary character.

The native measurement uses the retained text layout and active
scroll offset. It does not promise native candidate-window placement
or configure a platform input method.

All 27 focused native composing, intrinsic and styled-span cases passed.
