# Force-press recognition and arena settlement

All nine pinned `gestures/force_press_test.dart` registrations ran.
Seven pair with the mounted/native regressions:

- :14 starts once at 2.8/6.66, peaks once at 6.0/6.66, emits ten
  updates (including after a fall in pressure), and ends once at
  zero pressure. The start reports the actual contact point.
- :266 releases a light 2.5/6.66 contact without any force callback.
- :329, :428, and :482 exercise real competing pan and force
  recognizers. A 20×20 move makes the force recognizer withdraw
  before ordinary pan slop and the remaining pan starts immediately.
  Later high pressure cannot reclaim the stream. Lifting while
  contested starts and ends neither recognizer. Pressure 3.0/6.66
  without movement instead starts and updates force once, then ends
  on release; pan stays silent.
- :751 pressure outside the native device range clamps rather than
  throwing. The 8.0/6.66 sample starts, peaks and updates at 1.0.
- :825 a nonfinite sample after peak emits no update and does not
  swallow the eventual release.

The native pan initially stayed pending after force rejected. The
retained arena now gives an eligible sole drag the default win after
the move's explicit claims settle. The winning drag uses the current
move for its start position; an explicit slop requirement still
controls when a default-winning drag starts. Tap, trackpad and basic
force controls passed alongside the exact replay (18 focused cases).

Two signatures remain partial at this point. :200 supplies raw device
range limits 0, 1, -1 and 0.5 and expects the recognizer not to join.
Native devices with no sensing are already excluded, but normalized
reports intentionally do not use a physical maximum of 1 as a
capability flag. :554 passes the original ease-in curve and the
exact callback sequence in native tests, but the user interpolator
receives the normalized range 0–1 rather than the device's raw
0–6.66 bounds. The two different source boundaries are not counted
as paired for those cases.
