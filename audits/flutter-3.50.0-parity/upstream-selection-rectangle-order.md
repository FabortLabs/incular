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
