# EditableText toolbar during internal touch scrolling

## Short touch scroll with semantics off

Both pinned mobile variants of `widgets/editable_text_test.dart:19006`
passed. The mounted native comparison starts with a word selection and
a built toolbar, disables platform semantics, then moves a finger 20 px to win
the drag and a further 10 px to scroll. Before the fix the native editor did not scroll. A
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
touch drag, and replace the editing value before the release's
scheduled frame. The old toolbar does not reappear, including after a
second touch sequence immediately following the replacement. Both
cases keep the editor mounted and focused.

A native control keeps the original value instead: the toolbar stays
hidden when the selection is out of view, then reappears when the
next scroll brings the same selection back. All four focused native
scroll tests passed; the extra control adds no registration.

## Live drag origin

Pinned `widgets/editable_text_test.dart:15932` passed. The mounted
native editor updates `DragStartBehavior` without losing its element,
focus, or handles. For the first accepted handle move, Down includes
travel from the press and reaches a later line; Start uses the
winning position as the vertical origin and stays on the first line.
The same property controls the editor's direct viewport drag. In the
default Start mode the first 20 px hides the toolbar without moving
content; the next 10 px actually scrolls. All 34 focused native
scroll, handwriting, magnifier, and handle tests passed.

## Device geometry gate

An authored pinned Android probe uses a real multiline editor, live
touch scroll, and 120 px of bottom device padding. The selection stays
inside the editor but outside the drawable view. Flutter keeps the
toolbar closed after the scroll, then restores it when padding is
removed and the next scroll brings the selection into view.

The same native scenario initially failed because it only checked the
editor's own bounds. Its restore decision now also checks the visible
device rectangle, selection endpoints, and ancestor viewport geometry.
The older out-of-view and edited-value controls still pass. All six
native scroll tests passed; this authored comparison adds no upstream
registration and does not claim nested gesture arbitration.

## Live selection-handle tap callback

Pinned `widgets/editable_text_test.dart:15866` passed. A real touch on a
mounted collapsed knob initially invokes no application callback. A
retained rebuild supplies one; the next tap invokes it once without
recreating the editor or refocusing it. A drag does not invoke the tap
callback. Removing the callback stops notifications without replacing
the handle. All seven focused editor tests and 19 handle tests passed.
