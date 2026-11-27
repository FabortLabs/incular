# Selection bounds and directional drag offsets

Both pinned rendering originals pair. The rectangle classifier
checks all eight outside regions and the center. Drag offsets inside
the rectangle stay unchanged; outside points snap to the top
leading or bottom trailing corner in either writing direction.

A separate exact-edge probe covers the subtle difference between
classification and drag adjustment. Flutter excludes right and
bottom edges from the inside area. At the top-right corner the
classifier returns next but the drag still snaps to the first
corner; at the middle of the bottom edge the classifier returns
previous but the drag snaps to the last corner. The implementation
does not change Incular's general-purpose hit-test rectangles.
This slice checks the rectangle policy independently of mounted
selection gestures.

## Captured mouse drags beyond the hit box

Four authored pinned Flutter cases and their mounted native controls
exercise a wrapped label and two separately laid-out labels, forward
and backward. The single-label fixture can leave the first visual
row horizontally and still select through the last row. In the split
fixture, entering the gap stops at the edge of the label approached
from; continuing beyond the far label selects both. Content joins
without inventing a newline between separate labels.

Incular previously stopped extending when the pointer no longer hit
a label. The captured drag now stays in its original selection region,
walks that region's retained selectable order outside direct hits, and
snaps in each label's own transformed coordinates before asking the
text layout for a caret. Ordinary taps and keyboard shortcuts keep
their existing point policy. The 18 selected native runtime controls
and three geometry tests passed. This does not claim complete parity
for inline-fragment traversal or touch long-press dragging, and adds
no upstream registration credit.

A fifth authored case rotates a multiline label by 0.6 radians.
Both engines map the same local start and outside offset into the
window and select through the final row. The native test confirms
the outside point has no ordinary selectable hit, so the captured
drag, not hit testing the label again, delivers the edge.

Two more pinned authored cases exercise a Hebrew paragraph. Dragging
off the first row after starting at its visually right-hand edge
extends to the logical end; the reverse drag from the visually
left-hand edge of the last row extends to the logical beginning.
Both native directions select the original UTF-8 text with its line
break and match the reference. The corresponding unit geometry also
checks all eight outside regions in RTL.

## Touch long-press drag

Two pinned authored touch cases start a long press in the first or
last word, verify that whole word is initially selected, then drag
horizontally outside the paragraph without lifting the finger.
Flutter extends through the opposite logical end in both directions
and preserves the selection on release. Incular formerly kept
only the initial word. Its long-press path now shares the captured
region's rectangle routing; an outside point uses the snapped edge
instead of recomputing a word at that boundary. All 25 selected
native controls passed. This does not compare toolbar artwork or
the behavior of individual inline fragments.

## Listener offsets across labels and backward ranges

The exact pinned `widgets/selectable_region_test.dart:6060` passed.
A mounted mouse drag starts at 4, moves within the first label to
7 and 10, then into the second and third labels at flattened offsets
22 and 36. Incular previously counted a phantom character per label.
The backward drag preserves 30→0, rather than sorting the reported
endpoints. Clicks collapse at 3 and at 0. Ctrl+C still copies the
normalized text in document order with no inserted separator; the
test checks it through the runtime clipboard. This original uses only
ASCII. UTF-16 listener offsets on non-ASCII text are a separate check.

All five static drag and three keyboard shortcut tests passed. Six of
seven adjacent area-lifetime cases passed; the shared-affinity case
also fails unchanged master with this runner's font metrics, before the
new range conversion can affect it. That fixture is not counted here.

## Non-ASCII listener offsets

The pinned authored `selection_listener_utf16_test.dart` and native
mounted mouse fixture select across `é😀x` and `Zπ`. Flutter reports
1→5, whereas Incular previously reported the UTF-8 byte offsets 2→8.
The public range now uses UTF-16 code units after a single pass over
the preceding labels; internal text boundaries remain UTF-8. No
characters are added between labels, and Ctrl+C copies the unchanged
`😀xZ`. This authored probe adds no upstream registration credit.

The previously failing affinity fixture now locates the two distinct
visual stops of shared byte 3 from the retained shaping layout instead
of a fixed font's pixel coordinates. Dragging between the stops in
either direction still produces no content and places the collapsed
handle on the corresponding side. All 16 focused drag, keyboard
shortcut and selection lifetime tests passed.

## Empty-region mouse cursor and final newline

Both exact pinned `widgets/selectable_region_test.dart` registrations
passed. At :6653 the same pressed mouse visits the outside, empty
bordered region, and inner mouse annotation, activating grab,
grab, forbidden, and grab in order. This compares the runtime's
active cursor per device, not the OS cursor image. At :6728 a
top-left to bottom-right drag over a paragraph at line height
1.4 keeps the final newline, selected range 0→12 and the complete
plain text. Both native fixtures and all ten adjacent mouse cursor
session tests passed.
