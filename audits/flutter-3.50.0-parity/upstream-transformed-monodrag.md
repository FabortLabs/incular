# Transformed drag thresholds, positions and cancellation

Flutter projects movement onto the recognizer's local axis, then
measures the projected vector in physical world units. Before the
repair Incular used the world x/y components directly, so a control
rotated by 90° did not start its local horizontal drag on a 20px
vertical movement. The unchanged C09 retained test now passes:
20px starts the rotated and unrotated drags, while 17px does not.

The pinned transformed-monodrag originals pair seven registrations:
:15 and :380 check exact local/global coordinates and accumulated
movement without a transform; :71, :177, :436 and :542 cover both
axes under 2× and 0.5× scales; :283 covers a 45° horizontal drag.
The scale cases check both sides of the slop, start at the physical
19px cross-axis sample, report only the selected local-axis delta
on the following update, and report down/start/end exactly once.
A drag that loses to the competing tap below slop now reports one
cancel, not zero. The 45° case checks its global endpoint and local
delta against the pinned tolerances.

The authored pinned reference adds strict 17/18/19px threshold
checks at scales 4, 0.25, and -4. Under nonuniform scale, Down
behavior starts at the original press and maps the projected
first update back to the corrected world position. The later update
uses the actual pointer position. A core affine test verifies
that vector transforms remain independent of even large translations.
Unit lengths for each transformed local axis are cached at pointer
routing, avoiding two norms or inversions on every move.

The pinned vertical 45° original at :648 also ran but is still
open: it depends on winning by default when the competing tap
rejects, not merely on applying the correct scale to its local axis.
