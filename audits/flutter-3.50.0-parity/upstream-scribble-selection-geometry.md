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

## Visibility, platform gates, and geometry changes

Pinned iOS cases :496, :654, and :700 all passed. Of these, two are
paired: disabling handwriting sends no rectangle on show or a later
frame, and a 13 px viewport with 0.5 px scroll still sends all five
14 px boxes crossing both vertical edges. The mounted host also stays
silent for its browser protocol and non-iOS platforms while ordinary
keyboard show still succeeds.

The native :496 comparison retains the same client and tests exact
bounds after wrapping to 20 px, clipping to 45 px, aligning to x = 5,
and changing an idle scroll offset by 14 px. The rectangle ending exactly
at the top edge is included. An unchanged rebuild emits nothing. This
case is partial: the family emoji yields one grapheme at the same
UTF-16 index, x/y, and height, but the native fallback lays it out as
one joined glyph 17.43164 px wide on this test host, while Flutter's
test-font environment produces a 42 px box. The host sends its own
laid-out bounds rather than imposing Flutter's test-font width.
All four focused native tests passed.

## Selection-change cause during a handwriting interaction

Pinned iOS case :134 passed. The installed native text-input binding
now reports `StylusHandwriting` for a selection-only update between the
host's interaction-began and interaction-finished messages. The mounted
callback reports ordinary keyboard selection before and after that
interval. An identical packet emits no selection callback, and updating
the actual text during the interaction still reports a keyboard edit.
The four binding and 16 whole-editing-state tests passed.

## Toolbar stays mounted across duplicate show requests

Pinned iOS case :795 passed. The native test starts handwriting
and changes the selection through the installed host channel.
No toolbar appears until requested. Its first show succeeds; a
duplicate returns false, leaving the same keyed overlay mounted
across two further frames. All five focused binding tests passed.

## Discovering and focusing mounted editors

Pinned iOS :186 and :225 passed. The native binary messenger now
discovers retained editors without requiring an embedder to register
each one manually. The query returns the active view's real transformed
bounds. It rejects an edge-only intersection, a covered field, a
read-only field, and a field with handwriting disabled. It also rejects
an identifier retired by unmounting even after a replacement mounts.

The host focuses an unfocused editor by its returned identifier and
global point. Selection is collapsed at that position, its callback
reports `StylusHandwriting`, and the focused editor has a text input
connection. The initial character bounds are sent after editor
geometry and before keyboard show, consistent with Flutter's first
geometry pass. Explicit registry clients remain supported. Eleven
focused native tests passed across binding, geometry, and keyboard
ordering. Handwriting text placeholders remain open (:327 and :410).
