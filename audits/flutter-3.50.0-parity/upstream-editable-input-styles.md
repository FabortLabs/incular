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
