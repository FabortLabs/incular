# Multi-contact drag strategies

Pinned `gestures/drag_test.dart:497,602,717,852,1017,1182` passed.
The native tests drive a real mono-drag. For the first three
registrations they route separate horizontal and vertical contenders
through the actual managed arena and match callback order. The sum
policy delivers moves from both accepted contacts. The default and
explicit latest-pointer policies silence the older contact until
the active one leaves. Fallback uses the first *accepted* contact,
not the first registered one.

The axis boundary tests use four simultaneous contacts, moving
them in opposite directions within two system frames. Horizontal
and vertical drags match the ten expected amounts, including zero
for contacts that catch up to a boundary. The free pan matches
the average across the accepted contact count, including ±2.5
pixel second-frame updates. A supplementary test verifies that
the crossing motion participates in its original frame's boundary
even when the Start policy does not emit an update for that motion.

These comparisons are at the low-level recognizer boundary, like
the upstream tests. The existing scroll configuration uses the
same strategy type. Applying inherited strategy settings to every
retained scroll input path is separate work.

All 22 focused recognizer, device and scroll configuration tests passed.
