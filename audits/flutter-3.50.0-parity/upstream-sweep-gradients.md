# Flutter sweep gradient parity

Reference: pinned Flutter framework and engine at `ab2912035e160d9cad6306df307576da6128ebdb`.

## Partial sector rasterization

Flutter's public `SweepGradient` selects two angles, not merely a phase for a full turn. The authored `sweep_span_reference_test.dart` draws the gradient to a 64×64 picture and reads RGBA bytes at right, bottom, left, and top. An explicit 45°–135° red-to-blue sector clamps to red at right and blue at left/top; bottom is approximately [131, 0, 123, 255]. Decal makes all but the bottom sample transparent. Rotating decal by 90° leaves only the left sample visible, with the same interior color.

Incular's physical brush now carries both endpoints for explicit sectors and passes the same parameter and rotation through the square, rounded rectangle, and path shaders. The WGPU test uses each production WGSL sampling function with the actual paint packing and reads back pixels (GL/llvmpipe on Linux); it allows ±2 for interior conversion. The legacy physical full-turn constructor keeps its existing phase semantics. This authored probe is separate from the Flutter upstream registration count. Degenerate sectors, arbitrary transforms, and other GPU backends have not been verified by this probe.

## Box-relative descriptor

Pinned `packages/flutter/test/painting/gradient_test.dart` registrations
:432, :766, :800, :830, :848, :875, :895, :916, :934, and :954 ran by
exact name. `flutter350_sweep_gradients.rs` ports their stop offsets and
colors, box center, angular endpoints, midpoint transform selection,
scale, opacity replacement, and recolor. The Rust fixture checks angles
and stops exactly and allows 8-bit quantization of the interpolated
channels. The :794 value and absent/absent cases pass; Dart heap identity
has no equivalent guarantee for a returned Rust value, so that row remains
partial.

A second native fixture resolves logical centers against a non-origin
rectangle and verifies the actual retained paint command during a
radial-to-sweep animation. Linear, radial, and sweep box descriptors
all use the retained shader cache and the same cross-kind fade rule.
