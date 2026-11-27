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

## Live controller descriptions and disposal guards

Pinned `animation_controller_test.dart:429,1074` passed by exact name.
The native description stays on one line at rest, at the original
10ms-forward samples, during reverse, and after stopping or disposal.
It reflects the actual clock-driven values and direction, including
the pause and disposal markers, rather than formatting a static type.
The native description does not include Dart's identity hash.

Disposing a shared controller invalidates all of its handles. The
original six operations then reject use after disposal without changing
the retained value or restarting the animation. This is an observable
lifecycle check, not a claim that Dart's private ticker is represented
identically in Rust. All 28 focused controller tests passed.

## Missing timing configuration and duplicate disposal

The exact pinned `animation/animation_controller_test.dart:502`
registration passed. Incular rejects all four missing-timing operations
(forward, reverse, animate to 0.5, and repeat). After a valid first
disposal, a repeated attempt returns a typed error exposing both the
named diagnostic property and the original disposed controller identity.
The formatted message carries the same remediation and paused/disposed
state. Rust omits Dart's object hash and outer FlutterError formatting;
the panic wrapper still gives the actionable message. Nineteen focused
controller tests and eight nearby listener tests passed.
