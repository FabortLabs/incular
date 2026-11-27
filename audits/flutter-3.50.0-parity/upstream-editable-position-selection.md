# Selecting editable positions and words

Pinned `rendering/editable_test.dart:583,652,685,717,749` pass.
The mounted native editor uses the pinned Ahem font. Changing the
vertical viewport offset by 10px moves actual emitted glyph origins
up by 10px. The same (0,2) point then chooses byte 5 rather than 0.
A real touch tap on the second line moves to byte 6; long-press and
the adjacent word selection both select 5..9. Tapping and selection
preserve the fixed scroll position.

The raw two-point API preserves a reversed `base=3, extent=1`,
using the first point's affinity and not requesting focus. A
non-finite second point leaves the range untouched.

At the boundary between two leading spaces, the editable previously
selected both spaces on read-only Android. Pinned Flutter assigns the
exact edge to the next visual cluster. The paired Ahem probe verifies
positions at x=9, 10 and 11 all share offset 1, with affinity changing
from upstream to downstream at x=10. This resolves the Android range
to 1..2 and the two iOS cases to 1..6 before a word and a collapsed
caret at 1 in an all-space string. Distinct bidi stops stay distinct.

Three direct native fixtures and 32 adjacent input, handle, bidi and
viewport tests passed.
