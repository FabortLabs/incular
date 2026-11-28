# Mounted editor handwriting geometry

## Focus and connection lifetime

The pinned iOS variant of `widgets/editable_text_scribble_test.dart:34`
passed on the Linux Flutter runner. The corresponding native test first
failed because an attached editor sent no character bounds. With the fix,
its actual mocked Flutter host receives five local 14 × 14 glyph boxes
for `Text1` at x = 0, 14, 28, 42, 56. Each carries its UTF-16 start
and text direction. Settled frames and blur send no rectangles. The same
bounds are sent on refocus with a fresh input client.

The opt-in host reads retained layout, not the accessibility string or a
measurement of the unlaid-out controller value. Stable layout and paint
geometry reuse the computed bounds; the individual host connection owns
wire deduplication. The focused Scribble, composing geometry and full
Flutter input channel targets passed (20 native tests). This comparison
does not exercise a physical Apple Pencil, native UIKit delivery,
Scribble focus discovery, handwriting placeholders, or the user-driven
scroll gesture lifecycle.
