# RawTooltip trigger mode and long-press lifecycle

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.
All five selected original registrations passed on pinned Flutter.

The scoped `flutter350_raw_tooltip_trigger_parity` tests drive a real
mounted Runtime with injected pointer and frame timestamps and observe
the mounted transient overlay. Tap mode (:1757), rejecting a tap in
long-press mode and showing by its release frame (:1767), manual
mode (:1794), and mouse-vs-touch triggering (:2184) agree. Manual also
has a programmatic-show positive control. The neutral Rust tooltip
uses an opaque content marker in place of glyph comparison.

The held-press lifecycle (:573) differs. Flutter shows at the
long-press deadline, remains visible across the long hold, and
dismisses after the configured delay plus settling time measured
from release. Incular is absent before release, appears only after
release, and remains mounted at the compared expiry checkpoint.
The observed five flags (accepted/held/next held/released/expired)
are [false,false,false,true,true] vs Flutter [true,true,true,true,false].
Tap and touch-release controls continue to work. Rust's tooltip
routes via a RawGestureDetector, where observation is tied to raw
pointer events; its long-press controller path also installs a hide
deadline when triggered. This comparison does not isolate exit
animation timing independently of the failed held activation.

## Current mounted deadline behavior

The pinned originals at :573, :601 and :622 pass, and the corresponding
mounted native tests now pass. An accepted stationary long press reveals
the tooltip on the 500ms frame. It stays up without a dismissal deadline
while held, including the 9,999-day injected-clock case. Releasing the
captured touch starts the specified delay, including when release occurs
outside the anchor. A tap starts its own delay at release.

The authored hover probe compares a 100ms reveal and 60ms dismissal delay.
No extra pointer movement is needed to reveal or hide the tooltip, and it
stays visible throughout a day of injected time while hovered. Original
:829 passed on Flutter but uses a zero hover delay, so its native comparison
remains partial. The controller also registers delayed reveals for explicit
dismiss-all. The mounted event-clock checks do not claim reverse animation
pixel timing equivalence. While a deadline is pending the runtime uses
its existing frame-demand scheduler and stops scheduling when it clears.

Validation: 24 focused native tests passed across the controller, tooltip
trigger/Escape, and upstream long-press targets. All pinned originals listed
here and the authored hover probe passed.
