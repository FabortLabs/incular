# Drag release velocity limits

Four pinned `gestures/drag_test.dart` registrations (1367, 1456,
1479, 1499) passed. Native counterparts drive the actual pointer
recognizer with the original 0/25/50 ms sample timelines, and the
horizontal stress case with two samples at the first timestamp.
The sole recognizer gets the same early acceptance as the Flutter
arena. Each native recognizer is reused across its input table.

Pan scales the velocity vector to 8,000 logical pixels/s without
changing direction and leaves primary velocity unset. Horizontal and
vertical drags clamp only their own axes, set primary velocity to
the retained component, and zero the perpendicular component.
Purely perpendicular motion releases with zero velocity. The new
fixtures and the existing velocity sample-window controls passed.
