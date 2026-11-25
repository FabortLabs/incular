# IgnoreBaseline suppresses only the actual baseline

Pinned rendering `baseline_test.dart:83,114` and widget
`basic_test.dart:1762,1792,1856` all passed. The ignored-baseline cases at
:114 and :1792 assert both text children at Y=0, and the zero-area
Placeholder at :1856 lays out to 0×0. The targeted Rust test pairs those
three registrations through retained Row and IgnoreBaseline geometry.

The two controls use Flutter's specially embedded test font and assert a
72px Y offset. The Rust control checks the exact difference between the
two measured font baselines instead, so those controls are partial.
No font-dependent numeric match is claimed.

Incular used a generic Builder to conceal the baseline, but that stopped
working when builders began forwarding it correctly. IgnoreBaseline
now uses a render proxy that forwards the incoming constraints, child
size, and zero placement offset without forwarding the baseline. The
retained test removes and restores the wrapper; the two text baselines
and sizes stay the same while the smaller child's vertical offset changes.
The full focused custom-layout target and Stack direction controls passed.
