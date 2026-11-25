# AnimatedSize, SizedOverflowBox, controller runs, and navigation observers

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

- All five AnimatedSize originals pass in Flutter. Stable size
  grow and shrink both match all geometry samples (:20), and
  both clip during growth. Flutter does not clip at the equivalent
  shrink midpoint because the smaller child fits; Incular adds a
  rectangular clip. This is a paint command difference, not a
  claim that in-bounds pixels are wrong. Incular wraps the custom
  animated layout in ClipRect regardless of child overflow.
  onEnd fires once per completed run (:83), tight dimensions
  clamp (:129), and a text child at zero size works (:491).
  The 20-frame idle check (:276) matches public size/ticker behavior
  but not Flutter's private phase enum. See
  `flutter350_animated_size_upstream_parity`.
- Three original SizedOverflowBox cases pass. Absolute RTL
  alignment matches (:115); explicit resolution of directional
  alignment (:142) and a zero-area adapter (:169) are partial.
  The separate pinned test proves incoming loose constraints are
  passed to a 200×150 child inside a 100×100 wrapper on Flutter.
  Incular incorrectly caps the child at 100×100. The Dart and Rust
  tests compare both exact geometry rectangles. See
  `flutter_sized_overflow_geometry_parity` and
  `sized_overflow_child_constraints_test.dart`.
  See [the retained repair](upstream-sized-overflow-retained.md) for
  current results.
- Four original controller runs pass on Flutter. Inclusive velocity
  observations (:449) and both never-done/reverse simulation status
  traces (:1087, :1102) match. Timing at both duration permutations
  (:184) reaches the expected boundaries, but 9 of 16 interpolation
  values exceed Flutter's 1e-10 tolerance at the Rust f32 boundary;
  maximum error about 2.385e-8. This precision adaptation is
  partial, not evidence of a wrong duration. See
  `flutter350_upstream_animation_run_semantics`.
- Four exact Navigator observer registrations pass on Flutter.
  Rust matches push/pop, attachment/removal of the observer token,
  and replacement (:779, :848, :1295) through direct navigation
  and adjacent active-route events, not identical named-route
  UI or callback type. On explicit route removal (:1677), Flutter
  dispatches a distinct didRemove notification; Incular reports
  Popped. Stack advancement still works. See
  `flutter_navigator_observer_upstream_parity`.

## Current AnimatedSize clipping

Pinned :20 and the native retained counterpart now agree at all four size
checkpoints. The 150px box clips while its child grows to 200px, but does
not add a clip while shrinking around a child already sized at 100px.
The retained clip stage is reused when overflow returns and bypassed when
the child fits; a separate painted wrapper is no longer needed.
The authored `animated_size_clip_none_test.dart` also passes on both
runtimes: the deliberately overflowing child is painted without a clip.
Seventeen targeted AnimatedSize and adjacent AnimatedCrossFade tests passed.
These are clip-command comparisons, not native GPU captures.
