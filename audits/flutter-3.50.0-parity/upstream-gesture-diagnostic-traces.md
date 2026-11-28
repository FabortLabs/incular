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

## Pointer descriptions and tap-down state

A raw pointer sample now has brief and full single-line descriptions.
They use the recorded event phase, global position, and (when distinct)
local position; full output includes the real device, button mask,
contact, and synthesized marker rather than manufacturing defaults.

The tap diagnostic snapshot reads the real recognizer state and does not
change gesture arbitration. A raw primary-button press makes it possible
but does not report tap-down early. At the press timeout it reports
“sent tap down” exactly once; cancellation restores the ready state.
The formatted preview leaves out Dart object address hashes.

The pinned `gestures/events_test.dart:14` and
`gestures/debug_test.dart:207` originals passed, as did seven focused
native tests covering these descriptions and neighboring callbacks.

## Callback detail properties

The callback detail types now expose ordered inspector properties generated
from their actual positions, device, movement, velocity, force, and count.
Scale details now have Flutter-style identity defaults with zero contacts;
the explicit two-contact native constructor still behaves as before.
Seven focused native scenarios verify the property order, null values,
one-decimal diagnostic rounding, and real nonzero fields. Four adjacent
scale and pointer regressions also passed.

The pinned `gestures/debug_test.dart:223` passed. This comparison remains
partial: Flutter can construct some serial-tap and tap-drag details
without a device kind, whereas those public native callback values always
carry an explicit kind, including Unknown. The diagnostic output preserves
that distinction rather than printing a present Unknown device as null.
