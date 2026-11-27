# Layout-aware image filter configuration

All nine pinned ImageFilterConfig originals pass. Native checks verify
the context bounds, original direct filter and structured hashes,
exact short and display descriptions, ordered composition, and local
bounds in the resolved blur. The original filter is not rewritten for
different bounds; composed stages resolve in inner-to-outer order.

These are value and resolution contracts, not GPU sampling parity.
A sampled blur remains explicitly unsupported at the current retained
execution boundary rather than silently dropping its edge mode or
bounds. The wgpu implementation and pixel tests remain open.
