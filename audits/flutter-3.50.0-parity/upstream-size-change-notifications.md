# Layout change notifications

Both exact `widgets/size_changed_layout_notification_test.dart`
registrations passed on pinned Flutter 3.50.0. The native base listener
now sees the concrete SizeChanged event after a retained child changes
from 100×100 to 200×100, not on initial layout or unchanged relayout.
The test replaces the listener before resizing so the current callback,
not the old one, handles the event. The zero-area notifier measures
zero and does not emit an initial event.

The same base listener can receive scroll activity without also receiving
viewport metrics notifications. A native control verifies that its
handled update does not bubble to the outer listener. Twenty-three
focused native tests passed, including the existing retention, ordering
and notifier controls.
