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

## First valid decision in either member order

The other five registrations (66, 114, 125, 147, 159) passed. An
explicit acceptance after close delivers immediately. In either
member order, a second acceptance cannot replace the winner or
produce a second callback. The same ordering holds for two eager
acceptances, but both remain silent until close. This completes the
previous partial rows that had only checked terminal dispositions.
An additional native test opens another round from a rejection
callback; it verifies that callbacks do not run with the arena state
borrowed and that the new round is resolved independently.
