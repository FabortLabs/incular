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
