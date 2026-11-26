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

## Single and double tap on the same detector

An ordinary tap now waits 300 ms from release when the same detector can
also recognize a double tap. A matching second press followed by release
emits only the double, even if it remains down across the old expiry.
Without a second press, the single fires at expiry, not one millisecond
early. Flutter's actual routed trace also showed that a new ordinary
press supersedes the older candidate: a rejected second does not restore
the first; a distant second can emit its own single immediately; a
too-early second starts a new deadline. All seven authored Flutter
scenarios match the six Rust tests (one Rust test covers both rejection
paths). The remaining low-level same-frame onTapDown/Up/Cancel timing
and cross-detector competition are separate coverage gaps.

The two pinned `gestures/transformed_double_tap_test.dart` originals
(lines 12 and 53) also pair. At both 2× and 0.5×, a 99 px global
separation delivers a double tap; 101 px does not. Both targeted Rust
cases pass. The pre-existing direct tap-diagonal cancellation mismatch
is unchanged.

## Inner double tap versus an ancestor single tap

The hold now follows the pointer route, not just the callback's own
detector. When a child only accepts double taps and its parent only
accepts single taps, the first release stays silent. Both pinned Flutter
and Incular then deliver only the child's double on a qualifying second
tap; only the parent's new single if the second press is outside the
child; or the original parent's single at expiry if no second press
arrives. All three Rust fixtures failed before the fix and pass after it,
along with the eight same-detector and transform checks. The low-level
tap-down/up/cancel families and drop order during unmount are not covered
by this completion-callback slice.

The equivalent observable trace in upstream `double_tap_test.dart:733`
also passes. It stays partial because Flutter directly constructs and
sweeps two independent arena entries, whereas the Incular counterpart
routes actual taps through the retained parent and child detectors.

## Current overlapping release order

The exact FILO original at :252 passed on the pinned SDK again. Both
native overlapping-release orders passed: releasing pointer 2 before
pointer 1 emits nothing for pointer 2 and still recognizes pointer 1 once.
This closes the old ledger mismatch; it does not change the separate
partial arena-close comparison for outside-slop cancellation.

## Tap cancellation after the sole arena member wins

Pinned `Distance cancels tap` and its short-move control both passed.
Incular's recognizer test now explicitly registers and closes the
sole-member arena, delivers acceptance to the tap, and checks the
entire Down, Move, Up, and sweep trace. A move outside the circular
slop produces exactly one cancellation and no tap; repeating the
close cannot deliver a second down. A separate arena control waits
while there are two contenders and awards the survivor once.
This verifies the direct recognizer protocol; retained pointer
adapters can use it without changing their competing tap claims.

## Overlapping touch contacts on a mounted detector

The standalone recognizer comparisons above did not cover how the
runtime instantiates candidates per pointer. Four authored Flutter
`mounted_overlapping_tap_test.dart` probes and corresponding Incular
mount tests verify both release orders, a third press while a
suppressed contact is still held, cancellation of the primary, and
independent taps recognized by sibling detectors.

Pre-fix, while one detector had multiple touches held down, the
runtime reported taps for the later contacts. It now tracks the
primary and its concurrent contacts per mounted detector, without
blocking a separate detector or that widget's scale/drag recognizers.
Pointer cancellation, final release and unmount remove the
association. The first contact uses no additional pointer buffer
allocation; later overlapping contacts are the only ones stored in
the vector.

All four authored references and 22 focused native tests passed
(`flutter350_mounted_overlapping_taps`, transformed coordinates,
double-tap competition and tap-region semantics). The original
FIFO/FILO registrations were already counted; this live backfill is
not counted twice.
