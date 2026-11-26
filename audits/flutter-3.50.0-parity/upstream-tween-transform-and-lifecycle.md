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

## Native rounded save-layer clips

Retained, axis-aligned rounded clips using AntiAliasWithSaveLayer now
isolate the child draws in a clipped-size cached texture, then apply
smooth edge coverage once during compositing. The native 40 px
overlap fixture changed from no partially covered corner pixels to 19,
with no red fringe from the preceding paint. Its exterior and interior
control pixels are unchanged. Independent scene coverage tests still
exercise no-clip, hard-edge and the ordinary AA mode.

This covers the analytic retained rounded rectangle path. Ordinary
non-isolated AntiAlias and smooth arbitrary-path fallback still require
their own renderer implementation. The full three-mode target remains
red for ordinary AntiAlias; the save-layer target checks its behavior
separately. The comparison is against Flutter's edge/fringe assertions,
not an exact per-pixel match of the two renderers.

## Native ordinary rounded antialiasing

The Windows wgpu three-mode fixture now passes. Ordinary AntiAlias
applies fractional coverage per draw, so overlapping paint can produce
the same earlier-color fringe as Flutter. AntiAliasWithSaveLayer still
applies coverage once after composition and removes that fringe.
HardEdge and Clip.none retain their independent controls. The saved-layer
and invisible-border native regressions also pass.

This is the analytic axis-aligned rounded-rectangle path. General-path
fallbacks and passes with destination-reading effects still use binary
containment. The fixture compares coverage and compositing behavior rather
than requiring identical sample colors across renderers.

### Transformed hard Canvas clips

The raw Canvas probe translates by (4,4), scales by 2, and clips to
either a rounded rectangle or a polygon. Before the fix Windows wgpu
lost the white interior at (34,16) in both scaled cases because the
scissor only reflected translation. Now the scissor encloses the same
affine-transformed shape as the stencil. All four sampled positions
match the pinned Flutter result with and without scaling, including
the cut corner, the exterior, and the no-clip control. The rounded
save-layer and native path-fidelity checks also pass. Transformed
antialiased path coverage is a separate limitation.

### Default Canvas rounded coverage and widget clip originals

The raw Canvas rounded clip now defaults to antialiasing, and
positive uniform scaling preserves the same physical edge coverage.
The pinned Flutter and Windows wgpu probes both have fractional
corner pixels at 1× and 2×, matching their own scale controls;
explicit hard clips still have none. The save-layer and native
path-fidelity controls remain green.

Three `widgets/clip_test.dart` originals were run and passed.
For :101, the Rust builder's actual emitted rounded clip has zero
radii and AntiAlias coverage. For :933 and :964, the equivalent
centered Placeholder and rounded/oval clip measure zero and finish
painting at a zero viewport size. All 21 focused retained clip
tests pass. These original cases are separate from the authored
Canvas readback. Rotated and general-path antialiasing remain open.

### Oval coverage and retained clip-mode transitions

The pinned :107 and :123 originals verify default AntiAlias,
HardEdge, and None on one retained render object. The Rust ports
observe the same transitions on a stable render ID using a painted
child to expose the actual output mode. They also exercise save-layer
metadata and confirm a rotated shape keeps its cached fallback path.

Before this repair Windows wgpu rendered a default noncircular
ClipOval with no partial edge pixels. Ordinary antialiasing now
uses the ellipse in physical pixel coordinates, and the pinned
and Windows fixtures both distinguish it from the binary hard
control. All 28 retained clip and input checks plus the native
shape controls pass. Saved-layer oval coverage and arbitrary-path
antialiasing are separate from this render verification.

### Elliptical save-layer compositing

The pinned Flutter probe paints red and then white across the same
40 by 28 ellipse. Ordinary AntiAlias exposes some red at the partially
covered edge, but AntiAliasWithSaveLayer keeps fractional edge pixels
and removes that red fringe. The Windows wgpu before run had 16 partial
pixels (all with a red fringe) in ordinary AntiAlias and zero in saved
mode. After the fix it passes the same smoothness and compositing
assertions as Flutter, with unchanged interior and exterior controls.
Rounded save-layer, color-matrix/opacity compositing, and seven native
path/draw controls also pass. This is an authored probe, not an extra
original registration; rotated and general paths remain open.

### Supported application lifecycle transitions

The original AppLifecycleListener :91 and :124 cases passed again.
Incular now synthesizes the observable intermediate states when a host
event jumps from starting to inactive/suspended, active to suspended,
or suspended to active. An already explicit sequence is not duplicated.
The low-level direct transition API still applies exactly one requested
state. Per-window focus events do not synthesize app transitions.

The four gap assertions failed before the change. These cases remain
partial matches: Flutter's Hidden/show/restart and reversible Detached
are not part of the Incular lifecycle projection. Stopping and
Terminated keep their own shutdown meaning.

## Nullable borders and typed tween values

All nine selected pinned `animation/tween_test.dart` originals passed.
Six are fully paired: chaining (:99), nullable BorderTween (:128),
SizeTween (:139), RectTween (:151), ConstantTween (:182), and nullable
BorderRadiusTween (:218). Tests check numeric values, retained absence,
and one-line diagnostics where those originals require them.
A missing border endpoint scales its present sides instead of blending
their color with a default border. Raw interpolation can yield a zero
border or radius; transform preserves a genuinely absent endpoint.
The optional types don't replace existing nonnullable Rust aliases.

Animate and drive (:108 and :118) both update the driven value when
the controller changes, but are partial because the original separately
asserts Dart's animation details string. Matrix4Tween (:159) is partial:
the XY translation and scale projection and the rotation around Z match,
but Incular's affine transform does not represent the Z scale of 5 or
its interpolated value of 3. Six focused Rust targets passed.

## Driven tween diagnostics

Pinned `animation/tween_test.dart:108,118` passed. Both attachment
paths evaluate the live controller, giving 0.30 initially and 0.40
after the original 0.50 assignment. The complete description and its
details are each a single line, reflecting the concrete transform
and live value. The direct typed controller API is
`drive_animatable`; Incular's existing timestamped `drive` remains
unchanged. Also repaired stale test moves after TweenValue became
owned. All 45 focused tween, controller and animation-graph tests passed.
