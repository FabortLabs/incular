# Pointer button, slop and affine contracts

Seven pinned `gestures/events_test.dart` registrations pair. The
native tests assert the mouse and stylus side-button indices,
smallest set bit, and exactly-one-button predicate with the same
inputs as Flutter. Rust's portable raw event uses a 32-bit mask; it
reports out-of-range button indices as absent. Dart's wider native
integer mask is not claimed.

Default mouse hit/pan/scale tolerances are 1/2/1 logical pixels;
touch, stylus, inverted stylus, trackpad and unknown use 18/36/18.
A touch-slop override replaces hit slop and doubles for pan without
changing the precise mouse thresholds. The authored Flutter probe
checks a non-default trackpad override alongside the mouse control.

The optional affine point and delta functions now pair the
identity, translation, scale, cached end and absent-transform
examples. Listener and router projections share the same path.
A translation changes the local point but must not enter the
local delta: directly transforming two f32 positions and
subtracting used to erase a one-pixel move at large coordinates.
The authored Dart and Rust cases preserve the move with a
33,554,432 logical-pixel translation without pretending absolute
coordinates have identical f32/f64 precision. The original 3D
perspective case is outside this affine slice.

Two authored Flutter cases and 23 focused native checks passed.
The existing native `pointer_hit_test` case
`an_opaque_region_over_ignored_content_blocks_raw_but_not_retained_hits`
still fails on both the unmodified `00b10697` baseline and this batch
(retained ActionId(1) is absent). Its expectation was not changed or
counted as a pass.

The later [overlapping region and absorber fix](upstream-mouse-region-local-motion.md#overlapping-regions-and-absorber-boundaries)
identifies the off-box absorber and restores both the expected
retained action and the Flutter hover route.

## Default press and contact-motion buttons

The pinned :228 and :233 registrations passed. Synthetic contact presses
and contact moves default to the primary button (bit 1), without changing
the explicit-mask constructor used by native adapters. Native controls
also preserve an explicitly empty button mask and keep that motion in
contact rather than treating it as hover. A new motion initially reports
the same delta in global and local space. All 31 focused native tests passed.

## Copying a global pointer sample

All six pinned `copyWith` registrations at :660, :688, :716, :744,
:772 and :800 passed. Native checks use the six corresponding packet
shapes, retimestamp and reposition them, and verify pointer identity,
contact state, button masks, radii, distance, tilt, obscured state,
synthesized state and delta. The production resampler uses this same
copy path rather than separately writing global and local coordinates.
A routed stylus control preserves measured pressure and explicitly
projects the copied sample into its next target.

The registrations remain partial. Incular uses an optional normalized
force measurement rather than Flutter's non-null default raw pressure,
and a global Rust copy intentionally drops the previous local routing
rather than carrying Flutter's transform wrapper. All 34 focused native
copy, primitive, packet and resampler tests passed.

## Borrowed event projections

The original `transforming events` at :289 passed on the pinned SDK.
The native path now exposes borrowed projections of converted contacts,
scroll and pan/zoom without allocating or replacing the original packet.
At the original translation and scale it reports local position (60, 100)
and local delta (10, 10) for the moving samples, while preserving all
source metadata. A different target projects the original, not the first
projection. Enter/exit samples also preserve metadata via the raw value
path. A separate projective pan control verifies that motion is mapped
from both endpoints. All 35 targeted native tests passed.

The original remains partial: the raw enter/exit value path does not
expose Flutter's identity backlink, normalized force is not Flutter's raw
pressure reading, and retained widget hit testing is still affine.
