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

## Custom builders and retained menu content

Pinned `widgets/editable_text_test.dart:15633,15679,15746` all passed.
The native editor initially rejected an empty default action list even
though its custom builder supplied its own toolbar. It now opens this
builder on a real long press or a host show request, and an updated
builder can replace the menu's keyed child without refocusing the field.
An inline builder refresh keeps the same stateful menu and original live
state setter while replacing its current child.

A separate pinned authored probe confirms the bare custom builder
receives full overlay constraints, even when it returns a smaller
SizedBox directly. The native case matches this default; a builder can
position its own content inside that full viewport. Nine focused editor
tests and 14 existing context menu tests passed.

## Cut and paste may remove their editor

Pinned `widgets/editable_text_test.dart:15069,15120` both passed.
The native tests exercise the real Cut and Paste toolbar callbacks
with an in-memory clipboard. The edit fires the editor's change
listener, whose retained state update removes the editor. Both
operations complete, close the text client, and leave the replacement
mounted with no stale focus. Reusing the old toolbar callback does
nothing. Both native tests passed.

## Private scroll axis on mobile

Pinned `editable_text_test.dart:9693,9767` passed. Native carries the same
100 px-width fixtures and real finger drags. The overflowing single-line
iOS editor stays at its original offset, while the two-line editor
scrolls to the last line and stops at its maximum. Its field can scroll
without the drag being treated as a tap.

An authored Android Flutter probe supplies the same overflowing single-line
text and a 100 px horizontal drag. Its private viewport scrolls without
focusing the editor or setting a selection. Native formerly discarded the
drag because only multiline fields had a recognizer. The recognizer now
uses the editor's scroll axis and retains iOS's horizontal exclusion.
This is a touch/viewport comparison, not an equivalence claim for
post-release velocity, scroll physics overrides, or externally attached
editable scroll controllers. All 20 focused native editor tests passed.

## Upward reverse selections track the actively dragged end

Pinned **4226** (Android) and **4275** passed. In the 100-line native
fixture, a real leading-handle drag changes the range from `500→600` to
`0→600`. Before the correction the range changed but the viewport stayed
814 px down, at the stationary extent. It now reveals the moved base and
stays at offset zero after the handle is released. The non-Apple rule gives
a moved base priority; the usual extent-based reveal does not undo it.
Apple's extent behavior remains unchanged.

The second fixture starts at the bottom and drags a real mouse from
10 px above the bottom to 10 px below the top. The reverse selection
retains its anchor and the viewport no longer sits at its maximum. All
35 focused native scrollbar/selection/handle cases passed. The fixtures
do not require or claim parity for an externally attached editor
ScrollController.
