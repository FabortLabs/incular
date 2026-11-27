# Sliver clip content, overlap, and updates

Twelve pinned original registrations passed across
`widgets/sliver_clip_rect_test.dart` and `sliver_clip_rrect_test.dart`.
The native clips mount real text and verify glyph draw work. Retained
updates switch rectangular hard edge to antialiasing, and rounded
antialiasing to hard edge with a 10→20 radius change, without replacing
the child. Disabling the clip removes its drawing command without
losing the child or its paint.

For the rounded clip, paired fixtures verify the full local extent
for hits, both sides of the 50-pixel overlap in forward and reverse
scrolling, the corner and straight-cut hit probes, and the custom
30-pixel inset at offsets 0, 100 and 385. The native runtime sends
real down/up events at (10,90): the preserved shape receives the tap
only while the pinned header ignores pointers. Both the clip and
retained input fixtures passed (25 native tests).
