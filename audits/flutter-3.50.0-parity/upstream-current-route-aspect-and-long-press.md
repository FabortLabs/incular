# Current-route TapRegion, AspectRatio, and long-press routing

Pinned Flutter 3.50.0 `ab2912035e`; Incular production `c647cb85`.

- TapRegion current-route registration (:1133) passes against
  Flutter and differs in `flutter_tap_region_route_parity`.
  After push, Incular calls both pages; Flutter calls only the
  current route. After pop, both call just the original page.
  The old page's element was retained under the second route, and
  the popped element was gone. Counts are after each transition
  settles; no claim about callbacks during transitions, outside-up
  or consumption.
- AspectRatio (:26, :37) matches finite loose constraints and
  unbounded width in horizontal scrolling: 500×250, 250×500,
  and 1200×600 respectively. Zero area (:54) also matches for a
  centered Placeholder with equivalent tight zero constraints
  rather than a native platform view resize; partial.
  See `flutter_aspect_ratio_geometry_parity`.
- All five original long-press Flutter registrations pass.
  `flutter350_upstream_long_press` drives real Runtime events and
  an injected frame clock. At :61, activation is at 500 ms, not
  499; at :106, movement before acceptance cancels; at :365,
  movement after acceptance reaches the move and end callbacks.
  These three are partial because the direct recognizer's
  add/close checkpoints are not independently ported. At :455,
  changing buttons after acceptance does not cancel either.
  At :427 a mask-only move at 300 ms must cancel before
  activation. Incular misses Cancel and fires Start and End;
  the next valid primary press still works. The new mask reaches
  Runtime input but is dropped on the ordinary recognizer move path.

## AspectRatio physical view recheck

Pinned :54 passes. Its permanent Runtime port now delivers actual
physical view metrics rather than constructing an unrelated zero
constraint. The centered Placeholder lays out at zero when the
physical viewport is zero. Its original nonzero size returns on the
next nonzero viewport, including scale-factor conversion. All three
AspectRatio originals are fully compared; the Runtime fixture does not
claim native window-manager behavior.

## TapRegion route-current remediation

The expanded Rust fixture reproduced outside-down and outside-up on
both pages after a push, and a separate port of :1191 reproduced the
covered region swallowing a tap on the new page's button. Both exact
Flutter originals passed. RouteOutlet now publishes currentness separately
from visibility. A non-current region stays registered, preserving group
membership and inside callbacks, but cannot receive outside-down/up or
consume the outside press. Popped pages still mounted for an exit
animation are also marked non-current. The focused Runtime fixtures now
pass both cases, including restored callbacks after pop and the existing
primary consumption and group controls. The exit-animation branch and
semantic actions are not counted as new comparisons.

The broader targeted `runtime route_outlet` filter had the same
83 passing and 13 failing cases, at the same assertions, before and
after publishing route currentness. Those focus and retained-editor
failures remain open; the TapRegion checks do not certify route focus.

Pinned original :1053 and the third Rust route fixture pass. Pressing
the bottom-right navigation button counts as outside for the outgoing
route before the push. The next outside tap only reaches the new route,
and returning restores the first region's notifications. The
per-route down totals match at each checkpoint: (1,0), (2,1), (3,1).

## Long press button-mask remediation

Pinned :427 and :455 pass along with all five focused Rust Runtime
cases. The button mask now reaches the ordinary recognizer on both
down and move. A mask change before acceptance emits exactly one
cancel and can no longer start or end a long press later in that
stream. A subsequent primary press on the same widget still works.
A mask change after acceptance does not cancel. These rechecks do not
add new originals; the earlier mismatch remains historical evidence.

## Custom long press duration

Pinned :75 and :91 pass. The Rust recognizer now supports a supplied
duration, defaulting to 500 ms. The 100 ms sequence stays pending
through 99 ms and then starts; release before the ordinary deadline
cancels once and does not start later. The direct Rust recognizer
tests cover callback and timer behavior but do not claim an independent
Flutter addPointer/closeArena lifecycle. These are two partial matches;
all 25 targeted timing, deadline, and tap-cancel checks pass.

## Cancel-only long press

The authored pinned Flutter probe observed one cancellation on early
release, slop rejection, and explicit pointer cancel with only
`onLongPressCancel` installed. An accepted hold did not cancel when
released. Incular's old guard required an `onLongPressDown` callback
to have fired, suppressing the early release in the Rust baseline.
The guard now tracks a possible long press rather than whether a
user callback was installed; rejection consumes it once.

All 26 focused deadline and cancellation tests passed after the fix.
This is an authored case, not an additional original registration.

## Chord at contact start

Pinned :204 rejects a combined secondary/tertiary button mask before
reporting any long-press callbacks. Incular previously reported
`onLongPressDown` immediately. The recognizer now admits long press
only for an exact primary, secondary, or tertiary contact; another
gesture on the same combined recognizer remains independent. The
conformance test checks no Down, no acceptance after one second, no
release callback, and that the next valid contact starts normally.

All 21 focused tests passed. The original is partial because Incular's
raw contact entry does not expose Flutter's independent add/close phase.

## Long-press release callback order

The pinned authored probe reports start, long, end, up for primary,
secondary, and tertiary mouse contacts. Incular initially reported up
before end; a late release with no intervening deadline poll also sent
start and release callbacks before arena dispatch, then reported long.
Already accepted releases now emit End before Up. A late release carries
both phases as a single arena action and sends no long-press callbacks
until accepted. The mounted Runtime tests pass for all three buttons
with and without the deadline frame, and the direct recognizer proves
the late action is silent before dispatch. All 35 focused tests passed.
This is a custom comparison, not an original registration.

## Current disposition of button-mask regression

The exact :427 registration passed again on the pinned SDK. The mounted
runtime test reruns the whole rejecting stream, then starts and ends the next
accepted press on the same detector; it passed with the five other focused
long-press controls. The historic failure is no longer the current ledger value.

## Direct long-press arena checkpoints

Six pinned originals now pair with the standalone Rust recognizer:
`gestures/long_press_test.dart:61,75,91,106,204,365`. Registration
and closing the pointer's arena are independently silent. Routing
the actual down reports Down, and only the deadline can start the
hold. The 100 ms override, early release, pre-accept slop, unsupported
chord, and post-accept 100 × 200 move/release each match at every
checkpoint. Dispose reports no synthetic event.

Six direct tests, 18 timing/tap controls and six mounted long-press
regressions passed. This compares the direct single-recognizer
lifecycle; it does not claim a new raw multi-member-arena adapter.
