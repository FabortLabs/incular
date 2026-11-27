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

## Button changes in one press

Pinned `gestures/tap_test.dart:87, :745, :764, :784, :805` now
pair. Touch is rejected by a mouse/stylus allowlist, even when the
subsequent permitted streams reuse the same pointer ID.

Raw taps also now reject the original press as soon as its button
mask changes, including when the position is stationary. The old
path accepted the release and could click an unintended target. A
change before TapDown produces no callbacks; after TapDown it
produces exactly one cancellation. Restoring the mask or releasing
the rejected contact does not revive it, and the next genuine tap
works normally. Combined long-press and drag members continue to
use their own cancellation rules.

The retained runtime fixture confirms no TapMove, TapUp or click
comes through after a mouse chord changes. All five pinned Flutter
originals and 35 focused native checks passed.

## Tap movement and acceptance

Three more originals pair: mouse kind on down/up (:159), a pointer
cancel after the reported down (:568), and unbounded post-accept
movement with the original two position/delta pairs (:1105).

Tap recognizers now distinguish pre-accept and post-accept distance
limits, including an explicit unlimited policy. Reporting TapDown at
the deadline is not itself acceptance. Default tap distance is the
view's touch slop (18 logical pixels absent a setting), even for a
mouse; this does not change the precise slops of drags or multi-taps.
Before the fix a 2 px desktop mouse drift produced a cancellation.
The pinned authored and native retained probes both accept 2 and
18 px and cancel at 18.25 px. Native controls also exercise a
configured 4 px tolerance. All 44 focused native checks, the three
Flutter originals, and the authored Flutter probe passed.

## Tap callback failures

Pinned `gestures/tap_test.dart:432` reports an error from onTap
at release, not on press. The Rust callback preserves the original
typed panic object rather than flattening it to text, and a later
press still works. At :457 the winning tap withdrawn by a pointer
cancel identifies the callback as "spontaneous onTapCancel".

Each tap stage is isolated separately, so a failed onTapUp does not
skip onTap. The retained deadline and live mounted callback paths
use the same boundary. A focused retained test deliberately fails
a callback delivered after a double-tap deadline and checks that
the frame finishes and the error is not redelivered. This behavior
requires an unwinding panic strategy.

Both pinned originals and 28 focused native checks passed.

## Tap details held for a double tap

Previously the retained gesture path held the single onTap but
sent TapDown and TapUp as soon as the first pointer released.
Actual widgets could observe an apparently completed click while a
double tap still had a chance to win. The candidate now carries its
unreported details through the arena. When the single tap wins, it
reports remaining Down, Up, then onTap. A winning double tap does
not emit an ordinary Up. An already-reported long-hold Down is not
repeated on success, and the first held candidate is replaced without
an extra cancel when a new allowed pointer registers, as Flutter does.

The authored pinned and retained native tests verify short and long
first presses, both deadline and double-tap outcomes. Before the fix
the native probe observed [down, up] on the first release; afterwards
it waited. No original-registration credit is claimed for this probe.

## Double-tap button admission

Six pinned originals pair on the direct recognizer. Only an exact
primary mask is admitted by default. A custom filter can choose the
secondary button, or reject every button. A rejected press interrupts
the existing sequence without an early Down or Cancel, and the next
two matching presses still work. If the custom filter accepts both
buttons, a changed mask starts a fresh sequence instead of completing
a double tap. The latter is an additional native control.
Four new native cases failed before the fix; all 57 focused native
tests passed afterwards. This slice does not claim mounted arena
release timing or cross-detector competition for non-primary presses.

Five authored mounted comparisons also passed. A non-primary press
only interrupts a double candidate if it reaches that detector. It
does not emit a phantom primary single; an admitted secondary single
still completes normally. If the held primary had already delivered
TapDown, an inadmissible press cancels it once; a silent candidate
stays silent. The runtime now invalidates the held tap before releasing
the double-tap sequence on the hit route. All 22 focused native tests
passed. These are mounted backfill cases, not new original credit.

## Moved and cancelled double-tap contacts

Four authored pinned comparisons and all 59 focused native tests
passed. Before the fix all four native regressions failed. A moved
first contact no longer seeds a false double tap. This fixture uses a
qualifying inter-tap interval, so the minimum-time rule cannot hide an
invalid first contact. A moved or cancelled second contact delivers
one cancellation immediately and cannot be revived by moving back or
releasing. Cancelling a first contact stays silent. Down and Cancel
handlers still work if no completion handler is installed. These
authored checks supplement rather than repeat original credit.

## Double tap in an independent gesture arena

Six pinned originals pair through the standalone recognizer and the
actual managed arena. A passive contender cannot win the first sweep
while the first tap is held, but wins once the hold expires. Another
contender can explicitly win the first arena before the first sweep or
between taps without emitting double-tap callbacks. If it wins during
the qualified second press, it cancels exactly once with no completion
on release. A primary double recognizer competes with a primary single
for TapDown but never enters a secondary single's arena.

The hold follows the arena's generation rather than an ID that the
host can reuse. A clean double accepts both actual arena registrations;
dropping a waiting recognizer or unwinding a failed cancellation still
releases the passive contender. All 60 focused native tests passed.
The older unfinished-double comparison remains partial until its
standalone companion tap path also verifies stale release ownership.

## Serial tap arbitration

Seven exact `gestures/serial_tap_test.dart` originals now pair with
the real Incular managed arena. Normal tap waits even after its press
timeout, so serial wins in both registration orders and the normal
tap sends cancel, not tap. Immediate release competitors win only
when routed first, and a press-time winner cancels serial before
release. Against double tap in both registration orders, all three
serial counts complete in order and no double tap is dispatched.

Incular previously reported serial up when an external arena had not
yet decided, and could report both cancel and up on a synchronous
rejection. Serial now holds the release until acceptance. A wrong
pointer's later resolution cannot alter the next press, and cancellation
drops the old position instead of waiting for another event. Authored
checks exercise deferred and synchronous rejection and the actual
raw-gesture adapter, which reports arena decisions after routing.
Eighty-three focused arena and recognizer tests passed.

### Unfinished first double-tap arena

The exact pinned `gestures/double_tap_test.dart:733` passes, and
`flutter350_managed_tap_drag_competition` now covers the standalone
sequence with the same plain-Tap instance in both pointer arenas.
The double-tap contender holds only the first arena. The second
down, stationary move and release are routed only to plain Tap,
which fires once before the second sweep. Expiring the held first
arena still leaves the callback count at one; a stale result cannot
finish the newer tap. The separate mounted ancestor comparison
remains documented above, not substituted for this direct result.
