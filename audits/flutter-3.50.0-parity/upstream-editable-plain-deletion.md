# Focused editable character deletion shortcuts

Five pinned Backspace registrations ran through all 27 of their
platform expansions. Mounted native controls fire physical key down
and up against a focused multiline editor with the same text and
caret. They check that Backspace joins lines, does nothing at the
start or in a read-only editor, deletes a final character at the end,
and removes a whole family emoji at the preceding cluster boundary.
The read-only test also preserves upstream affinity; a successful
edit resets it to downstream. Focus stays attached to the same editor.

The native Fuchsia shortcut policy reuses Android's tested map. The
cluster boundary uses Flutter's equivalent UTF-16 offset and
Incular's UTF-8 byte offset. The separate upstream case placing a
selection inside a surrogate pair is not credited.
