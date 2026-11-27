# DecoratedBoxTransition paint order

The pinned authored `shape_decoration_transition_position_test.dart`
samples a 40×40 child's center. Flutter foreground paints opaque green
over opaque red, and its background mode leaves the center red. The
child still receives taps in either mode. A foreground Box-to-Shape tween
keeps the center green at 0, 0.25, 0.5, 0.75 and 1.

Before the native fix the pure foreground shape painted green before red,
and a foreground mixed tween changed order once it crossed from Box to
Shape. The transition now forwards position for both variants, and the
shape wrapper orders its already pointer-transparent paint around the
child. The default remains background. Retained tests verify the draw
sequence at each sample and send actual pointer presses/releases to
the child. Sixteen transition and 22 shape-border tests passed. These
are authored reference comparisons, not new original registrations or
native GPU pixel readback.
