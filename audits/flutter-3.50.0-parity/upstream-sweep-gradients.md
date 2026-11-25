# Flutter sweep gradient parity

Reference: pinned Flutter framework and engine at `ab2912035e160d9cad6306df307576da6128ebdb`.

## Partial sector rasterization

Flutter's public `SweepGradient` selects two angles, not merely a phase for a full turn. The authored `sweep_span_reference_test.dart` draws the gradient to a 64×64 picture and reads RGBA bytes at right, bottom, left, and top. An explicit 45°–135° red-to-blue sector clamps to red at right and blue at left/top; bottom is approximately [131, 0, 123, 255]. Decal makes all but the bottom sample transparent. Rotating decal by 90° leaves only the left sample visible, with the same interior color.

Incular's physical brush now carries both endpoints for explicit sectors and passes the same parameter and rotation through the square, rounded rectangle, and path shaders. The WGPU test uses each production WGSL sampling function with the actual paint packing and reads back pixels (GL/llvmpipe on Linux); it allows ±2 for interior conversion. The legacy physical full-turn constructor keeps its existing phase semantics. This authored probe is separate from the Flutter upstream registration count. Degenerate sectors, arbitrary transforms, and other GPU backends have not been verified by this probe.
