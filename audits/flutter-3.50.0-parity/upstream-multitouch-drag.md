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

These six originals are at the low-level recognizer boundary.
The separate retained comparison below exercises the inheritance
path from scroll configuration to actual touch input.

All 22 focused recognizer, device and scroll configuration tests passed.

## Inherited policy in a retained scroll view

The authored pinned `multitouch_scroll_view_test.dart` puts actual
touches on a ListView under ScrollConfiguration, with clamping
physics and the start-from-down policy. The first movement changes
offset 100 to 140. Two further movements in one frame yield offsets
160 (latest), 190 (sum) and 170 (boundary). The native baseline
returned 190 in all three modes. Physical scroll contacts now use
the same frame accumulator as the low-level recognizer and share
one viewport activity; the original owner can leave without
stopping the other contact. Both frameworks still accept a further
10-pixel update and end only on the final cancellation.

All three authored Flutter and 18 focused native scroll, fling,
keyboard-focus, wheel and nested single-axis control checks passed.
The Flutter probe verifies one start and one end notification, and
the native test verifies the same activity ID through the handoff.
No new original registration is counted. A multi-contact cross-axis
nested fling and OS aggregate trackpad input were not compared here.
