# ListView hit testing after scroll

Pinned Flutter 3.50.0 `ab2912035e160d9cad6306df307576da6128ebdb`.
Reference: all five registrations in `widgets/scrollable_list_hit_testing_test.dart`.

The native runtime drives the same 800-by-600 view, item extents, and
touch sequences, through hit testing and the gesture arena. A 280-pixel
drag makes rows 0 through 3 visible in both directions, with the same
item origins. Cached children are not equated with visible rows.
The tap reaches the scrolled row, not its old position; the partially
visible vertical last row does not receive taps at its offscreen center.
Forward and reversed lists honor both main-axis and physical cross-axis
padding, and a tap immediately after a clamped fling reaches the first
item without waiting for a subsequent frame.

All five pinned originals and five native counterparts passed, as did two
adjacent sliver padding drag/anchor cases. This batch backfills behavioral
tests; the existing implementation did not need a repair.
