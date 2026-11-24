# Additional official foundation and scrolling tests

- Pinned `foundation/change_notifier_test.dart` lines 207, 233, 256,
  397, 539 and 564 all ran and passed. The committed
  `incular-core` target `flutter350_change_notifier_parity` matched the
  five synchronous listener cases: immediate add/remove, duplicate
  listener and self-removal ordering, recursive notification, and several
  removals during dispatch. Rust owns subscriptions with tokens rather
  than removing by callback identity. This does not claim Dart's internal
  backing-array resizing behavior.
- `Value notifier` at line 397 remains a reference-only result. A
  scheduled `Signal` observation does not translate ValueNotifier's
  synchronous public listener API. Do not count this as a paired runtime
  outcome.
- Pinned `physics/newton_test.dart:270` matches the committed
  `incular-scroll` target `flutter_upstream_bouncing`. An infinite
  trailing extent never reaches a spring; the public bouncing
  configuration at DPR 20 matches the 1 px/s velocity tolerance and
  all the recorded position, velocity and completion assertions.
  Flutter's explicit custom spring constructor is not equivalent for
  trajectories that do reach a bound.
