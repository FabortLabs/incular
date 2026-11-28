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
