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

## Retained list correction under a growing leading row

Pinned :505 and the authored `sliver_padding_correction_reference_test.dart`
pass. The authored version uses controller jumps instead of drag
recognition to isolate the layout; both reference paths place row 2
at (0,100) before growth. Returning toward the start after row 0
doubles in height places row 0 at y=-200 in Flutter but y=0 in
Incular. The dedicated red `flutter350_sliver_padding_correction`
target documents this partial mismatch.

The direct padding control proves it forwards a correction already
emitted by its child. The variable-list or viewport path must still
produce, apply, and lay out the correction in the same frame. Passing
the direct wrapper check does not close the retained regression.
