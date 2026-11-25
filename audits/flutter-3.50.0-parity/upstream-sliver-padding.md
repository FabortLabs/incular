# SliverPadding geometry

## Padding only, signed overlap, and leading edges

Six selected originals in pinned `widgets/slivers_padding_test.dart`
and their Rust comparisons passed. A childless padding takes scroll
extent, moves the following sliver to (0,200) without a dummy mounted
child, and reports paint extent 200 then 150 after a jump to 50
(:292, :314). Padding before a fill-remaining child reduces its
600-pixel viewport height to 570 (:557). Direct render probes compare
positive overlap reduced by visible leading padding (:589), negative
overlap passed through (:618), and preservation of the child's paint
origin of 10 (:646).

The authored asymmetric padding probe checks all four axis directions
and their actual leading edges. The first Rust run sent 20 pixels
(top) to an upward-growing child where Flutter sent 40 (bottom). Both
axes now resolve their leading edge correctly. A separate Rust control
shows an actual child offset correction is forwarded unchanged with
no extra padding geometry. Four focused Rust targets passed (49 tests).
