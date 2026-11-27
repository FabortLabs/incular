# Shape border interpolation dispatch

Pinned `painting/shape_border_test.dart:21,33,39,50` passed.
Three registrations pair with the shared ShapeBorder path. Missing
endpoints stay empty; identical Circle and BorderDirectional
instances retain the same allocation without running any hook.
With different custom shapes, both forward hooks are tried first.
The other two hooks receive the inverted time: the reverse receiver
sees 0.75 when the requested progress is 0.25. Either reverse hook
can win, and a successful forward hook prevents further calls.

ShapeDecoration uses this same dispatch. The returned outline
covers exactly 75 of 100 pixels in the runtime test; filling,
clipping and hit testing all use the returned contour rather than
a discrete switch to the original outline. The separate Flutter
OutlinedBorder-specific entry point remains partial: Incular has
one shared ShapeBorder protocol, not two dynamic types.

Pinned `painting/border_test.dart:18,27` also passed. Native tests
verify all four edge values and actual physical paint coverage
when either symmetric pair is absent. Those two cases remain
partial because Dart's assertions check object identity of
individual sides; Rust stores these small border-side values by copy.
All 17 focused border, shape interpolation and paint tests passed.

## Circle and star contour interpolation

The authored `star_circle_morph_test.dart` compared a 5.5-point star
with an eccentric circle. At progress 0.25 toward the star (or 0.75
back toward the circle), the outline has 5.875 points, a 3-pixel
stroke and squash 0.475. The native implementation previously
fell back to the unchanged circle. It now reproduces the inner
radius, rounding, rotation and both 35-point inner/outer coverage
samples; paint emits the single requested stroke. Near two points,
the two directions instead use their original progress for the
point count: 2.15 and 2.05. Both match.

Two authored pinned checks and 32 focused native shape, parameter,
paint and interpolation checks passed. These are contour samples,
not upstream golden or GPU pixel comparisons, and add no original
registration.

### Stadium and rounded-rectangle paths through a star

The authored `star_stadium_rectangle_morph_test.dart` checks the two
stages between a stadium and a star and all three stages between a
rounded rectangle and a star. The contour passes through circle
geometry. A rounded rectangle first interpolates through a stadium.
The native code retains the original endpoint stroke within its
stage instead of applying a global lerp twice without regard to
the active phase. For a 2-pixel outline and 6-pixel star, the
pinned stadium phases measure 2.5 and 5.5; the rectangle phases
measure 7/3, 4 and 17/3. Reversing the endpoints and time agrees.

Both directions match the pinned 35-point inner and outer coverage
samples for each stage, including asymmetric rectangle corners.
Five authored Flutter cases and 29 focused native tests passed.
The original golden registrations have not been paired.
