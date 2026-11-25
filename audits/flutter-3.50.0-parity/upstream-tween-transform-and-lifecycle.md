# Tweens, transform geometry, state and host lifecycle, and raster clipping

Pinned Flutter 3.50.0 `ab2912035e`; unchanged Incular production `c647cb85`.

- Original IntTween, ReverseTween, ColorTween, StepTween and
  CurveTween registrations all match (:145, :191, :197, :205,
  :211). The color check uses Flutter's original 0.004 normalized
  sRGB component tolerance. It is not general color-space parity.
  See `flutter350_upstream_tween_values`.
- Off-center scale origin and origin-plus-alignment (:19, :165)
  match the original global touch hit/miss checks. A quarter-turn
  Row (:9) swaps size to 65×175 and hits both child positions.
  Zero area (:53) uses an empty child because Rust requires one;
  partial. No pixel comparisons for these transforms. See
  `flutter_transform_pivot_geometry_parity`.
- Three original stateful/framework cases passed. Replacing an
  outgoing subtree does not rebuild it (:60). The Rust equivalents
  of resync (:47) and unmount (:1776) preserve keyed element/render
  identity and release the closure/invalidate stale identities,
  but do not reproduce Dart's State lifecycle hooks and property
  errors. See `flutter_stateful_lifecycle_upstream_parity`.
- Two original lifecycle observer cases passed. Through the
  common startup, active, inactive and suspended states (partial),
  Incular omits Flutter's synthesized active-before-inactive
  on startup and inactive-before-suspended from active (:91,
  :124). Explicitly supplied intermediate states are delivered
  and identical state updates ignored. Flutter-only Hidden
  and reversible Detached are not claimed. See
  `flutter350_lifecycle_observer_parity`.
- An authored pixel comparison independently tests ClipRRect on
  Windows wgpu. For a 40×40 circular-radius-12 clip, Flutter
  default has partial corner coverage, HardEdge does not; a
  red fringe under opaque white is eliminated by using the
  saved compositing layer while maintaining smooth coverage.
  Incular produces binary hard-edge coverage for all three
  modes. Clip.none and inside/outside controls pass. This is
  a shared clip mode pipeline gap, not an independently
  isolated saved-layer color error. See
  `flutter350_clip_rrect_raster_modes` and
  `clip_rrect_raster_modes_test.dart`. No static count added
  for the authored pair.

## Exact nullable endpoints and integer precision

The C21 and C22 authored regressions now pass. An optional tween's
`evaluate` returns its original endpoint, including None, at exactly
zero or one. Its direct `lerp` retains the linear interpolation contract.
Integer tweens interpolate in double precision before rounding, so the
midpoint from 16,777,217 to 16,777,219 is 16,777,218. Using a wide
intermediate also avoids overflowing the difference of two signed
32-bit endpoints. All 19 focused Rust controller, physics and tween
regressions in the three named targets passed.

## Independent transform hit geometry

`Transform::transform_hit_tests(false)` now leaves the hit target at
its layout position and reports coordinates in that space. An explicit
origin and alignment form one pivot instead of multiple nested
translations. Positioned contributes its offset without adding a
phantom hit boundary before its transformed child.

The authored pinned Flutter and Rust retained fixtures keep the same
child and visual box (87,51,80,60) across the flag change. With hits
transformed, only the painted-only point arrives, locally (16.5,13.5);
without hit transformation, only the layout-only point arrives, locally
(5,4). The geometry check is not a GPU raster assertion. All 39 focused
transform, follower, pointer and stack tests passed.

## Rounded clip coverage in retained display lists

The renderer-neutral commands now retain HardEdge, AntiAlias, or
AntiAliasWithSaveLayer. Updating the mode on a retained ClipRRect does
not remeasure the child or replace its shape; a rotated clip carries the
same mode through its cached fallback path. Clip.none still emits no clip.
The focused renderer and widget suite passed all 23 selected checks.

This is coverage state propagation, not a raster fix. The pinned Flutter
pixel controls still pass and the Windows wgpu raster target independently
reproduced the hard-edge fallback for both antialiased modes.
