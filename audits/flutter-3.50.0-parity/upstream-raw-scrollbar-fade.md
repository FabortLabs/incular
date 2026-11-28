# RawScrollbar fade and real pointer events

Reference: Flutter 3.50.0 at `ab2912035e160d9cad6306df307576da6128ebdb`.

Mounted native runtime tests drive a 20-pixel touch scroll in a 4,000-pixel
view. The 90-pixel thumb moves to y=3 and stays fully opaque while held,
including after six seconds. On release it stays at alpha 102 for 600 ms,
reaches alpha 79 halfway through the 300 ms reversed animation, then
disappears. Re-entering the track during that fade restores alpha 102 at
the original coordinates. Hover also works before the first scroll.

An authored Flutter test and a native red/green test cover a previously
missed transition: a programmatic scroll while the mouse is over the
track starts a new timer. A subsequent genuine hover inside the same
track cancels that timer without requiring exit and re-entry. Once the
pointer leaves, the timer and exact opacity curve resume. Incular had
ignored the second hover because the hit scrollbar did not change.

All four selected Flutter originals, the authored Flutter test, and 10
focused native tests passed. The original at line 728 constructs a hover
event but does not dispatch it; it is recorded as partial rather than
counted as proof of real hover routing. The authored test dispatches
actual pointer events and verifies their effect.
