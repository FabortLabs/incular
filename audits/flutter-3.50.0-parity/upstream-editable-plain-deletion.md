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

Five forward Delete registrations ran through all 26 platform
expansions. They cover deleting at the start of a line or document,
preserving selection in a read-only editor, keeping upstream affinity
when Delete has nothing to remove at the end of the document, and
removing the entire following family emoji at a cluster boundary.

The document-end regression reproduced on Android: a zero-length
deletion reset upstream affinity even though the text had not changed.
The runtime now deletes the range in one controller transaction,
without first changing the selection, so an empty range is a no-op.
