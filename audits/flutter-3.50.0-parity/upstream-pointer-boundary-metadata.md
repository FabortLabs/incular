# Pointer pressure at mouse annotations

The pinned `gestures/events_test.dart` registrations at :116, :143,
:170 and :197 passed. Flutter's enter/exit constructors synthesize no
contact pressure, even when made from a pressed move. They retain
the pointer and device IDs, motion, buttons, contact state, and other
applicable metadata. The native constructors now do the same for
the modeled portable fields.

The authored `pressed_pointer_boundary_metadata_test.dart` sends an
actual stylus contact from outside to inside and back outside a
rendered mouse region. It preserves pointer 34, device 10, the
primary button and down state. Local positions are (30,50) and
(120,50); local deltas are (70,0) and (90,0). Flutter reports zero
pressure, while the native baseline leaked 0.55 on entry. Incular
now reports zero for both transitions and keeps the pen orientation
packet. The hover and pressed constructor controls also keep the
original synthesized flag and transformed coordinates.

Both focused and sixteen neighboring native tests passed. The four
original cases remain partial: portable samples do not model
Flutter's obscured flag, pressure ranges, size, radii, or distance
fields. The authored end-to-end case adds no original credit.

## The pointer plane of a paint matrix

The exact `gestures/events_test.dart:239` registration passed.
The pointer transform preserves identity and replaces only the
third row and column of a paint matrix with the identity z axis.
A uniform 3× paint scale therefore becomes a 3× planar pointer
scale without changing the input matrix.

The authored `projective_pointer_local_delta_test.dart` additionally
checks a nonconstant planar denominator. Pointer delta is the
difference between the two projected positions, not a transformed
vector at the origin. The API preserves the input sample, contact
states, and global coordinates. Native results match the reference
to float precision. Nine primitive, nine router and ten retained
transform tests passed. Incular's retained widget transforms are
still affine and reject true perspective; the raw pointer math
alone does not add a projective paint path.

## Full portable contact metadata

The four original constructors at :116, :143, :170 and :197 are
now fully paired. Their exact hover and pressed input fields reach
enter/exit without losing obscured state, source pressure bounds,
distance, size, radii, orientation or tilt. Synthesized state and
zero boundary pressure are also asserted. A real Runtime
MouseRegion receives that same packet when a stylus crosses its
bounds. Source pressure limits are distinct from normalized force,
so force-gesture behavior is unchanged. The packet remains a
plain copyable value; focused input allocation guards pass.

The four pinned originals and 32 focused Rust checks passed.
The shared mobile embedder type-checks. The extra Linux desktop
adapter test could not build offline because `ab_glyph 0.2.32`
is not cached, and specific OS acquisition of these optional
measurements is not claimed. A producer that does not have these
readings retains Flutter's payload defaults instead.
