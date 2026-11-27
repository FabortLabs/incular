# AnimatedContainer

Eight pinned `widgets/animated_container_test.dart` scenarios passed with
native equivalents: one-line diagnostics, no redundant active ticks on
identical targets, the exact RTL padding and alignment samples, a second run
that changes height without disturbing width, the physical transform pivot
samples, the default/no-clip to antialiased paint transition, and zero-area
layout after settling. Incular checks its own retained ticker count, not a
Dart callback queue. The original decoration control also snapshots Flutter's
private render-object diagnostic hierarchy and is not counted here.

Two supplemental Flutter tests change direction 50 ms into a 200 ms run.
Flutter resolves the *current* logical tween value immediately with the new
direction: alignment moves from x=175 to 525, and the transform-pivot
case moves from x=362.5 to 387.5. Incular previously stayed at x=175 and
x=362.5 respectively on that update frame and retargeted the animation.
Keeping both tween geometries directional until rendering restores those
positions, the original midpoint and finish, and one end callback on the
first frame past 200 ms. The target element is retained throughout.

The targeted container comparison passes ten native tests. The separate
implicit-animation integration target verifies the shared animation state.
