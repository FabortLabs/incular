# Legacy Web key payloads at the keyboard listener boundary

Both exact pinned “Web key event” registrations passed:
`keyboard_listener_test.dart` :42 and `raw_keyboard_listener_test.dart`
:46. Native tests decode the explicit DOM strings, location and
modifier mask before routing the event through a mounted listener.
The typed recipient receives left Meta down/up; the raw recipient
sees its original `MetaLeft` code, key, location and Meta mask.
As in Flutter's Web data, the mask reports Meta without telling
which side is held. A typed-only event does not gain legacy
metadata.

This verifies the explicit payload compatibility path, not a
browser session or automatic binding of native DOM event objects.
Sixteen focused and thirteen adjacent tests passed.
