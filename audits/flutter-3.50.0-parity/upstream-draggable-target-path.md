# Draggable target hit paths

Both pinned `widgets/draggable_test.dart:92,173` originals passed.
Their 300-wide upper target overlaps the left half of a separate
200-wide lower target. The pointer begins on the upper-only portion
and then enters the overlap. If the upper target rejected the payload,
entering the overlap now offers the lower target a chance to accept.
If the upper had accepted, it remains the only target to receive
the drop and the lower is never offered the payload.

Before the fix Incular searched only ancestors of the ordinary hit
target, so a translucent DragTarget with an empty child did not
receive an offer. The drag path now reads retained hit annotations
frontmost first, and a matching prefix can skip re-entry only after
acceptance or when the number of candidates did not grow. The mounted
native fixture compares actual offers and final accepted payloads.
Native controls also verify that a defer-to-child empty upper target
does not participate and an opaque upper target blocks a lower one.
All 35 focused mouse, target path and drag option tests passed.

## Feedback positions and update callbacks by axis

All eight pinned `widgets/draggable_test.dart` registrations at
:1108, :1125, :1142, :1162, :1179, :1248, :1285, and :1322 passed.
The native widget fixture tracks the actual feedback element and
its drag position after the original is replaced. Unrestricted
moves of ±(300,300) retain both components; locking each axis
retains only its component on both straight and diagonal moves.
Release removes feedback and restores the source.

The three update traces also compare real callback deltas. Before
movement the down-created feedback sends no update. The free drag
reports (10,10) once; vertical and horizontal locks report only
(0,10) or (10,0). Zero and suppressed-axis movement and release
do not add callbacks. Both focused native matrix tests passed.
