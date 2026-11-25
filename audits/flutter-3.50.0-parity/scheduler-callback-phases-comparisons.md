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

## Forced frame requests during pause

Pinned `scheduler/binding_test.dart:17` pairs the forced ticker's frame
request and completion. An authored counterpart pauses the UI before
starting it and verifies elapsed values 0 ms and 10 ms on two forced
frames. Stopping it completes its run; a final already requested frame
does not tick or request another. A one-shot forced frame coalesces
duplicate requests and delivers the waiting ordinary callback once.
Normal lifecycle suspension and host-driven restoration frames still
retain the previous scheduling policy.

The original at :12 is partial: Rust verifies the frame request and
callback delivery, not Flutter's native dispatcher callback registration.
Eighteen focused Rust checks, two named Flutter originals, and two
authored Flutter cases passed. The behavior is verified at the runtime
boundary; a physically suspended OS host still has to supply a frame.

## Scheduler priority values

The exact `scheduler/priority_test.dart:9` case now pairs both bounds:
adding 10,100 to Idle gives 10,000; subtracting 10,100 from Animation
gives 90,000. The bound limits each offset, not the accumulated value.
Rust also verifies negative offsets and machine integer edges without
overflow. Four focused Rust tests and the original Flutter case passed.
This is the public value API; prioritized UI task dispatch is separate.

## Time-dilation assertions

The originals at `scheduler/binding_test.dart:29,35` pair the
debug behavior: after setting the scale back to 1 there is no error;
at 3, the call fails with the caller's exact reason. Rust's companion
fallible API additionally exposes the active scale in a typed error.
Six focused Rust checks and both named Flutter originals passed.

## Prioritized UI-thread work

The exact `scheduler/scheduler_test.dart:74` passed on both sides.
All seven threshold stages matched, including withholding work when
blocked, running the highest priority one per turn, accepting new work
between stages, and returning false after the last task. Rust also
checks the default policy with real runtime frames: an animation-priority
task runs before the ticker, then an idle task waits until the ticker
stops. Tracked task results preserve the callback's value and report a
panic as an error in unwinding builds. The UI queue is separate from
the existing background task executor.

Nineteen focused Rust checks passed. The named case exercises queue
semantics, not Flutter's engine timer driver or asynchronous result
flattening; the native Rust runtime services eligible work before its
next delivered frame, one short task at a time.
