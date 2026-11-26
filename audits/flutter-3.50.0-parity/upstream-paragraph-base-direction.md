# Retained paragraph direction and tooltips

Pinned `widgets/raw_tooltip_test.dart:462` passes. Real long presses
open the same tooltip fixture under RTL, then LTR. Each mounted
overlay paragraph uses that ambient direction and has real shaped
text; the first tooltip expires before the second opens.

The audit uncovered a wider bug. Text resolved Directionality for
physical alignment but did not send it to the paragraph shaper.
Hebrew followed by Latin therefore stayed in RTL run order after an
ambient switch to LTR, even with the same fixed left alignment.
The direction is now a retained render property and layout/cache
input. A direction-only change reshapes the existing render object,
rather than being treated as paint-only. An explicit Text direction
wins over the ambient value.

The authored Flutter and native probes verify visual selection
ranges on the unchanged mixed text across both directions and an
explicit override. The authored probe adds no upstream registration.
All 26 selected native checks passed, including the mounted tooltip,
rich text, inherited context and text layout regressions.
