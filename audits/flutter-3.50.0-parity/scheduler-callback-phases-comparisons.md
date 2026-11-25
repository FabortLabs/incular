# Scheduler phase comparisons

Flutter 3.50.0 `ab2912035e` vs Incular `c647cb85`. The exact upstream
`packages/flutter/test/scheduler/binding_test.dart:44` regression matches:
a persistent callback registered by another persistent callback is
absent during the first frame and runs on the second.

Two additional pinned custom Flutter probes passed while the focused
Incular assertions failed:

- A post-frame callback registered from the persistent phase runs after
  the already queued post-frame callback in that same Flutter frame.
  In Incular it is delayed until the next frame. Transient→transient
  and post→post deferral are matching controls.
- The current frame timestamp equals the callback's timestamp in
  transient, persistent and post-frame callbacks in Flutter. Incular
  exposes that state only in the transient phase; both clear it between
  frames.

Reference `scheduler/binding.dart:790–805,1259–1272,1345–1377`;
Incular `crates/incular-animation/src/scheduler.rs:289–311` and
`crates/incular-runtime/src/frame.rs:2593,2744`.
The committed Rust target is `flutter350_scheduler_callback_phases`.
These cases are distinct from the previous ticker-start, detached-batch
cancellation and app suspension tests.

## Ticker scheduling order and muting

Flutter places ticker callbacks and ordinary callbacks in one ordered
transient batch. Previously Incular ran all ordinary callbacks first
and could deliver a muted ticker immediately if a callback unmuted it.
Both differences reproduced in focused Rust, then passed after sharing
one queue. A ticker's renewed request takes its real scheduling position.
Unmuting during dispatch joins the next batch. A ticker started muted
while idle does not accrue time before its first delivered tick; one
already ticked retains its elapsed time across mute and unmute.

The existing start-during-dispatch and stop/restart-in-dispatch Rust
controls were already green before this patch and remain green. The
pinned original `scheduler/ticker_test.dart:21` passed with a partial
Rust counterpart: all ticking/active counts pair, but Rust's idempotent
duplicate start does not emit Flutter's debug error or diagnostics.
Both authored Flutter cases and 47 focused Rust checks passed. The
queue reuses its dispatch buffers and stores a weak ticker reference
instead of allocating a closure for each recurring tick.

## Raw system frame timestamps

Exact `scheduler/scheduler_test.dart:241` pairs all four timestamps
after anchoring Rust's monotonic instants to the first raw frame:
the raw clock reports 2, 4, 6, 8 seconds while callback time advances
by 0, 2, 2, 3 seconds. A dilation change only affects the callback
clock. Rust also verifies the raw timestamp is still accessible while
idle after a frame. Thirteen focused Rust checks and the exact pinned
Flutter case passed. The Linux runner now ignores only the reporter's
suite setup/teardown markers when checking the exact selected names.
