# Pointer routes, cancellation and local coordinates

All nine pinned `gestures/pointer_router_test.dart` registrations
pass. Eight pair with the new native public router: routes select a
pointer ID, registrations preserve order, specific routes precede
global routes, additions wait for another event, and removal stops
delivery to a still-queued route immediately. The native target also
recursively delivers an event while a route is adding a listener and
checks the new route does not leak into the outer dispatch.

A route can independently map global input into its own coordinate
system. In the original transform example both global and specific
routes see the same unchanged world coordinates; the press at
(16,36) becomes (3,3), and moving to (20,40) yields (5,5) with
local delta (2,2). The tests also retain phase, timestamp and
physical device metadata. The window router receives raw contacts
after the hit path listeners, before the gesture arena handles them,
and receives the native wheel signal without suppressing the default
scroll action. The retained tree creates this router only if someone
uses it. Ordinary move dispatch shares snapshots rather than copying
callback lists on each move.

One failed callback is reported before the next route and does not
halt delivery. The original :151 registers a throwing callback but
does not route an event. The authored Flutter probe dispatches a
real failing route and confirms the attached router, callback and
transformed event. Native diagnostics supply the corresponding
route ID, weak router reference, event and panic payload; the
Dart diagnostic formatting and closure identity are not claimed
(:151 remains partial). This checks Rust's unwinding panic mode,
not a build that aborts on panic.

Twenty-seven focused native checks passed, including retained
pointer fields, wheel resolver and raw arena regression targets.
The additional Flutter error probe also passed. Incular input
coordinates use the existing 2D affine type, not a full Dart Matrix4.

```sh
cargo test --locked -j 1 -p incular-gestures --test flutter350_pointer_router -p incular-widgets --test pointer_event_fields --test pointer_signal_resolver --test raw_arena_recognizers -- --test-threads=1
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe pointer_router_report_test.dart
```
