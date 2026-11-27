# Tap and drag competition in a mounted arena

Five exact pinned `gestures/tap_and_drag_test.dart` cases paired:
:424, :453, :509, :543 and :649. The Incular tests mount two real
recognizer factories in a RawGestureDetector and route the original
moves through the retained arena. The winner and callback sequence
are asserted, not assigned by the test.

Two native defects caused the initial failures. Every erased factory
was keyed by the type of the Rc wrapper instead of the concrete
recognizer. A detector with multiple factories could therefore call
one recognizer twice for a sample and never mount its competitor.
After separating the keys, deferred acceptance still let a losing
pan process the same movement and lose its pending cancel. The raw
arena now resolves a candidate before forwarding that sample to the
next; the loser reports cancel before the winner starts.

The pinned controls cover a five-pixel mouse move beating tap slop,
an eager combined pan beating a pure pan, two vertical drags beating
their competing combination, and the non-eager combination yielding
to a pure pan. Six focused and four adjacent mounted raw arena tests
passed. Other tap-series timers and sole-member default wins are
independent cases.

## Horizontal wins, release ties, and factory replacement

The exact :577, :616, :839 and :902 registrations also pair. Horizontal
and diagonal motion cancel the vertical contender before the combined
start/update/end callbacks. A near-stationary release awards the first
registered combined member. Motion between the tap and pan thresholds
does not produce a spurious drag when both pan participants remain
unresolved until release; only the pure pan's cancel is observed.

An authored Flutter probe replaces a RawGestureDetector factory under
the same concrete type key. Incular's bulk and incremental builder
methods now follow the same replacement rule and keep the existing
key order. Previously the old factory remained active and raw routing
could visit it twice. The paired control delivers only the replacement
tap. Eleven focused and four adjacent native tests passed. The
authored factory control is not another original registration.

## Default wins after the sample is routed

The pinned :487 and :865 cases passed. Native :487 now lets the
only raw horizontal recognizer accept the press before a move that
exceeds overall pan distance but not the horizontal threshold. In
:865, a pure tap now leaves at its **tap** slop instead of staying
pending up to the wider pan slop. The remaining combined recognizer
wins before release and produces down, start and end. Reordering
the same mounted factories after a virtual one-second delay gives
the second tap to the first ordinary tap recognizer, without stale
state.

Default raw awards run after all routes have seen the current
sample, consistent with Flutter's deferred sole-member resolution.
A mounted ordinary vertical drag can still join before the raw
default is assigned and wins the mixed-route test. Explicit raw
resolutions still take effect immediately within their route.
Fourteen competition and four adjacent raw-widget tests passed.

## Losing a drag, then tapping or holding again

The pinned :682 and :744 cases passed. A standard horizontal drag
wins the 18.1-pixel movement over the non-eager combined recognizer
and a pending long press; the same physical pointer can then be
pressed again safely. The standalone long-press recognizer now
participates in the mounted raw arena. Its own withdrawal (including
a short release) reports cancel; an external loss stops tracking
without emitting a spurious second cancel.

The release sweep waits until the other raw contenders have processed
the release. The first tap's long-press withdrawal can therefore
cancel before the tap is reported. After the next consecutive tap,
the combined recognizer wins and delivers its numbered start/update/end
instead of a long press. Sixteen mounted competition cases, four
adjacent raw cases, and the direct long-press and tap-and-drag
suites passed. This comparison supplies the logical timestamps with
pointer samples; frame-driven raw deadlines are a separate check.

### Detector removal is not an arena loss

The authored `raw_long_press_dispose_test.dart` and retained native
fixture remove an idle raw detector after pointer down. The only
long-press candidate had already won by default, but had not met its
hold deadline. Removing the detector cancels the pending hold exactly
once. Neither a later frame nor the device release starts or ends it.
The router now passes the specific pointer to cancellation separately
from a competitor's rejection, which remains silent for long press.
This authored lifecycle test adds no original registration credit.
