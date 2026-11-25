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

## Scheduler debug banners

The pinned `scheduler/debug_test.dart:9` registration passed. Rust
pairs the clean, begin-only, end-only and reset assertions and preserves
the error reason. A focused control also verifies that enabled banners
bracket frame callbacks, while the default is silent. Six focused Rust
checks and the exact Flutter original passed. Rust uses per-thread
configuration and a replaceable diagnostic output rather than Flutter's
global debug print function.

## Transient registration without a frame request

The pinned `scheduler/scheduler_test.dart:339` passed. A callback queued
with `schedule_frame_callback_with_request(..., false)` joins the next
transient batch without making the scheduler or runtime request a frame.
The default still requests one. Both run at the timestamp of the supplied
frame and clear their demand afterward.

An authored Flutter control verifies that canceling the only callback
does not retract an already requested frame. Baseline Incular incorrectly
withdrew it. Canceling a passive callback requests nothing. Existing ticker
demand stays independently withdrawable when the last ticker stops or
mutes, and frame demand remains a constant-time query. Existing
start-during-frame ordering and callback-cancellation controls pass.

## Binding-free time dilation

The pinned `scheduler/time_dilation_test.dart:11` and its Rust counterpart
start at 1.0, set and read 2.0 successfully, and create no runtime or frame.
The neighboring slow/fast ticker tests still pass. This is independent of
the host's vsync or window state.

## Future-returning UI tasks

Pinned `scheduler/scheduler_test.dart:317` yields twice inside
an idle-priority task and awaits termination. Flutter's result
does not resolve until the task has completed its side effect.
Before the repair, Incular's synchronous scheduler resolved an
outer result containing an unpolled Rust future. The two yields
and the body never executed unless the caller polled that second
result independently.

Use `schedule_async_task` for a callback returning a Rust future.
Its result stays pending across both yields and resolves to 1
after the side effect. The initial callback enters the same
priority queue as synchronous UI work; once it starts, future
wakeups do not go back through the initial priority filter.
Dropping the observation handle does not cancel the task and a
panic during polling becomes an error without preventing the
next task from running.

A retained Runtime control parks a local, non-Send future,
checks that the resting host has no redraw pending, and wakes
it from a worker thread. Duplicate signals coalesce to one
host wake. The work resumes on its owning UI thread, not the
worker, and a stale waker after completion schedules nothing.
This is an in-process event-loop boundary check, not an OS
presentation or Dart microtask scheduling claim.

## Frame timing observers

Pinned `scheduler/scheduler_test.dart:229` reports an empty
batch to a failing callback. Incular's observer surface also
delivers an empty report, captures the error text ("Exception:
Test"), and continues to the next observer. An authored pinned
test and its Rust counterpart additionally replace subscriptions
during a report: the removed observer is skipped immediately
and the new one starts with the next report.

The Runtime application integration sends the completed
`FrameRecord` only once renderer metrics reach its existing
host path. The Rust fixture checks that preparation, encoding
and submission are merged into the CPU total before the
callback, rather than emitting the provisional UI-only record.
This is an Incular frame record, not a claim that Flutter's
Dart VM timeline event or engine vsync/raster timestamps
have been reproduced. The exact timeline event at :204 and
two warm-up scheduling cases at :148/:167 ran on the reference
and are still open on Incular.

## Ticker diagnostics and invalid restart

The pinned `ticker_test.dart:104` control now pairs the one-line
description and creation-stack option. Rust checks a named constructor
appears in that stack. Capture and storage are debug-only.

The existing :21 comparison now rejects a duplicate start in debug,
retains the original run, and reproduces active/ticking/count values
through mute, resume and stop. The diagnostic carries the summary,
reason and affected ticker with its stack. It remains partial because
Rust's error is a panic with those parts, not Flutter's three structured
DiagnosticsNode objects. Twelve focused Rust and the original Flutter
tests passed.

## Warm-up begin and draw

Pinned `scheduler/scheduler_test.dart:148` and :167 both passed.
Runtime warm-up owns one begin/draw pair. Transient callbacks run on
begin, their microtasks (including nested microtasks) drain before the
actual retained build/layout/paint, and post-frame callbacks run after
paint. :148 pairs the same checkpoint. :167 is partial: there is one
reservation across duplicate requests and queued prioritized work stays
locked until draw, but a Runtime driven directly by its Rust host has
two explicit phases rather than two Dart Zone Timer registrations.

The same Rust tests check the epoch does not absorb warm-up wall time,
the last native frame timestamp is unmodified, a previous normal frame
request survives, late microtasks request the next frame, and abandoning
a reservation does not block another runtime. The original :281 is
skipped in the pinned SDK and is not credited.

Application windows can now queue a warm-up before their next draw.
Queuing another window never locks the current window's normal frame,
and a zero-size window does not reserve the scheduler before drawing.
Hot reassembly queues the same warm-up path for each window. The
hot-reload-enabled focused Rust tests check the actual managed-window
path and the callback epoch after reassembly. Asynchronous task wakes
are coalesced until draw finishes. This does not claim a full desktop
pointer-input event lock or Dart timer integration.

## Performance preference handles

All four pinned `scheduler/performance_mode_test.dart` registrations
pair: a request sets the scheduler's reported mode; a conflicting
request cannot replace it; two compatible handles keep it until the
last release; and one handle emits balanced debug create/dispose
events for the same identity. Rust uses thread-bound owned handles,
so dropping the last handle also releases its preference. A
debug leak assertion checks unreleased requests.

This comparison is scoped request arbitration and lifetime, not
Dart VM GC behavior or changes to wgpu device selection. The original
registrations assert only the requested scheduler mode and handle
events, not measured latency, throughput, or process memory.

## Native timeline payload

Pinned `scheduler/scheduler_test.dart:204` passed. The Rust application
accepts the same ordered host timestamps and dispatches one diagnostic
event with exactly frame 1991, start 10000, elapsed 15000, build 5000,
raster 4000 and vsync overhead 5000, all times in microseconds. Batched
reports preserve order; invalid phase ordering emits nothing. Ordinary
renderer summaries still produce only the existing frame callbacks.

This is partial: Incular's typed event is named Incular.Frame rather
than a Dart VM service event, and native adapters without phase
timestamps do not invent a sample or a GPU timing.

## One-frame benchmark step

Pinned `scheduler/benchmarks_test.dart:43` passed. The deterministic
Rust frame driver pairs the 16 ms dirty-tree step: a real mounted
root is marked for rebuild, a ticker supplies the begin phase, its
microtask runs before persistent draw, and the single step increments
both phase counters by exactly one even with a following tick queued.
The retained root is not replaced. No workload timing benchmark is
claimed from either behavioral fixture.
