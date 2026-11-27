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
Mesa's software OpenGL adapter. Bounded blur and sampled blur applied
to an isolated child remain unsupported. The readbacks do not verify
Flutter's nominal kernel, large-sigma resampling, or platform pixel
equality and add no upstream-case credit.
