# RenderTransform view and local coordinates

All nine original registrations in `rendering/transform_test.dart`
passed on the pinned Flutter runner. The mounted native tests pass
the original 100×100 constraints and transform matrices through the
same render path used for widget geometry. They check all four points
both directions for identity, translation, and centered 180° rotation,
with and without the 20-pixel internal left padding.

The three registrations whose names say perspective actually pass
rotation matrices with a constant homogeneous denominator. Projecting
their local XY plane is affine. The native tests unproject both sides
of the center at a 45° tilt, project a nearly edge-on box to the center
line, and return zero when the projection is exactly singular. A
separate control rejects true variable-denominator perspective;
this is not retained 3D depth, general perspective, or a change to
the wgpu renderer.

All 14 targeted native render-coordinate tests passed.
