# Explicit animation controller disposal

Pinned `animation/animation_controller_listener_test.dart:9` passes.
The native fixture controls notifications directly before and after
disposal and verifies both value and status listeners are released,
including objects captured in the callbacks. A live clone no longer
keeps the registrations or scheduler ticker alive. Disposal from
inside a listener also suppresses later listeners in that snapshot.

Pinned `animation/animation_controller_test.dart:493` passes. The
disposed native controller still has a one-line diagnostic with
its disposed state and last known value. The separate structured
FlutterError diagnostic is not credited. The already paired memory
event original `:24` was also rerun.

Pinned `animation/futures_test.dart:122` now pairs. After real
scheduler ticks at 0 and 50 ms of the 100 ms run, explicit disposal
cancels the pending future across handles. The async body has not
resumed on return from disposal. On its next executor poll, it
catches TickerCanceled and completes without the success branch.
All 48 selected native tests passed.
