# Shared single-axis drag across contacts

Four exact `gestures/drag_test.dart` registrations passed (2056,
2120, 2185, 2253). The native fixture uses real mounted recognizers
and sends the original two contacts with horizontal drag and tap in
competition. Before the fix the second press incorrectly emitted both
another drag-down and a drag-start.

Pure application single-axis drags now share a monodrag recognizer
by window, mounted element, and axis, as the application pan already
did. Later contacts do not restart the sequence. A non-primary tap
contact has no user callbacks, but its presence no longer gives a
competing drag automatic sole-press acceptance. The mounted regression
checks FIFO and FILO cancellation ordering, a first contact winning
the drag before the other is lifted, and a second contact winning
after the first has left. Tap fires only for its primary contact;
drag start/end or final cancel fires exactly once in the right order.

Twenty-two targeted widget tests passed, including device routing,
absolute trackpad and touch, gesture buttons/settings, rotated drags,
and local callback geometry. Mixed detector configurations and the
scroll/drag-and-drop specialized paths keep their original routes.
Button-mask changes during a multi-contact arena are separate and
not credited in this batch.
