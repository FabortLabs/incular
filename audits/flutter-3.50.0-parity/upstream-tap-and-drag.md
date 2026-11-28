# TapAndDrag: primary contact, series and drag thresholds

The pinned Flutter `gestures/tap_and_drag_test.dart` ran 14 exact
registrations; all passed. The corresponding Incular target passes 14
tests. These are the sole-member cases, not the tests that arbitrate
between two different gesture recognizers.

- :129/:158/:178/:198 check the callback count across a run of close
  taps, timeout, distance and the fourth press after a cap of three.
- :238/:252 check drag start, update and end, including a drag that
  belongs to the third tap without losing its count.
- :347/:369 check a primary contact accepted before a second pointer
  enters. Only the first contact reports a tap in either release order.
- :390/:406/:933 distinguish a tap with small motion, a drag past tap
  tolerance but short of pan threshold, and a fresh normal tap afterward.
- :954 verifies the cancel-only trace and a new count for the next tap.
- :975 is partial: no drag-end callback is installed and the next tap
  still reports its down callback. Rust has no corresponding Flutter
  global error-reporting hook; the test checks no panic.
- :1013 checks the exact final global point (120,45) after three horizontal
  drag updates, not just that the callback fires.

Reference: `run-upstream-reference.ps1 -Test gestures/tap_and_drag_test.dart`
with the named cases in three batches. Incular:
`cargo test --locked -j 2 -p incular-gestures --test flutter350_upstream_tap_and_drag -- --test-threads=1`.

Three follow-ups (:292/:313/:326) also match. Both pointers can register
before the first is accepted without interfering in either release order.
An acceptance delivered before the down event is routed still emits
both expected tap callbacks. This uses the public external-arena hooks
to deliver the same acceptance order, not an inferred callback.

## Update metadata: horizontal tap-drag versus pan

The pinned framework retains the full local motion vector and leaves
`primaryDelta` null even for horizontal tap-drags, despite the general
description of axis-constrained updates. Incular now exposes that
optional field without zeroing the cross-axis motion. Both recognizers
are checked under 2× scaling on two successive diagonal moves, including
the source timestamp, global and local position, accumulated travel in
both spaces, touch kind, and tap count. Both authored Flutter cases and
all 18 native cases passed. The probe adds no original registrations.
