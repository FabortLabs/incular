# Raw EditableText visual defaults

Pinned Flutter 3.50.0 `widgets/editable_text_test.dart:1395` passed.
The native mounted control keeps a genuine selected range and focus
while inspecting the actual draw commands. A raw editor paints no
selection highlight by default, including under an orange
DefaultSelectionStyle, and it does not manufacture a focus border.
Supplying an explicit selection color paints it; clearing that color
removes the paint without moving focus or changing the selected range.

The styled Incular controls retain their existing highlight by
requesting it explicitly. Ordinary and spell-check selection colors
are independent; the spell-check toolbar checks still pass.

The control checking absence of a focus border is authored and does
not add a second upstream registration.

The broader check reproduced two stale alignment expectations on
the prior commit, independently of the color change. The Flutter
source reserves the 2 px default cursor plus 1 px of caret gap; the
native single-line LTR tests now account for the existing reservation
instead of comparing with the whole area. RTL fixture expectations
outside that native margin path are unchanged in this batch.
All 48 focused native controls pass, including color, spell-check,
and geometry. No motion or platform text policy changed here.

## Logical selection on focus and blur

The pinned Linux reference at `widgets/editable_text_test.dart:1344`
and the native port both pass. The initially unfocused editor keeps
the (5, 7) range and upstream affinity, then keeps them both while
focused and after blur. The comparison uses the original's native
branch; its conditional web select-all path is not newly verified here.
