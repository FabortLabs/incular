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

## Detaching an entered target while dragging

Pinned :2173 and :2789 passed. The mounted native comparison first lets
a control tap finish one drag. It then moves a second drag onto an
accepting target and removes the target while the pointer is held.
Removal does not end the source or deliver a drop. The release ends
the still-active source but does not call the detached target.

Previously the removed target remained in the active drag and failed
the retained-tree invariant with a dead element ID. Unmount now
silently drops both accepted and rejected target records for that
element. The native fixture also verifies that no stale leave is
delivered and checks tree invariants between removal and release.
All 27 focused native tests passed.

Separate pinned registrations :2849 and :2514 confirm that Flutter
keeps an established avatar alive when only its source disappears and
can present two independent avatars from one source. Incular's
separate mouse-source removal check currently cancels its local
avatar, and the typed context still exposes a single feedback slot.
Those are open lifecycle differences, not additional paired cases.

## Terminal pointer samples and diagonal locked-axis updates

The authored pinned cases send a horizontal-axis drag from (20,60)
through (145,75), then release or cancel at (260,80) over a different
target. Flutter reports one original pointer delta, (125,15), while
moving feedback only horizontally. It offers only the first target
and finishes from feedback position (135,50). Release drops there;
cancel does not drop. A terminal sample is not a movement update.

The native baseline instead reported (125,0) and a spurious (115,0)
on both terminal events. Incular now filters displacement but keeps
the original delta for callbacks that do fire. Release or cancel
uses the last real drag placement without a new offer or move.
Both pinned authored probes and all 40 focused native tests passed.
This authored comparison does not change the upstream count.
