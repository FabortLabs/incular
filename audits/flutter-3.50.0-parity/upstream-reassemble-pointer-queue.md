# Flutter 3.50 reassembly input lock

Pinned `foundation/reassemble_test.dart:21` and
`widgets/reassemble_test.dart:11` passed. The first sees the input lock
inside the reassembly hook; the second rebuilds a mounted text widget
without error.

The ordinary native event route originally sent two taps to the stale
callback after reassembly was requested but before the new tree drew.
The window now queues pointer packets while its warm-up is pending.
It keeps the original timestamps and order, dispatches them through the
normal input path after the rebuilt layout is ready, and carries any
new frame demand into the host's frame result. Keyboard and window
events are not placed in the pointer queue.

The pinned authored Flutter test uses the engine packet callback:
the before handler receives Down and Up normally, the next contact
is held during reassembly, and the rebuilt handler receives it in
order. The Rust hot-reload regression delivers both queued taps to
the rebuilt handler, not to stale code. The separate native warm-up
test checks packet timestamps and verifies ordinary input resumes.
The feature-enabled warm-up and reassembly targets passed 15 tests;
the default-feature warm-up and pointer targets passed 22.

## Physical packets

Pinned `gestures/locking_test.dart:42`, “Pointer events are locked during
reassemble,” passes. The retained listener and tap recognizer originally received
physical Down/Up as soon as the driver submitted the packet, before the warm-up
layout. Now the direct runtime route still validates samples at ingress, but
holds hit testing, listener delivery and scoped cancellation behind a completed
layout. The original event timestamps are retained. A separate native regression
verifies an intervening contact cancellation prevents the queued Up from tapping.
This does not assert a shared FIFO across physical and ordinary window routes.
The default-feature physical-packet and scheduler warm-up targets passed 23
tests.
