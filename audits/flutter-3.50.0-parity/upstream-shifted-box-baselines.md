# Align, Padding and fractional boxes forward real baselines

The pinned Flutter originals and three supplemental geometry probes
pass. Before the fix, each retained Incular wrapper placed its child
correctly but reported no baseline, so wrapping text in otherwise
pure layout widgets changed baseline alignment higher in the tree.

| Wrapper | Measured result in both engines |
| --- | --- |
| Align | A 100×100 child carrying a 40px baseline inside a 200×200 box reports 40, 90 and 140px at top-left, center and bottom-right. A retained update to a baselineless child clears it without changing placement. |
| Padding | Insets (7,13,15,11) yield a 122×124 wrapper, place the child at (7,13), and expose the 53px baseline. Removing the metric keeps the geometry and reports no baseline. |
| FractionallySizedBox | Factors of 0.5 under a 200×200 maximum and a 160px minimum height constrain the child to 100×100, place it at (0,30), and expose the 70px baseline. |

The selected `rendering/shifted_box_test.dart` registrations
:57, :71, :84, :183 and :200 remain partial because they invoke the
direct dry-baseline API. Incular verifies the laid-out metrics and
their child offsets, including the retained removal case.

The separate `rendering/positioned_box_test.dart` cases :13 and :25
assert only the actual box sizes. Their Flutter originals and retained
Rust checks agree on 200×200 when factors are absent and on
100×200 → 200×100 → 100×100 when changing the specified axes. These
are full behavioral comparisons, even though the Rust tests use its
public Widget façade. :50 verifies actual layout at
100×0 → 50×50 → 200×200; the direct intrinsic methods remain open.
