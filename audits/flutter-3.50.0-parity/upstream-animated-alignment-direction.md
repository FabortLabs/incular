# Animated alignment through direction changes

Five pinned original cases in `widgets/animated_align_test.dart` pass
on both sides. The 100×200 child in the RTL visual-to-directional
transition stays on the physical right edge throughout; its top-right
y is 0, 0, 200 and 400 at the original samples. A width factor in the
original Row and a height factor in the original Column each report 50
from a 100-pixel child. The two null-factor fixtures keep the child
100×100, including the Row inside the root-constrained SizedBox.shrink.

The authored `implicit_direction_tween_test.dart` exercises a physical
top-left to logical bottom-end tween in both animated alignment widgets.
Flipping from LTR to RTL at 100 ms changes the child position from
(75,75) directly to (0,75). At 150 ms the y coordinate is 112.5; at
200 ms the child reaches (0,150). The old implementation held (75,75)
at the flip because it baked direction into its tween target. Now the
logical and physical contributions interpolate before resolving direction.

OnEnd still fires once, strictly after 200 ms, not at the duration
itself; changing direction alone after completion relocates the child
without starting another run. The pinned Dart probe allows 1e-12
logical pixels for rounding in its double coordinates. The Rust
positions at these same samples are checked exactly. The separate strict
frame harness and its tolerance are unchanged. Thirty-nine focused
Rust tests and the two authored Dart cases pass.
