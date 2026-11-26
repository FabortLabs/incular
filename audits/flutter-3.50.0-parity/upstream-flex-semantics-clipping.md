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

Six focused native Flex tests passed on the final code. The broader
related sliver cache and box clip checks also passed. The upstream
records remain **partial**: they also assert Flutter's structured
debug overflow error summary. The retained accessibility behavior
matches, but Incular does not issue that diagnostic.
