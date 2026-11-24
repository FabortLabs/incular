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
