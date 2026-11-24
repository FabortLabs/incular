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
