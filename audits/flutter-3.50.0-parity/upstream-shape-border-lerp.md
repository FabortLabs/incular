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
