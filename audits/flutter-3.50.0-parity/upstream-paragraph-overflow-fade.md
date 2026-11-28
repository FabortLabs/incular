# Retained paragraph overflow and fade

The exact pinned `rendering/paragraph_test.dart:269` registration
passed. Incular applies every change to the same retained text,
at the 50 px limit in the original and with the pinned FlutterTest
font. The uncapped no-wrap Clip mode keeps both hard lines even
when the second line is wider than the box. A bounded Ellipsis
with maxLines still wraps before truncating, regardless of softWrap.
Uncapped Ellipsis instead stops at the first overwide hard line.
The resulting heights match the full clip and ellipsis matrix.

Fade now uses a real retained shader mask, rather than quietly
using clipping alone. The mask exists only when there is visual
overflow, and disappears when switching to Ellipsis or when the
complete paragraph fits. Native controls inspect the flattened
display list: a 10 px horizontal fade ends on the physical
trailing edge for LTR and RTL; a 30 px tall, three-line paragraph
fades over its last 5 px. The layer encloses actual glyph runs,
shadows when relevant, and retained inline widget draws.

The exact :1106 selection test also passed in Flutter. Native
records the selected range below and outside the fade mask,
even when inline content is inside it. The Flutter original
also inspects two handle-leader layers. Their placement is not
covered here, so :1106 remains partial. These are display-list
assertions, not an Incular window screenshot or pixel golden.

All 42 focused native paragraph, inline span, shader, and text
shadow tests passed with the pinned font fixture.
