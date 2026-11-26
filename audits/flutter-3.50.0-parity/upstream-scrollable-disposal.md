# Scrollable disposal and in-flight input

The pinned `widgets/scrollable_dispose_test.dart` :11 and :31 originals
passed (three executions: unvaried, iOS, and macOS). A mounted native list
moves on a ballistic frame, then is detached on the frame whose five-hour
advance would end that fling. The old controller has no clients or activity
after replacement. The second test uses the default iOS and macOS behaviors
over perpendicular nested scrollables. It verifies negative offset, springback
toward zero, suspension on the next finger down, detachment of both positions
while that finger remains down, and safe release after removal.

Both focused native runtime tests passed. The platform variants select
Flutter and Incular physics; they don't claim execution on iOS or macOS hosts.

## Ending the hold without disposal

A separate authored Flutter probe exercises the same held overscroll,
but leaves the viewport mounted. A release and a pointer cancellation
must both move the position back to its edge without another drag.
Before the fix the Incular Up case settled, but Cancel ended the
activity and stranded the position outside its bounds. Cancel now
hands the existing activity to its physics with zero velocity, as
Flutter's ScrollDragController.cancel does, without carrying motion
from the cancelled event. Standard in-range unsnapped physics can
still rest immediately.

Both authored Flutter scenarios passed. Seven focused native tests
passed, including iOS/macOS teardown and both-edge bouncing controls.
The authored probe does not increment the upstream registration count.
