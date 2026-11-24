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
