# Explicit gesture arena lifecycle

Five pinned `gestures/arena_test.dart` registrations passed (76, 86,
100, 136, 174). The Rust `GestureArenaManager` records actual
accept/reject callbacks with the pointer ID, including that no callback
has run at the intermediate checkpoints. It awards a sweep to the
first remaining member, defers it while held, does nothing when a hold
is released before any sweep, waits until registration is closed
before accepting an eager winner, and clears that winner if it
rejects while still open. Explicit resolution rejects other members
before notifying its winner; sweeping notifies the winner first.

The native default-winner control uses an explicit
`flush_deferred_resolutions` boundary in place of Dart's microtask and
verifies that a queued operation cannot affect a reused stream key.
This API models Flutter's explicit arena lifecycle. Incular's live
simultaneous-recognizer arbitration still uses its existing
`GestureArena`; these registrations do not claim equivalence for
every widget's integration with that runtime path.
