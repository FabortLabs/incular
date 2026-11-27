# GridPaper foreground and retained cache

Both original `widgets/grid_paper_test.dart` registrations passed on the
pinned SDK (10 and 18). The Rust fixtures paint a real grid before
comparing the identity of the published display buffer across identical
rebuilds. Changing the interval invalidates that buffer and changes the
line count. Mounting in a zero-area box does not crash or expand it.
Before this change `GridPaper` was a pass-through widget with no paint.

The two authored `grid_paper_stroke_classes_test.dart` cases compare raw
Flutter raster pixels against independent explicit line strokes at 4×.
They pin the default ARGB 7FC3E8F3 appearance and a custom color and
3-by-2 division count over black. The native fixtures verify the ordered
paint commands, including both vertical and horizontal end lines; widths
1, 0.5, and 0.25 logical pixels; the exact color; and that the strokes are
above the child. The pinned algorithm identifies the medium lines by
`interval / subdivisions`. A small native control preserves a fractional
interval, rather than rounding it up to one logical pixel.

The foreground painter itself never takes a hit and does not disable
the child. The retained paint buffer is reused when its geometry and
parameters do not change. Incular deliberately omits an invalid or
excessively dense grid (more than 16,384 strokes) without hiding the
child, so malformed values cannot run an unbounded paint loop. The
authored probes and resource guard add no original registrations.
Native verification is command-based, not wgpu pixel readback.
