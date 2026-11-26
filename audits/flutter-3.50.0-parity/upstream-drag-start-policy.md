# Drag start origin and source time

Pinned `gestures/drag_test.dart:293` passed. The retained test checks
all four cases with live horizontal and competing vertical recognizers.
A sole drag starts at down for both policies, reports (10,0) rather
than the diagonal raw movement, and preserves the timestamp of a zero
update. A competing drag starts at the 20-pixel threshold crossing.
`Start` uses the crossing position without an update; `Down` uses the
original position with a 20-pixel update. Both use the crossing event's
actual source time, not the paint or frame timestamp.
