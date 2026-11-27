# TweenAnimationBuilder retained lifecycle

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
Reference: `widgets/tween_animation_builder_test.dart` and
`lib/src/widgets/{tween_animation_builder,implicit_animations}.dart`.
Integer and nullable size interpolation follow `lib/src/animation/tween.dart`.

The native widget tests verify the full sampled value sequences for start,
non-start, retargeting, duration and curve changes, static child retention,
nullable sizes, and a zero-area view. Updating the start descriptor is not
a jump to its proposed begin. IntTween rounds ties away from zero; nullable
size interpolation treats a lone null as zero size, while an omitted begin
with a present target does not start an animation.

Before this patch, even a tick with no progress caused a redundant builder
call, and resetting the controller during retargeting could invalidate the
builder a second time. The builder now samples controller position without
subscribing to those synchronous assignments. Subsequent position changes
still rebuild. Completion follows the controller's status, rather than
sampling it from a value callback.

The authored curve-change probe confirms that a curve replacement on an
active tween immediately re-evaluates its current value, without resetting
time. The previous native assertion preserving the old curve contradicted
Flutter and was corrected. Completion at the original deadline is observed
on the first sample strictly after the configured duration.

All 15 original registrations passed in the pinned reference. The 17
focused native tween tests (including two authored checks) and all 14
adjacent completion tests passed. The broader implicit target produced 21
passes and two previously failing sliver paint checks; both failures were
reproduced with the pre-patch implicit animation source and are not treated
as evidence for this batch. No full workspace was run.
