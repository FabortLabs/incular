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
