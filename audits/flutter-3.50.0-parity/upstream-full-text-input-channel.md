# Flutter full editing-state channel

The exact pinned `services/text_input_test.dart:259` passed. Flutter
rejects the incoming range 2–3 for text of UTF-16 length 1 before
delivering it. The Rust fixture mounts and attaches a focused editor
containing `test3`, sends the actual JSON method, and confirms that
the typed error names the range start (2) and the text length (1).
The controller value, focus and outgoing edit state remain unchanged.
Both the active real ID and Flutter's debug-only -1 path are exercised.

Native controls preserve reversed selection order across an astral
character, convert composition to byte boundaries, reject partial
sentinels, malformed metadata and range endpoints inside a surrogate,
and check stale client identity before decoding the state. Diagnostics
do not include the supplied editing text. A valid update uses the
existing platform-to-controller path, so it does not echo the same
state back to the host.

The byte-addressed editor represents Flutter's absent selection as
an unselected caret at the end of the text. It validates the wire
types of affinity and directional flags but does not retain those
flags in the applied native state. Floating-cursor messages are
not covered here. Five delta-channel, three full-state, and eleven
lifecycle tests passed.
