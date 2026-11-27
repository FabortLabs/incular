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

### Linear, eased, and zero-area originals

Three exact pinned `widgets/transitions_test.dart` cases (42, 86, 134)
passed, and the corresponding native tests observe a mounted retainer
after updating its animation fraction. In the linear test these include
the initial 10 blur and 4 spread, the halfway 2.5 black-to-grey border
and five-pixel corner radius, half-size shadow geometry with unchanged
shadow alpha, and the absence of a shadow at completion. The eased
test checks Flutter's numeric tolerances on width, radius, blur and
spread with the foreground painting above the child. The completed
transition around a placeholder stays zero-sized in a zero-area view.

The original Dart color matcher compares floating channels to a
nearby color. The independent `decorated_transition_argb_test.dart`
probe checks the actual compact representation used by native:
linear fill FF808080 and border FF101010, eased fill FF515151 and
border FF161616. The native actual fill commands and border values
match those representations. The authored color probe does not add
another original registration. All 19 targeted native transition tests passed.
