# Flutter 3.50 aligned linear gradient interpolation

Pinned `painting/gradient_test.dart:131,151,183,211,229` passed. The
paired Rust fixtures reproduce opacity-only scale from an absent
endpoint, endpoint alignment interpolation, explicit and implied
stops, and differing stop counts. Each result has the original
geometry and stop positions and colors within the reference's
8-bit channel precision. Existing discrete tile mode selection
switches at one half as Flutter does.

The interpolation uses the sorted union of stop positions with
straight color channels and quantizes only once. It does not alter
the renderer's separate premultiplied spatial sampling. The prior
animated box decoration switched its whole aligned shader at half;
the interpolated descriptor now reaches actual display-list paint.
The authored pinned probe verifies mixed physical/logical alignment
for an animated box, both LTR and RTL, and exact quantized colors
for the three-stop example.

Raw arbitrary brushes, gradient transforms and the radial/sweep
descriptor matrices are separate work. The Rust widget devtools
feature also compiles with mixed alignments.

## Shader rotation, opacity and recoloring

Pinned :256, :290, :335, :358, :382 and :456 passed. A shader
rotation is compared by value and selected discretely at half.
Retained brush resolution rotates the interpolated endpoints about
the actual paint bounds, including their translated origin. Opacity
replacement assigns the same alpha to every stop; scale multiplies
the old alpha. Recoloring replaces RGB and alpha while retaining the
explicit stop offsets, geometry, tile mode, and rotation. The six
focused Rust cases passed. Arbitrary non-rotation transform matrices
are not represented by this descriptor.

## Absolute brush transitions at decoration paint

A new authored Flutter comparison exercises interpolated linear geometry
and a merged three-stop list, true focal radial radii/centers, sweep
angles, discrete tiling, and the transparent midpoint between different
shader families. Native BoxDecoration and ShapeDecoration now pass the
interpolated absolute brush to the actual display list, not one endpoint
selected at half. The quarter-step grayscale stop is 96/255. A missing
shader scales opacity, and compatible native implicit/explicit sweep
branches remain continuous near their endpoints.

The three authored Flutter tests and eleven focused Rust tests passed.
These authored comparisons do not change the pinned registration count.
Raw-to-box-relative coordinate changes remain separate. Native readback
from a WGPU device was not available in this environment.
