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

## Legacy Web key identity and location

The pinned `services/raw_keyboard_test.dart` Web group passed all
12 selected originals. The native `flutter350_web_raw_keyboard`
target checks both individual and combined modifier bits, fallback
to the left modifier when the DOM location is missing, letter
case versus logical identity, control and arrow identities, legacy
`Esc`, numpad location, unknown key plane, and payload equality.
The three checked-in lookup tables are reproducible from the pinned
Flutter source with `extract-web-raw-key-catalog.py`. Location and
named identity take precedence over printable keys and fallback.

Unknown DOM codes are decoded into the same reserved key plane; the
numeric suffix is a deterministic Incular hash, not Dart's runtime
hash. This matches the original range assertions. The Dart-specific
diagnostic string is partial, not counted: Rust preserves the same
five fields, but renders its own Debug format. The six adjacent
Fuchsia data cases also passed. This is packet-level verification, not
a claim of browser hardware testing.
