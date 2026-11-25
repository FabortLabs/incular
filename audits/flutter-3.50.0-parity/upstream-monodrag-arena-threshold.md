# Monodrag arena ownership and callback thresholds

Pinned `gestures/monodrag_test.dart` :16, :71 and :102 ran
and have complete paired Rust assertions.

- An initial Down records its original timestamp until the
  drag starts. Acceptance clears that pending timestamp; a
  later rejection and duplicate accept leave it clear and do
  not replay onStart. Before the repair, Incular returned a
  null source timestamp for a drag that won at the press.
- With `only_accept_drag_on_threshold` enabled, the same
  uncontested arena can resolve at the press without starting
  the drag. Down→Up with no movement emits no start, update, or
  end. With the flag off, the exact same press emits start and
  end, with no update.

Both authored Flutter and Rust probes then move 17, 18, and
19px with the flag enabled and an already-owned arena. Start
fires only at 19px, with the crossing coordinate and
timestamp; the crossing move is not replayed as an update
under Start behavior. The Rust control also reuses one
recognizer on a new press to ensure the previous arena win
does not bypass slop in a later stream.

## Start, Down, and event timestamps

Seven exact `gestures/drag_test.dart` registrations are paired.
:85 and :107 use the winning movement at (30,0) for default
and explicit Start behavior, with no replayed update. :460
holds (10,10) as the start under Down behavior and projects
the first update to (30,10) with delta (20,0).

:137, :1603, and :1665 have the same exact reporter name;
all three were selected and run together. A sole horizontal
drag emits one start, then the original movement and a
stationary zero-delta event as distinct updates, then an end
on release. Diagonal movement retains its true world position
while each update drops the y component of its delta. :423
anchors the monotonic test clock and observes the original
100/200/300ms timestamps, including the stationary sample.
