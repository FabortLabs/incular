# EditableText toolbar during internal touch scrolling

## Short touch scroll with semantics off

Both pinned mobile variants of `widgets/editable_text_test.dart:19006`
passed. The mounted native comparison starts with a word selection and
a built toolbar, disables platform semantics, then performs a 20 px
finger scroll. Before the fix the native editor did not scroll. A
vertical drag now scrolls its own multiline viewport without moving
the text selection. The toolbar is hidden while scrolling and, with
the same editing value and the selection still in view, a separate
paint frame mounts it again after release.

The native assertion checks the actual newly mounted toolbar and the
runtime's request for that frame. It does not get a frame indirectly
from semantics. One focused scroll test (both policies) and all 19
selection-handle tests passed. Fling behavior, nested viewport
arbitration, and other scroll cases remain separate.

## Editing-value invalidation and returning to view

Pinned Android :18773 and :18846 passed. The native tests select the
first word in the same 50,000-character document, drive a real 200 px
touch scroll, and replace the editing value before the release's
scheduled frame. The old toolbar does not reappear, including after a
second touch sequence immediately following the replacement. Both
cases keep the editor mounted and focused.

A native control keeps the original value instead: the toolbar stays
hidden when the selection is out of view, then reappears when the
next scroll brings the same selection back. All four focused native
scroll tests passed; the extra control adds no registration.
