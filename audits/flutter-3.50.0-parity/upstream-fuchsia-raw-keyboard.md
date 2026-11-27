# Legacy Fuchsia raw key delivery to retained listeners

Four exact pinned registrations passed: `keyboard_listener_test.dart`
:18 and :66, and `raw_keyboard_listener_test.dart` :18 and :73. Incular
now decodes an explicitly supplied Fuchsia HID record, rather than
pre-filling a normalized event in the fixture. The typed listener
receives left Meta down/up with the corresponding logical key. The
raw listener sees the original `0x700e3` HID usage, zero code point,
and left-only Meta mask. Both stop receiving messages when removed.

The compatibility catalog uses the pinned physical and logical maps.
A native control also checks printable characters, disjoint modifier
sides, and Fuchsia-plane IDs for unknown records. This compares source
payloads through the real mounted input router; it does not add an
Incular Fuchsia host or claim a hardware-device run. Fourteen focused
and thirteen adjacent native tests passed.
