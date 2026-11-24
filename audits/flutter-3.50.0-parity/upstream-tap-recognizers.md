# Serial, single, and double tap recognition

Reference: pinned Flutter 3.50.0, `packages/flutter/test/gestures/{serial_tap,tap,double_tap}_test.dart`.
Incular: `crates/incular-gestures/tests/flutter350_upstream_taps.rs`.
Replay: `cargo test --locked -j 2 -p incular-gestures --test flutter350_upstream_taps -- --test-threads=1`.
The reference tests ran in exact-name batches of up to five using
`run-upstream-reference.ps1`. Callback-family cases use the full
`Gestures of different buttons trigger correct callbacks:` group prefix.
Ignored replay logs are under `.tmp/flutter350-full-audit/`.

## Observations

- Serial taps: nine original recognizer-only cases verify the numbered callback
  trace, pointer cancel, motion past slop, timeout, distance, differing buttons,
  interleaved pointers, no callbacks, and secondary-only presses. For the
  different-button case an extra 100 ms comparison isolates button change
  from the original's 1 s timeout. Arena competition is not claimed here.
- Single taps: callback dispatch happens at release; reusing a pointer ID for
  a fresh tap emits exactly once per release. The duplicate title at :483
  instead exercises two competing recognizers, so it is not covered by the
  :186 fixture. FIFO overlapping releases match. At :252, when pointer 2
  releases before pointer 1, the Incular recognizer emits a tap for pointer 2
  while Flutter waits for pointer 1. This is a direct recognizer observation,
  not a claim about the widget tree's per-pointer routing.
- A diagonal move of (12,12) survives, and a 500 ms press without a long-press
  callback remains a tap. At :285, the direct Incular recognizer does not send
  the cancellation callback at the (15,15) move. Flutter's sole member is
  already accepted when its arena closes; Incular's raw direct handler does
  not model that close step. This is a partial result, not a verified native
  cancel-delivery defect.
- Primary/secondary/tertiary callback cases use the production button-mask
  classification and callback mapping, then drive the chosen recognizer. They
  check for no early callback, the correct down family after the deadline, and
  the exact release/cancel order, with no cross-family callbacks.
- Double taps: ten original recognizer cases cover positions, far and moved
  contacts, 5 s timeout and recovery, a 1 s held first press, overlaps and
  recovery, and 10 ms invalid pairs and recovery. Success asserts down details
  before completion; rejected attempts assert no callback at all. Button
  filter injection and inter-recognizer competition are not claimed here.

At the audited baseline: 30 of 32 targeted Rust tests passed; FIFO/FILO
order and slop cancellation are independently observable in the test output
(the two failures are FILO and outside-slop cancellation). The ledger records
31 fully paired original registrations and one partial comparison.

Remediation: unrelated contacts no longer replace, finish or cancel the
single active contact. Both overlapping release orders now pass, together
with an additional wrong-pointer move/cancel control. The focused gesture
targets pass 84 tests total. The separate partial outside-slop comparison
still fails in the direct handler without Flutter's arena-close step.
