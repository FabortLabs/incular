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

## IntrinsicHeight forwards baselines

The authored `intrinsic_height_baseline_test.dart` shows that in a
baseline-aligned Row, wrapping a child in IntrinsicHeight preserves real
baseline alignment, while adding IgnoreBaseline inside it removes the
influence and removing IgnoreBaseline restores it. Size is unchanged.
Incular's direct retained counterpart reproduced the missing baseline
before the fix and now checks both the actual baseline and its absence.
Existing intrinsic sizing and LayoutBuilder baseline targets passed.

Two separately paired pinned registrations in `widgets/basic_test.dart`
(:1892 and :1908) both retain a zero-size host with a PositionedDirectional
or IntrinsicHeight Placeholder child. The authored baseline probe does
not increment that count.
