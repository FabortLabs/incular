# Transformed paint outside a destination-reading blend

Transparent surfaces with a destination-reading effect use a cropped
compositor texture. Incular computed its scope with a translation-only
stack, even though draw lowering used the full affine. In a 64×64
scene with a disjoint Multiply layer at (4, 4), the old scope was only
23×7 pixels and cut off a white 2× transformed hairline at y=16.

The scope now composes nested affines in draw order. Primitive bounds are
projected to world coordinates, positive path strokes expand by their
local outline reach, and hairlines expand by their actual device-space
reach after projection. Antialiasing allowance follows the corresponding
shader, and retained effect bounds already in world space are not
transformed a second time.

The focused Rust regression proves the failure before the fix and checks
the tight target at 1× and 2× device scales. The real wgpu path shader
readback on llvmpipe GL verifies the same alpha coverage for full and
cropped target geometry, including the cropped origin offset. That
readback exercises the path pass, not a native window or a combined
Multiply presentation pass. The independent destination blend shader and
effect sampling checks also passed (14 native checks in total).

The pinned authored Flutter probe renders a transparent picture and
confirms that a disjoint Multiply group changes its own pixel from red to
black, while leaving the distant transformed zero-width line unchanged.
It is supplementary and does not increase the upstream registration count.
