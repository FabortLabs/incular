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
