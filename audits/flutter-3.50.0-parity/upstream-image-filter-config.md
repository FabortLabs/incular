# Layout-aware image filter configuration

All nine pinned ImageFilterConfig originals pass. Native checks verify
the context bounds, original direct filter and structured hashes,
exact short and display descriptions, ordered composition, and local
bounds in the resolved blur. The original filter is not rewritten for
different bounds; composed stages resolve in inner-to-outer order.

These are value and resolution contracts, not GPU sampling parity.

The wgpu backdrop renderer now executes unbounded sampled blur. Two
production-shader readbacks verify distinct RGBA samples for all four
tile modes (decal, clamp, repeat, mirror) at a texture edge, including
the downsample input. The run required a real wgpu adapter and used
Mesa's software OpenGL adapter.

Bounded backdrop blur additionally masks source texel centres in
widget-local coordinates before interpolation, preserving rotated
bounds without clipping the output. The final pass divides premultiplied
colour by accumulated alpha (zero alpha stays transparent), following
the pinned engine's Gaussian shader. A production-shader readback
checks exclusion of opaque neighbouring colour, a translucent source,
an empty pixel, and blur reaching beyond the source bounds. This does
not establish exact Flutter kernel, transformed-sigma, large-sigma
resampling, or platform pixel equality. Sampled blur applied to an
isolated child remains unsupported; neither readback adds upstream credit.
