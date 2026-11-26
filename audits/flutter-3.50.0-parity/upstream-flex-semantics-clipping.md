# Overflowed Flex accessibility bounds

Pinned `widgets/semantics_clipping_test.dart:11` and `:62` passed.
A 100px Flex contains three 75px semantic text boxes. Incular had
reported all three at full width, including the fully invisible box.
It now projects the active overflow clip into world coordinates.
The first label keeps its 75px rectangle, the second is clipped to
25px, and the third is not published as a separate node. If the last
two form one MergeSemantics boundary, the visible 25px node retains
the complete `2\n3` label.

The separate retained resize comparison found another fault: moving
the clip from 100px to 130px left the second semantic node at 25px.
The layout now invalidates its semantic subtree when the active clip
changes size or switches on/off, even if children did not move. The
pinned authored probe and Rust tests agree at 100, 130, 150, and 225px,
on a return to overflow, and across Clip.none. An edge-touching child
is excluded and an unaffected visible semantic identity is retained.

Six focused native Flex tests passed for the initial geometry change.
The broader related sliver cache and box clip checks also passed.

## Debug overflow summary

Both originals now pair, including their nonfatal debug overflow
summary. Incular's typed diagnostic carries the overflowing render,
axis, and amount, and formats the expected 125-pixel right overflow
for both semantics scenarios. The native collector only reports once
per painted render object, not from an offstage or zero-size layout.
Release builds do not generate these diagnostics, and the collector
does not alter paint output or add an on-screen indicator.

The pinned authored probe confirms the bottom summary when a vertical
Flex is first shown, no repeat report after a resize, and the exact
right-edge formatting at 125, 12.5, and 0.3125 pixels even under RTL.
Eleven focused Rust tests passed, covering native summaries, first
paint, offstage, repeated resizes, and actual clipped semantics.
