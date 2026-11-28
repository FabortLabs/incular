# Gesture arena and tap callback diagnostics

Pinned Flutter `gestures/debug_test.dart` lines 10, 68 and 127 were
replayed against Incular's real managed arena, pointer recognizer
and deferred tap dispatch. Arena diagnostics and callback tracing
are independent and opt-in. Opening, adding, closing, routing
down/up, sweeping, publishing the winner, and invoking
TapDown/TapUp/Tap were compared in order. When both channels are
enabled the winner is visible before the callback trace. Turning a
channel off does not suppress the actual input callbacks.

The native fixture checks actual down and up positions, no
notification during the pending input route, no duplicate result
after draining deferred resolution, and no output on disposal.
All three originals and 20 targeted native arena/callback tests
passed. The ledger marks these three partial rather than pairing
them: Dart's runtime-type/hash formatting, recognizer-state text
and indentation are not duplicated by the native typed trace API.
