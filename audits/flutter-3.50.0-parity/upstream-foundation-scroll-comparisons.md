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
## Typed value notifier

Pinned `change_notifier_test.dart:397` and the Rust equivalent
both notify once when replacing 2.0 with 3.0 and stay silent when
writing 3.0 again. Additional Rust regressions exercise reentrant
writes, a removed subscription, tracked reads, and real retained
ListenableBuilder output changing width only on unequal values.
All 23 selected core and widget tests pass. Rust listeners are
owned subscriptions; dropping a subscription unregisters it.

## Observer collections and numeric clamp

Both original `foundation/observer_list_test.dart` registrations (:9, :26)
pair insertion order, reverse removal, and the hashed list's reference
counts: ten adds of one value yield one distinct item that survives nine
removals. Additional Rust controls cover removal and reinsertion order
and slot reuse across 700 registrations. The hashed list uses the existing
Arena and a standard map; no benchmark is claimed.

The exact `foundation/math_test.dart:9` `clampDouble` registration pairs
all five results, including both infinities and NaN mapping to the upper
bound. Additional checks preserve signed zero and reject unordered/NaN
bounds with a Rust typed error. Three selected Flutter originals and
ten focused Rust checks pass; existing physics clamping is unchanged.
