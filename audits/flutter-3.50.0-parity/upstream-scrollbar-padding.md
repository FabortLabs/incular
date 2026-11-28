# RawScrollbar track padding, margins, and overscroll

Reference: Flutter 3.50.0 `packages/flutter/test/widgets/scrollbar_test.dart`.
The pinned original tests ran on Linux; the matching Incular probes in
`crates/incular-widgets/tests/flutter350_raw_scrollbar_padding.rs` exercised
mounted display lists and the real scroll controller.

| Reference lines | Verified behavior |
| --- | --- |
| 173, 211 | Signed main-axis margins affect thumb placement; cross-axis margins and ambient text direction place the overlay on the correct edge. |
| 355, 379, 411, 443 | Uneven insets place the track and thumb in all four axis directions. |
| 476 | The thumb keeps its normal extent while scrolling and shrinks toward the overscroll floor outside the range. |
| 2602, 2633 | Default padding comes from the nearest MediaQuery; explicit padding replaces it rather than adding to it. |
| 3626 | Directional padding resolves to the correct physical edge in LTR and RTL. |

Focused native run: 5 padding/overscroll probes and 8 adjacent paint
regressions passed. All 10 selected upstream originals passed. The native
overscroll probe uses large finite offsets to saturate at the floor rather
than sending unsupported non-finite controller offsets. Renderer unchanged.

## Thumb minimum inheritance and viewport cap

Flutter's overscroll floor inherits the configured normal minimum when no
explicit floor is supplied. Incular's paired `min_lengths` builder resolves
both values and checks their ordering in debug builds. Setting only the
normal minimum also updates the effective overscroll floor.

The mounted input tests measure the 21-pixel normal thumb and drive a real
touch overscroll through the bouncing viewport, checking both the explicit
8-pixel floor and the inherited 36-pixel floor. The padding test samples
controller offsets in order, including overscroll, to verify the thumb never
moves backward and that an infinite minimum is capped at the viewport.

All three pinned Flutter originals (522, 1749, 1884), one authored Flutter
inheritance probe, and 13 focused native tests passed. The renderer was not
changed.
