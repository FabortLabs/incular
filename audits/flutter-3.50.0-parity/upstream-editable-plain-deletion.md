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

An additional pinned registration ran across five platform variants
and confirms that forward Delete removes exactly one fully selected
family emoji. The paired authored probe
`editable_partial_grapheme_deletion_test.dart` uses base letters and
combining marks to put the selection endpoints inside graphemes
without splitting UTF-16 surrogate pairs. Both directions of selection,
as well as Backspace and Delete from inside a grapheme, now remove
every touched grapheme without leaving an orphaned combining mark.
Obscured fields still delete by code point. These authored probes do
not receive credit for the upstream cases that use an interior
UTF-16 surrogate offset.

## Word-modified Backspace and Delete

Ten pinned registrations ran across all 54 platform expansions.
Mounted native editors use Alt on the Apple policy and Control on
the common policy. Both directions delete the same middle word
without removing unrelated text; the end-of-document Backspace
also includes terminal punctuation. At a whole-family emoji
boundary the command removes only the neighboring cluster.
Read-only editors and commands with nothing to delete preserve
both their caret and its affinity. The interior-surrogate cases
remain unpaired.

## Line-modified Backspace and Delete

Fourteen pinned cases ran across all 84 platform expansions. The
native test uses the pinned FlutterTest face at 10 px and the
original 201 px viewport with zero cursor width, so exactly 20
characters fit on each soft-wrapped line. At offset 40, upstream
Backspace removes the preceding visual line and downstream Delete
removes the following one; the opposite pair are no-ops and preserve
affinity. The hard-break checks confirm that the newline is not
deleted with the rest of its line. The document edges, readonly
selection and whole emoji boundaries also match.
