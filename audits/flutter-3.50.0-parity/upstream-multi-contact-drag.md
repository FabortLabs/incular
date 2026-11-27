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

### Button change while another contact is pending

Pinned case 2321 passed. In the mounted native fixture the first
pointer changes from primary to secondary at (9.9, 9.9), before
either pointer has passed slop. The abandoned tap stops delaying
the other pointer's default drag acceptance. The native callbacks
are one start at the button change, one update on the other pointer
at (100, 100), then one end on its release. The invalid source does
not win its old arena. Fifteen targeted tests passed; normal lifts
still follow the previous cancellation and acceptance rules.

### Final-contact cancellation and default arena teardown

Pinned cases 2388 and 2473 passed. A vertical drag's four-contact
native fixture mounts a tap over the first three contact positions.
The fourth contact has only the same drag, so it wins without slop.
Lifting the second and canceling the fourth and third do not end the
gesture. Canceling the last tracked contact emits one end without
a tap callback.

A separate native managed-arena test uses one horizontal drag and
two independent tap registrations for pointers at the original position.
Retiring the first tap produces no immediate callback; the deferred
default starts the drag once. Its accepted pointer can release while
the other remains tracked, and rejecting the remaining drag entry and
disposing the other tap causes no crash or replay. Four direct and
seven mounted focused tests passed.
