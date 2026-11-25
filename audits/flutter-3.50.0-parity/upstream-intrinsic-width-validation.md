# IntrinsicWidth step validation and zero-area view

The two pinned widget registrations in `intrinsic_width_test.dart`
passed and now have complete behavioral comparisons in Incular.
Omitted and explicitly zero steps both lay out a fixed 100×50
child as 100×50. Debug builds reject negative width and height
steps through either Rust constructor style, matching Flutter's
widget assertions. Previously the Rust setters passed them into
layout without reporting the invalid input.

The zero-area case injects zero physical host metrics into
Runtime, checks those metrics, and completes the frame with the
placeholder and its intrinsic width wrapper at zero size. It is
a simulated host boundary in both test harnesses, not a native
window resize test. These registrations do not verify positive
step propagation to the child or a complete intrinsic query API.

## Positive steps and parent bounds

An authored pinned Flutter probe and its retained Rust equivalent
now check both boxes, not just the wrapper. The 103×47 input and
steps of (10,8) produce a 110×48 wrapper **and** a 110×48 child.
Changing to a width step of 7 and omitting the height step yields
105×47. Under a 100×46 maximum the child itself must fit the cap,
and already-tight bounds of 97×43 are passed through unchanged.
Both boxes retain the same origin in every case.

Before the repair, Incular rounded only the wrapper and left the
child at 103×47, even when it no longer fit. IntrinsicWidth now
passes final tightened bounds to the actual child and forwards its
real baseline without shifting it. The retained measure uses
separate axis probes when necessary and bypasses them for a tight
axis. It does not implement Dart's public dry intrinsic-query APIs;
the paired result is the concrete layout sequence above, not a
claim about every kind of speculative child measurement.
