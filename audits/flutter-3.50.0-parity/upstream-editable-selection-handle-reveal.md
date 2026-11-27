# Caret and selection handle reveal in a sliver group

The pinned original at `widgets/editable_text_test.dart:18201` passed.
The native fixture places a 44-pixel editor after a 600-pixel sliver and
before a 500-pixel sliver in a retained SliverMainAxisGroup. The field is
laid out at logical offset 600 and becomes focused. The viewport reveals
the caret bottom plus the actual handle clearance, rather than assuming
the field's ordinary scroll padding is sufficient.

The authored pinned `editable_handle_clearance_in_sliver_test.dart`
compares four configurations at 800×600 and DPR 1. Without selection
controls, bottom clearance is 20. A zero-size compact handle still needs
24 from its endpoint for the minimum 48-pixel interactive box. The
standard 22-pixel handle with an anchor 4 pixels above its box needs 39,
and a custom 60-pixel handle with a 6-pixel anchor needs 54. All four
native scroll offsets match the same bounds formula. The compact native
hit test also accepts a point 23 pixels below its tip and rejects one at
25 pixels. The builder controls appearance; this adds no design-system
dependencies or renderer changes. Two focused and fourteen adjacent
native tests passed.
