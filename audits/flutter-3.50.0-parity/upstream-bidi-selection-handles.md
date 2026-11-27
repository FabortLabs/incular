# Bidirectional selection handles

All six Android expansions of pinned
`widgets/text_selection_test.dart:2160` and the iOS case at :2212
pass. The native fixture mounts focused editors and checks their actual
visible handle state. On Android, the first and last visual selection
boxes choose the artwork. Arabic in an LTR field uses Right and Left.
A mixed Arabic/English selection can use Left for both handles, or Right
for both when the field direction changes. The two iOS handles instead
use the field direction without changing the anchors or the shaped
direction at either endpoint.

Incular formerly reported LTR for every endpoint. Drag input also
used the artwork to pick which selection boundary to move, so simply
swapping the artwork would move the wrong side. The native hit path
now tracks logical start/end separately. The authored pinned Flutter
and native Hebrew tests drag the right-looking start handle, move
the start to offset zero, and leave the end at character seven.
The range endpoint lookup uses retained clusters and does not allocate
boxes for the whole selected document. All three direct and 19 adjacent
native tests passed, including ordinary left-to-right touch drags,
collapsed handles, composing text, and sliver reveal.
