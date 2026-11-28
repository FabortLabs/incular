# Focused editor style updates

All four pinned `widgets/editable_text_styles_test.dart` cases passed.
A mounted Incular editor now publishes the resolved style to the
attached Flutter input control after layout. A change in bold text,
letter spacing, word spacing, or line-height override emits one actual
`TextInput.setStyle` call without replacing the editor, its focus, or
its client. The original line heights are 20 and 40 for a 20px font;
the font size remains in authored logical units.

A different control receives its first snapshot even if identical.
Repeated snapshots on one control and updates to a stale or missing
attachment do not dispatch. Adjacent native channel and binding
controls passed. The optional protocol host calls the publication
method after layout; ordinary native IME drivers are unchanged.

## Mounted text and strut accessibility overrides

Pinned `widgets/editable_text_test.dart:17710,17733` both passed.
The native tests set text after mounting and inspect the retained
render editor rather than an isolated style object. Ambient bold text
overrides authored weight 400 to 700. Line-height, letter-spacing, and
word-spacing overrides are all exactly 2.0.

Before the fix, the effective glyph style respected the line-height
override, but an authored 0.9 strut remained 0.9. After lowering, that
same strut now also has height 2.0, preserving its other properties.
All eight focused native editor style tests passed.
